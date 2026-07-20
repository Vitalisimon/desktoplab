import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import { join } from "node:path";
import test from "node:test";

import { EvidenceBundle, createSourceSnapshot, syncSourceSnapshot, verifyEvidenceBundle } from "./remote-sync.mjs";

const target = { id: "linux", platform: "linux", architecture: "x64", trustLevel: "trusted_physical" };

test("source snapshot includes only an exact clean tracked HEAD", () => {
  const fixture = gitFixture();
  try {
    const snapshot = createSourceSnapshot(fixture);
    assert.equal(snapshot.treeState, "clean");
    assert.match(snapshot.archiveSha256, /^[a-f0-9]{64}$/);
    writeFileSync(join(fixture, "untracked.txt"), "dirty");
    assert.throws(() => createSourceSnapshot(fixture), /clean tracked source tree/);
  } finally {
    rmSync(fixture, { recursive: true, force: true });
  }
});

test("remote sync verifies target archive hash and reports mismatch as incomplete", () => {
  const snapshot = { commit: "a".repeat(40), treeState: "clean", archive: Buffer.from("tar"), archiveSha256: "b".repeat(64), archiveBytes: 3 };
  const upload = () => ({ status: 0 });
  const run = (_target, command) => ({ status: 0, stdout: command.includes("sha256sum") ? `DESKTOPLAB_ARCHIVE_SHA256=${snapshot.archiveSha256}\n` : "" });
  const passed = syncSourceSnapshot(snapshot, target, "run-pass", { upload, run });
  assert.equal(passed.status, "complete");
  const mismatch = syncSourceSnapshot(snapshot, target, "run-mismatch", { upload, run: (_target, command) => ({ status: 0, stdout: command.includes("sha256sum") ? "DESKTOPLAB_ARCHIVE_SHA256=wrong\n" : "" }) });
  assert.equal(mismatch.state, "source_hash_mismatch");
  assert.equal(mismatch.status, "incomplete");
});

test("evidence collection resumes idempotently and verifies every artifact", () => {
  const fixture = mkdtempSync(join(os.tmpdir(), "desktoplab-evidence-test-"));
  const root = join(fixture, "bundle");
  try {
    const identity = { runId: "run-1", source: { commit: "a".repeat(40) }, target, startedAt: "2026-07-15T10:00:00Z" };
    const bundle = EvidenceBundle.begin(root, identity);
    bundle.addArtifact("test.log", Buffer.from(`password=private\n${os.homedir()}/repo\n`));
    const resumed = EvidenceBundle.resume(root);
    resumed.addArtifact("test.log", Buffer.from(`password=private\n${os.homedir()}/repo\n`));
    resumed.addArtifact("results.xml", Buffer.from("<testsuite/>"), "junit");
    resumed.finalize("2026-07-15T10:01:00Z");
    const verification = verifyEvidenceBundle(root);
    assert.equal(verification.valid, true);
    assert.match(readFileSync(join(root, "artifacts/test.log"), "utf8"), /\[REDACTED\]/);
    assert.doesNotMatch(readFileSync(join(root, "artifacts/test.log"), "utf8"), new RegExp(os.homedir()));
  } finally {
    rmSync(fixture, { recursive: true, force: true });
  }
});

test("tampered and incomplete bundles fail verification", () => {
  const fixture = mkdtempSync(join(os.tmpdir(), "desktoplab-evidence-test-"));
  const root = join(fixture, "bundle");
  try {
    const bundle = EvidenceBundle.begin(root, { runId: "run", startedAt: "now", source: {}, target });
    bundle.addArtifact("run.log", Buffer.from("ok"));
    assert.deepEqual(verifyEvidenceBundle(root).failures, ["bundle_incomplete"]);
    bundle.finalize("later");
    writeFileSync(join(root, "artifacts/run.log"), "tampered");
    assert.deepEqual(verifyEvidenceBundle(root).failures, ["hash_or_size_mismatch:run.log"]);
  } finally {
    rmSync(fixture, { recursive: true, force: true });
  }
});

test("remote sync source stays bounded", () => {
  const source = readFileSync(new URL("./remote-sync.mjs", import.meta.url), "utf8");
  const logical = source.split("\n").filter((line) => line.trim() && !line.trim().startsWith("//")).length;
  assert.ok(logical <= 300, `remote sync has ${logical} logical lines`);
});

function gitFixture() {
  const root = mkdtempSync(join(os.tmpdir(), "desktoplab-source-test-"));
  command(root, ["init", "-q"]);
  command(root, ["config", "user.name", "DesktopLab Test"]);
  command(root, ["config", "user.email", "desktoplab@example.invalid"]);
  writeFileSync(join(root, "tracked.txt"), "tracked\n");
  command(root, ["add", "tracked.txt"]);
  command(root, ["commit", "-qm", "fixture"]);
  return root;
}

function command(root, args) {
  const result = spawnSync("git", args, { cwd: root, encoding: "utf8", shell: false });
  assert.equal(result.status, 0, result.stderr);
}
