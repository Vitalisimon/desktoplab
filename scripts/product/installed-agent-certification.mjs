#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import { hashArtifact, readEmbeddedBuild } from "../packaging/artifact-provenance-core.mjs";
import { scoreExecutableCase } from "./agent-trace-score-core.mjs";
import { verifyInstalledAgentRecording } from "./installed-agent-recording-core.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");

export const installedAgentCases = [
  { id: "inspect", approval: false, evidence: ["repositoryFiles", "groundedAnswer"] },
  { id: "create", approval: true, evidence: ["createdFile", "sessionId"] },
  { id: "patch", approval: true, evidence: ["exactPatch", "diff"] },
  { id: "test_repair", approval: true, evidence: ["failedTest", "repair", "passedRerun"] },
  { id: "diff", approval: false, evidence: ["diff", "noPush"] },
];
export const installedAgentDriverTimeoutMs = 75 * 60 * 1000;

export function assessInstalledAgentEvidence({
  appPath,
  workspacePath,
  evidencePath,
  head = gitHead(),
  exists = existsSync,
  hash = hashPath,
  readBuild = readEmbeddedBuild,
  readEvidence = (path) => readFileSync(path, "utf8"),
  candidate = null,
  verifyRecording = verifyInstalledAgentRecording,
} = {}) {
  const failures = [];
  if (!appPath || !exists(appPath)) failures.push("installed app artifact missing");
  if (!workspacePath || !exists(join(workspacePath, ".git"))) {
    failures.push("real Git workspace missing");
  }
  if (!evidencePath || !exists(evidencePath)) failures.push("UI-driver evidence missing");
  if (failures.length > 0) return report("blocked", failures, null, { appPath, workspacePath, head });

  const evidence = JSON.parse(readEvidence(evidencePath));
  if (evidence.kind !== "desktoplab.installed-agent-evidence" || evidence.schemaVersion !== 2) {
    failures.push("installed evidence contract is invalid or obsolete");
  }
  const appHash = hash(appPath);
  let appBuild = null;
  try {
    appBuild = readBuild(appPath);
  } catch (error) {
    failures.push(`installed app build metadata is unavailable: ${error.message}`);
  }
  if (evidence.appHash !== appHash) failures.push("evidence app hash does not match installed artifact");
  if (evidence.commit !== head) failures.push("installed evidence commit is stale");
  if (appBuild?.commitSha !== head) failures.push("installed app embedded commit differs from current source");
  if (evidence.commit !== appBuild?.commitSha) failures.push("installed evidence commit differs from app metadata");
  if (!sameBuildEvidence(evidence.appBuild, appBuild)) failures.push("installed evidence build metadata differs from app metadata");
  if (candidate) failures.push(...candidateFailures(candidate, appBuild, appHash));
  if (!evidence.modelId || !evidence.quantization || !evidence.host) {
    failures.push("model, quantization, or host provenance missing");
  }
  const recording = verifyRecording({ evidence, appPath, workspacePath, repoRoot });
  failures.push(...recording.failures);
  const cases = installedAgentCases.map((expected) => assessCase(expected, recording.cases ?? []));
  failures.push(...cases.flatMap((entry) => entry.failures.map((failure) => `${entry.id}: ${failure}`)));
  const passed = failures.length === 0;
  return report(passed ? "pass" : "fail", failures, cases, {
    appPath,
    workspacePath,
    head,
    appHash,
    appBuild,
    candidateId: candidate?.candidateId ?? null,
    modelId: evidence.modelId,
    quantization: evidence.quantization,
    host: evidence.host,
    screenshots: (evidence.interactions ?? []).map((entry) => entry.screenshot).filter(Boolean),
    executionKind: "installed_app_ui",
    localModelRequestCount: recording.metrics?.localModelRequestCount ?? 0,
    realToolExecutionCount: recording.metrics?.realToolExecutionCount ?? 0,
    testControlRequests: recording.metrics?.testControlRequests ?? null,
  });
}

export function runInstalledAgentDriver({ appPath, workspacePath, evidencePath, driver, candidate = null, candidatePath = null }) {
  if (!driver || !existsSync(driver)) {
    return { status: "blocked", failures: ["DESKTOPLAB_INSTALLED_AGENT_DRIVER missing"] };
  }
  const driverArgs = installedAgentDriverArgs({ appPath, workspacePath, evidencePath, candidatePath });
  const result = spawnSync(driver, driverArgs, {
    cwd: repoRoot,
    encoding: "utf8",
    env: { ...process.env, DESKTOPLAB_TEST_CONTROLS: "0" },
    timeout: installedAgentDriverTimeoutMs,
  });
  if (result.status !== 0) {
    return { status: "fail", failures: [`UI driver failed: ${(result.stderr || result.stdout).trim()}`] };
  }
  return assessInstalledAgentEvidence({ appPath, workspacePath, evidencePath, candidate });
}

