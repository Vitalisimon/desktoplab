const digestPattern = /^sha256:[a-f0-9]{64}$/;
const terminalStatuses = [
  "pass",
  "failed",
  "partial",
  "blocked",
  "agent_failure",
  "timeout",
  "cancelled",
  "infrastructure_failure",
];

export function planAgentReliabilityRecovery(sourceReport) {
  const failures = validateSource(sourceReport);
  const runs = Array.isArray(sourceReport?.runs) ? sourceReport.runs : [];
  const infrastructure = runs.filter((run) => run.status === "infrastructure_failure");
  const preserved = runs.filter((run) => run.status !== "infrastructure_failure");
  if (infrastructure.length === 0) failures.push("no infrastructure_failure run is available");
  if (preserved.some((run) => run.status !== "pass")) {
    failures.push("non-infrastructure run is not pass and cannot be replaced");
  }
  return {
    kind: "desktoplab.agent-reliability-recovery-plan",
    schemaVersion: 1,
    status: failures.length === 0 ? "ready" : "blocked",
    candidateId: safeDigest(sourceReport?.candidateId),
    appHash: safeDigest(sourceReport?.appHash),
    campaignId: safeIdentifier(sourceReport?.campaignId),
    sourceManifestDigest: safeDigest(sourceReport?.manifestDigest),
    execution: "operator_explicit_only",
    preservedRunIds: preserved.map((run) => run.runId),
    eligibleRunDescriptors:
      failures.length === 0 ? infrastructure.map(recoveryDescriptor) : [],
    failures,
  };
}

export function reaggregateAgentReliabilityRecovery(sourceReport, replacementReport) {
  const plan = planAgentReliabilityRecovery(sourceReport);
  const failures = [...plan.failures];
  if (
    replacementReport?.kind !== "desktoplab.agent-reliability-campaign" ||
    replacementReport?.schemaVersion !== 3
  ) {
    failures.push("replacement report contract is invalid");
  }
  for (const field of ["candidateId", "appHash", "campaignId", "manifestDigest"]) {
    if (replacementReport?.[field] !== sourceReport?.[field]) {
      failures.push(`replacement ${field} does not match`);
    }
  }
  const replacements = new Map();
  for (const run of replacementReport?.runs ?? []) {
    if (!plan.eligibleRunDescriptors.some((eligible) => eligible.runId === run.runId)) {
      failures.push(`replacement run ${run.runId ?? "missing"} is not eligible`);
    } else if (replacements.has(run.runId)) {
      failures.push(`replacement run ${run.runId} is duplicated`);
    } else {
      replacements.set(run.runId, run);
    }
  }
  for (const eligible of plan.eligibleRunDescriptors) {
    if (!replacements.has(eligible.runId)) failures.push(`replacement run ${eligible.runId} missing`);
  }
  if (failures.length > 0) return { ...plan, status: "blocked", failures };

  const runs = sourceReport.runs.map((run) => replacements.get(run.runId) ?? run);
  const metrics = reliabilityMetrics(runs);
  const allPass = runs.length === sourceReport.plannedRunCount && runs.every((run) => run.status === "pass");
  return {
    ...sourceReport,
    status: allPass ? "pass" : "fail",
    completedRunCount: runs.length,
    metrics,
    runs,
    failures: allPass ? [] : ["recovered campaign does not pass every planned run"],
    recovery: {
      kind: "infrastructure_only_reaggregation",
      sourceStatus: sourceReport.status,
      preservedRunIds: plan.preservedRunIds,
      replacedRunIds: [...replacements.keys()],
      automaticRerun: false,
    },
  };
}

function validateSource(report) {
  const failures = [];
  if (report?.kind !== "desktoplab.agent-reliability-campaign" || report?.schemaVersion !== 3) {
    failures.push("source report contract is invalid");
  }
  if (!digestPattern.test(report?.candidateId ?? "")) failures.push("source candidateId is invalid");
  if (!digestPattern.test(report?.appHash ?? "")) failures.push("source appHash is invalid");
  if (!digestPattern.test(report?.manifestDigest ?? "")) failures.push("source manifestDigest is invalid");
  if (!Array.isArray(report?.runs) || report.runs.length !== report?.completedRunCount) {
    failures.push("source completed runs are inconsistent");
  }
  if (new Set((report?.runs ?? []).map((run) => run.runId)).size !== (report?.runs ?? []).length) {
    failures.push("source run ids are not unique");
  }
  if ((report?.runs ?? []).some((run) => !terminalStatuses.includes(run.status))) {
    failures.push("source contains a non-terminal run");
  }
  return failures;
}

function recoveryDescriptor(run) {
  return Object.fromEntries(
    ["runId", "candidateId", "appHash", "caseId", "seed", "profileId", "repetition", "timeoutMs", "campaignId"]
      .map((key) => [key, run[key]]),
  );
}

function reliabilityMetrics(runs) {
  const passCount = runs.filter((run) => run.status === "pass").length;
  const passRate = runs.length === 0 ? 0 : passCount / runs.length;
  const scores = runs.map((run) => run.score).filter(Number.isFinite);
  return {
    passCount,
    passRate,
    passAll: passCount === runs.length,
    passPowerK: passRate ** runs.length,
    worstOfN: scores.length === 0 ? null : Math.min(...scores),
    meanScore: mean(scores),
    scoreDispersion: deviation(scores),
    passRateConfidence95: wilson(passCount, runs.length),
    outcomes: Object.fromEntries(
      terminalStatuses.map((status) => [status, runs.filter((run) => run.status === status).length]),
    ),
  };
}

function mean(values) {
  return values.length === 0 ? null : values.reduce((sum, value) => sum + value, 0) / values.length;
}

function deviation(values) {
  if (values.length === 0) return null;
  const average = mean(values);
  return Math.sqrt(mean(values.map((value) => (value - average) ** 2)));
}

function wilson(successes, total) {
  if (total === 0) return { low: null, high: null };
  const z = 1.959963984540054;
  const p = successes / total;
  const denominator = 1 + (z * z) / total;
  const center = (p + (z * z) / (2 * total)) / denominator;
  const margin = (z * Math.sqrt(p * (1 - p) / total + z * z / (4 * total * total))) / denominator;
  return { low: Math.max(0, center - margin), high: Math.min(1, center + margin) };
}

function safeDigest(value) {
  return digestPattern.test(value ?? "") ? value : null;
}

function safeIdentifier(value) {
  return typeof value === "string" && /^[a-zA-Z0-9._-]{1,128}$/.test(value) ? value : null;
}
