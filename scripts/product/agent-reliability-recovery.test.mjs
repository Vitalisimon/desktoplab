import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  planAgentReliabilityRecovery,
  reaggregateAgentReliabilityRecovery,
} from "./agent-reliability-recovery-core.mjs";

const digest = (character) => `sha256:${character.repeat(64)}`;

test("recovery preserves passing runs and selects only infrastructure failures", () => {
  const source = report();
  const plan = planAgentReliabilityRecovery(source);

  assert.equal(plan.status, "ready");
  assert.equal(plan.execution, "operator_explicit_only");
  assert.deepEqual(plan.preservedRunIds, ["run-pass"]);
  assert.deepEqual(plan.eligibleRunDescriptors.map((run) => run.runId), ["run-infra"]);
});

test("recovery blocks any non-infrastructure product result", () => {
  const source = report();
  source.runs[0].status = "agent_failure";

  const plan = planAgentReliabilityRecovery(source);

  assert.equal(plan.status, "blocked");
  assert.match(plan.failures.join("\n"), /non-infrastructure run is not pass/);
  assert.deepEqual(plan.eligibleRunDescriptors, []);
});

test("reaggregation replaces only the eligible run with matching provenance", () => {
  const source = report();
  const replacement = report({
    runs: [{ ...source.runs[1], status: "pass", score: 1, completion: 1, trajectory: 1 }],
    completedRunCount: 1,
    plannedRunCount: 1,
  });

  const recovered = reaggregateAgentReliabilityRecovery(source, replacement);

  assert.equal(recovered.status, "pass");
  assert.equal(recovered.metrics.passCount, 2);
  assert.equal(recovered.metrics.passRate, 1);
  assert.deepEqual(recovered.recovery.replacedRunIds, ["run-infra"]);
  assert.equal(recovered.recovery.automaticRerun, false);
});

test("reaggregation rejects another candidate or extra run", () => {
  const source = report();
  const wrong = report({
    candidateId: digest("9"),
    runs: [{ ...source.runs[1], runId: "run-other", status: "pass" }],
    completedRunCount: 1,
  });

  const recovered = reaggregateAgentReliabilityRecovery(source, wrong);

  assert.equal(recovered.status, "blocked");
  assert.match(recovered.failures.join("\n"), /candidateId|not eligible/);
});

test("recovery modules stay focused", () => {
  for (const [name, maximum] of [
    ["agent-reliability-recovery-core.mjs", 190],
    ["agent-reliability-recovery.mjs", 70],
  ]) {
    const source = readFileSync(new URL(name, import.meta.url), "utf8");
    const logical = source.split("\n").filter((line) => line.trim()).length;
    assert.ok(logical <= maximum, `${name} has ${logical} logical lines, max ${maximum}`);
  }
});

function report(overrides = {}) {
  const base = {
    kind: "desktoplab.agent-reliability-campaign",
    schemaVersion: 3,
    status: "fail",
    campaignId: "campaign.recovery",
    candidateId: digest("a"),
    appHash: digest("b"),
    manifestDigest: digest("c"),
    plannedRunCount: 2,
    completedRunCount: 2,
    runs: [
      run("run-pass", "pass", 1),
      run("run-infra", "infrastructure_failure", null),
    ],
    failures: ["pass rate 0.5000 below 1.0000"],
  };
  return { ...base, ...overrides };
}

function run(runId, status, score) {
  return {
    runId,
    candidateId: digest("a"),
    appHash: digest("b"),
    campaignId: "campaign.recovery",
    caseId: "inspect",
    seed: runId === "run-pass" ? 7 : 19,
    profileId: "standard",
    repetition: 1,
    timeoutMs: 60_000,
    status,
    score,
    completion: score,
    trajectory: score,
  };
}
