#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { buildWindowsHostEvidence } from "./windows-host-evidence-core.mjs";
import { isGitContentClean } from "./git-content-clean.mjs";

if (process.platform !== "win32") throw new Error("Windows host evidence must be generated on Windows");
const root = process.cwd();
const commit = git(["rev-parse", "HEAD"]);
if (!isGitContentClean(root)) throw new Error("Windows host evidence requires a clean source tree");
const evidenceDir = path.join(root, "dist", "release");
const manifest = JSON.parse(fs.readFileSync(
  path.join(root, "dist", "desktoplab-packaging", "artifact-manifest.json"),
  "utf8",
));
const evidence = buildWindowsHostEvidence({
  manifest,
  commit,
  host: {
    hostname: os.hostname(),
    os: `Windows ${os.release()}`,
    architecture: os.arch(),
    runner: manifest.build.runner,
  },
  smokeLog: path.join(evidenceDir, "windows-install-smoke.log"),
});
fs.mkdirSync(evidenceDir, { recursive: true });
const output = path.join(evidenceDir, "windows-current-head-evidence.json");
fs.writeFileSync(output, `${JSON.stringify({ ...evidence, generatedAt: new Date().toISOString() }, null, 2)}\n`);
console.log(JSON.stringify(evidence));

function git(args) {
  return execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
}
