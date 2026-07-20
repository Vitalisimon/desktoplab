#!/usr/bin/env node
import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { sha256File } from "./artifact-provenance-core.mjs";
import { linuxSigningPlan, signedReleaseManifest, validateRpmSigningSubkey } from "./linux-release-signing-core.mjs";

const args = parseArgs(process.argv.slice(2));
const root = process.cwd();
if (args.mode === "dry-run") {
  console.log(JSON.stringify({
    status: "passed",
    mode: "dry-run",
    publicTrust: false,
    publicRequirements: ["public GitHub repository", "GitHub Actions OIDC", "cosign", "OpenPGP rpm signing key", "rpmsign"],
  }));
  process.exit(0);
}
const head = git(["rev-parse", "HEAD"]);
if (git(["status", "--porcelain=v1"])) throw new Error("Linux release signing requires a clean source tree");
const manifest = readJson(path.resolve(root, args.manifest));
const plan = linuxSigningPlan(manifest, head);
requirePublicBoundary();
const identity = requiredEnv("COSIGN_CERTIFICATE_IDENTITY_REGEXP");
const issuer = requiredEnv("COSIGN_CERTIFICATE_OIDC_ISSUER");
const rpmKeyId = requiredEnv("LINUX_RPM_OPENPGP_KEY_ID");
const releaseChannel = requiredEnv("LINUX_RELEASE_CHANNEL");
if (!/^[A-Fa-f0-9]{40,64}$/.test(rpmKeyId)) throw new Error("LINUX_RPM_OPENPGP_KEY_ID must be a full fingerprint");
requireCommand("cosign", ["version"]);
for (const command of ["gpg", "rpmsign", "rpm", "rpmkeys"]) requireCommand(command);
const output = path.resolve(root, args.outputDir);
if (fs.existsSync(output)) throw new Error(`refusing to overwrite Linux signed release directory: ${output}`);
fs.mkdirSync(path.dirname(output), { recursive: true });
const temporary = fs.mkdtempSync(path.join(path.dirname(output), ".linux-signing-"));
const rpmDatabase = fs.mkdtempSync(path.join(os.tmpdir(), "desktoplab-rpmdb-"));

try {
  const secretKeyListing = run("gpg", ["--batch", "--with-colons", "--fingerprint", "--list-secret-keys", rpmKeyId]);
  const { fingerprint } = validateRpmSigningSubkey(secretKeyListing, rpmKeyId);
  const rpmPublicKey = "desktoplab-rpm-signing-key.asc";
  const rpmPublicKeyPath = path.join(temporary, rpmPublicKey);
  fs.writeFileSync(rpmPublicKeyPath, run("gpg", ["--batch", "--armor", "--export", rpmKeyId]));
  run("rpm", ["--dbpath", rpmDatabase, "--initdb"]);
  run("rpmkeys", ["--dbpath", rpmDatabase, "--import", rpmPublicKeyPath]);
  const signedArtifacts = [];
  for (const artifact of plan.artifacts) {
    const staged = path.join(temporary, artifact.fileName);
    fs.copyFileSync(path.resolve(root, artifact.sourcePath), staged, fs.constants.COPYFILE_EXCL);
    let rpmSignatureState = null;
    if (artifact.format === "rpm") {
      run("rpmsign", ["--define", `_gpg_name ${rpmKeyId}`, "--addsign", staged]);
      run("rpmkeys", ["--dbpath", rpmDatabase, "--define", "_pkgverify_level signature", "--checksig", staged]);
      rpmSignatureState = "valid";
    }
    const bundle = `${staged}.sigstore.json`;
    run("cosign", ["sign-blob", "--yes", "--bundle", bundle, staged]);
    run("cosign", ["verify-blob", "--bundle", bundle, "--certificate-identity-regexp", identity, "--certificate-oidc-issuer", issuer, staged]);
    const stat = fs.statSync(staged);
    signedArtifacts.push({
      ...artifact,
      relativePath: path.relative(root, path.join(output, artifact.fileName)),
      sha256: sha256File(staged),
      sizeBytes: stat.size,
      signatureState: "signed",
      sigstoreBundle: `${artifact.fileName}.sigstore.json`,
      sigstoreBundleSha256: sha256File(bundle),
      rpmSignatureState,
    });
  }
  const release = signedReleaseManifest({
    plan,
    signedArtifacts,
    identity,
    issuer,
    releaseChannel,
    runner: process.env.GITHUB_RUN_ID ? `github:${process.env.GITHUB_RUN_ID}` : os.hostname(),
    rpmTrustRoot: { fingerprint, publicKey: rpmPublicKey, publicKeySha256: sha256File(path.join(temporary, rpmPublicKey)) },
  });
  const releasePath = path.join(temporary, "linux-signed-artifact-manifest.json");
  fs.writeFileSync(releasePath, `${JSON.stringify({ ...release, generatedAt: new Date().toISOString() }, null, 2)}\n`);
  const releaseBundle = `${releasePath}.sigstore.json`;
  run("cosign", ["sign-blob", "--yes", "--bundle", releaseBundle, releasePath]);
  run("cosign", ["verify-blob", "--bundle", releaseBundle, "--certificate-identity-regexp", identity, "--certificate-oidc-issuer", issuer, releasePath]);
  fs.renameSync(temporary, output);
  console.log(JSON.stringify(release));
} catch (error) {
  fs.rmSync(temporary, { recursive: true, force: true });
  throw error;
} finally {
  fs.rmSync(rpmDatabase, { recursive: true, force: true });
}

function requirePublicBoundary() {
  if (process.platform !== "linux") throw new Error("public Linux signing must run on Linux");
  if (process.env.GITHUB_ACTIONS !== "true") throw new Error("public Linux signing requires GitHub Actions trusted build context");
  if (process.env.DESKTOPLAB_PUBLIC_REPOSITORY !== "true") throw new Error("public Linux signing is blocked until the repository is public");
  requiredEnv("ACTIONS_ID_TOKEN_REQUEST_URL");
  requiredEnv("ACTIONS_ID_TOKEN_REQUEST_TOKEN");
}

function run(program, values) {
  const result = spawnSync(program, values, { cwd: root, encoding: "utf8", env: process.env, maxBuffer: 64 * 1024 * 1024 });
  if (result.status !== 0) throw new Error(`${program} failed: ${(result.stderr || result.stdout).trim()}`);
  return `${result.stdout ?? ""}${result.stderr ?? ""}`;
}

function requireCommand(command, versionArgs = ["--version"]) {
  const result = spawnSync(command, versionArgs, { encoding: "utf8" });
  if (result.status !== 0) throw new Error(`${command} is required for Linux release signing`);
}

function requiredEnv(name) {
  const value = process.env[name];
  if (!value?.trim()) throw new Error(`${name} is required for Linux release signing`);
  return value.trim();
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function git(values) {
  return execFileSync("git", values, { cwd: root, encoding: "utf8" }).trim();
}

function parseArgs(values) {
  const parsed = { mode: "dry-run", manifest: "dist/desktoplab-packaging/artifact-manifest.json", outputDir: "dist/release/linux-signed" };
  for (let index = 0; index < values.length; index += 1) {
    if (values[index] === "--mode") parsed.mode = values[++index];
    else if (values[index] === "--manifest") parsed.manifest = values[++index];
    else if (values[index] === "--output-dir") parsed.outputDir = values[++index];
    else throw new Error(`unsupported argument: ${values[index]}`);
  }
  if (!["dry-run", "public"].includes(parsed.mode)) throw new Error(`unsupported Linux signing mode: ${parsed.mode}`);
  return parsed;
}
