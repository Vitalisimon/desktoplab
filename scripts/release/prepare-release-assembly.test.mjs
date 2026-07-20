import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import test from "node:test";

test("prepare command emits a verified draft allowlist from an existing tag", (context) => {
  const root = mkdtempSync(join(tmpdir(), "desktoplab-release-assembly-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  write(root, ".gitignore", "dist/\n");
  git(root, ["init"]);
  git(root, ["config", "user.email", "release-test@desktoplab.invalid"]);
  git(root, ["config", "user.name", "DesktopLab Release Test"]);
  git(root, ["add", ".gitignore"]);
  git(root, ["commit", "-m", "fixture"]);
  git(root, ["tag", "-a", "v0.1.0-beta.1", "-m", "candidate"]);
  const head = git(root, ["rev-parse", "HEAD"]);

  const candidatePath = join(root, "dist", "release-candidate.json");
  const identity = {
    repository: "github.com/vitalisimon/desktoplab",
    commit: head,
    version: "0.1.0",
    channel: "beta",
    lockfiles: [],
  };
  writeJson(candidatePath, {
    kind: "desktoplab.release-candidate",
    schemaVersion: 1,
    candidateId: `sha256:${sha256(Buffer.from(JSON.stringify(identity)))}`,
    state: "cross_platform_pass",
    source: { repository: identity.repository, commit: head, treeState: "clean" },
    release: { version: identity.version, channel: identity.channel },
    lockfiles: [],
    payload: { sha256: "c".repeat(64) },
    transitions: [],
  });

  const artifact = Buffer.from("notarized candidate");
  const evidenceDir = join(root, "dist", "candidate");
  mkdirSync(evidenceDir, { recursive: true });
  writeFileSync(join(evidenceDir, "DesktopLab.dmg"), artifact);
  writeJson(join(evidenceDir, "artifact-manifest.json"), {
    kind: "desktoplab.artifact-provenance", schemaVersion: 2,
    build: { commitSha: head, treeState: "clean", channel: "beta", version: "0.1.0" },
    entries: [{ kind: "distribution_file", fileName: "DesktopLab.dmg", target: "macos-aarch64", sha256: sha256(artifact), sizeBytes: artifact.length, signatureState: "notarized" }],
  });
  const sbom = join(root, "dist", "sbom.json");
  writeJson(sbom, { bomFormat: "CycloneDX", specVersion: "1.5", metadata: { properties: [{ name: "desktoplab:sourceCommit", value: head }] } });
  const updater = join(root, "dist", "updater.json");
  writeJson(updater, { kind: "desktoplab.updater-disabled-proof", status: "passed", head, delivery: "disabled", hostedManifest: false, installPolicy: "manual-replacement" });
  const output = join(root, "dist", "assembly");
  const script = resolve("scripts/release/prepare-release-assembly.mjs");
  const result = spawnSync(process.execPath, [script, "--release-ref", "refs/tags/v0.1.0-beta.1", "--channel", "beta", "--evidence-dir", evidenceDir, "--sbom", sbom, "--updater-proof", updater, "--candidate", candidatePath, "--output-dir", output], { cwd: root, encoding: "utf8" });

  assert.equal(result.status, 0, result.stderr);
  assert.equal(JSON.parse(readFileSync(join(output, "release-manifest.json"), "utf8")).status, "draft-ready");
  const files = readFileSync(join(output, "release-files.txt"), "utf8");
  assert.match(files, /DesktopLab\.dmg/);
  assert.match(files, /SHA256SUMS\.txt/);
  assert.match(files, /release-candidate\.json/);
  assert.doesNotMatch(files, /\.app\//);
  assert.equal(JSON.parse(readFileSync(join(output, "release-candidate.json"), "utf8")).state, "draft_ready");
});

function git(cwd, args) {
  return execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
}

function write(root, path, value) {
  const destination = join(root, path);
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, value);
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value)}\n`);
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}
