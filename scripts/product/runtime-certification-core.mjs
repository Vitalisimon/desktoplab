const digestPattern = /^sha256:[a-f0-9]{64}$/;
const allowedRuntimeIds = new Set(["runtime.ollama", "runtime.lm-studio", "runtime.mlx-lm"]);
const requiredChecks = [
  "install_or_connect",
  "restart_recovery",
  "stale_process_recovery",
  "first_model_response",
  "protocol_tool_canary",
  "desktoplab_tool_execution",
  "approval_boundary",
  "user_owned_process_preservation",
  "egress_observation",
  "cleanup_ownership",
];

export function runtimeCertificationTemplate(expected, evidenceKind = "live_installed_app") {
  return {
    kind: "desktoplab.runtime-certification-evidence",
    schemaVersion: 1,
    evidenceKind,
    generatedAt: "",
    candidateId: expected?.candidateId ?? "",
    appHash: expected?.appHash ?? "",
    sourceHead: expected?.sourceHead ?? "",
    host: {
      platform: expected?.platform ?? "",
      architecture: expected?.architecture ?? "",
      hostIdSha256: "",
    },
    runtime: {
      id: expected?.runtimeId ?? "",
      version: "",
      ownership: expected?.ownership ?? "",
      source: "",
      integrity: "",
      endpoint: "",
    },
    model: {
      id: expected?.modelId ?? "",
      revision: expected?.modelRevision ?? "",
      license: "",
      quantization: "",
    },
    checks: Object.fromEntries(
      requiredChecks.map((id) => [id, { status: "not_run", evidenceSha256: "" }]),
    ),
    egress: { observed: false, destinations: [] },
    artifacts: [],
  };
}

export function assessRuntimeCertification(expected, evidence) {
  const failures = validateExpected(expected);
  failures.push(...validateEnvelope(expected, evidence));
  const checks = requiredChecks.map((id) => assessCheck(id, evidence?.checks?.[id]));
  failures.push(...checks.flatMap((check) => check.failures));
  if (evidence?.egress?.observed !== true) failures.push("egress observation is missing");
  if (!safeDestinations(evidence?.egress?.destinations)) failures.push("egress destinations are invalid");
  if (!validArtifacts(evidence?.artifacts)) failures.push("artifact digest set is invalid");

  const deterministicPass =
    failures.length === 0 && evidence?.evidenceKind === "deterministic_adapter";
  const livePass = failures.length === 0 && evidence?.evidenceKind === "live_installed_app";
  return {
    kind: "desktoplab.runtime-certification",
    schemaVersion: 1,
    status: livePass ? "pass" : deterministicPass ? "deterministic_pass" : "blocked",
    publicSupportClaim: livePass,
    evidenceClass: evidence?.evidenceKind ?? null,
    candidateId: safeDigest(evidence?.candidateId),
    appHash: safeDigest(evidence?.appHash),
    sourceHead: safeHead(evidence?.sourceHead),
    scope: {
      platform: safeIdentifier(evidence?.host?.platform),
      architecture: safeIdentifier(evidence?.host?.architecture),
      runtimeId: allowedRuntimeIds.has(evidence?.runtime?.id) ? evidence.runtime.id : null,
      runtimeVersion: safeVersion(evidence?.runtime?.version),
      ownership: safeOwnership(evidence?.runtime?.ownership),
      modelId: safeModelId(evidence?.model?.id),
      modelRevision: safeRevision(evidence?.model?.revision),
    },
    checks,
    distinctions: {
      deterministicAdapter: evidence?.evidenceKind === "deterministic_adapter",
      liveRuntimeCertification: livePass,
      installedAppCanary: false,
      agenticReliability: false,
      releaseReadiness: false,
    },
    failures: [...new Set(failures)],
  };
}

function validateExpected(expected) {
  const failures = [];
  if (expected?.kind !== "desktoplab.runtime-certification-expected" || expected?.schemaVersion !== 1) {
    failures.push("expected certification contract is invalid");
  }
  if (!digestPattern.test(expected?.candidateId ?? "")) failures.push("expected candidateId is invalid");
  if (!digestPattern.test(expected?.appHash ?? "")) failures.push("expected appHash is invalid");
  if (!safeHead(expected?.sourceHead)) failures.push("expected sourceHead is invalid");
  if (!allowedRuntimeIds.has(expected?.runtimeId)) failures.push("expected runtimeId is invalid");
  if (!safeIdentifier(expected?.platform) || !safeIdentifier(expected?.architecture)) {
    failures.push("expected platform or architecture is invalid");
  }
  if (!safeModelId(expected?.modelId) || !safeRevision(expected?.modelRevision)) {
    failures.push("expected model identity is invalid");
  }
  if (!safeOwnership(expected?.ownership)) failures.push("expected ownership is invalid");
  return failures;
}

