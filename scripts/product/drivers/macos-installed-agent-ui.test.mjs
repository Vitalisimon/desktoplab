import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import test from "node:test";

import { driverPlan, parseArgs } from "./macos-installed-agent-ui.mjs";
import { compileNativeAccessibilityHelper, macosAccessibilityDriverEvidence, nativeAccessibilityModulePath, nativeAccessibilitySourcePath } from "./macos-native-accessibility.mjs";
import { systemKeyboardEventsModulePath } from "./macos-system-keyboard-events.mjs";
import { versionedUiDriverFailures } from "../versioned-ui-driver-evidence.mjs";

test("driver plan is explicitly non-certifying and covers the canonical five cases", () => {
  const plan = driverPlan();
  assert.equal(plan.certifying, false);
  assert.deepEqual(plan.cases.map((entry) => entry.caseId), ["inspect", "create", "patch", "test_repair", "diff"]);
});

test("driver requires explicit candidate app workspace and evidence paths", () => {
  const args = parseArgs(["--app", "/Applications/DesktopLab.app", "--workspace", "/tmp/work", "--evidence", "/tmp/evidence.json", "--candidate", "/tmp/candidate.json"]);
  assert.equal(args.app, "/Applications/DesktopLab.app");
  assert.throws(() => parseArgs(["--shortcut"]), /unknown argument/);
});

test("driver uses Accessibility and persisted state without test-control or API shortcuts", () => {
  const source = readFileSync("scripts/product/drivers/macos-installed-agent-ui.mjs", "utf8");
  assert.match(source, /macosAccessibilityDriverEvidence/);
  assert.match(source, /productization_state/);
  assert.doesNotMatch(source, /\/v1\/test\/|unsafe-no-auth|promptEntered:\s*true|sendClicked:\s*true|approvalClicked:\s*true/);
  assert.doesNotMatch(source, /osascript|System Events/);
  const nativeModule = readFileSync(nativeAccessibilityModulePath, "utf8");
  const nativeSource = readFileSync(nativeAccessibilitySourcePath, "utf8");
  const keyboardSource = readFileSync(systemKeyboardEventsModulePath, "utf8");
  assert.match(nativeModule, /input: value/);
  assert.match(nativeModule, /screencapture/);
  assert.match(nativeSource, /AXUIElementCopyAttributeValue/);
  assert.match(nativeSource, /postToPid/);
  assert.match(nativeSource, /Repository path/);
  assert.match(nativeSource, /Open Repository/);
  assert.match(nativeSource, /kAXEnabledAttribute/);
  assert.match(nativeSource, /case "focus-prompt"/);
  assert.match(nativeSource, /Accessibility text input did not reach/);
  assert.match(nativeSource, /for character in value/);
  assert.doesNotMatch(nativeSource, /usleep/);
  assert.match(keyboardSource, /System Events/);
  assert.match(nativeModule, /nativeAccessibilityCommand\("focus-prompt"\)[\s\S]*runMacosSystemKeyboardEvent/);
  assert.doesNotMatch(keyboardSource, /entire contents|applicationProcesses|AXUIElement/);
  assert.doesNotMatch(nativeModule, /\[command, value\]/);
});

test("driver source stays within the approved focused guardrail", () => {
  const logical = readFileSync("scripts/product/drivers/macos-installed-agent-ui.mjs", "utf8").split("\n").filter((line) => line.trim()).length;
  assert.ok(logical <= 380, `macOS installed UI driver has ${logical} logical lines, limit 380`);
});

test("driver provenance binds the native wrapper and compiled helper source", () => {
  const evidence = macosAccessibilityDriverEvidence("scripts/product/drivers/macos-installed-agent-ui.mjs");
  assert.equal(evidence.dependencies.length, 3);
  assert.equal(evidence.keyboardTechnology, "macos_system_keyboard_events");
  assert.deepEqual(versionedUiDriverFailures(evidence, process.cwd()), []);
  evidence.dependencies[0].sha256 = `sha256:${"f".repeat(64)}`;
  assert.match(versionedUiDriverFailures(evidence, process.cwd()).join("\n"), /source hash mismatch|bundle hash mismatch/);
});

test("native Accessibility automation compiles and reports process trust on macOS", { skip: process.platform !== "darwin" }, () => {
  const helper = compileNativeAccessibilityHelper();
  const result = spawnSync(helper, ["trusted"], { input: "", encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.equal(result.stdout.trim(), "true");
});
