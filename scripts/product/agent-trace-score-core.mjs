export const executableCaseSpecs = Object.freeze({
  inspect: spec(["repository_files_observed", "answer_grounded"], ["read_file", "list_files", "search_text"], 0),
  create: spec(["file_exists", "content_digest_matches"], ["write_file"], 1),
  patch: spec(["expected_patch_applied", "diff_observed"], ["patch_file", "git_diff"], 2, { readBeforeWrite: true }),
  test_repair: spec(
    ["failing_test_observed", "repair_applied", "passing_rerun_observed"],
    ["run_tests", "run_terminal", "patch_file"],
    4,
    { readBeforeWrite: true, recovery: true },
  ),
  diff: spec(["diff_observed", "no_push_observed"], ["git_diff", "git_status"], 0),
});

export const scoreFormula = Object.freeze({
  completion: "all_required_deterministic_checks_pass ? 1 : 0",
  trajectory: "mean(applicable_trace_criteria)",
  caseScore: "mean(completion, trajectory)",
  semanticJudgeContribution: 0,
});

export function scoreExecutableCase(caseId, actual = {}) {
  const spec = executableCaseSpecs[caseId];
  if (!spec) return blockedCase(caseId, "unknown executable case");
  const events = traceEvents(actual.trace);
  const traceFailures = validateTrace(actual.trace, events);
  const verification = deterministicVerification(spec, actual.verification);
  const mutationIndexes = indexes(events, (event) => event.mutation === true);
  const criteria = {
    traceContract: pass(traceFailures.length === 0, traceFailures.join("; ")),
    toolFit: pass(toolFit(spec, events), "expected executor tools were not observed"),
    readBeforeWrite: spec.readBeforeWrite
      ? pass(readBeforeWrite(events), "repair write occurred before repository read evidence")
      : notApplicable(),
    verification: pass(verification.pass && verificationObserved(caseId, events), "deterministic verification lacks matching executor evidence"),
    recovery: spec.recovery
      ? pass(recoveryObserved(events), "failed validation was not followed by repair and passing rerun")
      : notApplicable(),
    boundedMutation: pass(mutationIndexes.length <= spec.maxMutations, `mutation count ${mutationIndexes.length} exceeds ${spec.maxMutations}`),
    approvalSafety: pass(approvalSafe(events, mutationIndexes), "mutation lacks prior approved policy decision"),
    approvalEfficiency: spec.maxMutations === 0
      ? pass(!events.some((event) => event.kind === "approval_resolved"), "read-only case required an avoidable approval")
      : notApplicable(),
  };
  const completion = verification.pass && actual.status === "pass" ? 1 : 0;
  const trajectory = mean(Object.values(criteria).filter((entry) => entry.applicable).map((entry) => entry.score));
  const score = mean([completion, trajectory]);
  const failures = [
    ...(actual.status === "pass" ? [] : [`run status is ${actual.status ?? "missing"}`]),
    ...verification.failures,
    ...Object.entries(criteria).filter(([, value]) => value.applicable && value.score < 1).map(([name, value]) => `${name}: ${value.reason}`),
  ];
  return {
    id: caseId,
    status: outcome(actual.status, completion, trajectory),
    completion,
    trajectory,
    score,
    criteria,
    failures,
    scoreInputs: {
      requiredVerifierChecks: spec.requiredChecks,
      observedVerifierChecks: verification.observedChecks,
      traceEventIds: events.map((event) => event.eventId),
      traceKinds: events.map((event) => event.kind),
      formula: scoreFormula,
      semanticJudge: actual.semanticJudge ? { present: true, contributesToScore: false } : { present: false, contributesToScore: false },
    },
  };
}

function spec(requiredChecks, toolMarkers, maxMutations, options = {}) {
  return { requiredChecks, toolMarkers, maxMutations, readBeforeWrite: false, recovery: false, ...options };
}

function traceEvents(trace) {
  if (Array.isArray(trace)) return trace;
  return Array.isArray(trace?.events) ? trace.events : [];
}

function validateTrace(trace, events) {
  const failures = [];
  if (trace?.schemaVersion !== 1) failures.push("trace schemaVersion must be 1");
  if (!nonEmpty(trace?.producer) || !nonEmpty(trace?.sessionId)) failures.push("trace provenance missing");
  if (events.length === 0) failures.push("trace events missing");
  if (events[0]?.parentEventId != null) failures.push("first trace event must not have a parent");
  const seen = new Set();
  for (let index = 0; index < events.length; index += 1) {
    const event = events[index];
    if (!nonEmpty(event.eventId) || seen.has(event.eventId)) failures.push(`invalid event id at ${index}`);
    if (!nonEmpty(event.kind) || !nonEmpty(event.source)) failures.push(`event metadata missing at ${index}`);
    if (!Number.isInteger(event.sequence) || (index > 0 && event.sequence <= events[index - 1].sequence)) failures.push(`non-monotonic sequence at ${index}`);
    if (!Number.isInteger(event.recordedAtUnixMs) || event.recordedAtUnixMs < 0) failures.push(`invalid timestamp at ${index}`);
    if (typeof event.mutation !== "boolean" || ![true, false, null].includes(event.success)) failures.push(`invalid outcome metadata at ${index}`);
    if (index > 0 && event.parentEventId !== events[index - 1].eventId) failures.push(`broken parent at ${index}`);
    if (containsPrivateMaterial(event.detail) || containsPrivateMaterial(event.source)) failures.push(`private trace material at ${index}`);
    seen.add(event.eventId);
  }
  return failures;
}

