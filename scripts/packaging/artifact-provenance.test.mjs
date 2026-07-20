import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { sha256File, verifyArtifactEvidence, writeArtifactEvidence } from "./artifact-provenance-core.mjs";

test("dependency lockfiles keep canonical LF bytes on every build host", () => {
  const attributes = fs.readFileSync(".gitattributes", "utf8");
  for (const lockfile of ["/Cargo.lock", "/package-lock.json", "/apps/desktop/src-tauri/Cargo.lock"]) {
    assert.match(attributes, new RegExp(`^${lockfile.replaceAll("/", "\\/")} text eol=lf$`, "m"));
  }
});

test("current-head app, manifest and checksums agree until the candidate mutates", () => {
  const fixture = provenanceFixture();
  assert.doesNotThrow(() => verify(fixture));
  fs.appendFileSync(path.join(fixture.app, "Contents", "MacOS", "desktoplab"), "mutation");
  assert.throws(() => verify(fixture), /artifact mutated/);
});

test("installed app must be the exact candidate from current HEAD", () => {
  const fixture = provenanceFixture();
  const installed = path.join(fixture.root, "Applications", "DesktopLab.app");
  fs.cpSync(fixture.app, installed, { recursive: true });
  assert.doesNotThrow(() => verify(fixture, installed));
  const metadataPath = path.join(installed, "Contents", "Resources", "DesktopLabBuild.json");
  const metadata = JSON.parse(fs.readFileSync(metadataPath, "utf8"));
  fs.writeFileSync(metadataPath, JSON.stringify({ ...metadata, commitSha: "b".repeat(40) }));
  assert.throws(() => verify(fixture, installed), /installed app commit/);
});

test("legacy installed apps without embedded provenance fail closed", () => {
  const fixture = provenanceFixture();
  const installed = path.join(fixture.root, "Applications", "DesktopLab.app");
  fs.cpSync(fixture.app, installed, { recursive: true });
  fs.rmSync(path.join(installed, "Contents", "Resources", "DesktopLabBuild.json"));

  assert.throws(() => verify(fixture, installed), /app build metadata is missing/);
});

function provenanceFixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "desktoplab-provenance-"));
  const evidenceDir = path.join(root, "evidence");
  const app = path.join(root, "candidate", "DesktopLab.app");
  const lockPath = path.join(root, "package-lock.json");
  fs.mkdirSync(path.join(app, "Contents", "MacOS"), { recursive: true });
  fs.mkdirSync(path.join(app, "Contents", "Resources"), { recursive: true });
  fs.writeFileSync(path.join(app, "Contents", "MacOS", "desktoplab"), "binary");
  fs.writeFileSync(lockPath, "locked");
  const build = {
    version: "0.1.0", commitSha: "a".repeat(40), channel: "dev", treeState: "clean",
    architecture: "arm64", runner: "test:darwin-arm64", workflow: "test",
    lockfiles: [{ path: "package-lock.json", sha256: sha256File(lockPath) }],
  };
  fs.writeFileSync(path.join(app, "Contents", "Resources", "DesktopLabBuild.json"), JSON.stringify(build));
  const manifest = writeArtifactEvidence({ root, evidenceDir, artifactPaths: [app], build, signatureStateFor: () => "adhoc_dev" });
  assert.equal(manifest.entries[0].signatureState, "adhoc_dev");
  return { root, evidenceDir, app, build };
}

function verify(fixture, installedAppPath = null) {
  return verifyArtifactEvidence({
    root: fixture.root, evidenceDir: fixture.evidenceDir, currentHead: fixture.build.commitSha,
    currentTreeState: "clean", installedAppPath,
  });
}
