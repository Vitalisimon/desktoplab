import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import { join } from "node:path";
import test from "node:test";

import { platformAdapter, validateVisualStep } from "./native-platform-adapters.mjs";
import { assessVisualEvidence, NativeVisualEvidenceDriver } from "./visual-evidence-driver.mjs";

test("platform adapters cover the shared operator contract with native commands", () => {
  for (const platform of ["macos", "linux", "windows"]) {
    const adapter = platformAdapter(platform);
    for (const kind of ["capture", "inspect", "click", "type", "scroll", "hotkey", "menu", "window"]) {
      assert.ok(adapter.requirements[kind] || adapter.alternatives?.[kind], `${platform}:${kind}`);
    }
  }
  assert.throws(() => validateVisualStep({ kind: "click", target: "DesktopLab" }), /click_target_required/);
  const source = readFileSync("scripts/visual-evidence/native-platform-adapters.mjs", "utf8");
  assert.match(source, /macosSystemKeyboardInvocation/);
  assert.match(source, /resolveCommand: compileNativeAccessibilityHelper/);
  assert.doesNotMatch(source, /System Events|entire contents|macosInspectScript/);
});

test("driver records native invocations without retaining typed secrets or local paths", () => {
  const root = mkdtempSync(join(os.tmpdir(), "desktoplab-visual-"));
  const bin = join(root, "bin");
  const environment = { ...process.env, PATH: `${bin}:${process.env.PATH}` };
  const driver = new NativeVisualEvidenceDriver({
    platform: "macos",
    evidenceRoot: join(root, "evidence"),
    environment,
    execute: () => ({ status: 0, stdout: `token=visible ${os.homedir()}/project`, stderr: "" }),
  });
  driver.capabilities = () => ({ platform: "macos", actions: { type: true }, selected: {}, remediation: "grant permissions" });
  const record = driver.run({ kind: "type", target: `${os.homedir()}/DesktopLab`, text: "secret input" });
  assert.equal(record.target, "[HOME]/DesktopLab");
  assert.equal(record.outcome.includes("visible"), false);
  assert.equal(JSON.stringify(record).includes("secret input"), false);
  const manifest = driver.finalize({ appVersion: "0.1.0", appHash: "sha256:app", commit: "abc", targetId: "mac" });
  assert.equal(JSON.stringify(manifest).includes("secret input"), false);
  rmSync(root, { recursive: true, force: true });
});

test("capture evidence hashes real bytes and requires explicit sensitive-content review", () => {
  const root = mkdtempSync(join(os.tmpdir(), "desktoplab-visual-"));
  const driver = new NativeVisualEvidenceDriver({ platform: "macos", evidenceRoot: join(root, "evidence") });
  driver.capabilities = () => ({ platform: "macos", actions: { capture: true }, selected: {}, remediation: "grant permissions" });
  driver.execute = (invocation) => {
    writeFileSync(invocation.args.at(-1), png(1280, 820, 2048));
    return { status: 0, stdout: "", stderr: "" };
  };
  const capture = driver.run({ kind: "capture", target: "DesktopLab", frameReview: "operator_confirmed_clean" });
  assert.equal(capture.frame.width, 1280);
  assert.equal(capture.frame.contentReview, "operator_confirmed_clean");
  const manifest = driver.finalize({ appVersion: "0.1.0", appHash: "sha256:app", commit: "abc", targetId: "mac" });
  assert.equal(assessVisualEvidence(manifest).status, "pass");
  assert.ok(readFileSync(join(root, "evidence", capture.frame.path)).length > 1024);
  rmSync(root, { recursive: true, force: true });
});

