import { createHash } from "node:crypto";
import { executableCaseSpecs, scoreExecutableCase } from "./agent-trace-score-core.mjs";
import { classifyAgentFailure } from "./agent-failure-taxonomy.mjs";
import { compareAgentConfigurations, fingerprintAgentConfiguration } from "./agent-configuration-fingerprint.mjs";
import { normalizeIsolation, sanitizedIsolation, sanitizedProvenance, sanitizeExecutorProvenance, validateExecutorProvenance, validateRunEvidence } from "./agent-reliability-evidence.mjs";

const terminalStatuses = new Set(["pass", "failed", "partial", "blocked", "timeout", "cancelled", "infrastructure_failure"]);
const digestPattern = /^sha256:[a-f0-9]{64}$/i;

export async function runReliabilityCampaign(manifest, { executor, executorProvenance } = {}) {
  const manifestFailures = validateManifest(manifest);
  const executorFailures = validateExecutorProvenance(executorProvenance);
  if (manifestFailures.length > 0 || executorFailures.length > 0 || typeof executor !== "function") {
    return blockedReport(manifest, [
      ...manifestFailures,
      ...executorFailures,
      ...(typeof executor === "function" ? [] : ["campaign executor missing"]),
    ]);
  }
  const descriptors = buildRunDescriptors(manifest);
  const runs = [];
  for (const descriptor of descriptors) {
    runs.push(await executeRun(descriptor, executor));
  }
  const isolationFailures = isolationViolations(runs);
  const scoredRuns = runs.filter((run) => ["pass", "failed", "partial", "blocked"].includes(run.status));
  const passCount = runs.filter((run) => run.status === "pass").length;
  const passRate = runs.length === 0 ? 0 : passCount / runs.length;
  const scores = scoredRuns.map((run) => run.score);
  const minimumPassRate = manifest.minimumPassRate ?? 1;
  const configuration = fingerprintAgentConfiguration(manifest.configuration);
  const comparison = manifest.baselineConfiguration
    ? compareAgentConfigurations(manifest.baselineConfiguration, manifest.configuration)
    : null;
  const failures = [
    ...isolationFailures,
    ...(passRate >= minimumPassRate ? [] : [`pass rate ${passRate.toFixed(4)} below ${minimumPassRate.toFixed(4)}`]),
    ...(runs.some((run) => !terminalStatuses.has(run.status)) ? ["unknown run status observed"] : []),
    ...(comparison?.comparison === "non_comparable_drift"
      ? [`configuration drift is not comparable: ${comparison.changedFactors.join(", ")}`]
      : []),
  ];
  return {
    kind: "desktoplab.agent-reliability-campaign",
    schemaVersion: 3,
    status: failures.length === 0 ? "pass" : "fail",
    campaignId: manifest.campaignId,
    candidateId: manifest.candidateId ?? null,
    appHash: manifest.appHash ?? null,
    manifestDigest: digest(stableJson(manifest)),
    configurationFingerprint: configuration.fingerprint,
    configuration: configuration.canonical,
    configurationComparison: comparison,
    executor: sanitizeExecutorProvenance(executorProvenance),
    plannedRunCount: descriptors.length,
    completedRunCount: runs.length,
    metrics: {
      passCount,
      passRate,
      passAll: passCount === runs.length,
      passPowerK: passRate ** runs.length,
      worstOfN: scores.length === 0 ? null : Math.min(...scores),
      meanScore: mean(scores),
      scoreDispersion: standardDeviation(scores),
      passRateConfidence95: wilson(passCount, runs.length),
      outcomes: outcomeCounts(runs),
    },
    runs,
    failures,
  };
}

export function buildRunDescriptors(manifest) {
  const descriptors = [];
  const configuration = fingerprintAgentConfiguration(manifest.configuration).canonical;
  for (const caseId of manifest.cases) {
    for (const seed of manifest.seeds) {
      for (let repetition = 1; repetition <= manifest.repetitions; repetition += 1) {
        const profileId = manifest.profilesBySeed?.[String(seed)] ?? "standard";
        const runId = digest(`${manifest.candidateId ?? "unbound"}\0${manifest.campaignId}\0${caseId}\0${seed}\0${profileId}\0${repetition}\0${stableJson(configuration)}`).slice(7, 23);
        descriptors.push({
          runId: `run-${runId}`,
          candidateId: manifest.candidateId ?? null,
          appHash: manifest.appHash ?? null,
          caseId,
          seed,
          profileId,
          repetition,
          timeoutMs: manifest.timeoutMs,
          campaignId: manifest.campaignId,
          configuration,
        });
      }
    }
  }
  return descriptors;
}

