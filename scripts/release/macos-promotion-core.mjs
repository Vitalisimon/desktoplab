export function assessMacosPromotion({ candidate, certification, safeSigning, appHash, appBuild, currentHead }) {
  const failures = [];
  if (candidate?.kind !== "desktoplab.release-candidate" || candidate?.schemaVersion !== 1) {
    failures.push("candidate admission contract is invalid");
  }
  if (candidate?.state !== "pre_sign_pass") failures.push("candidate is not ready for signed promotion");
  if (candidate?.source?.commit !== currentHead) failures.push("candidate commit differs from current public HEAD");
  if (candidate?.source?.commit !== appBuild?.commitSha) failures.push("candidate commit differs from app metadata");
  if (candidate?.release?.channel !== appBuild?.channel) failures.push("candidate channel differs from app metadata");
  if (candidate?.payload?.sha256 !== appHash) failures.push("candidate app hash differs from prepared payload");
  if (JSON.stringify(candidate?.lockfiles) !== JSON.stringify(appBuild?.lockfiles)) {
    failures.push("candidate lock hashes differ from app metadata");
  }

  if (certification?.kind !== "desktoplab.installed-agent-certification" || certification?.schemaVersion !== 3) {
    failures.push("installed-agent certification contract is invalid");
  }
  if (certification?.status !== "pass" || certification?.liveClaim !== true) {
    failures.push("installed-agent certification did not pass");
  }
  if (certification?.provenance?.candidateId !== candidate?.candidateId) {
    failures.push("installed-agent certification belongs to another candidate");
  }
  if (certification?.provenance?.appHash !== `sha256:${appHash}`) {
    failures.push("installed-agent certification belongs to another app hash");
  }
  if (certification?.provenance?.appBuild?.commitSha !== appBuild?.commitSha) {
    failures.push("installed-agent certification app metadata is stale");
  }
  if (certification?.deterministicEvidenceAccepted !== false) {
    failures.push("deterministic evidence cannot authorize macOS promotion");
  }

  const safeRun = safeSigning?.runs?.at?.(-1);
  if (safeSigning?.kind !== "desktoplab.safe-signing-regression" || safeRun?.status !== "pass") {
    failures.push("safe-signing regression did not pass");
  }
  if (safeRun?.head !== currentHead || safeRun?.treeState !== "clean") {
    failures.push("safe-signing regression is stale or dirty");
  }
  if (safeRun?.candidateId !== candidate?.candidateId || safeRun?.preparedAppSha256 !== appHash) {
    failures.push("safe-signing regression belongs to another candidate payload");
  }
  if (safeRun?.steps?.find((step) => step.id === "installed-agent")?.status !== "passed") {
    failures.push("safe-signing regression lacks passing installed-agent evidence");
  }

  return {
    kind: "desktoplab.macos-promotion-admission",
    schemaVersion: 1,
    status: failures.length === 0 ? "pass" : "fail",
    candidateId: candidate?.candidateId ?? null,
    sourceCommit: currentHead ?? null,
    preparedAppSha256: appHash ?? null,
    failures,
  };
}
