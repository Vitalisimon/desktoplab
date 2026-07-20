#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import process from "node:process";

import { assessReleaseTag } from "./release-tag-policy-core.mjs";

const args = parseArgs(process.argv.slice(2));
if (!args.candidate || !args.releaseRef) throw new Error("release tag policy requires --candidate and --release-ref");
const candidate = JSON.parse(readFileSync(resolve(args.candidate), "utf8"));
const report = assessReleaseTag({
  candidate,
  releaseRef: args.releaseRef,
  objectType: git(["cat-file", "-t", args.releaseRef]),
  tagCommit: git(["rev-parse", `${args.releaseRef}^{commit}`]),
});
console.log(JSON.stringify(report, null, 2));
if (report.status !== "pass") process.exitCode = 1;

function git(values) {
  return execFileSync("git", values, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

function parseArgs(values) {
  const parsed = {};
  for (let index = 0; index < values.length; index += 1) {
    if (values[index] === "--candidate") parsed.candidate = values[++index];
    else if (values[index] === "--release-ref") parsed.releaseRef = values[++index];
  }
  return parsed;
}