function validateEnvelope(expected, evidence) {
  const failures = [];
  if (
    evidence?.kind !== "desktoplab.runtime-certification-evidence" ||
    evidence?.schemaVersion !== 1
  ) {
    failures.push("runtime certification evidence contract is invalid");
  }
  if (!["deterministic_adapter", "live_installed_app"].includes(evidence?.evidenceKind)) {
    failures.push("runtime certification evidence class is invalid");
  }
  for (const [actual, wanted, label] of [
    [evidence?.candidateId, expected?.candidateId, "candidateId"],
    [evidence?.appHash, expected?.appHash, "appHash"],
    [evidence?.sourceHead, expected?.sourceHead, "sourceHead"],
    [evidence?.host?.platform, expected?.platform, "platform"],
    [evidence?.host?.architecture, expected?.architecture, "architecture"],
    [evidence?.runtime?.id, expected?.runtimeId, "runtimeId"],
    [evidence?.runtime?.ownership, expected?.ownership, "ownership"],
    [evidence?.model?.id, expected?.modelId, "modelId"],
    [evidence?.model?.revision, expected?.modelRevision, "modelRevision"],
  ]) {
    if (actual !== wanted) failures.push(`${label} does not match expected certification scope`);
  }
  if (!digestPattern.test(evidence?.host?.hostIdSha256 ?? "")) failures.push("host identity digest missing");
  if (!safeVersion(evidence?.runtime?.version)) failures.push("runtime version missing");
  if (!safeSource(evidence?.runtime?.source)) failures.push("runtime source is invalid");
  if (!digestPattern.test(evidence?.runtime?.integrity ?? "")) failures.push("runtime integrity missing");
  if (!loopbackEndpoint(evidence?.runtime?.endpoint)) failures.push("runtime endpoint is not loopback");
  if (!safeLicense(evidence?.model?.license)) failures.push("model license is invalid");
  if (!safeVersion(evidence?.model?.quantization)) failures.push("model quantization missing");
  return failures;
}

function assessCheck(id, check) {
  const failures = [];
  if (check?.status !== "pass") failures.push(`${id}: status is not pass`);
  if (!digestPattern.test(check?.evidenceSha256 ?? "")) failures.push(`${id}: evidence digest missing`);
  return { id, status: failures.length === 0 ? "pass" : "blocked", failures };
}

function validArtifacts(artifacts) {
  return (
    Array.isArray(artifacts) &&
    artifacts.length > 0 &&
    artifacts.every(
      (artifact) =>
        safeIdentifier(artifact?.kind) &&
        digestPattern.test(artifact?.sha256 ?? "") &&
        safeRelativePath(artifact?.path),
    )
  );
}

function safeDestinations(destinations) {
  return (
    Array.isArray(destinations) &&
    destinations.length > 0 &&
    destinations.every(
      (destination) =>
        safeIdentifier(destination?.purpose) &&
        typeof destination?.host === "string" &&
        /^[a-zA-Z0-9.-]{1,253}$/.test(destination.host),
    )
  );
}

function safeRelativePath(value) {
  return (
    typeof value === "string" &&
    !value.startsWith("/") &&
    !value.includes("\\") &&
    value.split("/").every((part) => part && part !== "." && part !== "..")
  );
}

function loopbackEndpoint(value) {
  return typeof value === "string" && /^http:\/\/(127\.0\.0\.1|localhost):[1-9][0-9]{0,4}$/.test(value);
}

function safeDigest(value) {
  return digestPattern.test(value ?? "") ? value : null;
}

function safeHead(value) {
  return typeof value === "string" && /^[a-f0-9]{40}$/.test(value) ? value : null;
}

function safeIdentifier(value) {
  return typeof value === "string" && /^[a-zA-Z0-9._-]{1,128}$/.test(value) ? value : null;
}

function safeVersion(value) {
  return typeof value === "string" && /^[a-zA-Z0-9._+()-]{1,128}$/.test(value) ? value : null;
}

function safeOwnership(value) {
  return ["desktoplab_managed", "user_owned"].includes(value) ? value : null;
}

function safeModelId(value) {
  return typeof value === "string" && /^[a-zA-Z0-9._/-]{1,200}$/.test(value) && !value.includes("..")
    ? value
    : null;
}

function safeRevision(value) {
  return typeof value === "string" && /^[a-f0-9]{40,64}$/.test(value) ? value : null;
}

function safeLicense(value) {
  return typeof value === "string" && /^[a-zA-Z0-9.-]{1,64}$/.test(value) ? value : null;
}

function safeSource(value) {
  try {
    return ["https:", "file:"].includes(new URL(value).protocol);
  } catch {
    return false;
  }
}
