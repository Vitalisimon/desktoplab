#!/usr/bin/env node
import { mkdirSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import process from "node:process";

import { hashArtifact } from "../packaging/artifact-provenance-core.mjs";
import { aggregateRun, appendRun, runCommand } from "./regression-gate-core.mjs";

const args = parseArgs(process.argv.slice(2));
const startedAt = new Date();
const runId = `${startedAt.toISOString()}-${process.pid}`;
const inputs = candidateInputs(args, runId);
const reportPath = resolve(args.report ?? "dist/release/candidate/safe-signing-regression.json");
const steps = requiredSteps(inputs);
const results = [];

for (const step of steps) {
  console.log(`\n[${step.id}] ${step.command} ${step.args.join(" ")}`);
  const result = runCommand(step, { dryRun: args.dryRun });
  results.push(result);
  console.log(`${result.status} (${result.durationMs}ms)`);
}

const aggregate = aggregateRun(results);
const run = {
  runId,
  startedAt: startedAt.toISOString(),
  finishedAt: new Date().toISOString(),
  durationMs: Date.now() - startedAt.getTime(),
  dryRun: args.dryRun,
  status: args.dryRun ? "blocked" : aggregate.status,
  counts: aggregate,
  head: gitValue(["rev-parse", "HEAD"]),
  treeState: gitValue(["status", "--short"]) ? "dirty" : "clean",
  candidateId: args.dryRun ? null : readCandidateId(inputs.candidate),
  preparedAppSha256: !args.dryRun && existsSync(inputs.app) ? hashArtifact(inputs.app).sha256 : null,
  host: { platform: process.platform, arch: process.arch, node: process.version },
  steps: results,
};
const previous = existsSync(reportPath) ? JSON.parse(readFileSync(reportPath, "utf8")) : null;
const report = appendRun(previous, run);
mkdirSync(dirname(reportPath), { recursive: true });
writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(`\nReport: ${reportPath}`);
console.log(`Safe-signing regression status: ${run.status}`);
process.exitCode = run.status === "pass" ? 0 : 1;

function requiredSteps(inputs) {
  const uiManifest = process.env.DESKTOPLAB_STABLE_UI_MANIFEST ?? "dist/product/stable-ui-qa/desktop/manifest.json";
  const manualReview = process.env.DESKTOPLAB_INSTALLED_UI_MANUAL_REVIEW ?? "dist/release/installed-ui-manual-review.json";
  const installedDriver = process.env.DESKTOPLAB_INSTALLED_AGENT_DRIVER ?? "scripts/product/drivers/macos-installed-agent-ui.mjs";
  const reliabilityUiDriver = "scripts/product/drivers/macos-installed-agent-reliability-ui.mjs";
  const reliabilityVerifier = "scripts/product/recorded-agent-reliability-driver.mjs";
  return [
    command("clean-tree", "git", ["status", "--porcelain=v1"], { rejectOutput: true }),
    command("candidate-payload", "node", ["scripts/release/candidate-admission.mjs", "verify", "--candidate", inputs.candidate, "--app", inputs.app]),
    command("rust-workspace", "cargo", ["test", "--locked", "--workspace"]),
    command("tauri-tests", "cargo", ["test", "--locked", "--manifest-path", "apps/desktop/src-tauri/Cargo.toml"]),
    command("frontend-typecheck", "npm", ["--prefix", "apps/desktop", "run", "typecheck"]),
    command("frontend-tests", "npm", ["--prefix", "apps/desktop", "run", "test"]),
    command("frontend-line-guard", "npm", ["--prefix", "apps/desktop", "run", "line-guard"]),
    command("product-truth", "npm", ["run", "product:truth:real"]),
    command("installed-agent", "node", ["scripts/product/installed-agent-certification.mjs", "--app", inputs.app, "--workspace", inputs.workspace, "--evidence", inputs.evidence, "--candidate", inputs.candidate, "--driver", installedDriver, "--report", inputs.certification], { timeoutMs: 90 * 60 * 1000 }),
    command("measured-agent-runtime", "node", ["scripts/product/agent-parity-eval.mjs", "--evidence", inputs.certification, "--json", "--report", inputs.runtime]),
    command("agent-reliability-recording", "node", [reliabilityUiDriver, "--app", inputs.app, "--candidate", inputs.candidate, "--output-root", inputs.reliabilityRoot, "--manifest", inputs.reliabilityManifest, "--catalog", inputs.reliabilityCatalog], { timeoutMs: 4 * 60 * 60 * 1000 }),
    command("agent-reliability-campaign", "node", ["scripts/product/agent-reliability-campaign.mjs", "--manifest", inputs.reliabilityManifest, "--driver", reliabilityVerifier, "--report", inputs.campaign], { env: { DESKTOPLAB_RELIABILITY_CATALOG: inputs.reliabilityCatalog }, timeoutMs: 90 * 60 * 1000 }),
    command("agent-release-gates", "node", ["scripts/release/agent-release-gates.mjs", "--candidate", inputs.candidate, "--runtime", inputs.runtime, "--campaign", inputs.campaign, "--executor", reliabilityVerifier, "--ui-driver", reliabilityUiDriver, "--ui-driver-dependency", installedDriver, "--output", inputs.agentGates]),
    command("beta-quick", "node", ["scripts/product/beta-gauntlet.mjs", "--profile", "quick", "--report", "dist/release/beta-quick.json"]),
    command("beta-full", "node", ["scripts/product/beta-gauntlet.mjs", "--profile", "full", "--prebuilt-candidate", "--report", "dist/release/beta-full.json"]),
    command("stable-ui", "node", ["scripts/product/stable-ui-certification.mjs", "--manifest", uiManifest, "--manual-review", manualReview, "--candidate", inputs.candidate, "--app", inputs.app]),
    command("npm-advisories", "npm", ["audit", "--omit=dev", "--audit-level=high"]),
    command("cargo-advisories", "cargo", ["audit"]),
    command("tracked-secret-scan", "node", ["scripts/security/scan-tracked-secrets.mjs"]),
  ];
}

function command(id, executable, commandArgs, options = {}) {
  return { id, command: executable, args: commandArgs, required: true, ...options };
}

function gitValue(commandArgs) {
  const result = runCommand({ id: "git-metadata", command: "git", args: commandArgs });
  return result.status === "passed" ? result.outputTail.trim() : null;
}

function parseArgs(argv) {
  const parsed = { report: null, dryRun: false, candidate: null, app: null, workspace: null, evidence: null, certification: null, runtime: null, campaign: null, agentGates: null };
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === "--report") parsed.report = argv[++index];
    else if (argv[index] === "--dry-run") parsed.dryRun = true;
    else if (argv[index] === "--candidate") parsed.candidate = argv[++index];
    else if (argv[index] === "--app") parsed.app = argv[++index];
    else if (argv[index] === "--workspace") parsed.workspace = argv[++index];
    else if (argv[index] === "--evidence") parsed.evidence = argv[++index];
    else if (argv[index] === "--certification") parsed.certification = argv[++index];
    else if (argv[index] === "--runtime") parsed.runtime = argv[++index];
    else if (argv[index] === "--campaign") parsed.campaign = argv[++index];
    else if (argv[index] === "--agent-gates") parsed.agentGates = argv[++index];
    else if (argv[index] === "--run-root") parsed.runRoot = argv[++index];
    else if (argv[index] === "--reliability-root") parsed.reliabilityRoot = argv[++index];
    else if (argv[index] === "--reliability-manifest") parsed.reliabilityManifest = argv[++index];
    else if (argv[index] === "--reliability-catalog") parsed.reliabilityCatalog = argv[++index];
  }
  return parsed;
}

