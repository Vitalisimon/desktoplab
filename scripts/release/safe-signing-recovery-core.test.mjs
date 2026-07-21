import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import test from "node:test";

import { assessSafeSigningRecovery, createSafeSigningRecoveryRun } from "./safe-signing-recovery-core.mjs";

const context = {
  head: "abc123",
  treeState: "clean",
  candidateId: "sha256:candidate",
  preparedAppSha256: "app-hash",
};

test("verification-only recovery replaces only cheap invariants and allowlisted failed gates", () => {
  const expectedSteps = steps();
  const sourceReport = report(expectedSteps, {
    "agent-release-gates": "failed",
    "stable-ui": "failed",
  });
  const assessment = assessSafeSigningRecovery({ sourceReport, expectedSteps, context });

  assert.equal(assessment.status, "ready");
  assert.deepEqual(assessment.rerunStepIds, [
    "clean-tree",
    "candidate-payload",
    "agent-release-gates",
    "stable-ui",
  ]);

  const recovered = createSafeSigningRecoveryRun({
    assessment,
    rerunResults: assessment.rerunStepIds.map((id) => ({ ...expectedSteps.find((step) => step.id === id), status: "passed" })),
    context,
    sourceReportSha256: "source-report-hash",
    runId: "recovery-1",
    startedAt: "2026-07-21T10:00:00.000Z",
    finishedAt: "2026-07-21T10:00:01.000Z",
  });

  assert.equal(recovered.status, "pass");
  assert.deepEqual(recovered.steps.map(({ id, status }) => [id, status]), expectedSteps.map(({ id }) => [id, "passed"]));
  assert.deepEqual(recovered.recovery, {
    kind: "verification_only",
    sourceReportSha256: "source-report-hash",
    sourceRunId: "source-run",
    sourceRunStatus: "blocked",
    reverifiedStepIds: assessment.rerunStepIds,
  });
});

test("recovery rejects failures in expensive or behavioral evidence", () => {
  for (const id of ["installed-agent", "agent-reliability-campaign", "beta-full"]) {
    const expectedSteps = steps();
    const assessment = assessSafeSigningRecovery({
      sourceReport: report(expectedSteps, { [id]: "failed" }),
      expectedSteps,
      context,
    });
    assert.equal(assessment.status, "blocked", id);
    assert.match(assessment.failures.join("\n"), new RegExp(`${id}.*not recoverable`));
  }
});

test("recovery rejects provenance, tree and payload mismatches", () => {
  const expectedSteps = steps();
  for (const [field, value] of [
    ["head", "other-head"],
    ["treeState", "dirty"],
    ["candidateId", "sha256:other"],
    ["preparedAppSha256", "other-app"],
  ]) {
    const assessment = assessSafeSigningRecovery({
      sourceReport: report(expectedSteps, { "stable-ui": "failed" }),
      expectedSteps,
      context: { ...context, [field]: value },
    });
    assert.equal(assessment.status, "blocked", field);
    assert.match(assessment.failures.join("\n"), new RegExp(field));
  }
});

test("recovery rejects incomplete, duplicated or altered prior plans", () => {
  const expectedSteps = steps();
  const variants = [
    report(expectedSteps.slice(1), { "stable-ui": "failed" }),
    report([...expectedSteps, expectedSteps[0]], { "stable-ui": "failed" }),
    report(expectedSteps.map((step) => step.id === "beta-full" ? { ...step, args: ["changed"] } : step), { "stable-ui": "failed" }),
  ];
  for (const sourceReport of variants) {
    const assessment = assessSafeSigningRecovery({ sourceReport, expectedSteps, context });
    assert.equal(assessment.status, "blocked");
  }
});

test("recovery remains blocked unless every planned recheck passes", () => {
  const expectedSteps = steps();
  const assessment = assessSafeSigningRecovery({
    sourceReport: report(expectedSteps, { "stable-ui": "failed" }),
    expectedSteps,
    context,
  });
  const rerunResults = assessment.rerunStepIds.map((id) => ({
    ...expectedSteps.find((step) => step.id === id),
    status: id === "stable-ui" ? "failed" : "passed",
  }));
  const recovered = createSafeSigningRecoveryRun({
    assessment,
    rerunResults,
    context,
    sourceReportSha256: "hash",
    runId: "recovery-failed",
    startedAt: "2026-07-21T10:00:00.000Z",
    finishedAt: "2026-07-21T10:00:01.000Z",
  });
  assert.equal(recovered.status, "blocked");
});

test("safe-signing recovery modules stay below focused line guards", () => {
  for (const [name, maximum] of [
    ["safe-signing-recovery-core.mjs", 130],
    ["safe-signing-recovery.mjs", 110],
    ["safe-signing-regression-plan.mjs", 90],
  ]) {
    const source = readFileSync(new URL(name, import.meta.url), "utf8");
    const logicalLines = source.split("\n").filter((line) => line.trim() && !line.trim().startsWith("//")).length;
    assert.ok(logicalLines <= maximum, `${name} has ${logicalLines} logical lines (max ${maximum})`);
  }
});

test("recovery cannot overwrite the blocked source report", () => {
  const result = spawnSync(process.execPath, [
    "scripts/release/safe-signing-recovery.mjs",
    "--recover-from", "/tmp/same-safe-signing-report.json",
    "--run-root", "/tmp/original-safe-signing-run",
    "--report", "/tmp/same-safe-signing-report.json",
  ], { encoding: "utf8" });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /must not overwrite its source report/);
});

function steps() {
  return [
    step("clean-tree", "git", ["status", "--porcelain=v1"]),
    step("candidate-payload", "node", ["candidate", "verify"]),
    step("installed-agent", "node", ["installed"]),
    step("agent-reliability-campaign", "node", ["campaign"]),
    step("agent-release-gates", "node", ["gates", "--complete-bundle"]),
    step("beta-full", "node", ["beta", "full"]),
    step("stable-ui", "node", ["stable", "review-v2"]),
  ];
}

function step(id, command, args) {
  return { id, command, args, required: true };
}

function report(expectedSteps, failures) {
  return {
    kind: "desktoplab.safe-signing-regression",
    schemaVersion: 1,
    status: "blocked",
    latestRunId: "source-run",
    runs: [{
      runId: "source-run",
      dryRun: false,
      status: "blocked",
      head: context.head,
      treeState: context.treeState,
      candidateId: context.candidateId,
      preparedAppSha256: context.preparedAppSha256,
      steps: expectedSteps.map((entry) => ({ ...entry, status: failures[entry.id] ?? "passed" })),
    }],
  };
}
