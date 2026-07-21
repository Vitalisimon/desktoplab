#!/usr/bin/env node
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import process from "node:process";

import { hashArtifact } from "../packaging/artifact-provenance-core.mjs";
import { appendRun, runCommand } from "./regression-gate-core.mjs";
import { assessSafeSigningRecovery, createSafeSigningRecoveryRun } from "./safe-signing-recovery-core.mjs";
import { candidateInputs, requiredSafeSigningSteps } from "./safe-signing-regression-plan.mjs";

const args = parseArgs(process.argv.slice(2));
if (!args.recoverFrom) throw new Error("safe-signing recovery requires --recover-from");
if (!args.runRoot) throw new Error("safe-signing recovery requires the original --run-root");
const reportPath = resolve(args.report ?? "dist/release/candidate/safe-signing-recovery.json");
if (resolve(args.recoverFrom) === reportPath) throw new Error("recovery output must not overwrite its source report");
const started = new Date();
const runId = `${started.toISOString()}-${process.pid}-recovery`;
const inputs = candidateInputs(args, runId);
const expectedSteps = requiredSafeSigningSteps(inputs);
const sourceBytes = readFileSync(resolve(args.recoverFrom));
const sourceReport = JSON.parse(sourceBytes.toString("utf8"));
const context = {
  head: gitValue(["rev-parse", "HEAD"]),
  treeState: gitValue(["status", "--short"]) ? "dirty" : "clean",
  candidateId: readCandidateId(inputs.candidate),
  preparedAppSha256: existsSync(inputs.app) ? hashArtifact(inputs.app).sha256 : null,
};
const assessment = assessSafeSigningRecovery({ sourceReport, expectedSteps, context });
const rerunResults = [];
if (assessment.status === "ready") {
  for (const id of assessment.rerunStepIds) {
    const step = expectedSteps.find((entry) => entry.id === id);
    console.log(`\n[recheck:${step.id}] ${step.command} ${step.args.join(" ")}`);
    const result = runCommand(step);
    rerunResults.push(result);
    console.log(`${result.status} (${result.durationMs}ms)`);
  }
}
const finished = new Date();
const run = createSafeSigningRecoveryRun({
  assessment,
  rerunResults,
  context,
  sourceReportSha256: `sha256:${createHash("sha256").update(sourceBytes).digest("hex")}`,
  runId,
  startedAt: started.toISOString(),
  finishedAt: finished.toISOString(),
  host: { platform: process.platform, arch: process.arch, node: process.version },
});
const previous = existsSync(reportPath) ? JSON.parse(readFileSync(reportPath, "utf8")) : null;
mkdirSync(dirname(reportPath), { recursive: true });
writeFileSync(reportPath, `${JSON.stringify(appendRun(previous, run), null, 2)}\n`);
console.log(`\nReport: ${reportPath}`);
console.log(`Safe-signing recovery status: ${run.status}`);
for (const failure of run.recoveryFailures ?? []) console.error(`- ${failure}`);
process.exitCode = run.status === "pass" ? 0 : 1;

function gitValue(commandArgs) {
  const result = runCommand({ id: "git-metadata", command: "git", args: commandArgs });
  return result.status === "passed" ? result.outputTail.trim() : null;
}

function readCandidateId(path) {
  if (!existsSync(path)) return null;
  try { return JSON.parse(readFileSync(path, "utf8")).candidateId ?? null; }
  catch { return null; }
}

function parseArgs(argv) {
  const parsed = {};
  const names = new Map([
    ["--recover-from", "recoverFrom"], ["--report", "report"], ["--candidate", "candidate"],
    ["--app", "app"], ["--workspace", "workspace"], ["--evidence", "evidence"],
    ["--certification", "certification"], ["--runtime", "runtime"], ["--campaign", "campaign"],
    ["--agent-gates", "agentGates"], ["--run-root", "runRoot"], ["--reliability-root", "reliabilityRoot"],
    ["--reliability-manifest", "reliabilityManifest"], ["--reliability-catalog", "reliabilityCatalog"],
  ]);
  for (let index = 0; index < argv.length; index += 1) {
    const name = names.get(argv[index]);
    if (!name || index + 1 >= argv.length) throw new Error(`unknown or incomplete argument ${argv[index]}`);
    parsed[name] = argv[++index];
  }
  return parsed;
}
