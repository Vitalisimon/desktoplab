#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import process from "node:process";

import { hashArtifact, readEmbeddedBuild } from "../packaging/artifact-provenance-core.mjs";
import { assessMacosPromotion } from "./macos-promotion-core.mjs";

const args = parseArgs(process.argv.slice(2));
for (const [label, path] of [["candidate", args.candidate], ["certification", args.certification], ["safe-signing", args.safeSigning], ["app", args.app]]) {
  if (!path || !existsSync(resolve(path))) throw new Error(`${label} path is missing`);
}

const report = assessMacosPromotion({
  candidate: readJson(args.candidate),
  certification: readJson(args.certification),
  safeSigning: readJson(args.safeSigning),
  appHash: hashArtifact(resolve(args.app)).sha256,
  appBuild: readEmbeddedBuild(resolve(args.app)),
  currentHead: execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim(),
});
console.log(JSON.stringify(report, null, 2));
if (report.status !== "pass") process.exitCode = 1;

function readJson(path) {
  return JSON.parse(readFileSync(resolve(path), "utf8"));
}

function parseArgs(values) {
  const parsed = {};
  for (let index = 0; index < values.length; index += 1) {
    if (values[index] === "--candidate") parsed.candidate = values[++index];
    else if (values[index] === "--certification") parsed.certification = values[++index];
    else if (values[index] === "--safe-signing") parsed.safeSigning = values[++index];
    else if (values[index] === "--app") parsed.app = values[++index];
  }
  return parsed;
}
