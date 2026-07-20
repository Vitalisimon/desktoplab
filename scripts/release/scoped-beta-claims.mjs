#!/usr/bin/env node
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { assessScopedBetaClaims } from "./scoped-beta-claims-core.mjs";
import { resolveEvidencePath } from "./scoped-beta-evidence-core.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const head = git(["rev-parse", "HEAD"]);
const dirty = git(["status", "--porcelain=v1"]).length > 0;
if (dirty) throw new Error("scoped beta claims require a clean source tree");

const exportAudit = commandJson("node", ["scripts/product/public-export-audit.mjs"]);
command("node", ["scripts/product/product-claim-guard.mjs"]);
const frontierGate = commandJson("node", ["scripts/product/frontier-local-gate.mjs", "--claim", "--json"], [0, 2]);
const artifactManifest = readJson("dist/desktoplab-packaging/artifact-manifest.json");
const supplyChain = readJson("dist/release/supply-chain/evidence.json");
const shortHead = head.slice(0, 8);
const report = assessScopedBetaClaims({
  head,
  exportAudit,
  artifactManifest,
  supplyChain,
  publicClaims: readJson("docs-public/release-claims.json"),
  frontierGate,
  linuxEvidence: optionalEvidence("DESKTOPLAB_LINUX_CURRENT_HEAD_EVIDENCE", [
    `dist/release/linux/${shortHead}/linux-current-head-evidence.json`,
    "dist/release/linux-current-head-evidence.json",
  ]),
  windowsEvidence: optionalEvidence("DESKTOPLAB_WINDOWS_CURRENT_HEAD_EVIDENCE", [
    `dist/release/windows/${shortHead}/windows-current-head-evidence.json`,
    "dist/release/windows-current-head-evidence.json",
  ]),
  providerEvidence: optionalEvidence("DESKTOPLAB_LIVE_PROVIDER_EVIDENCE"),
  installedAgentEvidence: optionalEvidence("DESKTOPLAB_INSTALLED_AGENT_CURRENT_HEAD_EVIDENCE", [
    `dist/release/macos/${shortHead}/installed-agent-certification.json`,
    "dist/release/installed-agent-certification.json",
  ]),
});
const output = join(root, "dist/release/scoped-beta-claims.json");
mkdirSync(dirname(output), { recursive: true });
writeFileSync(output, `${JSON.stringify({ ...report, generatedAt: new Date().toISOString() }, null, 2)}\n`);
console.log(JSON.stringify(report, null, 2));
if (report.status !== "pass") process.exitCode = 1;

function optionalEvidence(name, candidates = []) {
  const path = resolveEvidencePath({
    explicitPath: process.env[name],
    candidates: candidates.map((candidate) => resolve(root, candidate)),
  });
  return path ? readJson(path) : null;
}

function readJson(path) {
  const absolute = resolve(root, path);
  if (!existsSync(absolute)) throw new Error(`required evidence missing: ${path}`);
  return JSON.parse(readFileSync(absolute, "utf8"));
}

function commandJson(program, args, accepted = [0]) {
  return JSON.parse(command(program, args, accepted));
}

function command(program, args, accepted = [0]) {
  const result = spawnSync(program, args, { cwd: root, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
  if (!accepted.includes(result.status)) throw new Error(`${program} ${args.join(" ")} failed: ${(result.stderr || result.stdout).trim()}`);
  return result.stdout.trim();
}

function git(args) {
  return execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
}