async function executeRun(descriptor, executor) {
  const startedAt = Date.now();
  try {
    const output = await executor(descriptor);
    if (["timeout", "cancelled", "infrastructure_failure"].includes(output?.status)) {
      return operationalRun(descriptor, output, startedAt);
    }
    const canonicalIsolation = normalizeIsolation(output?.isolation);
    const evidenceFailures = validateRunEvidence(descriptor, output, canonicalIsolation);
    const scored = scoreExecutableCase(descriptor.caseId, output);
    const status = evidenceFailures.length === 0 ? scored.status : "blocked";
    const classification = status === "pass" ? null : classifyAgentFailure({
      status,
      scoreResult: scored,
      trace: output?.trace,
      verification: output?.verification,
      originalStopReason: output?.stopReason,
    });
    return {
      ...descriptor,
      status,
      score: evidenceFailures.length === 0 ? scored.score : 0,
      completion: evidenceFailures.length === 0 ? scored.completion : 0,
      trajectory: evidenceFailures.length === 0 ? scored.trajectory : 0,
      durationMs: traceDuration(output?.trace) ?? elapsed(startedAt),
      isolation: sanitizedIsolation(canonicalIsolation, digest),
      provenance: sanitizedProvenance(output?.provenance),
      traceDigest: digest(stableJson(output?.trace ?? null)),
      verifierDigest: digest(stableJson(output?.verification ?? null)),
      classification,
      failures: [...scored.failures, ...evidenceFailures],
    };
  } catch (error) {
    return operationalRun(
      descriptor,
      { status: "infrastructure_failure", reason: error instanceof Error ? error.message : String(error) },
      startedAt,
    );
  }
}

function operationalRun(descriptor, output, startedAt) {
  const classification = classifyAgentFailure({
    status: output.status,
    trace: output.trace,
    verification: output.verification,
    originalStopReason: output.reason,
  });
  return {
    ...descriptor,
    status: output.status,
    score: null,
    completion: null,
    trajectory: null,
    durationMs: elapsed(startedAt),
    isolation: sanitizedIsolation(output.isolation, digest),
    traceDigest: null,
    verifierDigest: null,
    classification,
    failures: [boundedReason(output.reason ?? output.status)],
  };
}

function validateManifest(manifest) {
  const failures = [];
  if (manifest?.kind !== "desktoplab.agent-reliability-manifest" || manifest?.schemaVersion !== 1) failures.push("unsupported campaign manifest");
  if (!nonEmpty(manifest?.campaignId)) failures.push("campaignId missing");
  if (!digestPattern.test(manifest?.candidateId ?? "")) failures.push("candidateId digest missing");
  if (!digestPattern.test(manifest?.appHash ?? "")) failures.push("appHash digest missing");
  if (!Array.isArray(manifest?.cases) || manifest.cases.length === 0 || manifest.cases.some((id) => !executableCaseSpecs[id])) failures.push("campaign cases invalid");
  if (!Array.isArray(manifest?.seeds) || manifest.seeds.length === 0 || manifest.seeds.some((seed) => !Number.isInteger(seed))) failures.push("integer seeds required");
  if (manifest?.profilesBySeed != null) {
    if (!manifest.profilesBySeed || typeof manifest.profilesBySeed !== "object" || Array.isArray(manifest.profilesBySeed)) failures.push("profilesBySeed invalid");
    for (const seed of manifest?.seeds ?? []) {
      if (!nonEmpty(manifest.profilesBySeed?.[String(seed)])) failures.push(`profile missing for seed ${seed}`);
    }
  }
  if (!Number.isInteger(manifest?.repetitions) || manifest.repetitions < 1 || manifest.repetitions > 100) failures.push("repetitions must be between 1 and 100");
  if (!Number.isInteger(manifest?.timeoutMs) || manifest.timeoutMs < 1_000) failures.push("timeoutMs must be at least 1000");
  const configuration = fingerprintAgentConfiguration(manifest?.configuration);
  failures.push(...configuration.failures.map((failure) => `configuration: ${failure}`));
  if (manifest?.baselineConfiguration) {
    const baseline = fingerprintAgentConfiguration(manifest.baselineConfiguration);
    failures.push(...baseline.failures.map((failure) => `baselineConfiguration: ${failure}`));
  }
  if (manifest?.minimumPassRate != null && (!Number.isFinite(manifest.minimumPassRate) || manifest.minimumPassRate < 0 || manifest.minimumPassRate > 1)) failures.push("minimumPassRate must be between 0 and 1");
  return failures;
}

