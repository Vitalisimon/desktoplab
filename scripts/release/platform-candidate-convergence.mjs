#!/usr/bin/env node
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import process from "node:process";

import { assessPlatformCandidateConvergence } from "./platform-candidate-convergence-core.mjs";

const args = parseArgs(process.argv.slice(2));
if (!args.candidate || args.evidence.length === 0) throw new Error("platform convergence requires candidate and evidence paths");
const report = assessPlatformCandidateConvergence({
  candidate: readJson(args.candidate),
  evidence: args.evidence.map(readJson),
});
if (args.output) {
  const output = resolve(args.output);
  mkdirSync(dirname(output), { recursive: true });
  writeFileSync(output, `${JSON.stringify(report, null, 2)}\n`);
}
console.log(JSON.stringify(report, null, 2));
if (report.status !== "pass") process.exitCode = 1;

function readJson(path) {
  return JSON.parse(readFileSync(resolve(path), "utf8"));
}

function parseArgs(values) {
  const parsed = { evidence: [] };
  for (let index = 0; index < values.length; index += 1) {
    if (values[index] === "--candidate") parsed.candidate = values[++index];
    else if (values[index] === "--evidence") parsed.evidence.push(values[++index]);
    else if (values[index] === "--output") parsed.output = values[++index];
  }
  return parsed;
}
