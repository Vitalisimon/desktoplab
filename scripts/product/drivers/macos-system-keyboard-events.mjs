import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

export const systemKeyboardEventsModulePath = fileURLToPath(import.meta.url);

const keyCodes = new Map([
  ["enter", 36],
  ["return", 36],
  ["escape", 53],
  ["tab", 48],
  ["space", 49],
]);
const modifierNames = new Map([
  ["command", "command down"],
  ["shift", "shift down"],
  ["option", "option down"],
  ["control", "control down"],
]);

export function macosSystemKeyboardInvocation(keys) {
  if (!Array.isArray(keys) || keys.length === 0) throw new Error("macOS keyboard event requires keys");
  const key = String(keys.at(-1)).toLowerCase();
  const modifiers = keys.slice(0, -1).map((value) => {
    const modifier = modifierNames.get(String(value).toLowerCase());
    if (!modifier) throw new Error(`unsupported macOS keyboard modifier: ${value}`);
    return modifier;
  });
  const using = modifiers.length > 0 ? ` using {${modifiers.join(",")}}` : "";
  const code = keyCodes.get(key);
  const action = code === undefined ? `keystroke item 1 of argv${using}` : `key code ${code}${using}`;
  const script = `on run argv\ntell application "System Events" to ${action}\nend run`;
  return { command: "osascript", args: ["-e", script, ...(code === undefined ? [key] : [])] };
}

export function runMacosSystemKeyboardEvent(keys) {
  const invocation = macosSystemKeyboardInvocation(keys);
  execFileSync(invocation.command, invocation.args, { stdio: "pipe", timeout: 30_000 });
}
