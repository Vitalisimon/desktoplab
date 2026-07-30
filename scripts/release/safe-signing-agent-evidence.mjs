#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

import { hashArtifact, readEmbeddedBuild, sha256File } from "../packaging/artifact-provenance-core.mjs";
import { buildMeasuredParityReport } from "../product/agent-parity-eval.mjs";
import { macosAccessibilityDriverEvidence } from "../product/drivers/macos-native-accessibility.mjs";
import { versionedModuleBundle } from "../product/versioned-module-bundle.mjs";
import { assessAgentReleaseGates } from "./agent-release-gates-core.mjs";
import { assessSafeSigningAgentEvidence } from "./safe-signing-agent-evidence-core.mjs";

const args = parseArgs(process.argv.slice(2));
for (const name of ["candidate", "app", "certification", "campaign", "runtimeOutput", "agentGatesOutput", "report", "executor", "uiDriver", "installedUiDriver"]) {
  if (!args[name]) throw new Error(`safe-signing agent evidence requires --${flag(name)}`);
}
for (const name of ["candidate", "app", "certification", "campaign", "executor", "uiDriver", "installedUiDriver", ...args.uiDriverDependencies, ...args.installedUiDriverDependencies]) {
  const path = typeof name === "string" && name in args ? args[name] : name;
  if (!existsSync(resolve(path))) throw new Error(`safe-signing agent evidence path is missing: ${path}`);
}

const candidate = readJson(args.candidate);
const certification = readJson(args.certification);
const campaign = readJson(args.campaign);
const runtime = buildMeasuredParityReport(certification);
const executor = await versionedModuleBundle(args.executor, "scripts");
const uiDriver = macosAccessibilityDriverEvidence(args.uiDriver, args.uiDriverDependencies);
const installedUiDriver = macosAccessibilityDriverEvidence(args.installedUiDriver, args.installedUiDriverDependencies);
const releaseGates = assessAgentReleaseGates({
  candidate,
  runtime,
  campaign,
  expectedExecutorSha256: executor.entrySha256,
  expectedExecutorBundleSha256: executor.bundleSha256,
  expectedUiDriverSha256: uiDriver.sha256,
  expectedUiDriverBundleSha256: uiDriver.bundleSha256,
});
const app = resolve(args.app);
const report = assessSafeSigningAgentEvidence({
  candidate,
  appSha256: hashArtifact(app).sha256,
  appBuild: readEmbeddedBuild(app),
  certification,
  runtime,
  releaseGates,
  currentHead: git(["rev-parse", "HEAD"]),
  treeState: git(["status", "--short"]) ? "dirty" : "clean",
  certificationSha256: `sha256:${sha256File(resolve(args.certification))}`,
  campaignSha256: `sha256:${sha256File(resolve(args.campaign))}`,
  expectedInstalledUiDriverSha256: installedUiDriver.sha256,
  expectedInstalledUiDriverBundleSha256: installedUiDriver.bundleSha256,
});
writeJson(args.runtimeOutput, runtime);
writeJson(args.agentGatesOutput, releaseGates);
writeJson(args.report, report);
console.log(JSON.stringify(report, null, 2));
if (report.status !== "pass") process.exitCode = 1;

function git(values) {
  return execFileSync("git", values, { encoding: "utf8" }).trim();
}

function readJson(path) {
  return JSON.parse(readFileSync(resolve(path), "utf8"));
}

function writeJson(path, value) {
  const target = resolve(path);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, `${JSON.stringify(value, null, 2)}\n`, { mode: 0o600 });
}

function flag(name) {
  return name.replace(/[A-Z]/g, (value) => `-${value.toLowerCase()}`);
}

function parseArgs(values) {
  const parsed = { uiDriverDependencies: [], installedUiDriverDependencies: [] };
  const names = new Map([
    ["--candidate", "candidate"], ["--app", "app"], ["--certification", "certification"],
    ["--campaign", "campaign"], ["--runtime-output", "runtimeOutput"],
    ["--agent-gates-output", "agentGatesOutput"], ["--report", "report"],
    ["--executor", "executor"], ["--ui-driver", "uiDriver"], ["--installed-ui-driver", "installedUiDriver"],
  ]);
  for (let index = 0; index < values.length; index += 1) {
    if (values[index] === "--ui-driver-dependency") parsed.uiDriverDependencies.push(values[++index]);
    else if (values[index] === "--installed-ui-driver-dependency") parsed.installedUiDriverDependencies.push(values[++index]);
    else {
      const name = names.get(values[index]);
      if (!name || index + 1 >= values.length) throw new Error(`unknown or incomplete argument ${values[index]}`);
      parsed[name] = values[++index];
    }
  }
  return parsed;
}
