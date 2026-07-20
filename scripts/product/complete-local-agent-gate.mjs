#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";

const planPath = "docs/superpowers/plans/2026-07-08-coding-agent-parity.md";
const safeSigningPlanPath = "docs/superpowers/plans/2026-07-10-safe-signing-readiness.md";
const requiredEvidence = [
  "docs/evidence/coding-agent-parity-matrix.md",
  "docs/evidence/agent-parity-eval-cases.md",
  "docs/evidence/live-agent-certification-harness.md",
  "docs/evidence/manual-agent-clickthrough-harness.md",
  "docs/evidence/installed-agent-ui-qa.md",
  "docs/evidence/codex-claude-parity-scorecard.md",
  "apps/desktop/tests/product/agent-parity-installed.spec.ts",
  "apps/desktop/tests/product/workbench-visual.spec.ts",
  "scripts/product/live-agent-certification.mjs",
  "scripts/product/installed-agent-certification.mjs",
  "scripts/product/installed-agent-certification.test.mjs",
  "scripts/product/installed-agent-recording-core.mjs",
  "scripts/product/drivers/macos-installed-agent-ui.mjs",
  "scripts/product/stable-ui-certification.mjs",
  "scripts/product/stable-ui-certification.test.mjs",
  "scripts/product/manual-agent-clickthrough.mjs",
  "crates/desktoplab-control-plane/tests/local_api_agent_long_running_jobs.rs",
];
const requiredInstalledProofTerms = [
  "repository inspection response",
  "named file creation after approval",
  "existing file patch after approval",
  "validation command approval/execution",
  "retry prompt response",
  "Git diff evidence visible in installed UI",
  "local commit proposal",
];
const failures = [];

requireExisting(planPath);
requireExisting(safeSigningPlanPath);
for (const path of requiredEvidence) requireExisting(path);
checkTaskStatuses();
checkSafeSigningAgentTasks();
checkInstalledProofTerms();
checkScorecard();
checkClaimPhraseGate();
checkStrictInstalledHarness();

if (failures.length > 0) {
  console.error("Complete local agent gate failed:");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

function checkStrictInstalledHarness() {
  const source = [
    readFileSync("scripts/product/installed-agent-certification.mjs", "utf8"),
    readFileSync("scripts/product/installed-agent-recording-core.mjs", "utf8"),
  ].join("\n");
  for (const term of [
    "deterministicEvidenceAccepted: false",
    "installed evidence contract is invalid or obsolete",
    "verifyInstalledAgentRecording",
    "testControlRequests: uniqueEvents",
    "localModelRequestCount < 1",
    "realToolExecutionCount < 1",
    "evidence app hash does not match installed artifact",
  ]) {
    if (!source.includes(term)) failures.push(`installed harness missing strict term ${term}`);
  }
  const certification = readFileSync("scripts/product/installed-agent-certification.mjs", "utf8");
  if (/evidence\.cases|evidence\.promptEntered|evidence\.sendClicked|evidence\.approvalClicked/.test(certification)) {
    failures.push("installed certification still trusts operator-authored case outcomes");
  }
}

console.log("Complete local agent gate passed");

function requireExisting(path) {
  if (!existsSync(path)) failures.push(`missing evidence ${path}`);
}

function checkTaskStatuses() {
  if (!existsSync(planPath)) return;
  const source = readFileSync(planPath, "utf8");
  for (let task = 29; task <= 43; task += 1) {
    const nextTask = task + 1;
    const start = source.indexOf(`### Task ${task} -`);
    const end = source.indexOf(`### Task ${nextTask} -`, start + 1);
    const section = start === -1 ? "" : source.slice(start, end === -1 ? undefined : end);
    if (!/^Status: (completed|implemented)$/m.test(section)) {
      failures.push(`task ${task} is not completed/implemented in ${planPath}`);
    }
  }
}

function checkSafeSigningAgentTasks() {
  if (!existsSync(safeSigningPlanPath)) return;
  const source = readFileSync(safeSigningPlanPath, "utf8");
  for (let task = 60; task <= 68; task += 1) {
    const nextTask = task + 1;
    const start = source.indexOf(`### Task ${task} -`);
    const end = source.indexOf(`### Task ${nextTask} -`, start + 1);
    const section = start === -1 ? "" : source.slice(start, end === -1 ? undefined : end);
    if (!/^Status: (completed|implemented)$/m.test(section)) {
      failures.push(`safe-signing agent task ${task} is not completed/implemented`);
    }
  }
}

function checkInstalledProofTerms() {
  if (!existsSync("docs/evidence/coding-agent-parity-matrix.md")) return;
  const matrix = readFileSync("docs/evidence/coding-agent-parity-matrix.md", "utf8");
  for (const term of requiredInstalledProofTerms) {
    if (!matrix.includes(term)) failures.push(`coding-agent parity matrix missing installed proof term: ${term}`);
  }
}

function checkScorecard() {
  const result = spawnSync(process.execPath, ["scripts/product/agent-parity-eval.mjs", "--json"], {
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 16,
  });
  const report = JSON.parse(result.stdout);
  if (report.status !== "pass") failures.push(`agent parity eval requires current installed evidence: ${report.failures.join("; ")}`);
}

function checkClaimPhraseGate() {
  const publicSources = [
    "docs-public/runtime-and-provider-support.md",
    "docs/public-beta-readiness-gate.md",
    "docs/evidence/codex-claude-parity-scorecard.md",
  ];
  const phrase = /comparable to Codex\/Claude Code/i;
  const scorecardVerified = failures.every((failure) => !failure.includes("agent parity eval"));
  for (const path of publicSources) {
    if (!existsSync(path)) continue;
    const source = readFileSync(path, "utf8");
    if (phrase.test(source) && !scorecardVerified) {
      failures.push(`${path}: comparable to Codex/Claude Code requires verified scorecard rows`);
    }
  }
}
