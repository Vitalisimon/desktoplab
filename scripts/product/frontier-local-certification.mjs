#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { frontierCertificationCases } from "./live-agent-certification.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const HIGH_END_PROFILES = new Set(["custom_frontier_rig", "dgx_station_class"]);
const MAX_EVIDENCE_AGE_MS = 7 * 24 * 60 * 60 * 1000;

export function frontierEvidenceTemplate(now = new Date()) {
  return {
    kind: "desktoplab.frontier-local-live-evidence",
    schemaVersion: 1,
    evidenceKind: "live_installed_app",
    generatedAt: now.toISOString(),
    operator: "",
    app: { path: "/Applications/DesktopLab.app", commit: "", artifactSha256: "" },
    host: {
      profile: "",
      hostname: "",
      gpu: "",
      cpu: "",
      acceleratorMemoryBytes: 0,
      systemMemoryBytes: 0,
      storageFreeBytes: 0,
      driver: "",
    },
    runtime: { id: "", version: "", endpoint: "", ownership: "", health: "" },
    model: {
      id: "",
      sizeClass: "",
      quantization: "",
      contextLengthTokens: 0,
      license: "",
    },
    retrieval: {
      mode: "",
      indexGeneration: "",
      freshness: "",
      secretRedactionVerified: false,
    },
    cases: frontierCertificationCases.map((certCase) => ({
      id: certCase.id,
      status: "not_run",
      startedAt: "",
      completedAt: "",
      durationMs: 0,
      checks: Object.fromEntries(certCase.requiredChecks.map((check) => [check, false])),
      quality: { correctness: 0, grounding: 0, instructionFollowing: 0 },
      performance: { timeToFirstEventMs: 0, totalLatencyMs: 0, peakMemoryPressurePercent: 0 },
      artifacts: { transcript: "", screenshot: "", workspaceEvidence: "" },
      notes: "",
    })),
  };
}

export function buildFrontierCertificationReport({
  evidence = null,
  now = new Date(),
  exists = existsSync,
  maxEvidenceAgeMs = MAX_EVIDENCE_AGE_MS,
} = {}) {
  if (!evidence) return blockedReport(["missing live installed-app evidence"]);
  const failures = validateEnvelope(evidence, { now, exists, maxEvidenceAgeMs });
  const cases = frontierCertificationCases.map((definition) =>
    scoreCase(definition, evidence.cases?.find((candidate) => candidate.id === definition.id), exists),
  );
  failures.push(...cases.flatMap((certCase) => certCase.failures));
  const functionalPass = failures.length === 0 && cases.every((certCase) => certCase.functionalPass);
  const qualityScore = average(cases.map((certCase) => certCase.qualityScore));
  const performanceScore = average(cases.map((certCase) => certCase.performanceScore));
  const qualityPass = qualityScore >= 0.8;
  const performancePass = performanceScore >= 0.5;
  if (!qualityPass) failures.push(`quality score ${qualityScore.toFixed(2)} < 0.80`);
  if (!performancePass) failures.push(`performance score ${performanceScore.toFixed(2)} < 0.50`);
  const status = functionalPass && qualityPass && performancePass ? "pass" : "fail";
  return {
    kind: "desktoplab.frontier-local-certification",
    schemaVersion: 1,
    status,
    frontierLocalClaim: status === "pass",
    sourceEvidenceKind: evidence.evidenceKind ?? null,
    generatedAt: evidence.generatedAt ?? null,
    app: evidence.app ?? null,
    host: evidence.host ?? null,
    runtime: evidence.runtime ?? null,
    model: evidence.model ?? null,
    retrieval: evidence.retrieval ?? null,
    functional: { status: functionalPass ? "pass" : "fail", passed: cases.filter((item) => item.functionalPass).length, total: cases.length },
    quality: { status: qualityPass ? "pass" : "fail", score: qualityScore, threshold: 0.8 },
    performance: { status: performancePass ? "pass" : "fail", score: performanceScore, threshold: 0.5 },
    cases,
    failures: [...new Set(failures)],
  };
}

function validateEnvelope(evidence, { now, exists, maxEvidenceAgeMs }) {
  const failures = [];
  if (evidence.kind !== "desktoplab.frontier-local-live-evidence") failures.push("invalid evidence kind");
  if (evidence.evidenceKind !== "live_installed_app") failures.push("evidence is not from the installed app");
  const generatedAt = Date.parse(evidence.generatedAt ?? "");
  if (!Number.isFinite(generatedAt)) failures.push("missing valid generatedAt");
  else if (now.getTime() - generatedAt > maxEvidenceAgeMs || generatedAt > now.getTime() + 60_000) failures.push("live evidence is stale or future-dated");
  requireText(evidence.operator, "operator", failures);
  requireArtifact(evidence.app?.path, "installed app", failures, exists);
  requireText(evidence.app?.commit, "app commit", failures);
  if (!/^[a-f0-9]{64}$/i.test(evidence.app?.artifactSha256 ?? "")) failures.push("missing app artifact SHA-256");
  if (!HIGH_END_PROFILES.has(evidence.host?.profile)) failures.push("host is not a detected high-end profile");
  for (const field of ["hostname", "gpu", "cpu", "driver"]) requireText(evidence.host?.[field], `host ${field}`, failures);
  if (!(evidence.host?.acceleratorMemoryBytes > 0)) failures.push("missing accelerator memory evidence");
  if (!(evidence.host?.storageFreeBytes > 0)) failures.push("missing storage evidence");
  for (const field of ["id", "version", "endpoint", "ownership"]) requireText(evidence.runtime?.[field], `runtime ${field}`, failures);
  if (evidence.runtime?.health !== "healthy") failures.push("runtime health is not healthy");
  for (const field of ["id", "sizeClass", "quantization", "license"]) requireText(evidence.model?.[field], `model ${field}`, failures);
  if (!(evidence.model?.contextLengthTokens >= 131_072)) failures.push("model context is below 131072 tokens");
  requireText(evidence.retrieval?.mode, "retrieval mode", failures);
  requireText(evidence.retrieval?.indexGeneration, "retrieval index generation", failures);
  if (evidence.retrieval?.freshness !== "fresh") failures.push("retrieval index is not fresh");
  if (evidence.retrieval?.secretRedactionVerified !== true) failures.push("secret redaction is not verified");
  return failures;
}

