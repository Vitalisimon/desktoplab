import { assertCandidate } from "./candidate-admission-core.mjs";
import { validateExecutorProvenance } from "../product/agent-reliability-evidence.mjs";

const requiredCases = ["inspect", "create", "patch", "test_repair", "diff"];
const zeroToleranceCases = new Set(["create", "patch", "diff"]);
const completedAgentStatuses = new Set(["pass", "failed", "partial", "blocked"]);

export function assessAgentReleaseGates({ candidate, runtime, campaign, expectedExecutorSha256, expectedExecutorBundleSha256, expectedUiDriverSha256, expectedUiDriverBundleSha256 }) {
  const runtimeFailures = [];
  try {
    assertCandidate(candidate);
  } catch (error) {
    runtimeFailures.push(error.message);
  }
  if (candidate?.state !== "payload_built") runtimeFailures.push("candidate must be in payload_built state");
  if (runtime?.kind !== "desktoplab.measured-agent-parity" || runtime?.schemaVersion !== 1) {
    runtimeFailures.push("runtime parity contract is invalid");
  }
  if (runtime?.status !== "pass" || runtime?.controlPlane?.status !== "pass") {
    runtimeFailures.push("runtime control-plane gate did not pass");
  }
  if (runtime?.provenance?.candidateId !== candidate?.candidateId) {
    runtimeFailures.push("runtime parity belongs to another candidate");
  }
  if (runtime?.provenance?.appHash !== `sha256:${candidate?.payload?.sha256 ?? ""}`) {
    runtimeFailures.push("runtime parity belongs to another app payload");
  }

  const modelFailures = [];
  if (campaign?.kind !== "desktoplab.agent-reliability-campaign" || campaign?.schemaVersion !== 3) {
    modelFailures.push("model campaign contract is invalid");
  }
  if (campaign?.status !== "pass") modelFailures.push("model reliability campaign did not pass");
  if (campaign?.candidateId !== candidate?.candidateId) modelFailures.push("model campaign belongs to another candidate");
  if (campaign?.appHash !== `sha256:${candidate?.payload?.sha256 ?? ""}`) {
    modelFailures.push("model campaign belongs to another app payload");
  }
  const executorFailures = validateExecutorProvenance(campaign?.executor);
  if (executorFailures.length > 0) modelFailures.push(...executorFailures.map((failure) => `model campaign executor provenance invalid: ${failure}`));
  if (campaign?.executor?.schemaVersion !== 2 || campaign?.executor?.id !== "recorded-agent-reliability-driver.mjs" || campaign?.executor?.sha256 !== expectedExecutorSha256) {
    modelFailures.push("model campaign was not produced by the versioned reliability verifier");
  }
  if (campaign?.executor?.bundleSha256 !== expectedExecutorBundleSha256) {
    modelFailures.push("model campaign reliability verifier dependency bundle does not match release HEAD");
  }
  if ((campaign?.completedRunCount ?? 0) < 15 || campaign?.completedRunCount !== campaign?.plannedRunCount) {
    modelFailures.push("model campaign requires at least 15 completed planned runs");
  }
  if ((campaign?.metrics?.passRate ?? 0) < 0.9) modelFailures.push("model campaign pass rate is below 0.90");
  const outcomes = campaign?.metrics?.outcomes ?? {};
  if ((outcomes.timeout ?? 0) > 0 || (outcomes.cancelled ?? 0) > 0 || (outcomes.agent_failure ?? 0) > 0) {
    modelFailures.push("model campaign contains agent, timeout or cancellation failures");
  }

  const runs = campaign?.runs ?? [];
  const completedAgentRuns = runs.filter((run) => completedAgentStatuses.has(run.status));
  const infrastructureFailureCount = outcomes.infrastructure_failure ?? 0;
  const warnings = infrastructureFailureCount > 0 ? [`model campaign retained ${infrastructureFailureCount} infrastructure failure without classifying it as an agent failure`] : [];
  if (runs.some((run) => run.candidateId !== candidate?.candidateId)) {
    modelFailures.push("model campaign contains runs from another candidate");
  }
  if (completedAgentRuns.some((run) => run.provenance?.uiDriverSha256 !== expectedUiDriverSha256)) {
    modelFailures.push("model campaign contains runs from an unversioned UI driver");
  }
  if (completedAgentRuns.some((run) => run.provenance?.uiDriverBundleSha256 !== expectedUiDriverBundleSha256)) {
    modelFailures.push("model campaign contains runs from an unattested UI driver dependency bundle");
  }
  for (const caseId of requiredCases) {
    const cases = completedAgentRuns.filter((run) => run.caseId === caseId);
    const passes = cases.filter((run) => run.status === "pass").length;
    if (cases.length < 3) modelFailures.push(`${caseId} requires at least three completed agent runs`);
    if (zeroToleranceCases.has(caseId) && passes !== cases.length) {
      modelFailures.push(`${caseId} is a zero-tolerance release capability`);
    } else if (!zeroToleranceCases.has(caseId) && cases.length > 0 && passes / cases.length < 2 / 3) {
      modelFailures.push(`${caseId} pass rate is below two thirds`);
    }
  }

  const failures = [...runtimeFailures, ...modelFailures];
  return {
    kind: "desktoplab.agent-release-gates",
    schemaVersion: 2,
    status: failures.length === 0 ? "pass" : "fail",
    candidateId: candidate?.candidateId ?? null,
    runtimeGate: {
      status: runtimeFailures.length === 0 ? "pass" : "fail",
      failures: runtimeFailures,
    },
    modelGate: {
      status: modelFailures.length === 0 ? "pass" : "fail",
      model: campaign?.configuration?.model ?? null,
      configurationFingerprint: campaign?.configurationFingerprint ?? null,
      completedRunCount: campaign?.completedRunCount ?? 0,
      completedAgentRunCount: completedAgentRuns.length,
      infrastructureFailureCount,
      passRate: campaign?.metrics?.passRate ?? null,
      warnings,
      failures: modelFailures,
    },
    failures,
  };
}
