#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { verifyArtifactEvidence } from "./artifact-provenance-core.mjs";
import { gitTreeState } from "./git-tree-state.mjs";

const args = parseArgs(process.argv.slice(2));
const root = process.cwd();
const manifest = verifyArtifactEvidence({
  root,
  evidenceDir: path.resolve(root, args.evidence ?? "dist/desktoplab-packaging"),
  currentHead: git(["rev-parse", "HEAD"]),
  currentTreeState: gitTreeState(root),
  installedAppPath: args.installedApp ? path.resolve(args.installedApp) : null,
});
console.log(`Current-head packaging provenance verified for ${manifest.entries.length} artifact(s) at ${manifest.build.commitSha}`);

function git(values) {
  return execFileSync("git", values, { cwd: root, encoding: "utf8" }).trim();
}

function parseArgs(values) {
  const parsed = {};
  for (let index = 0; index < values.length; index += 1) {
    if (values[index] === "--installed-app") parsed.installedApp = values[++index];
    else if (values[index] === "--evidence") parsed.evidence = values[++index];
  }
  return parsed;
}
