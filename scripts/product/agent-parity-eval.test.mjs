import assert from "node:assert/strict";
import test from "node:test";

import { buildMeasuredParityReport } from "./agent-parity-eval.mjs";
import { passingExecutableCase } from "./test-fixtures/agent-trace-score-fixture.mjs";

test("measured parity blocks missing and deterministic evidence", () => {
  assert.equal(buildMeasuredParityReport(null).status, "blocked");
  const report = buildMeasuredParityReport({
    kind: "desktoplab.installed-agent-certification",
    deterministicEvidenceAccepted: true,
    provenance: { executionKind: "deterministic-dev" },
  });
  assert.equal(report.status, "blocked");
});

test("current installed outcomes produce measured control-plane parity", () => {
  const report = buildMeasuredParityReport(completeCertification());
  assert.equal(report.status, "pass");
  assert.equal(report.overall, 1);
  assert.equal(report.controlPlane.status, "pass");
  assert.equal(report.modelQuality.status, "strong");
  assert.equal(report.promptDiversity.languages.length, 2);
  assert.equal(report.promptDiversity.includesFailureRepair, true);
});

test("corrupting a deterministic verifier check lowers the score and fails the gate", () => {
  const certification = completeCertification();
  certification.cases
    .find((entry) => entry.id === "test_repair")
    .verification.checks.find((check) => check.id === "passing_rerun_observed").passed = false;
  const report = buildMeasuredParityReport(certification);
  assert.equal(report.status, "fail");
  assert.ok(report.overall < 1);
  assert.ok(report.failures.some((failure) => failure.includes("passing_rerun_observed")));
});

test("semantic judge cannot rescue missing executable evidence", () => {
  const certification = completeCertification();
  const create = certification.cases.find((entry) => entry.id === "create");
  create.verification = null;
  create.semanticJudge = { score: 1, rationale: "looks correct" };
  const report = buildMeasuredParityReport(certification);

  assert.equal(report.status, "fail");
  assert.equal(report.results.find((entry) => entry.id === "create").scores.completion, 0);
  assert.equal(report.scoreFormula.semanticJudgeContribution, 0);
});

test("model quality remains separate from control-plane correctness", () => {
  const certification = completeCertification();
  for (const entry of certification.cases) {
    entry.modelQuality = { correctness: 0.5, grounding: 0.5, instructionFollowing: 0.5 };
  }
  const report = buildMeasuredParityReport(certification);
  assert.equal(report.controlPlane.status, "pass");
  assert.equal(report.modelQuality.status, "weak");
});

function completeCertification() {
  const common = {
    status: "pass",
    promptEntered: true,
    sendClicked: true,
    sessionContinuous: true,
    latencyMs: 1200,
    modelQuality: { correctness: 0.9, grounding: 0.9, instructionFollowing: 0.9 },
  };
  return {
    kind: "desktoplab.installed-agent-certification",
    schemaVersion: 3,
    status: "pass",
    deterministicEvidenceAccepted: false,
    provenance: {
      executionKind: "installed_app_ui",
      modelId: "qwen2.5-coder:7b",
      testControlRequests: 0,
    },
    cases: ["inspect", "create", "patch", "test_repair", "diff"].map((id) => ({
      ...passingExecutableCase(id),
      ...common,
      id,
    })),
  };
}