test("native inspection output becomes bounded accessibility evidence", () => {
  const root = mkdtempSync(join(os.tmpdir(), "desktoplab-visual-"));
  const driver = new NativeVisualEvidenceDriver({ platform: "windows", evidenceRoot: root });
  driver.capabilities = () => ({ platform: "windows", actions: { inspect: true }, selected: {}, remediation: "interactive session" });
  driver.execute = () => ({ status: 0, stdout: JSON.stringify({ viewport: { width: 1280, height: 820 }, nodes: [{ id: "composer", name: "token=private", bounds: { x: 20, y: 20, width: 400, height: 80 } }] }), stderr: "" });
  const record = driver.run({ kind: "inspect", target: "DesktopLab" });
  assert.equal(record.accessibility.nodes[0].name, "token=[REDACTED]");
  rmSync(root, { recursive: true, force: true });
});

test("operator actions retain before and after frame identity", () => {
  const root = mkdtempSync(join(os.tmpdir(), "desktoplab-visual-"));
  const driver = new NativeVisualEvidenceDriver({ platform: "macos", evidenceRoot: root });
  driver.capabilities = () => ({ platform: "macos", actions: { capture: true, click: true }, selected: {}, remediation: "permissions" });
  let frame = 0;
  driver.execute = (invocation) => {
    if (invocation.command === "screencapture") writeFileSync(invocation.args.at(-1), png(1280, 820, 2048 + frame++));
    return { status: 0, stdout: "", stderr: "" };
  };
  const action = driver.runWithFrames({ kind: "click", target: "DesktopLab", coordinates: { x: 50, y: 60 } }, { frameReview: "operator_confirmed_clean" });
  assert.notEqual(action.beforeSha256, action.afterSha256);
  assert.equal(action.expectChange, true);
  rmSync(root, { recursive: true, force: true });
});

test("assessment blocks clipped, overlapped, stale and unreviewed evidence", () => {
  const manifest = {
    kind: "desktoplab.native-visual-evidence",
    evidenceKind: "installed_app_operator_driver",
    agentToolAvailable: false,
    status: "complete",
    records: [
      { sequence: 1, status: "passed", frame: { byteSize: 2000, width: 1280, height: 820, contentReview: "potentially_sensitive" } },
      { sequence: 2, status: "passed", accessibility: { viewport: { width: 100, height: 100 }, nodes: [
        { id: "a", exclusive: true, bounds: { x: 90, y: 0, width: 20, height: 20 } },
        { id: "b", exclusive: true, bounds: { x: 95, y: 0, width: 5, height: 20 } },
      ] } },
      { sequence: 3, status: "passed", expectChange: true, beforeSha256: "same", afterSha256: "same" },
    ],
  };
  const assessment = assessVisualEvidence(manifest);
  assert.equal(assessment.status, "blocked");
  assert.ok(assessment.failures.some((failure) => failure.startsWith("clipped_node")));
  assert.ok(assessment.failures.some((failure) => failure.startsWith("overlap")));
  assert.ok(assessment.failures.includes("stale_ui:3"));
  assert.ok(assessment.failures.includes("sensitive_frame_not_reviewed:1"));
});

test("missing native capability fails with remediation instead of simulating success", () => {
  const root = mkdtempSync(join(os.tmpdir(), "desktoplab-visual-"));
  const driver = new NativeVisualEvidenceDriver({ platform: "linux", evidenceRoot: root, environment: { PATH: "" } });
  assert.equal(driver.capabilities().actions.click, false);
  assert.throws(() => driver.run({ kind: "click", target: "DesktopLab", coordinates: { x: 2, y: 3 } }), (error) => {
    assert.match(error.message, /visual_capability_unavailable:click/);
    assert.match(error.remediation, /xdotool/);
    return true;
  });
  rmSync(root, { recursive: true, force: true });
});

function png(width, height, payloadSize) {
  const bytes = Buffer.alloc(payloadSize, 1);
  Buffer.from("89504e470d0a1a0a", "hex").copy(bytes, 0);
  bytes.writeUInt32BE(13, 8);
  bytes.write("IHDR", 12, "ascii");
  bytes.writeUInt32BE(width, 16);
  bytes.writeUInt32BE(height, 20);
  return bytes;
}
