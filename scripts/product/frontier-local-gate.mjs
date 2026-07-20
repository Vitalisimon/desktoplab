#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const MAX_REPORT_AGE_MS = 7 * 24 * 60 * 60 * 1000;

export const requiredFrontierTaskEvidence = [
  [45, ["docs/evidence/frontier-local-runtime-readiness.md", "scripts/product/product-claim-guard.mjs"]],
  [46, ["docs/evidence/frontier-local-hardware-profiles.md", "crates/desktoplab-hardware-wizard/tests/frontier_host_probe.rs"]],
  [47, ["crates/desktoplab-runtime/src/high_end.rs", "crates/desktoplab-runtime/tests/high_end_runtime_contract.rs"]],
  [48, ["crates/desktoplab-control-plane/tests/local_api_high_end_runtime_health.rs"]],
  [49, ["docs/evidence/frontier-local-model-catalog.md", "crates/desktoplab-compatibility/tests/frontier_model_catalog.rs"]],
  [50, ["crates/desktoplab-model-manager/tests/huge_model_storage.rs"]],
  [51, ["docs/evidence/repo-rag-architecture.md", "crates/desktoplab-workspace/tests/repo_retrieval.rs"]],
  [52, ["crates/desktoplab-workspace/tests/repo_retrieval_security.rs"]],
  [53, ["crates/desktoplab-agent-engine/tests/context_planner.rs"]],
  [54, ["docs/evidence/frontier-local-certification.md", "scripts/product/frontier-local-certification.mjs"]],
  [55, ["docs/evidence/frontier-local-multi-user-scheduling.md", "crates/desktoplab-control-plane/tests/frontier_multi_user_scheduling.rs"]],
  [56, ["docs/evidence/frontier-local-setup-ux.md", "apps/desktop/tests/product/frontier-local-setup.spec.ts"]],
  [57, ["docs/evidence/custom-rig-equivalence-matrix.md"]],
];

export function buildFrontierLocalGateReport({
  claimRequested = false,
  certificationReport = null,
  now = new Date(),
  exists = existsSync,
  readText = (path) => readFileSync(path, "utf8"),
  currentCommit = gitHead(),
} = {}) {
  if (!claimRequested) {
    return {
      kind: "desktoplab.frontier-local-gate",
      schemaVersion: 1,
      status: "not_applicable",
      claimRequested: false,
      frontierLocalClaimAllowed: false,
      message: "Ordinary private beta does not use the frontier-local claim gate.",
      failures: [],
    };
  }
  const failures = [
    ...taskEvidenceFailures(exists),
    ...taskPlanFailures(readText),
    ...certificationFailures(certificationReport, { now, exists, currentCommit }),
  ];
  return {
    kind: "desktoplab.frontier-local-gate",
    schemaVersion: 1,
    status: failures.length === 0 ? "pass" : "blocked",
    claimRequested: true,
    frontierLocalClaimAllowed: failures.length === 0,
    checkedCommit: currentCommit,
    requiredTasks: requiredFrontierTaskEvidence.map(([task]) => task),
    certificationGeneratedAt: certificationReport?.generatedAt ?? null,
    failures: [...new Set(failures)],
  };
}

function taskEvidenceFailures(exists) {
  return requiredFrontierTaskEvidence.flatMap(([task, paths]) =>
    paths
      .filter((path) => !exists(resolve(repoRoot, path)))
      .map((path) => `Task ${task} missing evidence ${path}`),
  );
}

function taskPlanFailures(readText) {
  const path = resolve(repoRoot, "docs/superpowers/plans/2026-07-08-coding-agent-parity.md");
  let plan;
  try {
    plan = readText(path);
  } catch {
    return ["missing canonical Tasks 45-57 plan evidence"];
  }
  return requiredFrontierTaskEvidence.flatMap(([task]) => {
    const section = taskSection(plan, task);
    if (!section) return [`Task ${task} section missing from canonical plan`];
    if (/Status:\s*planned\b/i.test(section)) return [`Task ${task} remains planned`];
    if (!/- \[x\] Commit with message/i.test(section)) return [`Task ${task} dedicated commit evidence is unchecked`];
    return [];
  });
}

function taskSection(plan, task) {
  const start = plan.indexOf(`### Task ${task} -`);
  if (start < 0) return null;
  const end = plan.indexOf(`### Task ${task + 1} -`, start + 1);
  return plan.slice(start, end < 0 ? plan.length : end);
}

