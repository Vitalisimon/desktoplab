#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { currentVisualPlatform } from "./native-platform-adapters.mjs";
import { assessVisualEvidence, NativeVisualEvidenceDriver } from "./visual-evidence-driver.mjs";

const scenarioPath = argument("--scenario");
if (!scenarioPath || !existsSync(resolve(scenarioPath))) fail("visual scenario is required");
const scenario = JSON.parse(readFileSync(resolve(scenarioPath), "utf8"));
if (scenario.kind !== "desktoplab.native-visual-scenario" || scenario.operatorAcknowledged !== true) fail("operator acknowledgement is required");
if (!Array.isArray(scenario.steps) || scenario.steps.length === 0) fail("visual scenario has no steps");

const driver = new NativeVisualEvidenceDriver({
  platform: scenario.platform ?? currentVisualPlatform(),
  evidenceRoot: resolve(scenario.evidenceRoot),
});
console.log(JSON.stringify({ kind: "desktoplab.native-visual-capabilities", ...driver.capabilities() }, null, 2));
for (const step of scenario.steps) {
  if (step.beforeAfter === true) driver.runWithFrames(step, { frameReview: step.frameReview, expectChange: step.expectChange !== false });
  else driver.run(step);
}
const manifest = driver.finalize(scenario.identity ?? {});
const assessment = assessVisualEvidence(manifest);
console.log(JSON.stringify(assessment, null, 2));
process.exitCode = assessment.status === "pass" ? 0 : 1;

function argument(name) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : null;
}

function fail(message) {
  console.error(message);
  process.exit(2);
}
