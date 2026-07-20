import { accessSync, constants } from "node:fs";
import { delimiter, join } from "node:path";

import { compileNativeAccessibilityHelper } from "../product/drivers/macos-native-accessibility.mjs";
import { macosSystemKeyboardInvocation } from "../product/drivers/macos-system-keyboard-events.mjs";

const ACTIONS = new Set(["capture", "inspect", "click", "type", "scroll", "hotkey", "menu", "window"]);

export function platformAdapter(platform) {
  if (platform === "macos") return macosAdapter;
  if (platform === "linux") return linuxAdapter;
  if (platform === "windows") return windowsAdapter;
  throw new Error(`unsupported_visual_platform:${platform}`);
}

export function currentVisualPlatform(value = process.platform) {
  if (value === "darwin") return "macos";
  if (value === "win32") return "windows";
  if (value === "linux") return "linux";
  throw new Error(`unsupported_visual_platform:${value}`);
}

export function validateVisualStep(step) {
  if (!ACTIONS.has(step?.kind)) throw new Error("unsupported_visual_action");
  if (!step.target || typeof step.target !== "string") throw new Error("visual_target_required");
  if (step.kind === "click" && !validPoint(step.coordinates) && !step.selector) throw new Error("click_target_required");
  if (step.kind === "type" && typeof step.text !== "string") throw new Error("type_text_required");
  if (step.kind === "scroll" && !Number.isFinite(step.deltaY)) throw new Error("scroll_delta_required");
  if (step.kind === "hotkey" && (!Array.isArray(step.keys) || step.keys.length === 0)) throw new Error("hotkey_keys_required");
  if (step.kind === "menu" && (!Array.isArray(step.menuPath) || step.menuPath.length === 0)) throw new Error("menu_path_required");
  return step;
}

export function executableAvailable(executable, environment = process.env) {
  const path = environment.PATH ?? "";
  const extensions = process.platform === "win32" ? ["", ".exe", ".cmd"] : [""];
  return path.split(delimiter).some((directory) => extensions.some((extension) => {
    try {
      accessSync(join(directory, `${executable}${extension}`), constants.X_OK);
      return true;
    } catch {
      return false;
    }
  }));
}

const macosAdapter = {
  platform: "macos",
  requirements: {
    capture: ["screencapture"],
    inspect: ["xcrun"],
    click: ["xcrun"],
    type: ["xcrun"],
    scroll: ["xcrun"],
    hotkey: ["osascript"],
    menu: ["xcrun"],
    window: ["xcrun"],
  },
  remediation: "Grant Screen Recording for capture and Accessibility for inspection and input in System Settings > Privacy & Security.",
  invocation(step, outputPath) {
    if (step.kind === "capture") return { command: "screencapture", args: ["-x", "-t", "png", outputPath] };
    if (step.kind === "hotkey") return macosSystemKeyboardInvocation(step.keys);
    if (step.kind === "inspect") return macosNativeInvocation("visual-inspect", "");
    if (step.kind === "click") return macosNativeInvocation(step.coordinates ? "visual-click-point" : "visual-click-named", step.coordinates ? `${step.coordinates.x},${step.coordinates.y}` : step.selector);
    if (step.kind === "type") return macosNativeInvocation("visual-type", step.text);
    if (step.kind === "scroll") return macosNativeInvocation("visual-scroll", String(step.deltaY));
    if (step.kind === "menu") return macosNativeInvocation("visual-menu", step.menuPath.at(-1));
    if (step.kind === "window") return macosNativeInvocation("visual-window", "");
    throw new Error("unsupported_visual_action");
  },
};

function macosNativeInvocation(action, stdin) {
  return { command: "desktoplab-native-accessibility", resolveCommand: compileNativeAccessibilityHelper, args: [action], stdin };
}

