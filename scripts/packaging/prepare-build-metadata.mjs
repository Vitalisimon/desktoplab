#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { sha256File } from "./artifact-provenance-core.mjs";
import { gitTreeState } from "./git-tree-state.mjs";

const root = process.cwd();
const evidenceDir = path.join(root, "dist", "desktoplab-packaging");
const tauri = JSON.parse(fs.readFileSync(path.join(root, "apps/desktop/src-tauri/tauri.conf.json"), "utf8"));
const lockfiles = ["Cargo.lock", "package-lock.json", "apps/desktop/src-tauri/Cargo.lock"].map((lockPath) => ({ path: lockPath, sha256: sha256File(path.join(root, lockPath)) }));
const channel = process.env.DESKTOPLAB_RELEASE_CHANNEL ?? "dev";
const macosSigningIdentity = process.env.DESKTOPLAB_MACOS_SIGNING_IDENTITY?.trim() || "-";
const build = {
  version: tauri.version,
  commitSha: git(["rev-parse", "HEAD"]),
  channel,
  treeState: gitTreeState(root),
  architecture: process.arch,
  runner: process.env.DESKTOPLAB_RUNNER_PROFILE ?? `local:${process.platform}-${process.arch}`,
  workflow: process.env.DESKTOPLAB_BUILD_WORKFLOW
    ?? (channel === "dev" ? "npm run desktop:package:dev" : "npm run desktop:package:macos:release"),
  lockfiles,
  ...(process.platform === "win32" && process.env.WINDOWS_SIGNING_CERTIFICATE_THUMBPRINT
    ? { signingTrustMode: (process.env.WINDOWS_SIGNING_TRUST_MODE ?? "Test").toLowerCase() }
    : {}),
};
fs.mkdirSync(evidenceDir, { recursive: true });
const metadataPath = path.join(evidenceDir, "DesktopLabBuild.json");
const configPath = path.join(evidenceDir, "tauri-build-metadata.json");
fs.writeFileSync(metadataPath, `${JSON.stringify(build, null, 2)}\n`);
const bundle = {
  resources: { [metadataPath]: "DesktopLabBuild.json" },
  ...(process.platform === "darwin" ? { macOS: { signingIdentity: macosSigningIdentity } } : {}),
  ...(process.platform === "win32" && process.env.WINDOWS_SIGNING_CERTIFICATE_THUMBPRINT
    ? {
        windows: {
          certificateThumbprint: process.env.WINDOWS_SIGNING_CERTIFICATE_THUMBPRINT.replaceAll(" ", ""),
          digestAlgorithm: "sha256",
          ...(process.env.WINDOWS_SIGNING_TIMESTAMP_URL ? { timestampUrl: process.env.WINDOWS_SIGNING_TIMESTAMP_URL } : {}),
        },
      }
    : {}),
};
fs.writeFileSync(configPath, `${JSON.stringify({ bundle }, null, 2)}\n`);
process.stdout.write(configPath);

function git(args) {
  return execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
}
