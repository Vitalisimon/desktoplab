#!/usr/bin/env node
import { mkdirSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import process from "node:process";

import { hashArtifact } from "../packaging/artifact-provenance-core.mjs";
import { aggregateRun, appendRun, runCommand } from "./regression-gate-core.mjs";
import { candidateInputs, requiredSafeSigningSteps } from "./safe-signing-regression-plan.mjs";

const args = parseArgs(process.argv.slice(2));
const startedAt = new Date();
const runId = `${startedAt.toISOString()}-${process.pid}`;
const inputs = candidateInputs(args, runId);
const reportPath = resolve(args.report ?? "dist/release/candidate/safe-signing-regression.json");
const steps = requiredSafeSigningSteps(inputs);
const results = [];

for (const step of steps) {
  console.log(`\n[${step.id}] ${step.command} ${step.args.join(" ")}`);
  const result = runCommand(step, { dryRun: args.dryRun });
  results.push(result);
  console.log(`${result.status} (${result.durationMs}ms)`);
}

const aggregate = aggregateRun(results);
const run = {
  runId,
  startedAt: startedAt.toISOString(),
  finishedAt: new Date().toISOString(),
  durationMs: Date.now() - startedAt.getTime(),
  dryRun: args.dryRun,
  status: args.dryRun ? "blocked" : aggregate.status,
  counts: aggregate,
  head: gitValue(["rev-parse", "HEAD"]),
  treeState: gitValue(["status", "--short"]) ? "dirty" : "clean",
  candidateId: args.dryRun ? null : readCandidateId(inputs.candidate),
  preparedAppSha256: !args.dryRun && existsSync(inputs.app) ? hashArtifact(inputs.app).sha256 : null,
  host: { platform: process.platform, arch: process.arch, node: process.version },
  steps: results,
};
const previous = existsSync(reportPath) ? JSON.parse(readFileSync(reportPath, "utf8")) : null;
const report = appendRun(previous, run);
mkdirSync(dirname(reportPath), { recursive: true });
writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(`\nReport: ${reportPath}`);
console.log(`Safe-signing regression status: ${run.status}`);
process.exitCode = run.status === "pass" ? 0 : 1;

function gitValue(commandArgs) {
  const result = runCommand({ id: "git-metadata", command: "git", args: commandArgs });
  return result.status === "passed" ? result.outputTail.trim() : null;
}

function parseArgs(argv) {
  const parsed = { report: null, dryRun: false, candidate: null, app: null, workspace: null, evidence: null, certification: null, runtime: null, campaign: null, agentGates: null };
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === "--report") parsed.report = argv[++index];
    else if (argv[index] === "--dry-run") parsed.dryRun = true;
    else if (argv[index] === "--candidate") parsed.candidate = argv[++index];
    else if (argv[index] === "--app") parsed.app = argv[++index];
    else if (argv[index] === "--workspace") parsed.workspace = argv[++index];
    else if (argv[index] === "--evidence") parsed.evidence = argv[++index];
    else if (argv[index] === "--certification") parsed.certification = argv[++index];
    else if (argv[index] === "--runtime") parsed.runtime = argv[++index];
    else if (argv[index] === "--campaign") parsed.campaign = argv[++index];
    else if (argv[index] === "--agent-gates") parsed.agentGates = argv[++index];
    else if (argv[index] === "--run-root") parsed.runRoot = argv[++index];
    else if (argv[index] === "--reliability-root") parsed.reliabilityRoot = argv[++index];
    else if (argv[index] === "--reliability-manifest") parsed.reliabilityManifest = argv[++index];
    else if (argv[index] === "--reliability-catalog") parsed.reliabilityCatalog = argv[++index];
  }
  return parsed;
}

function readCandidateId(path) {
  if (!existsSync(path)) return null;
  try {
    return JSON.parse(readFileSync(path, "utf8")).candidateId ?? null;
  } catch {
    return null;
  }
}
