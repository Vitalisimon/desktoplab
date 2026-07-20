import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { buildFrontierLocalGateReport } from "./frontier-local-gate.mjs";

const now = new Date("2026-07-10T12:00:00.000Z");

test("ordinary private beta does not invoke frontier claim requirements", () => {
  const report = buildFrontierLocalGateReport({ claimRequested: false });
  assert.equal(report.status, "not_applicable");
  assert.equal(report.frontierLocalClaimAllowed, false);
  assert.deepEqual(report.failures, []);
});

test("frontier claim blocks without a real high-end report", () => {
  const report = buildFrontierLocalGateReport({
    claimRequested: true,
    certificationReport: null,
    exists: () => true,
    readText: () => completedTaskPlan(),
    currentCommit: "commit.live",
  });
  assert.equal(report.status, "blocked");
  assert.ok(report.failures.includes("missing real high-end host certification report"));
});

test("frontier claim accepts only a fresh exact live installed-app report", () => {
  const report = buildFrontierLocalGateReport({
    claimRequested: true,
    certificationReport: passingCertification(),
    now,
    exists: () => true,
    readText: () => completedTaskPlan(),
    currentCommit: "commit.live",
  });
  assert.equal(report.status, "pass");
  assert.equal(report.frontierLocalClaimAllowed, true);
  assert.deepEqual(report.requiredTasks, [45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57]);
});

test("frontier claim rejects stale deterministic runtime model and RAG evidence", () => {
  const certification = passingCertification();
  certification.generatedAt = "2026-06-01T00:00:00.000Z";
  certification.sourceEvidenceKind = "deterministic_contract";
  certification.runtime.health = "degraded";
  certification.model.quantization = "";
  certification.retrieval.freshness = "stale";
  const report = buildFrontierLocalGateReport({ claimRequested: true, certificationReport: certification, now, exists: () => true, readText: () => completedTaskPlan(), currentCommit: "commit.live" });
  assert.equal(report.status, "blocked");
  assert.ok(report.failures.some((failure) => failure.includes("deterministic")));
  assert.ok(report.failures.some((failure) => failure.includes("stale")));
  assert.ok(report.failures.some((failure) => failure.includes("runtime")));
  assert.ok(report.failures.some((failure) => failure.includes("model")));
  assert.ok(report.failures.some((failure) => failure.includes("RAG")));
});

test("frontier gate source stays below line guard", () => {
  const source = readFileSync(new URL("./frontier-local-gate.mjs", import.meta.url), "utf8");
  const logicalLines = source.split("\n").filter((line) => line.trim() && !line.trim().startsWith("//")).length;
  assert.ok(logicalLines <= 260, `frontier-local-gate.mjs has ${logicalLines} logical lines`);
});

test("beta gauntlet adds frontier gate only for an explicit claim", () => {
  const directory = mkdtempSync(join(tmpdir(), "desktoplab-frontier-gate-"));
  const ordinaryPath = join(directory, "ordinary.json");
  const frontierPath = join(directory, "frontier.json");
  runGauntlet(["--profile", "quick", "--dry-run", "--allow-dirty", "--report", ordinaryPath]);
  runGauntlet(["--profile", "quick", "--dry-run", "--allow-dirty", "--frontier-local-claim", "--frontier-certification", "dist/product/frontier-local-certification.json", "--report", frontierPath]);
  const ordinary = JSON.parse(readFileSync(ordinaryPath, "utf8"));
  const frontier = JSON.parse(readFileSync(frontierPath, "utf8"));
  assert.equal(ordinary.steps.some((step) => step.id === "frontier-local-gate"), false);
  assert.equal(frontier.steps.some((step) => step.id === "frontier-local-gate"), true);
});

function completedTaskPlan() {
  return Array.from({ length: 13 }, (_, index) => 45 + index)
    .map((task) => `### Task ${task} - Evidence\n\nStatus: implemented\n\n- [x] Commit with message \`task ${task}\`.\n`)
    .join("\n");
}

function passingCertification() {
  return {
    kind: "desktoplab.frontier-local-certification",
    status: "pass",
    frontierLocalClaim: true,
    sourceEvidenceKind: "live_installed_app",
    generatedAt: "2026-07-10T11:00:00.000Z",
    app: { path: "/Applications/DesktopLab.app", commit: "commit.live", artifactSha256: "a".repeat(64) },
    host: { profile: "dgx_station_class" },
    runtime: { id: "runtime.nim", version: "live", health: "healthy" },
    model: { id: "model.frontier", quantization: "FP4", contextLengthTokens: 262_144 },
    retrieval: { mode: "hybrid_local", indexGeneration: "generation.live", freshness: "fresh", secretRedactionVerified: true },
    functional: { status: "pass", passed: 7, total: 7 },
    quality: { status: "pass", score: 0.9 },
    performance: { status: "pass", score: 0.8 },
    cases: Array.from({ length: 7 }, (_, index) => ({ id: `case.${index}`, functionalPass: true, artifacts: { transcript: `/evidence/${index}.json`, screenshot: `/evidence/${index}.png`, workspaceEvidence: `/evidence/${index}.workspace.json` } })),
  };
}

function runGauntlet(args) {
  execFileSync("node", ["scripts/product/beta-gauntlet.mjs", ...args], {
    cwd: new URL("../..", import.meta.url),
    stdio: "pipe",
  });
}
