import { createHash } from "node:crypto";

const digestPattern = /^sha256:[a-f0-9]{64}$/i;

export function fingerprintAgentConfiguration(configuration) {
  const failures = validateConfiguration(configuration);
  if (failures.length > 0) {
    return { status: "blocked", fingerprint: null, canonical: null, failures };
  }
  const canonical = canonicalConfiguration(configuration);
  return {
    status: "pass",
    fingerprint: digest(stableJson(canonical)),
    canonical,
    failures: [],
  };
}

export function compareAgentConfigurations(baseline, candidate) {
  const left = fingerprintAgentConfiguration(baseline);
  const right = fingerprintAgentConfiguration(candidate);
  if (left.status !== "pass" || right.status !== "pass") {
    return {
      status: "blocked",
      comparison: "not_available",
      changedFactors: [],
      failures: [...left.failures, ...right.failures],
    };
  }
  const leftFactors = comparableFactors(left.canonical);
  const rightFactors = comparableFactors(right.canonical);
  const changedFactors = Object.keys(leftFactors)
    .filter((key) => stableJson(leftFactors[key]) !== stableJson(rightFactors[key]))
    .sort();
  return {
    status: "pass",
    comparison: changedFactors.length === 0
      ? "directly_comparable"
      : changedFactors.length === 1
        ? "controlled_ab"
        : "non_comparable_drift",
    changedFactors,
    baselineFingerprint: left.fingerprint,
    candidateFingerprint: right.fingerprint,
    failures: [],
  };
}

function canonicalConfiguration(configuration) {
  return {
    schemaVersion: 2,
    model: {
      id: bounded(configuration.model.id),
      digest: configuration.model.digest.toLowerCase(),
      quantization: bounded(configuration.model.quantization),
    },
    runtime: {
      id: bounded(configuration.runtime.id),
      version: bounded(configuration.runtime.version),
      backendId: bounded(configuration.runtime.backendId),
      backendVersion: bounded(configuration.runtime.backendVersion),
    },
    toolSchemaDigest: configuration.toolSchemaDigest.toLowerCase(),
    approvalMode: bounded(configuration.approvalMode),
    contextPlan: {
      digest: configuration.contextPlan.digest.toLowerCase(),
      budgetTokens: configuration.contextPlan.budgetTokens,
      compactionPolicy: bounded(configuration.contextPlan.compactionPolicy),
    },
    adaptivePolicy: {
      digest: configuration.adaptivePolicy.digest.toLowerCase(),
      contextWindowTokens: configuration.adaptivePolicy.contextWindowTokens,
      requestTimeoutSeconds: configuration.adaptivePolicy.requestTimeoutSeconds,
      modelMaximumTokens: configuration.adaptivePolicy.modelMaximumTokens,
    },
    plugins: [...configuration.plugins]
      .map((plugin) => ({
        id: bounded(plugin.id),
        version: bounded(plugin.version),
        digest: plugin.digest.toLowerCase(),
      }))
      .sort((left, right) => left.id.localeCompare(right.id)),
    harnessVersion: bounded(configuration.harnessVersion),
    hostCapabilities: {
      os: configuration.hostCapabilities.os,
      arch: bounded(configuration.hostCapabilities.arch),
      memoryClassGb: configuration.hostCapabilities.memoryClassGb,
      acceleratorClass: bounded(configuration.hostCapabilities.acceleratorClass),
      gpuMemoryClassGb: configuration.hostCapabilities.gpuMemoryClassGb ?? null,
      unifiedMemory: configuration.hostCapabilities.unifiedMemory === true,
    },
  };
}

