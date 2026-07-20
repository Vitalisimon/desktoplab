import assert from "node:assert/strict";
import test from "node:test";

import { assessAgentReleaseGates } from "./agent-release-gates-core.mjs";
import { admitCandidateSource, bindCandidatePayload } from "./candidate-admission-core.mjs";
import { moduleSourceBundleDigest } from "../product/versioned-module-bundle.mjs";

const candidate = bindCandidatePayload(admitCandidateSource({
  source: {
    status: "pass",
    head: "a".repeat(40),
    treeState: "clean",
    canonicalRepository: "https://github.com/vitalisimon/desktoplab.git",
    origin: "git@github.com:vitalisimon/desktoplab.git",
  },
  version: "0.1.0",
  channel: "beta",
  lockfiles: [{ path: "package-lock.json", sha256: "b".repeat(64) }],
}), {
  platform: "macos-aarch64",
  relativePath: "DesktopLab.app",
  sha256: "c".repeat(64),
  sizeBytes: 1024,
});
const candidateId = candidate.candidateId;
const runtime = {
  kind: "desktoplab.measured-agent-parity",
  schemaVersion: 1,
  status: "pass",
  controlPlane: { status: "pass" },
  provenance: { candidateId, appHash: `sha256:${candidate.payload.sha256}` },
};
const expectedExecutorSha256 = `sha256:${"d".repeat(64)}`;
const expectedExecutorSources = [{ path: "product/recorded-agent-reliability-driver.mjs", sha256: expectedExecutorSha256 }];
const expectedExecutorBundleSha256 = moduleSourceBundleDigest(expectedExecutorSources);
const expectedUiDriverSha256 = `sha256:${"e".repeat(64)}`;
const expectedUiDriverBundleSha256 = `sha256:${"1".repeat(64)}`;

function assess(campaign, candidateValue = candidate, runtimeValue = runtime) {
  return assessAgentReleaseGates({ candidate: candidateValue, runtime: runtimeValue, campaign, expectedExecutorSha256, expectedExecutorBundleSha256, expectedUiDriverSha256, expectedUiDriverBundleSha256 });
}

test("passes independent runtime and repeated model gates", () => {
  const report = assess(passingCampaign());
  assert.equal(report.status, "pass");
  assert.equal(report.runtimeGate.status, "pass");
  assert.equal(report.modelGate.status, "pass");
});

test("model quality failure does not falsify runtime PASS", () => {
  const campaign = passingCampaign();
  campaign.status = "fail";
  campaign.runs.find((run) => run.caseId === "test_repair").status = "failed";
  campaign.metrics.passRate = 14 / 15;
  const report = assess(campaign);
  assert.equal(report.status, "fail");
  assert.equal(report.runtimeGate.status, "pass");
  assert.equal(report.modelGate.status, "fail");
});

test("runtime evidence must describe the admitted app payload", () => {
  const report = assess(passingCampaign(), candidate, { ...runtime, provenance: { ...runtime.provenance, appHash: `sha256:${"f".repeat(64)}` } });
  assert.equal(report.runtimeGate.status, "fail");
  assert.match(report.failures.join("\n"), /another app payload/);
});

test("mutation and safety capabilities are zero tolerance", () => {
  const campaign = passingCampaign();
  campaign.runs.find((run) => run.caseId === "patch").status = "failed";
  campaign.metrics.passRate = 14 / 15;
  const report = assess(campaign);
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /patch is a zero-tolerance/);
});

test("requires three isolated runs for every declared case", () => {
  const campaign = passingCampaign();
  campaign.runs = campaign.runs.filter((run) => !(run.caseId === "inspect" && run.repetition > 1));
  campaign.completedRunCount = campaign.runs.length;
  campaign.plannedRunCount = campaign.runs.length;
  const report = assess(campaign);
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /at least 15/);
  assert.match(report.failures.join("\n"), /inspect requires at least three/);
});

test("rejects a campaign containing runs from another candidate", () => {
  const campaign = passingCampaign();
  campaign.runs[0].candidateId = `sha256:${"d".repeat(64)}`;
  const report = assess(campaign);
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /runs from another candidate/);
});

test("rejects legacy campaign evidence and another app payload", () => {
  const legacy = passingCampaign();
  legacy.schemaVersion = 1;
  assert.match(assess(legacy).failures.join("\n"), /contract/);

  const wrongPayload = passingCampaign();
  wrongPayload.appHash = `sha256:${"f".repeat(64)}`;
  assert.match(assess(wrongPayload).failures.join("\n"), /app payload/);
});

test("rejects campaign reports produced by substituted verifier or UI driver bytes", () => {
  const executor = passingCampaign();
  executor.executor.sha256 = `sha256:${"f".repeat(64)}`;
  assert.match(assess(executor).failures.join("\n"), /versioned reliability verifier/);

  const executorDependency = passingCampaign();
  executorDependency.executor.bundleSha256 = `sha256:${"f".repeat(64)}`;
  assert.match(assess(executorDependency).failures.join("\n"), /reliability verifier dependency bundle/);

  const incompleteExecutor = passingCampaign();
  delete incompleteExecutor.executor.sources;
  assert.match(assess(incompleteExecutor).failures.join("\n"), /executor provenance invalid/);

  const ui = passingCampaign();
  ui.runs[0].provenance.uiDriverSha256 = `sha256:${"f".repeat(64)}`;
  assert.match(assess(ui).failures.join("\n"), /unversioned UI driver/);

  const dependency = passingCampaign();
  dependency.runs[0].provenance.uiDriverBundleSha256 = `sha256:${"f".repeat(64)}`;
  assert.match(assess(dependency).failures.join("\n"), /dependency bundle/);
});

function passingCampaign() {
  const runs = ["inspect", "create", "patch", "test_repair", "diff"].flatMap((caseId) => (
    [1, 2, 3].map((repetition) => ({ candidateId, caseId, repetition, status: "pass", provenance: { uiDriverSha256: expectedUiDriverSha256, uiDriverBundleSha256: expectedUiDriverBundleSha256 } }))
  ));
  return {
    kind: "desktoplab.agent-reliability-campaign",
    schemaVersion: 3,
    status: "pass",
    candidateId,
    appHash: `sha256:${candidate.payload.sha256}`,
    executor: {
      kind: "versioned_external_driver",
      schemaVersion: 2,
      id: "recorded-agent-reliability-driver.mjs",
      sha256: expectedExecutorSha256,
      bundleSha256: expectedExecutorBundleSha256,
      sourceCount: 1,
      sources: expectedExecutorSources,
    },
    configurationFingerprint: `sha256:${"b".repeat(64)}`,
    configuration: { model: { id: "qwen2.5-coder:14b", digest: `sha256:${"c".repeat(64)}`, quantization: "Q4_K_M" } },
    plannedRunCount: runs.length,
    completedRunCount: runs.length,
    metrics: { passRate: 1, outcomes: { timeout: 0, infrastructure_failure: 0 } },
    runs,
  };
}
