import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  buildFrontierCertificationReport,
  frontierEvidenceTemplate,
} from "./frontier-local-certification.mjs";
import { frontierCertificationCases } from "./live-agent-certification.mjs";

const now = new Date("2026-07-10T10:00:00.000Z");

test("frontier harness exposes all required live agent cases", () => {
  assert.deepEqual(frontierCertificationCases.map((item) => item.id), [
    "large_repo_inspection",
    "cross_file_refactor",
    "failing_test_repair",
    "long_context_recall",
    "rag_grounded_answer",
    "terminal_validation",
    "commit_proposal",
  ]);
});

test("frontier certification blocks without installed-app evidence", () => {
  const report = buildFrontierCertificationReport({ now });
  assert.equal(report.status, "blocked_live_evidence");
  assert.equal(report.frontierLocalClaim, false);
});

test("frontier certification separates functional quality and performance pass", () => {
  const evidence = passingEvidence();
  const report = buildFrontierCertificationReport({ evidence, now, exists: () => true });

  assert.equal(report.status, "pass");
  assert.equal(report.frontierLocalClaim, true);
  assert.deepEqual(report.functional, { status: "pass", passed: 7, total: 7 });
  assert.equal(report.quality.status, "pass");
  assert.equal(report.performance.status, "pass");
  assert.ok(report.cases.every((item) => item.artifacts.transcript));
});

test("frontier certification rejects stale deterministic or incomplete evidence", () => {
  const evidence = passingEvidence();
  evidence.generatedAt = "2026-06-01T00:00:00.000Z";
  evidence.evidenceKind = "deterministic_contract";
  evidence.cases[0].checks.repositoryGrounded = false;

  const report = buildFrontierCertificationReport({ evidence, now, exists: () => true });
  assert.equal(report.status, "fail");
  assert.equal(report.frontierLocalClaim, false);
  assert.ok(report.failures.includes("evidence is not from the installed app"));
  assert.ok(report.failures.includes("live evidence is stale or future-dated"));
  assert.ok(report.failures.some((failure) => failure.includes("repositoryGrounded")));
});

test("frontier certification requires real artifact paths", () => {
  const report = buildFrontierCertificationReport({
    evidence: passingEvidence(),
    now,
    exists: () => false,
  });
  assert.equal(report.status, "fail");
  assert.ok(report.failures.some((failure) => failure.includes("missing installed app artifact")));
  assert.ok(report.failures.some((failure) => failure.includes("transcript artifact")));
});

test("frontier harness source stays below line guard", () => {
  const source = readFileSync(new URL("./frontier-local-certification.mjs", import.meta.url), "utf8");
  const logicalLines = source.split("\n").filter((line) => line.trim() && !line.trim().startsWith("//")).length;
  assert.ok(logicalLines <= 300, `frontier-local-certification.mjs has ${logicalLines} logical lines`);
});

function passingEvidence() {
  const evidence = frontierEvidenceTemplate(now);
  evidence.operator = "certification.operator";
  evidence.app = { path: "/Applications/DesktopLab.app", commit: "abc123", artifactSha256: "a".repeat(64) };
  evidence.host = {
    profile: "dgx_station_class",
    hostname: "frontier-host",
    gpu: "GB300",
    cpu: "Grace",
    acceleratorMemoryBytes: 748_000_000_000,
    systemMemoryBytes: 748_000_000_000,
    storageFreeBytes: 2_000_000_000_000,
    driver: "driver-live",
  };
  evidence.runtime = { id: "runtime.nim", version: "live-version", endpoint: "http://127.0.0.1:8000", ownership: "desktoplab", health: "healthy" };
  evidence.model = { id: "frontier-model", sizeClass: "600B", quantization: "FP4", contextLengthTokens: 262_144, license: "verified-license" };
  evidence.retrieval = { mode: "hybrid_local", indexGeneration: "generation-live", freshness: "fresh", secretRedactionVerified: true };
  evidence.cases = evidence.cases.map((item, index) => ({
    ...item,
    status: "pass",
    startedAt: `2026-07-10T10:0${index}:00.000Z`,
    completedAt: `2026-07-10T10:0${index}:30.000Z`,
    durationMs: 30_000,
    checks: Object.fromEntries(Object.keys(item.checks).map((check) => [check, true])),
    quality: { correctness: 0.9, grounding: 0.9, instructionFollowing: 0.9 },
    performance: { timeToFirstEventMs: 2_000, totalLatencyMs: 30_000, peakMemoryPressurePercent: 80 },
    artifacts: {
      transcript: `/evidence/${item.id}.transcript.json`,
      screenshot: `/evidence/${item.id}.png`,
      workspaceEvidence: `/evidence/${item.id}.workspace.json`,
    },
  }));
  return evidence;
}
