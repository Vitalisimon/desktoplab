#!/usr/bin/env node
import { mkdirSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import process from "node:process";

import { hashArtifact } from "../packaging/artifact-provenance-core.mjs";
import { aggregateRun, appendRun, runCommand } from "./regression-gate-core.mjs";
import { candidateInputs, requiredSafeSigningSteps } from "./safe-signing-regression-plan.mjs";

const args = parseArgs(process.argv.slice(2));
if (!["fresh-recording", "verified-reuse"].includes(args.agentEvidenceMode)) {
  throw new Error("safe-signing requires --agent-evidence-mode fresh-recording|verified-reuse");
}
if (args.agentEvidenceMode === "verified-reuse"
  && (!args.reuseAgentCertification || !args.reuseAgentCampaign)) {
  throw new Error("safe-signing verified reuse requires both agent certification and campaign evidence");
}
if (args.agentEvidenceMode === "fresh-recording"
  && (args.reuseAgentCertification || args.reuseAgentCampaign)) {
  throw new Error("safe-signing fresh recording cannot accept reused agent evidence");
}
for (const name of ["runtimeOllamaManaged", "runtimeLmStudioExisting", "runtimeLmStudioManaged", "runtimeMlxLmManaged"]) {
  if (!args[name]) throw new Error("safe-signing requires all four live runtime certification reports");
}
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
  const parsed = { report: null, dryRun: false, candidate: null, app: null, workspace: null, evidence: null, certification: null, runtime: null, campaign: null, agentGates: null, agentEvidenceMode: null, reuseAgentCertification: null, reuseAgentCampaign: null, runtimeOllamaManaged: null, runtimeLmStudioExisting: null, runtimeLmStudioManaged: null, runtimeMlxLmManaged: null, runtimeMatrix: null };
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
    else if (argv[index] === "--reuse-agent-certification") parsed.reuseAgentCertification = argv[++index];
    else if (argv[index] === "--reuse-agent-campaign") parsed.reuseAgentCampaign = argv[++index];
    else if (argv[index] === "--runtime-ollama-managed") parsed.runtimeOllamaManaged = argv[++index];
    else if (argv[index] === "--runtime-lm-studio-existing") parsed.runtimeLmStudioExisting = argv[++index];
    else if (argv[index] === "--runtime-lm-studio-managed") parsed.runtimeLmStudioManaged = argv[++index];
    else if (argv[index] === "--runtime-mlx-lm-managed") parsed.runtimeMlxLmManaged = argv[++index];
    else if (argv[index] === "--runtime-matrix") parsed.runtimeMatrix = argv[++index];
    else if (argv[index] === "--agent-evidence-mode") parsed.agentEvidenceMode = argv[++index];
    else throw new Error(`unknown or incomplete argument ${argv[index]}`);
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