function isolationViolations(runs) {
  const failures = [];
  for (const field of ["workspacePath", "statePath"]) {
    const seen = new Map();
    for (const run of runs) {
      const value = run.isolation?.[field];
      if (!nonEmpty(value)) {
        failures.push(`${run.runId}: isolation ${field} missing`);
      } else if (seen.has(value)) {
        failures.push(`${run.runId}: isolation ${field} reused from ${seen.get(value)}`);
      } else {
        seen.set(value, run.runId);
      }
    }
  }
  for (const field of ["workspaceId", "sessionId"]) {
    for (const run of runs) {
      const value = run.isolation?.[field];
      if (!nonEmpty(value)) failures.push(`${run.runId}: isolation ${field} missing`);
    }
  }
  const traceDigests = new Map();
  for (const run of runs) {
    if (traceDigests.has(run.traceDigest)) {
      failures.push(`${run.runId}: trace evidence reused from ${traceDigests.get(run.traceDigest)}`);
    } else if (run.traceDigest) {
      traceDigests.set(run.traceDigest, run.runId);
    }
  }
  return failures;
}

function outcomeCounts(runs) {
  return Object.fromEntries([...terminalStatuses].map((status) => [status, runs.filter((run) => run.status === status).length]));
}

function wilson(successes, total) {
  if (total === 0) return { low: null, high: null };
  const z = 1.959963984540054;
  const p = successes / total;
  const denominator = 1 + (z * z) / total;
  const center = (p + (z * z) / (2 * total)) / denominator;
  const margin = z * Math.sqrt((p * (1 - p) + (z * z) / (4 * total)) / total) / denominator;
  return { low: Math.max(0, center - margin), high: Math.min(1, center + margin) };
}

function blockedReport(manifest, failures) {
  return { kind: "desktoplab.agent-reliability-campaign", schemaVersion: 3, status: "blocked", campaignId: manifest?.campaignId ?? null, candidateId: manifest?.candidateId ?? null, appHash: manifest?.appHash ?? null, executor: null, plannedRunCount: 0, completedRunCount: 0, metrics: null, runs: [], failures };
}

function stableJson(value) {
  if (Array.isArray(value)) return `[${value.map(stableJson).join(",")}]`;
  if (value && typeof value === "object") return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${stableJson(value[key])}`).join(",")}}`;
  return JSON.stringify(value);
}

function digest(value) {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
}

function boundedReason(value) {
  return String(value).replace(/\/Users\/[^\s]+|[A-Za-z]:\\Users\\[^\s]+/g, "[PATH_REDACTED]").slice(0, 240);
}

function elapsed(startedAt) {
  return Math.max(0, Date.now() - startedAt);
}

function traceDuration(trace) {
  const events = Array.isArray(trace?.events) ? trace.events : [];
  if (events.length < 2) return null;
  return Math.max(0, events.at(-1).recordedAtUnixMs - events[0].recordedAtUnixMs);
}

function mean(values) {
  return values.length === 0 ? null : values.reduce((sum, value) => sum + value, 0) / values.length;
}

function standardDeviation(values) {
  if (values.length === 0) return null;
  const average = mean(values);
  return Math.sqrt(mean(values.map((value) => (value - average) ** 2)));
}

function nonEmpty(value) {
  return typeof value === "string" && value.trim().length > 0;
}
