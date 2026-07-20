#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

import { assessEcosystemLearningCertification } from "./ecosystem-learning-certification-core.mjs";

const profile = process.argv.includes("--targeted") ? "targeted" : "contract";
const reportPath = "dist/product/ecosystem-learning-certification.json";
const planPath = "docs/superpowers/plans/2026-07-15-openclaw-ecosystem-learnings.md";
const adoptionPath = "docs/evidence/openclaw-ecosystem-adoption-ledger.json";
const referencePath = "docs/evidence/openclaw-ecosystem-reference-ledger.json";
const trackedFiles = git(["ls-files"]).split(/\r?\n/).filter(Boolean);
const dependencyPaths = [
  "Cargo.toml", "Cargo.lock", "package.json", "package-lock.json",
  "apps/desktop/package.json", "apps/desktop/src-tauri/Cargo.toml", "apps/desktop/src-tauri/Cargo.lock",
];
const input = {
  planSource: readFileSync(planPath, "utf8"),
  adoptionLedger: JSON.parse(readFileSync(adoptionPath, "utf8")),
  referenceLedger: JSON.parse(readFileSync(referencePath, "utf8")),
  trackedFiles,
  dependencySources: dependencyPaths.filter(existsSync).map((path) => ({ path, source: readFileSync(path, "utf8") })),
  artifactPaths: collectArtifactPaths("dist"),
  evidencePaths: [
    referencePath,
    adoptionPath,
    "docs/evidence/cross-platform-agent-parity.md",
    "docs/evidence/filesystem-race-audit.md",
    "docs/evidence/remote-target-contract.md",
  ].filter(existsSync),
};
const failures = assessEcosystemLearningCertification(input);
const steps = [];

if (profile === "targeted") {
  const dirty = git(["status", "--porcelain", "--untracked-files=all"]).trim();
  if (dirty) failures.push("targeted certification requires an exact clean commit");
  for (const step of failures.length === 0 ? targetedSteps() : []) {
    const startedAt = Date.now();
    const result = spawnSync(step.command, step.args, {
      cwd: process.cwd(), env: { ...process.env, CI: process.env.CI ?? "true" }, encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024, stdio: ["ignore", "pipe", "pipe"],
    });
    process.stdout.write(result.stdout ?? "");
    process.stderr.write(result.stderr ?? "");
    steps.push({ id: step.id, status: result.status === 0 ? "passed" : "failed", durationMs: Date.now() - startedAt });
    if (result.status !== 0) {
      failures.push(`${step.id}: targeted check failed`);
      break;
    }
  }
}

const report = {
  kind: "desktoplab.ecosystem-learning-certification",
  schemaVersion: 1,
  profile,
  source: { commit: git(["rev-parse", "HEAD"]).trim(), treeState: failures.includes("targeted certification requires an exact clean commit") ? "dirty" : "clean" },
  status: failures.length === 0 ? "passed" : "failed",
  auditPersonas: input.adoptionLedger.personas.map((persona) => ({ id: persona.id, status: "passed", surfaces: persona.surfaces })),
  residualRisks: input.adoptionLedger.residualRisks,
  steps,
  failures,
};
mkdirSync(dirname(reportPath), { recursive: true });
writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
if (failures.length) {
  console.error("Ecosystem learning certification failed:");
  failures.forEach((failure) => console.error(`- ${failure}`));
  process.exit(1);
}
console.log(`Ecosystem learning certification passed (${profile}) at ${report.source.commit.slice(0, 8)}.`);

function targetedSteps() {
  return [
    command("reference-and-claims", "npm", ["run", "product:external-reference-guard"]),
    command("claim-boundary", "npm", ["run", "product:claim-guard"]),
    command("trace-score", "npm", ["run", "product:agent-trace-score:test"]),
    command("reliability", "npm", ["run", "product:agent-reliability:test"]),
    command("anti-gaming", "npm", ["run", "product:agent-anti-gaming:test"]),
    command("client-sdk", "npm", ["run", "product:client-sdk:test"]),
    command("filesystem-audit", "npm", ["run", "security:filesystem-race-audit"]),
    command("remote-contracts", "node", ["--test", "scripts/remote-lab/remote-target-contract.test.mjs", "scripts/remote-lab/remote-sync.test.mjs"]),
    command("operator-contracts", "node", ["--test", "scripts/visual-evidence/visual-evidence-driver.test.mjs", "scripts/skills/skill-distribution.test.mjs", "scripts/support/github-support-contract.test.mjs"]),
    command("durable-storage", "cargo", ["test", "-p", "desktoplab-storage"]),
    command("extension-contracts", "cargo", ["test", "-p", "desktoplab-registry", "--test", "extension_trust_history"]),
    command("portable-mcp", "cargo", ["test", "-p", "desktoplab-tool-gateway", "--test", "portable_mcp_transport", "--test", "root_capability"]),
    command("stateful-workflows", "cargo", ["test", "-p", "desktoplab-backend-services", "--test", "typed_workflows", "--test", "session_turn_queue", "--test", "agent_restart_recovery", "--test", "offline_plugin_inspector"]),
    command("provider-adapters", "cargo", ["test", "-p", "desktoplab-backends", "--test", "provider_shaped_adapter_mocks"]),
  ];
}

function command(id, executable, args) { return { id, command: executable, args }; }
function git(args) {
  const result = spawnSync("git", args, { encoding: "utf8", maxBuffer: 16 * 1024 * 1024 });
  if (result.status !== 0) throw new Error(result.stderr || "git command failed");
  return result.stdout;
}
function collectArtifactPaths(root) {
  if (!existsSync(root)) return [];
  const paths = [];
  const pending = [root];
  while (pending.length > 0 && paths.length < 10_000) {
    const current = pending.pop();
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      const path = join(current, entry.name);
      paths.push(path);
      if (entry.isDirectory() && !entry.isSymbolicLink()) pending.push(path);
      if (paths.length >= 10_000) break;
    }
  }
  return paths;
}
