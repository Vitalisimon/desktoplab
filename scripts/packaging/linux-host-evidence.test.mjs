import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { buildLinuxHostEvidence } from "./linux-host-evidence-core.mjs";

test("Linux host evidence binds all passing package smokes to one clean commit", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "desktoplab-linux-evidence-"));
  const logs = {};
  const entries = [];
  for (const [format, artifact] of [["appimage", "bundle/DesktopLab.AppImage"], ["deb", "bundle/DesktopLab.deb"], ["rpm", "bundle/DesktopLab.rpm"]]) {
    logs[format] = path.join(root, `${format}.log`);
    fs.writeFileSync(logs[format], `${JSON.stringify({ platform: "linux-x64", artifact, installState: "passed", launchState: "passed", localApiState: "passed", setupState: "auth_required", cleanupState: "passed" })}\n`);
    entries.push({ relativePath: artifact, sha256: format.repeat(64).slice(0, 64), sizeBytes: 10, signatureState: "unsigned_dev" });
  }
  const evidence = buildLinuxHostEvidence({ manifest: { build: { commitSha: "abc", treeState: "clean" }, entries }, smokeLogs: logs, host: {}, commit: "abc" });
  assert.equal(evidence.status, "pass");
  assert.deepEqual(Object.keys(evidence.packages), ["appimage", "deb", "rpm"]);
  assert.equal(evidence.publicTrust, false);
});

test("Linux host evidence rejects a stale manifest", () => {
  assert.throws(() => buildLinuxHostEvidence({ manifest: { build: { commitSha: "old", treeState: "clean" } }, smokeLogs: {}, host: {}, commit: "new" }), /current-head/);
});