const linuxAdapter = {
  platform: "linux",
  alternatives: { capture: [["grim"], ["gnome-screenshot"], ["scrot"]] },
  requirements: {
    inspect: ["python3"],
    click: ["xdotool"],
    type: ["xdotool"],
    scroll: ["xdotool"],
    hotkey: ["xdotool"],
    menu: ["python3"],
    window: ["xdotool"],
  },
  remediation: "Run a graphical session, install grim/gnome-screenshot/scrot and xdotool, and expose AT-SPI through python3-pyatspi.",
  invocation(step, outputPath, capabilities) {
    if (step.kind === "capture") {
      const tool = capabilities.selected.capture;
      if (tool === "grim") return { command: "grim", args: [outputPath] };
      if (tool === "gnome-screenshot") return { command: "gnome-screenshot", args: ["-f", outputPath] };
      return { command: "scrot", args: [outputPath] };
    }
    if (step.kind === "inspect") return { command: "python3", args: ["-c", linuxInspectScript] };
    return linuxInputInvocation(step);
  },
};

const windowsAdapter = {
  platform: "windows",
  requirements: Object.fromEntries([...ACTIONS].map((action) => [action, ["powershell.exe"]])),
  remediation: "Run DesktopLab visual certification in an interactive desktop session and allow UI Automation and screen capture.",
  invocation(step, outputPath) {
    const source = windowsScript(step, outputPath);
    return { command: "powershell.exe", args: ["-NoLogo", "-NoProfile", "-NonInteractive", "-EncodedCommand", Buffer.from(source, "utf16le").toString("base64")] };
  },
};

function linuxInputInvocation(step) {
  if (step.kind === "click") return { command: "xdotool", args: ["mousemove", String(step.coordinates.x), String(step.coordinates.y), "click", "1"] };
  if (step.kind === "type") return { command: "xdotool", args: ["type", "--clearmodifiers", "--", step.text] };
  if (step.kind === "scroll") return { command: "xdotool", args: ["click", step.deltaY < 0 ? "5" : "4"] };
  if (step.kind === "hotkey") return { command: "xdotool", args: ["key", step.keys.join("+")] };
  if (step.kind === "menu") return { command: "python3", args: ["-c", linuxMenuScript, step.menuPath.join("\u001f")] };
  if (step.kind === "window") return { command: "xdotool", args: ["getactivewindow", "windowactivate"] };
  throw new Error("unsupported_visual_action");
}

function windowsScript(step, outputPath) {
  if (step.kind === "capture") return `Add-Type -AssemblyName System.Windows.Forms; Add-Type -AssemblyName System.Drawing; $b=[System.Windows.Forms.Screen]::PrimaryScreen.Bounds; $i=New-Object System.Drawing.Bitmap $b.Width,$b.Height; $g=[System.Drawing.Graphics]::FromImage($i); $g.CopyFromScreen($b.Location,[System.Drawing.Point]::Empty,$b.Size); $i.Save('${escapePowerShell(outputPath)}',[System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $i.Dispose()`;
  if (step.kind === "inspect") return "$ErrorActionPreference='Stop'; Add-Type -AssemblyName UIAutomationClient; Add-Type -AssemblyName System.Windows.Forms; $e=[System.Windows.Automation.AutomationElement]::FocusedElement; $b=[System.Windows.Forms.Screen]::PrimaryScreen.Bounds; $nodes=@(); if($e){$r=$e.Current.BoundingRectangle; $nodes+=@{id='focused';name=$e.Current.Name;bounds=@{x=$r.X;y=$r.Y;width=$r.Width;height=$r.Height};exclusive=$false}}; @{viewport=@{width=$b.Width;height=$b.Height};nodes=$nodes}|ConvertTo-Json -Depth 6 -Compress";
  if (step.kind === "type") return `Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait('${escapeSendKeys(step.text)}')`;
  if (step.kind === "hotkey") return `Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait('${windowsSendKeys(step.keys)}')`;
  const point = step.coordinates ?? { x: 0, y: 0 };
  if (step.kind === "click" && step.coordinates) return `$sig='[DllImport("user32.dll")] public static extern bool SetCursorPos(int X,int Y); [DllImport("user32.dll")] public static extern void mouse_event(uint f,uint dx,uint dy,uint d,UIntPtr e);'; Add-Type -MemberDefinition $sig -Name Native -Namespace DesktopLab; [DesktopLab.Native]::SetCursorPos(${point.x},${point.y}); [DesktopLab.Native]::mouse_event(2,0,0,0,[UIntPtr]::Zero); [DesktopLab.Native]::mouse_event(4,0,0,0,[UIntPtr]::Zero)`;
  if (step.kind === "click") return windowsInvokeByName(step.selector);
  if (step.kind === "scroll") return `$sig='[DllImport("user32.dll")] public static extern void mouse_event(uint f,uint dx,uint dy,int d,UIntPtr e);'; Add-Type -MemberDefinition $sig -Name Native -Namespace DesktopLab; [DesktopLab.Native]::mouse_event(2048,0,0,${Math.trunc(step.deltaY)},[UIntPtr]::Zero)`;
  if (step.kind === "menu") return windowsInvokeByName(step.menuPath.at(-1));
  if (step.kind === "window") return "Add-Type -AssemblyName UIAutomationClient; $e=[System.Windows.Automation.AutomationElement]::FocusedElement; if(-not $e){exit 3}";
  throw new Error("unsupported_visual_action");
}

