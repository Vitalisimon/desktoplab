import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { hashArtifact } from "./artifact-provenance-core.mjs";
import { verifyMacosCandidateInstall } from "./verify-macos-candidate-install.mjs";

test("exact copied candidate and embedded build metadata pass", () => {
  const fixture = appFixture();
  const result = verifyMacosCandidateInstall(fixture);
  assert.equal(result.status, "pass", result.failures.join("\n"));
  assert.equal(result.sourceHash, result.installedHash);
});

test("copy drift and stale embedded source fail closed", () => {
  const fixture = appFixture();
  writeFileSync(join(fixture.installedApp, "Contents", "Resources", "drift.txt"), "drift");
  const drift = verifyMacosCandidateInstall(fixture);
  assert.equal(drift.status, "fail");
  assert.ok(drift.failures.includes("installed app bytes differ from prepared app"));

  const stale = appFixture();
  stale.candidate.source.commit = "f".repeat(40);
  const staleResult = verifyMacosCandidateInstall(stale);
  assert.ok(staleResult.failures.includes("installed app commit differs from candidate source"));
});

test("installer verifies before and after atomic replacement with rollback", () => {
  const source = readFileSync("scripts/packaging/install-macos-candidate.sh", "utf8");
  assert.match(source, /candidate-admission\.mjs verify/);
  assert.equal((source.match(/verify-macos-candidate-install\.mjs/g) ?? []).length, 3);
  assert.match(source, /mv "\$target" "\$backup"/);
  assert.match(source, /mv "\$backup" "\$target"/);
  assert.match(source, /single-install verification failed/);
  assert.doesNotMatch(source, /kill -9|pkill -9/);
});

function appFixture() {
  const root = mkdtempSync(join(tmpdir(), "desktoplab-install-verify-"));
  const sourceApp = join(root, "source", "DesktopLab.app");
  const installedApp = join(root, "Applications", "DesktopLab.app");
  const commit = "a".repeat(40);
  for (const app of [sourceApp, installedApp]) {
    mkdirSync(join(app, "Contents", "Resources"), { recursive: true });
    writeFileSync(join(app, "Contents", "MacOS.bin"), "binary");
    writeFileSync(join(app, "Contents", "Resources", "DesktopLabBuild.json"), JSON.stringify({
      kind: "desktoplab.embedded-build",
      schemaVersion: 1,
      commitSha: commit,
      channel: "beta",
      architecture: "arm64",
      lockfiles: [{ path: "Cargo.lock", sha256: "b".repeat(64) }],
    }));
  }
  const payload = hashArtifact(sourceApp).sha256;
  return {
    sourceApp,
    installedApp,
    candidate: {
      kind: "desktoplab.release-candidate",
      schemaVersion: 1,
      candidateId: `sha256:${createHash("sha256").update("candidate").digest("hex")}`,
      state: "payload_built",
      source: { commit },
      payload: { sha256: payload },
    },
  };
}
