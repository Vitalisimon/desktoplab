import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { chmodSync, existsSync, mkdtempSync, readFileSync, realpathSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { runMacosSystemKeyboardEvent, systemKeyboardEventsModulePath } from "./macos-system-keyboard-events.mjs";

export const nativeAccessibilityModulePath = fileURLToPath(import.meta.url);
export const nativeAccessibilitySourcePath = fileURLToPath(new URL("./macos-native-accessibility.swift", import.meta.url));
const repoRoot = resolve(dirname(nativeAccessibilityModulePath), "../../..");
let helperDirectory = null;
let helperPath = null;

export function compileNativeAccessibilityHelper() {
  if (helperPath && existsSync(helperPath)) return helperPath;
  helperDirectory = mkdtempSync(join(tmpdir(), "desktoplab-native-accessibility-"));
  helperPath = join(helperDirectory, "desktoplab-native-accessibility");
  execFileSync("xcrun", ["swiftc", nativeAccessibilitySourcePath, "-framework", "AppKit", "-framework", "ApplicationServices", "-o", helperPath], { stdio: "pipe" });
  chmodSync(helperPath, 0o700);
  return helperPath;
}

export function nativeAccessibilityCommand(command, value = "") {
  return execFileSync(compileNativeAccessibilityHelper(), [command], {
    input: value,
    encoding: "utf8",
    timeout: 30_000,
    stdio: ["pipe", "pipe", "pipe"],
  }).trim();
}

export const macosAccessibilityUi = {
  trusted: () => nativeAccessibilityCommand("trusted") === "true",
  ready: () => nativeAccessibilityCommand("ready") === "true",
  activate: () => nativeAccessibilityCommand("activate"),
  hasButton: (name) => nativeAccessibilityCommand("button-exists", name) === "true",
  buttonEnabled: (name) => nativeAccessibilityCommand("button-enabled", name) === "true",
  clickButton: (name) => nativeAccessibilityCommand("click-button", name),
  setPrompt: (prompt) => nativeAccessibilityCommand("set-prompt", prompt),
  send: (method) => {
    if (method !== "keyboard") return nativeAccessibilityCommand("click-button", "Send prompt");
    nativeAccessibilityCommand("focus-prompt");
    return runMacosSystemKeyboardEvent(["enter"]);
  },
  openProject: (path) => nativeAccessibilityCommand("open-project", path),
  diagnostics: () => JSON.parse(nativeAccessibilityCommand("diagnostics")),
  capture: (path) => execFileSync("screencapture", ["-x", "-R", nativeAccessibilityCommand("window-bounds"), path]),
  quit: () => nativeAccessibilityCommand("quit"),
};

export function macosAccessibilityDriverEvidence(driverPath, extraDependencies = []) {
  const driver = realpathSync(driverPath);
  const dependencyPaths = [nativeAccessibilityModulePath, nativeAccessibilitySourcePath, systemKeyboardEventsModulePath, ...extraDependencies].map((path) => realpathSync(path));
  const dependencies = [...new Set(dependencyPaths)].map(sourceRecord);
  const sources = [sourceRecord(driver), ...dependencies];
  return {
    ...sources[0],
    technology: "macos_native_accessibility",
    keyboardTechnology: "macos_system_keyboard_events",
    dependencies,
    bundleSha256: digest(sources.map((entry) => `${relative(repoRoot, entry.path)}\0${entry.sha256}`).join("\0")),
  };
}

function sourceRecord(path) { return { path, sha256: digest(readFileSync(path)) }; }
function digest(value) { return `sha256:${createHash("sha256").update(value).digest("hex")}`; }

process.once("exit", () => {
  if (helperDirectory) rmSync(helperDirectory, { recursive: true, force: true });
});
