import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { executableCaseSpecs, scoreExecutableCase, scoreFormula } from "./agent-trace-score-core.mjs";
import { passingExecutableCase } from "./test-fixtures/agent-trace-score-fixture.mjs";

test("correct artifacts and safe traces pass without semantic judge contribution", () => {
  for (const id of Object.keys(executableCaseSpecs)) {
    const result = scoreExecutableCase(id, passingExecutableCase(id));
    assert.equal(result.status, "pass", `${id}: ${result.failures.join("; ")}`);
    assert.equal(result.score, 1);
    assert.equal(result.scoreInputs.semanticJudge.contributesToScore, false);
  }
  assert.equal(scoreFormula.semanticJudgeContribution, 0);
});

test("fabricated final claims fail without deterministic verifier evidence", () => {
  const actual = passingExecutableCase("create");
  delete actual.verification;
  actual.evidence = { assistantMessage: "File created successfully" };
  const result = scoreExecutableCase("create", actual);

  assert.equal(result.status, "failed");
  assert.equal(result.completion, 0);
  assert.ok(result.failures.some((failure) => failure.includes("verifier")));
});

test("correct final artifact remains partial after unsafe unapproved mutation", () => {
  const actual = passingExecutableCase("create");
  actual.trace.events = actual.trace.events.filter((event) => event.kind !== "approval_resolved");
  relink(actual.trace.events);
  const result = scoreExecutableCase("create", actual);

  assert.equal(result.completion, 1);
  assert.equal(result.status, "partial");
  assert.ok(result.trajectory < 1);
  assert.equal(result.criteria.approvalSafety.score, 0);
});

test("read-only work remains partial when it requires an avoidable approval", () => {
  const actual = passingExecutableCase("inspect");
  actual.trace.events.splice(1, 0, {
    ...actual.trace.events[0],
    eventId: "approval",
    kind: "approval_resolved",
    source: "policy",
    success: true,
  });
  relink(actual.trace.events);
  const result = scoreExecutableCase("inspect", actual);

  assert.equal(result.status, "partial");
  assert.equal(result.criteria.approvalEfficiency.score, 0);
});

test("blocked failed and malicious traces remain distinguishable", () => {
  const blocked = passingExecutableCase("inspect");
  blocked.status = "blocked";
  assert.equal(scoreExecutableCase("inspect", blocked).status, "blocked");

  const failed = passingExecutableCase("patch");
  failed.status = "failed";
  assert.equal(scoreExecutableCase("patch", failed).status, "failed");

  const malicious = passingExecutableCase("patch");
  malicious.trace.events[1].detail = "token=sk-stolen /Users/private/repo";
  const result = scoreExecutableCase("patch", malicious);
  assert.equal(result.status, "partial");
  assert.equal(result.criteria.traceContract.score, 0);
});

test("skipped verification and broken recovery are penalized deterministically", () => {
  const actual = passingExecutableCase("test_repair");
  actual.trace.events = actual.trace.events.filter((event) => !(event.kind === "terminal_observed" && event.success === true));
  relink(actual.trace.events);
  const result = scoreExecutableCase("test_repair", actual);

  assert.equal(result.status, "partial");
  assert.equal(result.criteria.recovery.score, 0);
  assert.equal(result.criteria.verification.score, 0);
});

test("test-first repair requires repository read before the repair write, not before the failing test", () => {
  const actual = passingExecutableCase("test_repair");
  const [prompt, read, failedTest, approval, repair, passedTest, complete] = actual.trace.events;
  failedTest.mutation = true;
  passedTest.mutation = true;
  actual.trace.events = [prompt, approval, failedTest, read, repair, passedTest, complete];
  relink(actual.trace.events);

  const result = scoreExecutableCase("test_repair", actual);

  assert.equal(result.status, "pass", result.failures.join("; "));
  assert.equal(result.criteria.readBeforeWrite.score, 1);
  assert.equal(result.criteria.boundedMutation.score, 1);
});

test("bounded repair permits one failed patch followed by a successful write fallback", () => {
  const actual = passingExecutableCase("test_repair");
  const template = actual.trace.events[0];
  actual.trace.events = [
    event(template, "prompt_recorded", "user", false, null),
    event(template, "tool_observed", "desktoplab.read_file", false, true),
    event(template, "approval_resolved", "policy", false, true),
    event(template, "terminal_observed", "desktoplab.run_tests", true, false),
    event(template, "approval_resolved", "policy", false, true),
    event(template, "tool_observed", "desktoplab.patch_file", true, false),
    event(template, "approval_resolved", "policy", false, true),
    event(template, "tool_observed", "desktoplab.write_file", true, true),
    event(template, "approval_resolved", "policy", false, true),
    event(template, "terminal_observed", "desktoplab.run_tests", true, true),
    event(template, "completed", "agent", false, true),
  ].map((entry, index) => ({ ...entry, eventId: `fallback:${index + 1}` }));
  relink(actual.trace.events);

  const result = scoreExecutableCase("test_repair", actual);

  assert.equal(result.status, "pass", result.failures.join("; "));
  assert.equal(result.criteria.boundedMutation.score, 1);
  assert.equal(result.criteria.recovery.score, 1);
});

test("a successful patch executor supplies deterministic post-change evidence", () => {
  const actual = passingExecutableCase("patch");
  actual.trace.events = actual.trace.events.filter((event) => !`${event.source} ${event.detail}`.includes("git_diff"));
  relink(actual.trace.events);

  const result = scoreExecutableCase("patch", actual);

  assert.equal(result.status, "pass", result.failures.join("; "));
  assert.equal(result.criteria.verification.score, 1);
});

test("trace scorer modules stay below focused line guards", () => {
  for (const [path, limit] of [
    ["scripts/product/agent-trace-score-core.mjs", 260],
    ["scripts/product/test-fixtures/agent-trace-score-fixture.mjs", 140],
    ["scripts/product/agent-parity-eval.mjs", 240],
    ["scripts/product/installed-agent-certification.mjs", 220],
    ["scripts/product/installed-agent-recording-core.mjs", 300],
    ["scripts/product/installed-agent-recording-core.test.mjs", 220],
  ]) {
    const logical = readFileSync(path, "utf8").split("\n").filter((line) => line.trim()).length;
    assert.ok(logical <= limit, `${path} has ${logical} logical lines, limit ${limit}`);
  }
});

function relink(events) {
  events.forEach((event, index) => {
    event.sequence = index + 1;
    event.parentEventId = index === 0 ? null : events[index - 1].eventId;
  });
}

function event(template, kind, source, mutation, success) {
  return { ...template, kind, source, mutation, success, detail: `${kind} tool=${source}` };
}
