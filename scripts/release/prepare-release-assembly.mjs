#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { basename, join, relative, resolve } from "node:path";

import { sha256File } from "../packaging/artifact-provenance-core.mjs";
import { transitionCandidate } from "./candidate-admission-core.mjs";
import { buildReleaseAssembly, validateReleaseSource } from "./release-assembly-core.mjs";

const root = process.cwd();
const args = parseArgs(process.argv.slice(2));
const head = git(["rev-parse", "HEAD"]);
if (git(["status", "--porcelain=v1"])) throw new Error("release assembly requires a clean source tree");
const source = validateReleaseSource({
  releaseRef: args.releaseRef,
  channel: args.channel,
  head,
  tagCommit: git(["rev-parse", `${args.releaseRef}^{commit}`]),
  tagObjectType: git(["cat-file", "-t", args.releaseRef]),
});
const candidate = readJson(args.candidate);
if (candidate.state !== "cross_platform_pass" || candidate.source?.commit !== source.commit) {
  throw new Error("release assembly requires exact cross-platform candidate acceptance");
}
const evidence = args.evidenceDirs.map(readPlatformEvidence);
const sbom = readJson(args.sbom);
const updaterProof = readJson(args.updaterProof);
const assembly = buildReleaseAssembly({ source, platformEvidence: evidence.map((item) => item.value), sbom, updaterProof });

mkdirSync(args.outputDir, { recursive: true });
const assets = [];
for (const artifact of assembly.artifacts) {
  const sourcePath = findUnique(args.evidenceDirs, artifact.fileName);
  verifyFile(sourcePath, artifact);
  assets.push(copyAsset(sourcePath, args.outputDir, artifact.fileName));
}
for (const verification of assembly.verificationAssets) {
  const sourcePath = findUnique(args.evidenceDirs, verification.fileName);
  if (sha256File(sourcePath) !== verification.sha256) throw new Error(`${verification.fileName} differs from signed evidence`);
  assets.push(copyAsset(sourcePath, args.outputDir, verification.fileName));
}
assets.push(copyAsset(args.sbom, args.outputDir, "sbom.cdx.json"));
assets.push(copyAsset(args.updaterProof, args.outputDir, "updater-disabled-proof.json"));
for (const item of evidence) {
  assets.push(copyAsset(item.path, args.outputDir, item.outputName));
  if (item.value.kind === "desktoplab.linux-signed-release") {
    const bundle = `${item.path}.sigstore.json`;
    if (!existsSync(bundle)) throw new Error("Linux signed manifest Sigstore bundle is missing");
    const copied = copyAsset(bundle, args.outputDir, `${item.outputName}.sigstore.json`);
    assembly.verificationAssets.push({ fileName: basename(copied), sha256: sha256File(copied), role: "signed-manifest-sigstore-bundle" });
    assets.push(copied);
  }
}

const manifestPath = join(args.outputDir, "release-manifest.json");
writeFileSync(manifestPath, `${JSON.stringify({ ...assembly, generatedAt: new Date().toISOString() }, null, 2)}\n`);
assets.push(manifestPath);
const draftCandidate = transitionCandidate(candidate, { to: "draft_ready", evidence: assembly });
const candidatePath = join(args.outputDir, "release-candidate.json");
writeFileSync(candidatePath, `${JSON.stringify(draftCandidate, null, 2)}\n`);
assets.push(candidatePath);
const notesPath = join(args.outputDir, "draft-release-notes.md");
writeFileSync(notesPath, releaseNotes(source));
assets.push(notesPath);
const sumsPath = join(args.outputDir, "SHA256SUMS.txt");
writeFileSync(sumsPath, `${assets.map((file) => `${sha256File(file)}  ${basename(file)}`).join("\n")}\n`);
assets.push(sumsPath);
const listPath = join(args.outputDir, "release-files.txt");
writeFileSync(listPath, `${assets.map((file) => relative(root, file)).join("\n")}\n`);
console.log(JSON.stringify({ status: "draft-ready", tag: source.tag, commit: source.commit, assetCount: assets.length, releaseFiles: relative(root, listPath) }));

function readPlatformEvidence(directory) {
  const candidates = [
    ["linux-signed-artifact-manifest.json", "linux-signed-artifact-manifest.json"],
    ["signpath-provenance.json", "windows-signpath-provenance.json"],
    ["artifact-manifest.json", "artifact-manifest.json"],
  ];
  for (const [name, outputName] of candidates) {
    const path = join(directory, name);
    if (existsSync(path)) return { path, outputName: uniqueEvidenceName(outputName, directory), value: readJson(path) };
  }
  throw new Error(`platform evidence manifest missing from ${directory}`);
}

function uniqueEvidenceName(name, directory) {
  return name === "artifact-manifest.json" ? `${basename(directory)}-artifact-manifest.json` : name;
}

function findUnique(directories, fileName) {
  const matches = directories.flatMap((directory) => walk(directory)).filter((path) => basename(path) === fileName);
  if (matches.length !== 1) throw new Error(`expected one ${fileName}, found ${matches.length}`);
  return matches[0];
}

function walk(directory) {
  const output = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isSymbolicLink()) throw new Error(`release evidence contains symlink: ${path}`);
    if (entry.isDirectory()) output.push(...walk(path));
    else if (entry.isFile()) output.push(path);
  }
  return output;
}

function verifyFile(path, artifact) {
  const stat = lstatSync(path);
  if (!stat.isFile() || stat.size !== artifact.sizeBytes || sha256File(path) !== artifact.sha256) throw new Error(`${artifact.fileName} differs from signed evidence`);
}

function copyAsset(source, outputDir, name = basename(source)) {
  const destination = join(outputDir, name);
  copyFileSync(source, destination);
  if (!statSync(destination).isFile()) throw new Error(`release asset is not a file: ${name}`);
  return destination;
}

function releaseNotes(source) {
  return `# DesktopLab ${source.tag}\n\nStatus: draft\n\nChannel: ${source.channel}\n\nSource commit: ${source.commit}\n\nIn-app updates are disabled for this build. Installation and rollback remain manual.\n\nThis draft is private release assembly evidence and is not authorization to publish binaries.\n`;
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function git(values) {
  return execFileSync("git", values, { cwd: root, encoding: "utf8" }).trim();
}

function parseArgs(values) {
  const parsed = { evidenceDirs: [], outputDir: resolve("dist/release/assembly") };
  for (let index = 0; index < values.length; index += 1) {
    const name = values[index];
    const value = values[++index];
    if (!value) throw new Error(`${name} requires a value`);
    if (name === "--release-ref") parsed.releaseRef = value;
    else if (name === "--channel") parsed.channel = value;
    else if (name === "--evidence-dir") parsed.evidenceDirs.push(resolve(value));
    else if (name === "--sbom") parsed.sbom = resolve(value);
    else if (name === "--updater-proof") parsed.updaterProof = resolve(value);
    else if (name === "--candidate") parsed.candidate = resolve(value);
    else if (name === "--output-dir") parsed.outputDir = resolve(value);
    else throw new Error(`unsupported argument: ${name}`);
  }
  if (!parsed.releaseRef || !parsed.channel || !parsed.sbom || !parsed.updaterProof || !parsed.candidate || parsed.evidenceDirs.length === 0) throw new Error("release assembly arguments are incomplete");
  return parsed;
}
