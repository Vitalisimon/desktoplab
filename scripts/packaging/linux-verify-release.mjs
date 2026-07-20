#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { sha256File } from "./artifact-provenance-core.mjs";
import { validateSignedReleaseShape } from "./linux-release-signing-core.mjs";

const directory = path.resolve(process.cwd(), process.argv[2] ?? "dist/release/linux-signed");
const manifestPath = path.join(directory, "linux-signed-artifact-manifest.json");
const manifest = validateSignedReleaseShape(JSON.parse(fs.readFileSync(manifestPath, "utf8")));
const identity = process.env.COSIGN_CERTIFICATE_IDENTITY_REGEXP ?? manifest.sigstore.identity;
const issuer = process.env.COSIGN_CERTIFICATE_OIDC_ISSUER ?? manifest.sigstore.issuer;
requireCommand("cosign", ["version"]);
requireCommand("rpm");
requireCommand("rpmkeys");
verifySigstore(manifestPath, `${manifestPath}.sigstore.json`);
const rpmPublicKey = path.join(directory, manifest.rpmOpenPgp.publicKey);
if (sha256File(rpmPublicKey) !== manifest.rpmOpenPgp.publicKeySha256) throw new Error("RPM public signing key differs from the release manifest");
const rpmDatabase = fs.mkdtempSync(path.join(os.tmpdir(), "desktoplab-rpmdb-"));

try {
  run("rpm", ["--dbpath", rpmDatabase, "--initdb"]);
  run("rpmkeys", ["--dbpath", rpmDatabase, "--import", rpmPublicKey]);
  for (const artifact of manifest.artifacts) {
    const artifactPath = path.join(directory, artifact.fileName);
    const bundlePath = path.join(directory, artifact.sigstoreBundle);
    if (sha256File(artifactPath) !== artifact.sha256 || fs.statSync(artifactPath).size !== artifact.sizeBytes) {
      throw new Error(`${artifact.fileName} differs from the signed release manifest`);
    }
    if (sha256File(bundlePath) !== artifact.sigstoreBundleSha256) {
      throw new Error(`${artifact.fileName} Sigstore bundle differs from the signed release manifest`);
    }
    verifySigstore(artifactPath, bundlePath);
    if (artifact.format === "rpm") {
      run("rpmkeys", ["--dbpath", rpmDatabase, "--define", "_pkgverify_level signature", "--checksig", artifactPath]);
    }
  }
  console.log(JSON.stringify({ status: "passed", commit: manifest.commit, platform: manifest.platform, publicTrust: true }));
} finally {
  fs.rmSync(rpmDatabase, { recursive: true, force: true });
}

function verifySigstore(file, bundle) {
  run("cosign", ["verify-blob", "--bundle", bundle, "--certificate-identity-regexp", identity, "--certificate-oidc-issuer", issuer, file]);
}

function requireCommand(command, versionArgs = ["--version"]) {
  const result = spawnSync(command, versionArgs, { encoding: "utf8" });
  if (result.status !== 0) throw new Error(`${command} is required to verify Linux releases`);
}

function run(program, values) {
  const result = spawnSync(program, values, { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
  if (result.status !== 0) throw new Error(`${program} failed: ${(result.stderr || result.stdout).trim()}`);
  return `${result.stdout ?? ""}${result.stderr ?? ""}`;
}