function validateConfiguration(configuration) {
  const failures = [];
  if (!configuration || typeof configuration !== "object") return ["configuration missing"];
  requiredString(failures, configuration.model?.id, "model.id");
  requiredDigest(failures, configuration.model?.digest, "model.digest");
  requiredString(failures, configuration.model?.quantization, "model.quantization");
  for (const field of ["id", "version", "backendId", "backendVersion"]) {
    requiredString(failures, configuration.runtime?.[field], `runtime.${field}`);
  }
  requiredDigest(failures, configuration.toolSchemaDigest, "toolSchemaDigest");
  requiredString(failures, configuration.approvalMode, "approvalMode");
  requiredDigest(failures, configuration.contextPlan?.digest, "contextPlan.digest");
  requiredString(failures, configuration.contextPlan?.compactionPolicy, "contextPlan.compactionPolicy");
  if (!Number.isInteger(configuration.contextPlan?.budgetTokens) || configuration.contextPlan.budgetTokens < 1) failures.push("contextPlan.budgetTokens invalid");
  requiredDigest(failures, configuration.adaptivePolicy?.digest, "adaptivePolicy.digest");
  for (const field of ["contextWindowTokens", "requestTimeoutSeconds", "modelMaximumTokens"]) {
    if (!Number.isInteger(configuration.adaptivePolicy?.[field]) || configuration.adaptivePolicy[field] < 1) failures.push(`adaptivePolicy.${field} invalid`);
  }
  if (!Array.isArray(configuration.plugins)) failures.push("plugins must be an array");
  const pluginIds = new Set();
  for (const [index, plugin] of (configuration.plugins ?? []).entries()) {
    requiredString(failures, plugin?.id, `plugins[${index}].id`);
    requiredString(failures, plugin?.version, `plugins[${index}].version`);
    requiredDigest(failures, plugin?.digest, `plugins[${index}].digest`);
    if (pluginIds.has(plugin?.id)) failures.push(`plugins[${index}].id duplicate`);
    pluginIds.add(plugin?.id);
  }
  requiredString(failures, configuration.harnessVersion, "harnessVersion");
  if (!configuration.hostCapabilities || !["darwin", "linux", "win32"].includes(configuration.hostCapabilities.os)) failures.push("hostCapabilities.os invalid");
  requiredString(failures, configuration.hostCapabilities?.arch, "hostCapabilities.arch");
  requiredString(failures, configuration.hostCapabilities?.acceleratorClass, "hostCapabilities.acceleratorClass");
  if (!Number.isFinite(configuration.hostCapabilities?.memoryClassGb) || configuration.hostCapabilities.memoryClassGb <= 0) failures.push("hostCapabilities.memoryClassGb invalid");
  if (configuration.hostCapabilities?.gpuMemoryClassGb != null && (!Number.isFinite(configuration.hostCapabilities.gpuMemoryClassGb) || configuration.hostCapabilities.gpuMemoryClassGb < 0)) failures.push("hostCapabilities.gpuMemoryClassGb invalid");
  if (typeof configuration.hostCapabilities?.unifiedMemory !== "boolean") failures.push("hostCapabilities.unifiedMemory invalid");
  return failures;
}

function comparableFactors(canonical) {
  return {
    model: canonical.model,
    runtime: canonical.runtime,
    toolSchema: canonical.toolSchemaDigest,
    approvalMode: canonical.approvalMode,
    contextPlan: canonical.contextPlan,
    adaptivePolicy: canonical.adaptivePolicy,
    plugins: canonical.plugins,
    harness: canonical.harnessVersion,
    hardware: canonical.hostCapabilities,
  };
}

function requiredString(failures, value, field) {
  if (typeof value !== "string" || value.trim().length === 0 || value.length > 160) failures.push(`${field} invalid`);
}

function requiredDigest(failures, value, field) {
  if (!digestPattern.test(value ?? "")) failures.push(`${field} invalid`);
}

function bounded(value) {
  return value.slice(0, 160);
}

function stableJson(value) {
  if (Array.isArray(value)) return `[${value.map(stableJson).join(",")}]`;
  if (value && typeof value === "object") return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${stableJson(value[key])}`).join(",")}}`;
  return JSON.stringify(value);
}

function digest(value) {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
}
