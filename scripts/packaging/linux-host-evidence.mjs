#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { buildLinuxHostEvidence } from "./linux-host-evidence-core.mjs";

if (process.platform !== "linux") throw new Error("Linux host evidence must be generated on Linux");
const root = process.cwd();
const commit = git(["rev-parse", "HEAD"]);
if (git(["status", "--porcelain=v1"])) throw new Error("Linux host evidence requires a clean source tree");
const evidenceDir = path.join(root, "dist", "release");
const manifest = JSON.parse(fs.readFileSync(path.join(root, "dist", "desktoplab-packaging", "artifact-manifest.json"), "utf8"));
const evidence = buildLinuxHostEvidence({
  manifest,
  commit,
  host: {
    hostname: os.hostname(),
    os: readOsRelease(),
    architecture: os.arch(),
    runner: manifest.build.runner,
    rpmEnvironment: "fedora:41-container",
  },
  smokeLogs: {
    appimage: path.join(evidenceDir, "linux-appimage-smoke.log"),
    deb: path.join(evidenceDir, "linux-deb-smoke.log"),
    rpm: path.join(evidenceDir, "linux-rpm-smoke.log"),
  },
});
fs.mkdirSync(evidenceDir, { recursive: true });
const output = path.join(evidenceDir, "linux-current-head-evidence.json");
fs.writeFileSync(output, `${JSON.stringify({ ...evidence, generatedAt: new Date().toISOString() }, null, 2)}\n`);
console.log(JSON.stringify(evidence));

function git(args) {
  return execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
}

function readOsRelease() {
  const values = Object.fromEntries(fs.readFileSync("/etc/os-release", "utf8").split(/\r?\n/).filter(Boolean).map((line) => {
    const [key, ...rest] = line.split("=");
    return [key, rest.join("=").replace(/^"|"$/g, "")];
  }));
  return `${values.NAME ?? "Linux"} ${values.VERSION_ID ?? "unknown"}`;
}