function certificationFailures(report, { now, exists, currentCommit }) {
  if (!report) return ["missing real high-end host certification report"];
  const failures = [];
  if (report.kind !== "desktoplab.frontier-local-certification") failures.push("invalid certification report kind");
  if (report.status !== "pass" || report.frontierLocalClaim !== true) failures.push("certification report is not a live pass");
  if (report.sourceEvidenceKind !== "live_installed_app") failures.push("certification evidence is deterministic or not installed-app live evidence");
  const generatedAt = Date.parse(report.generatedAt ?? "");
  if (!Number.isFinite(generatedAt) || now.getTime() - generatedAt > MAX_REPORT_AGE_MS || generatedAt > now.getTime() + 60_000) failures.push("certification report is missing, stale or future-dated");
  if (!report.app?.path || !exists(report.app.path)) failures.push("certified app artifact is missing");
  if (report.app?.commit !== currentCommit) failures.push("certified app commit does not match current HEAD");
  if (!/^[a-f0-9]{64}$/i.test(report.app?.artifactSha256 ?? "")) failures.push("certified app SHA-256 is missing");
  if (!new Set(["custom_frontier_rig", "dgx_station_class"]).has(report.host?.profile)) failures.push("certified host is not a high-end profile");
  if (report.runtime?.health !== "healthy" || !report.runtime?.id || !report.runtime?.version) failures.push("runtime evidence is missing or unhealthy");
  if (!report.model?.id || !report.model?.quantization || !(report.model?.contextLengthTokens >= 131_072)) failures.push("model evidence is incomplete or context is below 131072 tokens");
  if (report.retrieval?.freshness !== "fresh" || !report.retrieval?.mode || !report.retrieval?.indexGeneration || report.retrieval?.secretRedactionVerified !== true) failures.push("RAG evidence is missing, stale or unredacted");
  if (report.functional?.status !== "pass" || report.functional?.passed !== 7 || report.functional?.total !== 7) failures.push("functional certification cases did not all pass");
  if (report.quality?.status !== "pass" || !(report.quality?.score >= 0.8)) failures.push("quality certification threshold did not pass");
  if (report.performance?.status !== "pass" || !(report.performance?.score >= 0.5)) failures.push("performance certification threshold did not pass");
  if (!Array.isArray(report.cases) || report.cases.length !== 7) failures.push("certification case evidence is incomplete");
  for (const certCase of report.cases ?? []) {
    if (certCase.functionalPass !== true) failures.push(`${certCase.id ?? "unknown"} functional evidence failed`);
    for (const artifact of Object.values(certCase.artifacts ?? {})) {
      if (typeof artifact !== "string" || !exists(artifact)) failures.push(`${certCase.id ?? "unknown"} artifact is missing`);
    }
  }
  return failures;
}

function gitHead() {
  try {
    return execFileSync("git", ["rev-parse", "HEAD"], { cwd: repoRoot, encoding: "utf8" }).trim();
  } catch {
    return null;
  }
}

function parseArgs(argv) {
  const args = { claim: process.env.DESKTOPLAB_FRONTIER_LOCAL_CLAIM === "1", certification: "dist/product/frontier-local-certification.json", output: null, json: false, help: false };
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === "--claim") args.claim = true;
    else if (argv[index] === "--certification") args.certification = argv[++index];
    else if (argv[index] === "--output") args.output = argv[++index];
    else if (argv[index] === "--json") args.json = true;
    else if (argv[index] === "--help") args.help = true;
  }
  return args;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log("Usage: node scripts/product/frontier-local-gate.mjs [--claim] [--certification path] [--output path] [--json]");
    return;
  }
  const certificationPath = resolve(repoRoot, args.certification);
  const certificationReport = args.claim && existsSync(certificationPath) ? JSON.parse(readFileSync(certificationPath, "utf8")) : null;
  const report = buildFrontierLocalGateReport({ claimRequested: args.claim, certificationReport });
  if (args.output) {
    const output = resolve(repoRoot, args.output);
    mkdirSync(dirname(output), { recursive: true });
    writeFileSync(output, `${JSON.stringify(report, null, 2)}\n`);
  }
  if (args.json) console.log(JSON.stringify(report, null, 2));
  else console.log(`Frontier-local gate: ${report.status}`);
  for (const failure of report.failures) console.error(`FAIL: ${failure}`);
  if (report.status === "blocked") process.exitCode = 2;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
