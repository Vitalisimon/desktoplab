#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { writeArtifactEvidence } from "./artifact-provenance-core.mjs";
import { signatureState } from "./macos-signature-policy.mjs";
import { windowsAuthenticodeState } from "./windows-authenticode-state.mjs";

const root = process.cwd();
const args = parseArgs(process.argv.slice(2));
const evidenceDir = path.join(root, "dist", "desktoplab-packaging");
const build = JSON.parse(fs.readFileSync(path.join(evidenceDir, "DesktopLabBuild.json"), "utf8"));
const bundleDir = path.resolve(root, args.bundleDir ?? "apps/desktop/src-tauri/target/debug/bundle");
const artifactPaths = findArtifacts(bundleDir);
if (artifactPaths.length === 0) throw new Error("packaging produced no supported artifacts");
const manifest = writeArtifactEvidence({ root, evidenceDir, artifactPaths, build, signatureStateFor });
console.log(`Recorded ${manifest.entries.length} artifact(s) for ${build.commitSha}`);

function findArtifacts(directory) {
  const found = [];
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const fullPath = path.join(directory, entry.name);
    if (entry.isDirectory() && entry.name.endsWith(".app")) found.push(fullPath);
    else if (entry.isDirectory()) found.push(...findArtifacts(fullPath));
    else if (/\.(dmg|msi|exe|AppImage|deb|rpm)$/.test(entry.name)) found.push(fullPath);
  }
  return found;
}

function signatureStateFor(artifactPath) {
  if (process.platform === "win32" && /\.(exe|msi)$/.test(artifactPath)) {
    return windowsAuthenticodeState(artifactPath);
  }
  if (process.platform !== "darwin") return "unsigned_dev";
  const staple = spawnSync("xcrun", ["stapler", "validate", artifactPath], { encoding: "utf8" });
  if (staple.status === 0) return "notarized";
  if (!artifactPath.endsWith(".app")) return "unsigned_dev";
  const result = spawnSync("codesign", ["-dvvv", artifactPath], { encoding: "utf8" });
  if (result.status !== 0) return "invalid";
  return signatureState(`${result.stdout ?? ""}${result.stderr ?? ""}`);
}

function parseArgs(values) {
  const parsed = {};
  for (let index = 0; index < values.length; index += 1) {
    if (values[index] === "--bundle-dir") parsed.bundleDir = values[++index];
    else throw new Error(`unsupported argument: ${values[index]}`);
  }
  if (parsed.bundleDir === undefined && values.includes("--bundle-dir")) {
    throw new Error("--bundle-dir requires a path");
  }
  return parsed;
}
