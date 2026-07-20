#!/usr/bin/env node
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import process from "node:process";

import { macosAccessibilityDriverEvidence } from "../product/drivers/macos-native-accessibility.mjs";
import { versionedModuleBundle } from "../product/versioned-module-bundle.mjs";
import { assessAgentReleaseGates } from "./agent-release-gates-core.mjs";

const args = parseArgs(process.argv.slice(2));
for (const name of ["candidate", "runtime", "campaign"]) {
  if (!args[name]) throw new Error(`agent release gates require --${name}`);
}
const expectedUiDriver = macosAccessibilityDriverEvidence(
  args.uiDriver ?? "scripts/product/drivers/macos-installed-agent-ui.mjs",
  args.uiDriverDependencies,
);
const expectedExecutor = await versionedModuleBundle(
  args.executor ?? "scripts/product/recorded-agent-reliability-driver.mjs",
  "scripts",
);
const report = assessAgentReleaseGates({
  candidate: readJson(args.candidate),
  runtime: readJson(args.runtime),
  campaign: readJson(args.campaign),
  expectedExecutorSha256: expectedExecutor.entrySha256,
  expectedExecutorBundleSha256: expectedExecutor.bundleSha256,
  expectedUiDriverSha256: expectedUiDriver.sha256,
  expectedUiDriverBundleSha256: expectedUiDriver.bundleSha256,
});
if (args.output) {
  const output = resolve(args.output);
  mkdirSync(dirname(output), { recursive: true });
  writeFileSync(output, `${JSON.stringify(report, null, 2)}\n`);
}
console.log(JSON.stringify(report, null, 2));
if (report.status !== "pass") process.exitCode = 1;

function readJson(path) {
  return JSON.parse(readFileSync(resolve(path), "utf8"));
}

function parseArgs(values) {
  const parsed = { uiDriverDependencies: [] };
  for (let index = 0; index < values.length; index += 1) {
    if (values[index] === "--candidate") parsed.candidate = values[++index];
    else if (values[index] === "--runtime") parsed.runtime = values[++index];
    else if (values[index] === "--campaign") parsed.campaign = values[++index];
    else if (values[index] === "--output") parsed.output = values[++index];
    else if (values[index] === "--executor") parsed.executor = values[++index];
    else if (values[index] === "--ui-driver") parsed.uiDriver = values[++index];
    else if (values[index] === "--ui-driver-dependency") parsed.uiDriverDependencies.push(values[++index]);
  }
  return parsed;
}