function scoreCase(definition, evidence, exists) {
  const failures = [];
  if (!evidence) return failedCase(definition, [ `${definition.id}: missing case evidence` ]);
  if (evidence.status !== "pass") failures.push(`${definition.id}: live status is not pass`);
  for (const check of definition.requiredChecks) {
    if (evidence.checks?.[check] !== true) failures.push(`${definition.id}: missing check ${check}`);
  }
  for (const [name, path] of Object.entries(evidence.artifacts ?? {})) {
    requireArtifact(path, `${definition.id} ${name}`, failures, exists);
  }
  if (Object.keys(evidence.artifacts ?? {}).length !== 3) failures.push(`${definition.id}: incomplete artifact set`);
  const qualityValues = [evidence.quality?.correctness, evidence.quality?.grounding, evidence.quality?.instructionFollowing];
  const qualityScore = average(qualityValues.map(scoreValue));
  const performanceScore = scorePerformance(evidence.performance);
  if (!(evidence.durationMs > 0)) failures.push(`${definition.id}: missing measured duration`);
  if (!validChronology(evidence.startedAt, evidence.completedAt)) failures.push(`${definition.id}: invalid case chronology`);
  return { ...definition, functionalPass: failures.length === 0, qualityScore, performanceScore, durationMs: evidence.durationMs ?? null, artifacts: evidence.artifacts ?? {}, failures };
}

function failedCase(definition, failures) {
  return { ...definition, functionalPass: false, qualityScore: 0, performanceScore: 0, durationMs: null, artifacts: {}, failures };
}

function scorePerformance(performance = {}) {
  const latency = performance.totalLatencyMs;
  const firstEvent = performance.timeToFirstEventMs;
  const pressure = performance.peakMemoryPressurePercent;
  if (!(latency > 0) || !(firstEvent >= 0) || !(pressure >= 0)) return 0;
  const latencyScore = latency <= 120_000 ? 1 : latency <= 240_000 ? 0.75 : latency <= 480_000 ? 0.5 : 0.25;
  const firstEventScore = firstEvent <= 10_000 ? 1 : firstEvent <= 30_000 ? 0.75 : firstEvent <= 60_000 ? 0.5 : 0.25;
  const pressureScore = pressure <= 90 ? 1 : pressure <= 98 ? 0.5 : 0.25;
  return Number(average([latencyScore, firstEventScore, pressureScore]).toFixed(2));
}

function validChronology(startedAt, completedAt) {
  const start = Date.parse(startedAt ?? "");
  const end = Date.parse(completedAt ?? "");
  return Number.isFinite(start) && Number.isFinite(end) && end >= start;
}

function scoreValue(value) {
  return typeof value === "number" && value >= 0 && value <= 1 ? value : 0;
}

function requireText(value, label, failures) {
  if (typeof value !== "string" || value.trim().length === 0) failures.push(`missing ${label}`);
}

function requireArtifact(path, label, failures, exists) {
  requireText(path, `${label} path`, failures);
  if (typeof path === "string" && path.length > 0 && !exists(path)) failures.push(`missing ${label} artifact ${path}`);
}

function average(values) {
  if (values.length === 0) return 0;
  return Number((values.reduce((sum, value) => sum + value, 0) / values.length).toFixed(2));
}

function blockedReport(failures) {
  return { kind: "desktoplab.frontier-local-certification", schemaVersion: 1, status: "blocked_live_evidence", frontierLocalClaim: false, functional: null, quality: null, performance: null, cases: [], failures };
}

function parseArgs(argv) {
  const args = { evidence: null, template: null, report: null, json: false, help: false };
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === "--evidence") args.evidence = argv[++index];
    else if (argv[index] === "--template") args.template = argv[++index];
    else if (argv[index] === "--report") args.report = argv[++index];
    else if (argv[index] === "--json") args.json = true;
    else if (argv[index] === "--help") args.help = true;
  }
  return args;
}

function writeJson(path, value) {
  const target = resolve(repoRoot, path);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, `${JSON.stringify(value, null, 2)}\n`);
  return target;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log("Usage: node scripts/product/frontier-local-certification.mjs [--template path] [--evidence path] [--report path] [--json]");
    return;
  }
  if (args.template) {
    console.log(`Frontier evidence template written: ${writeJson(args.template, frontierEvidenceTemplate())}`);
    return;
  }
  const evidence = args.evidence ? JSON.parse(readFileSync(resolve(repoRoot, args.evidence), "utf8")) : null;
  const report = buildFrontierCertificationReport({ evidence });
  if (args.report) writeJson(args.report, report);
  if (args.json) console.log(JSON.stringify(report, null, 2));
  else console.log(`Frontier-local certification: ${report.status}`);
  for (const failure of report.failures) console.error(`FAIL: ${failure}`);
  if (report.status === "fail") process.exitCode = 1;
  if (report.status === "blocked_live_evidence") process.exitCode = 2;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
