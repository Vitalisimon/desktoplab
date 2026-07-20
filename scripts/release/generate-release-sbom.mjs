#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

import { buildCycloneDx, sha256File } from "../security/supply-chain-evidence-core.mjs";
import { npmPackagesFromLock } from "./release-sbom-core.mjs";

const root = process.cwd();
const output = resolve(root, valueAfter("--output") ?? "dist/release/assembly/sbom.cdx.json");
const expectedCommit = valueAfter("--commit") ?? git(["rev-parse", "HEAD"]);
const head = git(["rev-parse", "HEAD"]);
if (head !== expectedCommit) throw new Error(`SBOM commit ${expectedCommit} differs from HEAD ${head}`);
if (git(["status", "--porcelain=v1"])) throw new Error("release SBOM requires a clean source tree");

const lockPaths = ["Cargo.lock", "package-lock.json", "apps/desktop/src-tauri/Cargo.lock"];
const lockHashes = lockPaths.map((path) => ({ path, sha256: sha256File(join(root, path)) }));
const cargoMetadata = JSON.parse(execFileSync("cargo", ["metadata", "--locked", "--format-version", "1"], {
  cwd: root,
  encoding: "utf8",
  maxBuffer: 64 * 1024 * 1024,
}));
const npmLock = JSON.parse(readFileSync(join(root, "package-lock.json"), "utf8"));
const npmPackages = npmPackagesFromLock(npmLock, (packagePath) => readManifest(packagePath));
const tauri = JSON.parse(readFileSync(join(root, "apps/desktop/src-tauri/tauri.conf.json"), "utf8"));
const sbom = buildCycloneDx({ commit: head, version: tauri.version, cargoMetadata, npmPackages, lockHashes });

mkdirSync(dirname(output), { recursive: true });
writeFileSync(output, `${JSON.stringify(sbom, null, 2)}\n`);
console.log(JSON.stringify({ status: "passed", output, commit: head, components: sbom.components.length }));

function readManifest(packagePath) {
  const path = join(root, packagePath, "package.json");
  return existsSync(path) ? JSON.parse(readFileSync(path, "utf8")) : {};
}

function valueAfter(name) {
  const index = process.argv.indexOf(name);
  return index < 0 ? null : process.argv[index + 1];
}

function git(args) {
  return execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
}
