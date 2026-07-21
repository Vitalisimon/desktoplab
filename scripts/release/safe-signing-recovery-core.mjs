const recoverableFailures = new Set(["agent-release-gates", "stable-ui"]);
const invariantRechecks = ["clean-tree", "candidate-payload"];
const signatureFields = ["command", "args", "required", "rejectOutput", "timeoutMs"];

export function assessSafeSigningRecovery({ sourceReport, expectedSteps, context }) {
  const failures = [];
  if (sourceReport?.kind !== "desktoplab.safe-signing-regression" || sourceReport?.schemaVersion !== 1) {
    failures.push("source report contract is invalid");
  }
  const sourceRun = sourceReport?.runs?.at(-1);
  if (!sourceRun || sourceReport?.latestRunId !== sourceRun?.runId) failures.push("source report latest run is invalid");
  if (sourceReport?.status !== "blocked" || sourceRun?.status !== "blocked") failures.push("source run must be blocked");
  if (sourceRun?.dryRun !== false) failures.push("source run must be certifying, not dry-run evidence");
  for (const field of ["head", "treeState", "candidateId", "preparedAppSha256"]) {
    if (sourceRun?.[field] !== context?.[field]) failures.push(`${field} does not match the source run`);
  }
  if (context?.treeState !== "clean") failures.push("treeState must be clean");

  const expectedById = uniqueSteps(expectedSteps, "expected", failures);
  const sourceById = uniqueSteps(sourceRun?.steps ?? [], "source", failures);
  for (const id of expectedById.keys()) if (!sourceById.has(id)) failures.push(`source plan is missing ${id}`);
  for (const id of sourceById.keys()) if (!expectedById.has(id)) failures.push(`source plan contains unexpected ${id}`);

  const failedStepIds = [];
  for (const [id, expected] of expectedById) {
    const previous = sourceById.get(id);
    if (!previous) continue;
    if (previous.status === "passed") {
      if (!sameSignature(previous, expected)) failures.push(`passed step ${id} does not match the current plan`);
    } else {
      failedStepIds.push(id);
      if (!recoverableFailures.has(id)) failures.push(`${id} failure is not recoverable`);
    }
  }
  if (failedStepIds.length === 0) failures.push("source run has no failed step to recover");

  return {
    status: failures.length === 0 ? "ready" : "blocked",
    failures,
    sourceRun,
    expectedSteps,
    rerunStepIds: failures.length === 0
      ? [...invariantRechecks, ...failedStepIds.filter((id) => !invariantRechecks.includes(id))]
      : [],
  };
}

export function createSafeSigningRecoveryRun({ assessment, rerunResults, context, sourceReportSha256, runId, startedAt, finishedAt, host }) {
  const failures = [...assessment.failures];
  const resultById = uniqueSteps(rerunResults, "recheck", failures);
  for (const id of assessment.rerunStepIds) {
    const result = resultById.get(id);
    if (!result) failures.push(`missing recheck result for ${id}`);
    else if (result.status !== "passed") failures.push(`recheck did not pass for ${id}`);
  }
  for (const id of resultById.keys()) {
    if (!assessment.rerunStepIds.includes(id)) failures.push(`unexpected recheck result for ${id}`);
  }
  const sourceById = new Map((assessment.sourceRun?.steps ?? []).map((step) => [step.id, step]));
  const steps = assessment.expectedSteps.map((step) => resultById.get(step.id) ?? sourceById.get(step.id) ?? step);
  const blocked = steps.filter((step) => step.required !== false && step.status !== "passed").length;
  const finishedMs = Date.parse(finishedAt);
  const startedMs = Date.parse(startedAt);
  const run = {
    runId,
    startedAt,
    finishedAt,
    durationMs: Number.isFinite(finishedMs - startedMs) ? finishedMs - startedMs : null,
    dryRun: false,
    status: failures.length === 0 && blocked === 0 ? "pass" : "blocked",
    counts: { status: failures.length === 0 && blocked === 0 ? "pass" : "blocked", passed: steps.length - blocked, blocked },
    ...context,
    host,
    steps,
    recovery: {
      kind: "verification_only",
      sourceReportSha256,
      sourceRunId: assessment.sourceRun?.runId ?? null,
      sourceRunStatus: assessment.sourceRun?.status ?? null,
      reverifiedStepIds: assessment.rerunStepIds,
    },
  };
  if (failures.length > 0) run.recoveryFailures = failures;
  return run;
}

function uniqueSteps(steps, label, failures) {
  const byId = new Map();
  for (const step of steps ?? []) {
    if (!step?.id) failures.push(`${label} plan contains a step without id`);
    else if (byId.has(step.id)) failures.push(`${label} plan duplicates ${step.id}`);
    else byId.set(step.id, step);
  }
  return byId;
}

function sameSignature(previous, expected) {
  return signatureFields.every((field) => JSON.stringify(previous[field] ?? null) === JSON.stringify(expected[field] ?? null));
}