export function installedAgentDriverArgs({ appPath, workspacePath, evidencePath, candidatePath }) {
  const args = ["--app", appPath, "--workspace", workspacePath, "--evidence", evidencePath];
  if (candidatePath) args.push("--candidate", candidatePath);
  return args;
}

function assessCase(expected, cases) {
  const actual = cases.find((entry) => entry.id === expected.id) ?? {};
  const failures = [];
  if (actual.promptEntered !== true || actual.sendClicked !== true) failures.push("prompt was not driven through UI");
  if (actual.sessionContinuous !== true) failures.push("session continuity not observed");
  if (expected.approval && actual.approvalClicked !== true) failures.push("approval was not clicked through UI");
  const executable = scoreExecutableCase(expected.id, actual);
  if (executable.status !== "pass") failures.push(...executable.failures);
  return {
    id: expected.id,
    status: failures.length === 0 ? "pass" : "fail",
    promptEntered: actual.promptEntered === true,
    sendClicked: actual.sendClicked === true,
    sessionContinuous: actual.sessionContinuous === true,
    approvalClicked: actual.approvalClicked === true,
    latencyMs: actual.latencyMs ?? null,
    modelQuality: actual.modelQuality ?? null,
    evidence: actual.evidence ?? {},
    verification: actual.verification ?? null,
    trace: actual.trace ?? null,
    trajectory: executable.trajectory,
    executableScore: executable.score,
    scoreInputs: executable.scoreInputs,
    failures,
  };
}

function report(status, failures, cases, provenance) {
  return {
    kind: "desktoplab.installed-agent-certification",
    schemaVersion: 3,
    status,
    liveClaim: status === "pass",
    deterministicEvidenceAccepted: false,
    provenance,
    cases: cases ?? [],
    failures,
  };
}

function gitHead() {
  const result = spawnSync("git", ["rev-parse", "HEAD"], { cwd: repoRoot, encoding: "utf8" });
  return result.status === 0 ? result.stdout.trim() : null;
}

function hashPath(path) {
  return `sha256:${hashArtifact(path).sha256}`;
}

function sameBuildEvidence(evidenceBuild, appBuild) {
  if (!evidenceBuild || !appBuild) return false;
  for (const key of ["commitSha", "channel", "architecture"]) {
    if (evidenceBuild[key] !== appBuild[key]) return false;
  }
  return JSON.stringify(evidenceBuild.lockfiles) === JSON.stringify(appBuild.lockfiles);
}

function candidateFailures(candidate, appBuild, appHash) {
  const failures = [];
  if (candidate.kind !== "desktoplab.release-candidate" || candidate.schemaVersion !== 1) {
    return ["installed certification candidate contract is invalid"];
  }
  if (!appBuild || candidate.source?.commit !== appBuild.commitSha) failures.push("candidate source differs from installed app metadata");
  if (!appBuild || candidate.release?.channel !== appBuild.channel) failures.push("candidate channel differs from installed app metadata");
  if (!appBuild || JSON.stringify(candidate.lockfiles) !== JSON.stringify(appBuild.lockfiles)) failures.push("candidate lock hashes differ from installed app metadata");
  if (candidate.payload?.sha256 !== appHash.replace(/^sha256:/, "")) failures.push("candidate payload hash differs from installed app");
  return failures;
}

function parseArgs(argv) {
  const args = { app: "/Applications/DesktopLab.app", workspace: null, evidence: null, report: null, candidate: null, driver: process.env.DESKTOPLAB_INSTALLED_AGENT_DRIVER };
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === "--app") args.app = argv[++index];
    else if (argv[index] === "--workspace") args.workspace = argv[++index];
    else if (argv[index] === "--evidence") args.evidence = argv[++index];
    else if (argv[index] === "--report") args.report = argv[++index];
    else if (argv[index] === "--candidate") args.candidate = argv[++index];
    else if (argv[index] === "--driver") args.driver = argv[++index];
  }
  return args;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const args = parseArgs(process.argv.slice(2));
  const candidate = args.candidate ? JSON.parse(readFileSync(resolve(args.candidate), "utf8")) : null;
  const result = args.driver
    ? runInstalledAgentDriver({ appPath: args.app, workspacePath: args.workspace, evidencePath: args.evidence, driver: args.driver, candidate, candidatePath: args.candidate })
    : assessInstalledAgentEvidence({ appPath: args.app, workspacePath: args.workspace, evidencePath: args.evidence, candidate });
  if (args.report) {
    const reportPath = resolve(args.report);
    mkdirSync(dirname(reportPath), { recursive: true });
    writeFileSync(reportPath, `${JSON.stringify(result, null, 2)}\n`);
  }
  console.log(JSON.stringify(result, null, 2));
  process.exitCode = result.status === "pass" ? 0 : 1;
}
