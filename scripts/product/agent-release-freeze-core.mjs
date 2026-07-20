import { createHash } from "node:crypto";
import { readFileSync, statSync } from "node:fs";
import { relative, resolve } from "node:path";

const digestPattern = /^sha256:[a-f0-9]{64}$/i;

export function verifyAgentReleaseFreeze(freeze, { repoRoot, observed = null } = {}) {
  const failures = validateEnvelope(freeze);
  const sources = [];
  for (const entry of freeze?.sources ?? []) {
    const path = resolve(repoRoot, entry.path ?? "");
    const inside = relative(resolve(repoRoot), path);
    let actual = null;
    try {
      if (!inside || inside.startsWith("..") || !statSync(path).isFile()) throw new Error("invalid source path");
      actual = digest(readFileSync(path));
    } catch {
      failures.push(`${entry.path ?? "unknown"}: frozen source missing`);
    }
    if (actual && actual !== entry.sha256?.toLowerCase()) failures.push(`${entry.path}: frozen source drifted`);
    sources.push({ path: entry.path ?? null, expectedSha256: entry.sha256 ?? null, actualSha256: actual });
  }
  const localFailures = observed ? compareObserved(freeze, observed) : [];
  failures.push(...localFailures);
  return {
    kind: "desktoplab.agent-release-freeze-verification",
    schemaVersion: 1,
    status: failures.length === 0 ? "pass" : "fail",
    freezeDigest: digest(Buffer.from(stableJson(freeze))),
    sourceCount: sources.length,
    localObservationRequired: observed !== null,
    sources,
    failures,
  };
}

function validateEnvelope(freeze) {
  const failures = [];
  if (freeze?.kind !== "desktoplab.agent-release-freeze" || freeze?.schemaVersion !== 1) failures.push("unsupported agent release freeze");
  if (freeze?.state !== "frozen_for_reliability") failures.push("agent release freeze is not active");
  required(failures, freeze?.model?.id, "model.id");
  required(failures, freeze?.model?.runtimeRef, "model.runtimeRef");
  requiredDigest(failures, freeze?.model?.digest, "model.digest");
  requiredDigest(failures, freeze?.model?.capabilityFingerprint, "model.capabilityFingerprint");
  required(failures, freeze?.model?.quantization, "model.quantization");
  required(failures, freeze?.runtime?.id, "runtime.id");
  required(failures, freeze?.runtime?.version, "runtime.version");
  required(failures, freeze?.runtime?.protocol, "runtime.protocol");
  for (const field of ["contextWindowTokens", "requestTimeoutSeconds", "modelMaximumTokens", "hostMemoryGb"]) {
    if (!Number.isInteger(freeze?.adaptivePolicy?.[field]) || freeze.adaptivePolicy[field] < 1) failures.push(`adaptivePolicy.${field} invalid`);
  }
  if (!Array.isArray(freeze?.sources) || freeze.sources.length === 0) failures.push("frozen sources missing");
  const paths = new Set();
  for (const [index, entry] of (freeze?.sources ?? []).entries()) {
    required(failures, entry?.path, `sources[${index}].path`);
    requiredDigest(failures, entry?.sha256, `sources[${index}].sha256`);
    if (paths.has(entry?.path)) failures.push(`sources[${index}].path duplicate`);
    paths.add(entry?.path);
  }
  return failures;
}

function compareObserved(freeze, observed) {
  const failures = [];
  for (const [field, expected] of [
    ["model.id", freeze.model.id],
    ["model.runtimeRef", freeze.model.runtimeRef],
    ["model.digest", freeze.model.digest],
    ["model.capabilityFingerprint", freeze.model.capabilityFingerprint],
    ["model.quantization", freeze.model.quantization],
    ["runtime.id", freeze.runtime.id],
    ["runtime.version", freeze.runtime.version],
    ["runtime.protocol", freeze.runtime.protocol],
  ]) {
    const actual = field.split(".").reduce((value, key) => value?.[key], observed);
    if (String(actual ?? "").toLowerCase() !== String(expected).toLowerCase()) failures.push(`${field} differs from freeze`);
  }
  for (const field of ["contextWindowTokens", "requestTimeoutSeconds", "modelMaximumTokens", "hostMemoryGb"]) {
    if (observed.adaptivePolicy?.[field] !== freeze.adaptivePolicy[field]) failures.push(`adaptivePolicy.${field} differs from freeze`);
  }
  return failures;
}

function required(failures, value, field) {
  if (typeof value !== "string" || value.trim().length === 0 || value.length > 200) failures.push(`${field} invalid`);
}

function requiredDigest(failures, value, field) {
  if (!digestPattern.test(value ?? "")) failures.push(`${field} invalid`);
}

function digest(value) {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
}

function stableJson(value) {
  if (Array.isArray(value)) return `[${value.map(stableJson).join(",")}]`;
  if (value && typeof value === "object") return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${stableJson(value[key])}`).join(",")}}`;
  return JSON.stringify(value);
}
