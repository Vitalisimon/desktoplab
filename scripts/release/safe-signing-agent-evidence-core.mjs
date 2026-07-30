import { assertCandidate } from "./candidate-admission-core.mjs";

const digest = /^sha256:[a-f0-9]{64}$/;

export function assessSafeSigningAgentEvidence(context) {
  const {
    candidate, appSha256, appBuild, certification, runtime, releaseGates,
    currentHead, treeState, certificationSha256, campaignSha256,
    expectedInstalledUiDriverSha256, expectedInstalledUiDriverBundleSha256,
  } = context;
  const failures = [];
  try {
    assertCandidate(candidate);
  } catch (error) {
    failures.push(error.message);
  }
  if (candidate?.state !== "payload_built") failures.push("candidate must be in payload_built state");
  if (currentHead !== candidate?.source?.commit) failures.push("candidate differs from current public HEAD");
  if (treeState !== "clean") failures.push("verified reuse requires a clean source tree");
  if (appSha256 !== candidate?.payload?.sha256) failures.push("installed app differs from prepared payload");
  if (appBuild?.commitSha !== candidate?.source?.commit) failures.push("installed app build differs from candidate source");
  if (appBuild?.channel !== candidate?.release?.channel) failures.push("installed app channel differs from candidate");
  if (appBuild?.treeState !== "clean") failures.push("installed app was not built from a clean tree");
  if (JSON.stringify(appBuild?.lockfiles) !== JSON.stringify(candidate?.lockfiles)) {
    failures.push("installed app lockfiles differ from candidate");
  }

  if (certification?.kind !== "desktoplab.installed-agent-certification" || certification?.schemaVersion !== 3) {
    failures.push("installed-agent certification contract is invalid");
  }
  if (certification?.status !== "pass" || certification?.liveClaim !== true) {
    failures.push("installed-agent certification did not pass");
  }
  if (certification?.deterministicEvidenceAccepted !== false) {
    failures.push("deterministic evidence cannot authorize verified reuse");
  }
  const provenance = certification?.provenance;
  if (provenance?.candidateId !== candidate?.candidateId) failures.push("installed-agent certification belongs to another candidate");
  if (provenance?.appHash !== `sha256:${appSha256}`) failures.push("installed-agent certification belongs to another app payload");
  if (provenance?.head !== currentHead || provenance?.appBuild?.commitSha !== currentHead) {
    failures.push("installed-agent certification belongs to another source revision");
  }
  if (provenance?.executionKind !== "installed_app_ui") failures.push("installed-agent certification is not live UI evidence");
  if (provenance?.uiDriverSha256 !== expectedInstalledUiDriverSha256
    || provenance?.uiDriverBundleSha256 !== expectedInstalledUiDriverBundleSha256) {
    failures.push("installed-agent certification UI driver differs from release HEAD");
  }
  if ((provenance?.localModelRequestCount ?? 0) < 1) failures.push("installed-agent certification lacks local model requests");
  if ((provenance?.realToolExecutionCount ?? 0) < 1) failures.push("installed-agent certification lacks real tool executions");
  if (provenance?.testControlRequests !== 0) failures.push("installed-agent certification used test controls");

  if (runtime?.kind !== "desktoplab.measured-agent-parity" || runtime?.schemaVersion !== 1
    || runtime?.status !== "pass" || runtime?.controlPlane?.status !== "pass") {
    failures.push("recomputed measured parity did not pass");
  }
  if (runtime?.provenance?.candidateId !== candidate?.candidateId
    || runtime?.provenance?.appHash !== `sha256:${appSha256}`) {
    failures.push("recomputed measured parity belongs to another candidate payload");
  }
  if (releaseGates?.kind !== "desktoplab.agent-release-gates" || releaseGates?.schemaVersion !== 2
    || releaseGates?.status !== "pass" || releaseGates?.runtimeGate?.status !== "pass"
    || releaseGates?.modelGate?.status !== "pass") {
    failures.push("recomputed agent release gates did not pass");
  }
  if (releaseGates?.candidateId !== candidate?.candidateId) failures.push("agent release gates belong to another candidate");
  if (provenance?.modelId !== releaseGates?.modelGate?.model?.id
    || provenance?.quantization !== releaseGates?.modelGate?.model?.quantization) {
    failures.push("canary and reliability campaign use different model envelopes");
  }
  if (!digest.test(certificationSha256 ?? "") || !digest.test(campaignSha256 ?? "")) {
    failures.push("source evidence digests are invalid");
  }

  return {
    kind: "desktoplab.safe-signing-agent-evidence-import",
    schemaVersion: 1,
    status: failures.length === 0 ? "pass" : "fail",
    mode: "verified_reuse",
    candidateId: candidate?.candidateId ?? null,
    sourceCommit: currentHead ?? null,
    appHash: appSha256 ? `sha256:${appSha256}` : null,
    sources: { certificationSha256, campaignSha256 },
    derived: {
      runtimeStatus: runtime?.status ?? null,
      releaseGatesStatus: releaseGates?.status ?? null,
      warnings: releaseGates?.modelGate?.warnings ?? [],
    },
    failures,
  };
}