const linuxInspectScript = String.raw`import pyatspi, json
d=pyatspi.Registry.getDesktop(0)
nodes=[]
for i in range(d.childCount):
 a=d.getChildAtIndex(i)
 for j in range(a.childCount):
  w=a.getChildAtIndex(j)
  try:
   x,y,width,height=w.queryComponent().getExtents(pyatspi.DESKTOP_COORDS)
   nodes.append({'id':str(i)+'-'+str(j),'name':w.name,'bounds':{'x':x,'y':y,'width':width,'height':height},'exclusive':False})
  except Exception: pass
print(json.dumps({'viewport':{'width':pyatspi.Registry.getDesktop(0).queryComponent().getExtents(pyatspi.DESKTOP_COORDS).width,'height':pyatspi.Registry.getDesktop(0).queryComponent().getExtents(pyatspi.DESKTOP_COORDS).height},'nodes':nodes}))`;

const linuxMenuScript = String.raw`import pyatspi, sys
parts=sys.argv[1].split('\x1f')
d=pyatspi.Registry.getDesktop(0)
def walk(node):
 yield node
 for i in range(node.childCount):
  yield from walk(node.getChildAtIndex(i))
matches=[node for node in walk(d) if node.name==parts[-1]]
if not matches: raise SystemExit(3)
actions=matches[0].queryAction()
if actions.nActions < 1 or not actions.doAction(0): raise SystemExit(4)`;

function windowsInvokeByName(name) {
  const escaped = escapePowerShell(name);
  return `$ErrorActionPreference='Stop'; Add-Type -AssemblyName UIAutomationClient; $root=[System.Windows.Automation.AutomationElement]::RootElement; $c=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty,'${escaped}'); $e=$root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$c); if(-not $e){exit 3}; $p=$null; if(-not $e.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern,[ref]$p)){exit 4}; ([System.Windows.Automation.InvokePattern]$p).Invoke()`;
}

function validPoint(value) {
  return Number.isFinite(value?.x) && Number.isFinite(value?.y) && value.x >= 0 && value.y >= 0;
}

function escapePowerShell(value = "") {
  return String(value).replaceAll("'", "''");
}

function escapeSendKeys(value = "") {
  return String(value).replace(/[+^%~(){}\[\]]/g, "{$&}").replaceAll("'", "''");
}

function windowsSendKeys(keys) {
  const modifiers = { control: "^", shift: "+", alt: "%", command: "^" };
  return `${keys.slice(0, -1).map((key) => modifiers[key] ?? "").join("")}${escapeSendKeys(keys.at(-1))}`;
}