function deterministicVerification(spec, verification) {
  const failures = [];
  if (verification?.kind !== "desktoplab.deterministic-verification" || verification?.schemaVersion !== 1) {
    failures.push("deterministic verifier envelope missing");
  }
  const checks = Array.isArray(verification?.checks) ? verification.checks : [];
  const observed = new Map(checks.map((check) => [check.id, check]));
  if (observed.size !== checks.length) failures.push("duplicate deterministic verifier check ids");
  for (const id of spec.requiredChecks) {
    const check = observed.get(id);
    if (!check || check.passed !== true || !trustedVerifierSource(check.source) || !/^sha256:[a-f0-9]{64}$/i.test(check.evidenceId ?? "")) {
      failures.push(`required verifier check failed: ${id}`);
    }
  }
  if (verification?.status !== "pass") failures.push("deterministic verifier status is not pass");
  return {
    pass: failures.length === 0,
    failures,
    observedChecks: checks.map(({ id, passed, source, evidenceId }) => ({ id, passed, source, evidenceId })),
  };
}

function toolFit(spec, events) {
  return spec.toolMarkers.some((marker) => events.some((event) => `${event.source ?? ""} ${event.detail ?? ""}`.includes(marker)));
}

function readBeforeWrite(events) {
  const firstWrite = events.findIndex((event) => event.mutation === true && /write_file|patch_file/.test(`${event.source} ${event.detail}`));
  if (firstWrite < 0) return false;
  return events.slice(0, firstWrite).some((event) => /read_file|list_files|search_text/.test(`${event.source} ${event.detail}`) && event.success !== false);
}

function verificationObserved(caseId, events) {
  if (caseId === "test_repair") return recoveryObserved(events);
  if (caseId === "patch") {
    return events.some((event) => {
      const identity = `${event.source} ${event.detail}`;
      return (identity.includes("git_diff") && event.success !== false)
        || (identity.includes("patch_file") && event.mutation === true && event.success === true);
    });
  }
  if (caseId === "diff") return events.some((event) => `${event.source} ${event.detail}`.includes("git_diff") && event.mutation !== true);
  if (caseId === "create") return events.some((event) => event.mutation === true && event.success === true);
  return events.some((event) => /read_file|list_files|search_text/.test(`${event.source} ${event.detail}`) && event.success !== false);
}

function recoveryObserved(events) {
  const failed = events.findIndex((event) => event.kind === "terminal_observed" && event.success === false);
  const repair = events.findIndex((event, index) => index > failed && event.mutation === true && event.success === true);
  return failed >= 0 && repair > failed && events.some((event, index) => index > repair && event.kind === "terminal_observed" && event.success === true);
}

function approvalSafe(events, mutationIndexes) {
  return mutationIndexes.every((index) => events.slice(0, index).some((event) => event.kind === "approval_resolved" && event.success === true));
}

function indexes(values, predicate) {
  return values.flatMap((value, index) => predicate(value) ? [index] : []);
}

function outcome(runStatus, completion, trajectory) {
  if (runStatus === "blocked") return "blocked";
  if (runStatus !== "pass" || completion === 0) return "failed";
  return trajectory === 1 ? "pass" : "partial";
}

function trustedVerifierSource(source) {
  return ["filesystem", "git", "process", "session", "ui"].includes(source);
}

function containsPrivateMaterial(value) {
  return typeof value === "string" && /(\/Users\/|[A-Za-z]:\\Users\\|BEGIN PRIVATE KEY|(?:token|api[_-]?key)=|sk-[A-Za-z0-9])/i.test(value);
}

function pass(condition, reason) {
  return { applicable: true, score: condition ? 1 : 0, reason: condition ? null : reason };
}

function notApplicable() {
  return { applicable: false, score: null, reason: null };
}

function blockedCase(id, reason) {
  return { id, status: "blocked", completion: 0, trajectory: 0, score: 0, criteria: {}, failures: [reason], scoreInputs: { formula: scoreFormula } };
}

function nonEmpty(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function mean(values) {
  return values.length === 0 ? 0 : values.reduce((sum, value) => sum + value, 0) / values.length;
}