function candidateInputs(parsed, runId) {
  const runRoot = resolve(parsed.runRoot ?? `dist/release/candidate/regression-runs/${safeRunId(runId)}`);
  const reliabilityRoot = resolve(parsed.reliabilityRoot ?? join(runRoot, "reliability-recording"));
  return {
    candidate: resolve(parsed.candidate ?? "dist/release/candidate/admission.json"),
    app: resolve(parsed.app ?? "/Applications/DesktopLab.app"),
    workspace: resolve(parsed.workspace ?? join(runRoot, "installed-agent-workspace")),
    evidence: resolve(parsed.evidence ?? join(runRoot, "installed-agent-evidence.json")),
    certification: resolve(parsed.certification ?? join(runRoot, "installed-agent-certification.json")),
    runtime: resolve(parsed.runtime ?? join(runRoot, "measured-agent-parity.json")),
    campaign: resolve(parsed.campaign ?? join(runRoot, "agent-reliability-campaign.json")),
    agentGates: resolve(parsed.agentGates ?? join(runRoot, "agent-release-gates.json")),
    reliabilityRoot,
    reliabilityManifest: resolve(parsed.reliabilityManifest ?? join(reliabilityRoot, "manifest.json")),
    reliabilityCatalog: resolve(parsed.reliabilityCatalog ?? join(reliabilityRoot, "catalog.json")),
  };
}

function safeRunId(value) {
  return value.replace(/[^a-zA-Z0-9._-]/g, "_");
}

function readCandidateId(path) {
  if (!existsSync(path)) return null;
  try {
    return JSON.parse(readFileSync(path, "utf8")).candidateId ?? null;
  } catch {
    return null;
  }
}
