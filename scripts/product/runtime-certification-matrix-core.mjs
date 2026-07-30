const digestPattern = /^sha256:[a-f0-9]{64}$/;
const headPattern = /^[a-f0-9]{40}$/;

export const requiredRuntimeRoutes = Object.freeze([
  { key: "ollama_managed", runtimeId: "runtime.ollama", ownership: "desktoplab_managed" },
  { key: "lm_studio_existing", runtimeId: "runtime.lm-studio", ownership: "user_owned" },
  { key: "lm_studio_managed", runtimeId: "runtime.lm-studio", ownership: "desktoplab_managed" },
  { key: "mlx_lm_managed", runtimeId: "runtime.mlx-lm", ownership: "desktoplab_managed" },
]);

export function assessRuntimeCertificationMatrix({
  candidate,
  appHash,
  sourceHead,
  reports = {},
  reportDigests = {},
}) {
  const failures = validateCandidate(candidate, appHash, sourceHead);
  const target = platformTarget(candidate?.payload?.platform);
  if (!target) failures.push("candidate platform is not supported by the runtime matrix");

  const routes = requiredRuntimeRoutes.map((contract) =>
    assessRoute(contract, {
      report: reports[contract.key],
      reportDigest: reportDigests[contract.key],
      candidate,
      appHash,
      sourceHead,
      target,
    }),
  );
  failures.push(...routes.flatMap((route) => route.failures));
  const digests = routes.map((route) => route.reportSha256).filter(Boolean);
  if (new Set(digests).size !== digests.length) failures.push("runtime certification reports must be distinct");

  return {
    kind: "desktoplab.runtime-certification-matrix",
    schemaVersion: 1,
    status: failures.length === 0 ? "pass" : "blocked",
    publicSupportClaim: failures.length === 0,
    candidateId: validDigest(candidate?.candidateId),
    appHash: validDigest(appHash),
    sourceHead: validHead(sourceHead),
    target,
    routes: routes.map(({ failures: routeFailures, ...route }) => ({
      ...route,
      status: routeFailures.length === 0 ? "pass" : "blocked",
    })),
    failures: [...new Set(failures)],
  };
}

function assessRoute(contract, { report, reportDigest, candidate, appHash, sourceHead, target }) {
  const failures = [];
  if (report?.kind !== "desktoplab.runtime-certification" || report?.schemaVersion !== 1) {
    failures.push(`${contract.key}: certification contract is invalid`);
  }
  if (report?.status !== "pass" || report?.publicSupportClaim !== true) {
    failures.push(`${contract.key}: live public-support certification did not pass`);
  }
  if (report?.evidenceClass !== "live_installed_app"
    || report?.distinctions?.liveRuntimeCertification !== true) {
    failures.push(`${contract.key}: evidence is not live installed-app runtime certification`);
  }
  for (const [actual, expected, label] of [
    [report?.candidateId, candidate?.candidateId, "candidateId"],
    [report?.appHash, appHash, "appHash"],
    [report?.sourceHead, sourceHead, "sourceHead"],
    [report?.scope?.platform, target?.platform, "platform"],
    [report?.scope?.architecture, target?.architecture, "architecture"],
    [report?.scope?.runtimeId, contract.runtimeId, "runtimeId"],
    [report?.scope?.ownership, contract.ownership, "ownership"],
  ]) {
    if (actual !== expected) failures.push(`${contract.key}: ${label} does not match the required route`);
  }
  if (!safeText(report?.scope?.runtimeVersion)
    || !safeText(report?.scope?.modelId)
    || !safeText(report?.scope?.modelRevision)) {
    failures.push(`${contract.key}: exact runtime and model identity are missing`);
  }
  if (!Array.isArray(report?.checks)
    || report.checks.length !== 10
    || report.checks.some((check) => check?.status !== "pass")) {
    failures.push(`${contract.key}: lifecycle proof set is incomplete`);
  }
  if (!digestPattern.test(reportDigest ?? "")) failures.push(`${contract.key}: report digest is invalid`);
  return {
    key: contract.key,
    runtimeId: contract.runtimeId,
    ownership: contract.ownership,
    runtimeVersion: safeText(report?.scope?.runtimeVersion),
    modelId: safeText(report?.scope?.modelId),
    modelRevision: safeText(report?.scope?.modelRevision),
    reportSha256: validDigest(reportDigest),
    failures,
  };
}

function validateCandidate(candidate, appHash, sourceHead) {
  const failures = [];
  if (candidate?.kind !== "desktoplab.release-candidate" || candidate?.schemaVersion !== 1) {
    failures.push("candidate contract is invalid");
  }
  if (candidate?.state !== "payload_built") failures.push("runtime matrix requires a payload_built candidate");
  if (!digestPattern.test(candidate?.candidateId ?? "")) failures.push("candidateId is invalid");
  if (!digestPattern.test(appHash ?? "") || appHash !== `sha256:${candidate?.payload?.sha256}`) {
    failures.push("installed app differs from the candidate payload");
  }
  if (!headPattern.test(sourceHead ?? "") || sourceHead !== candidate?.source?.commit) {
    failures.push("source HEAD differs from the candidate source");
  }
  if (candidate?.source?.treeState !== "clean") failures.push("candidate source was not clean");
  return failures;
}

function platformTarget(platform) {
  if (platform === "macos-aarch64") return { platform: "macos", architecture: "arm64" };
  return null;
}

function validDigest(value) {
  return digestPattern.test(value ?? "") ? value : null;
}

function validHead(value) {
  return headPattern.test(value ?? "") ? value : null;
}

function safeText(value) {
  return typeof value === "string" && value.length > 0 ? value : null;
}
