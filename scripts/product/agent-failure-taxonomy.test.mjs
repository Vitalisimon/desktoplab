import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { classifyAgentFailure, renderAgentFailureSummary } from "./agent-failure-taxonomy.mjs";

test("adversarial evidence selects deterministic primary and multiple findings", () => {
  const classification = classifyAgentFailure({
    status: "failed",
    originalStopReason: "verification skipped after unsafe mutation",
    scoreResult: {
      criteria: {
        approvalSafety: { score: 0 },
        verification: { score: 0 },
        toolFit: { score: 0 },
      },
      failures: [],
    },
    verification: { status: "fail", checks: [] },
    trace: { events: [{ kind: "completed", source: "agent", success: true, detail: "completed" }] },
  });

  assert.equal(classification.primary, "unsafe_mutation");
  assert.deepEqual(
    classification.findings.map((finding) => finding.code),
    ["unsafe_mutation", "hallucinated_completion", "tool_misuse", "skipped_verification"],
  );
  assert.ok(renderAgentFailureSummary(classification).includes("additional issues"));
});

test("operational and agent failures keep original stop reason separate", () => {
  const timeout = classifyAgentFailure({ status: "timeout", originalStopReason: "model timeout" });
  assert.equal(timeout.primary, "timeout");
  assert.equal(timeout.originalStopReason, "model timeout");

  const environment = classifyAgentFailure({ status: "infrastructure_failure", originalStopReason: "runtime unavailable" });
  assert.equal(environment.primary, "environment_unavailable");

  const inference = classifyAgentFailure({ status: "failed", originalStopReason: "local_inference_failed" });
  assert.equal(inference.primary, "local_inference_failure");
  assert.equal(inference.userMessage, "Local inference failed before the agent could continue.");

  const transport = classifyAgentFailure({ status: "failed", originalStopReason: "ollama_request_failed" });
  assert.equal(transport.primary, "model_transport_failure");

  const delegation = classifyAgentFailure({
    status: "failed",
    trace: { events: [{ kind: "delegation", source: "a2a", success: false, detail: "failed" }] },
  });
  assert.equal(delegation.primary, "failed_delegation");

  const validation = classifyAgentFailure({ status: "failed", originalStopReason: "tests_failed:1" });
  assert.equal(validation.primary, "validation_failed");
  assert.match(validation.userMessage, /repair the issue/);

  const genericValidation = classifyAgentFailure({
    status: "failed",
    originalStopReason: "model decisions exhausted",
    trace: { events: [{ kind: "terminal_observed", source: "desktoplab.run_tests", success: false }] },
  });
  assert.equal(genericValidation.primary, "validation_failed");

  const repairedValidation = classifyAgentFailure({
    status: "failed",
    trace: { events: [
      { kind: "terminal_observed", source: "desktoplab.run_tests", success: false },
      { kind: "terminal_observed", source: "desktoplab.run_tests", success: true },
    ] },
  });
  assert.equal(repairedValidation.primary, "unclassified");
});

test("repeated loops memory misses and verifier gaming use stable codes", () => {
  const failedEvent = { kind: "tool_observed", source: "desktoplab.read_file", success: false, detail: "not found" };
  assert.equal(classifyAgentFailure({ status: "failed", trace: { events: [failedEvent, failedEvent, failedEvent] } }).primary, "repeated_error_loop");
  assert.equal(classifyAgentFailure({ status: "failed", originalStopReason: "workspace memory missing" }).primary, "memory_miss");
  assert.equal(
    classifyAgentFailure({
      status: "failed",
      trace: { events: [{ kind: "tool_observed", source: "filesystem", success: true, detail: "read holdout answers" }] },
    }).primary,
    "suspected_verifier_gaming",
  );
});

test("unknown cases remain unclassified and private reasons are redacted", () => {
  const classification = classifyAgentFailure({
    status: "failed",
    originalStopReason: "odd failure /Users/private/repo token=secret-value",
  });
  assert.equal(classification.primary, "unclassified");
  assert.ok(!classification.originalStopReason.includes("/Users/private"));
  assert.ok(!classification.originalStopReason.includes("secret-value"));
});

test("taxonomy source stays below line guard", () => {
  const logical = readFileSync("scripts/product/agent-failure-taxonomy.mjs", "utf8")
    .split("\n")
    .filter((line) => line.trim()).length;
  assert.ok(logical <= 220, `agent-failure-taxonomy.mjs has ${logical} logical lines`);
});
