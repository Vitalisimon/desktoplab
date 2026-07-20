import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { buildWindowsHostEvidence } from "./windows-host-evidence-core.mjs";

function fixture(overrides = {}) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "desktoplab-windows-evidence-"));
  const smokeLog = path.join(root, "windows-install-smoke.log");
  fs.writeFileSync(smokeLog, `${JSON.stringify({
    platform: "windows-x64",
    artifact: "DesktopLab_0.1.0_x64-setup.exe",
    signatureState: "valid",
    installState: "passed",
    launchState: "passed",
    localApiState: "passed",
    setupState: "auth_required",
    cleanupState: "passed",
  })}\n`);
  return {
    commit: "abc",
    host: { hostname: "windows-host" },
    smokeLog,
    manifest: {
      build: { commitSha: "abc", treeState: "clean", runner: "physical:windows", signingTrustMode: "test" },
      entries: [{
        relativePath: "apps\\desktop\\bundle\\DesktopLab_0.1.0_x64-setup.exe",
        fileName: "DesktopLab_0.1.0_x64-setup.exe",
        target: "windows-x64",
        sha256: "a".repeat(64),
        sizeBytes: 42,
        signatureState: "signed",
      }],
    },
    ...overrides,
  };
}

test("Windows host evidence binds a signed passing smoke to one clean commit", () => {
  const evidence = buildWindowsHostEvidence(fixture());
  assert.equal(evidence.status, "pass");
  assert.equal(evidence.publicTrust, false);
  assert.equal(evidence.artifact.signatureState, "signed");
  assert.equal(evidence.smoke.signatureState, "valid");
});

test("Windows host evidence rejects stale or unsigned manifests", () => {
  const stale = fixture();
  stale.manifest.build.commitSha = "old";
  assert.throws(() => buildWindowsHostEvidence(stale), /current-head/);

  const unsigned = fixture();
  unsigned.manifest.entries[0].signatureState = "unsigned_dev";
  assert.throws(() => buildWindowsHostEvidence(unsigned), /signed artifact/);
});

test("Windows host evidence rejects a non-physical runner", () => {
  const simulated = fixture();
  simulated.manifest.build.runner = "ci:windows";
  assert.throws(() => buildWindowsHostEvidence(simulated), /physical Windows runner/);
});

test("Windows host certification sources stay reviewable", () => {
  for (const [file, maximum] of [
    ["scripts/packaging/windows-host-evidence-core.mjs", 100],
    ["scripts/packaging/windows-host-evidence.mjs", 70],
    ["scripts/packaging/git-content-clean.mjs", 50],
    ["scripts/packaging/windows-host-certify.ps1", 70],
  ]) {
    assert.ok(fs.readFileSync(file, "utf8").split(/\r?\n/).length <= maximum, `${file} exceeds ${maximum} lines`);
  }
});
