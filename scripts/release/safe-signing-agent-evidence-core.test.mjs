import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { admitCandidateSource, bindCandidatePayload } from "./candidate-admission-core.mjs";
import { assessSafeSigningAgentEvidence } from "./safe-signing-agent-evidence-core.mjs";

const head = "a".repeat(40);
const appSha256 = "b".repeat(64);
const lockfiles = [{ path: "package-lock.json", sha256: "c".repeat(64) }];
const candidate = bindCandidatePayload(admitCandidateSource({
  source: {
    status: "pass",
    head,
    treeState: "clean",
    canonicalRepository: "github.com/vitalisimon/desktoplab",
    origin: "github.com/vitalisimon/desktoplab",
  },
  version: "0.1.0",
  channel: "beta",
  lockfiles,
}), {
  platform: "macos-aarch64",
  relativePath: "DesktopLab.app",
  sha256: appSha256,
  sizeBytes: 1024,
});
const appBuild = { commitSha: head, channel: "beta", treeState: "clean", lockfiles };
const certification = {
  kind: "desktoplab.installed-agent-certification",
  schemaVersion: 3,
  status: "pass",
  liveClaim: true,
  deterministicEvidenceAccepted: false,
  provenance: {
    candidateId: candidate.candidateId,
    appHash: `sha256:${appSha256}`,
    head,
    appBuild,
    executionKind: "installed_app_ui",
    uiDriverSha256: `sha256:${"2".repeat(64)}`,
    uiDriverBundleSha256: `sha256:${"3".repeat(64)}`,
    modelId: "gemma4:12b",
    quantization: "Q4_K_M",
    localModelRequestCount: 5,
    realToolExecutionCount: 5,
    testControlRequests: 0,
  },
};
const runtime = {
  kind: "desktoplab.measured-agent-parity",
  schemaVersion: 1,
  status: "pass",
  controlPlane: { status: "pass" },
  provenance: certification.provenance,
};
const releaseGates = {
  kind: "desktoplab.agent-release-gates",
  schemaVersion: 2,
  status: "pass",
  candidateId: candidate.candidateId,
  runtimeGate: { status: "pass", failures: [] },
  modelGate: { status: "pass", model: { id: "gemma4:12b", quantization: "Q4_K_M" }, failures: [] },
  failures: [],
};
const context = {
  candidate,
  appSha256,
  appBuild,
  certification,
  runtime,
  releaseGates,
  currentHead: head,
  treeState: "clean",
  certificationSha256: `sha256:${"d".repeat(64)}`,
  campaignSha256: `sha256:${"e".repeat(64)}`,
  expectedInstalledUiDriverSha256: certification.provenance.uiDriverSha256,
  expectedInstalledUiDriverBundleSha256: certification.provenance.uiDriverBundleSha256,
};

test("admits exact standalone agent evidence for verified safe-signing reuse", () => {
  const report = assessSafeSigningAgentEvidence(context);
  assert.equal(report.status, "pass", report.failures.join("; "));
  assert.equal(report.mode, "verified_reuse");
  assert.equal(report.candidateId, candidate.candidateId);
  assert.equal(report.appHash, `sha256:${appSha256}`);
  assert.equal(report.sources.certificationSha256, context.certificationSha256);
  assert.equal(report.sources.campaignSha256, context.campaignSha256);
});

test("rejects stale source, payload and certification provenance", () => {
  const stale = assessSafeSigningAgentEvidence({
    ...context,
    currentHead: "f".repeat(40),
    treeState: "dirty",
    appSha256: "1".repeat(64),
    certification: {
      ...certification,
      provenance: { ...certification.provenance, candidateId: "other", testControlRequests: 1 },
    },
  });
  assert.equal(stale.status, "fail");
  assert.match(stale.failures.join("\n"), /public HEAD/);
  assert.match(stale.failures.join("\n"), /clean/);
  assert.match(stale.failures.join("\n"), /prepared payload/);
  assert.match(stale.failures.join("\n"), /another candidate/);
  assert.match(stale.failures.join("\n"), /test controls/);
});

test("rejects deterministic canaries and failed derived gates", () => {
  const report = assessSafeSigningAgentEvidence({
    ...context,
    certification: { ...certification, deterministicEvidenceAccepted: true },
    runtime: { ...runtime, status: "fail" },
    releaseGates: { ...releaseGates, status: "fail", modelGate: { status: "fail" } },
  });
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /deterministic/);
  assert.match(report.failures.join("\n"), /measured parity/);
  assert.match(report.failures.join("\n"), /release gates/);
});

test("rejects substituted canary drivers and a different campaign model", () => {
  const report = assessSafeSigningAgentEvidence({
    ...context,
    expectedInstalledUiDriverBundleSha256: `sha256:${"4".repeat(64)}`,
    releaseGates: {
      ...releaseGates,
      modelGate: { ...releaseGates.modelGate, model: { id: "other", quantization: "Q8_0" } },
    },
  });
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /UI driver/);
  assert.match(report.failures.join("\n"), /different model envelopes/);
});

test("verified reuse modules remain focused", () => {
  for (const [path, maximum] of [
    ["scripts/release/safe-signing-agent-evidence-core.mjs", 120],
    ["scripts/release/safe-signing-agent-evidence.mjs", 150],
  ]) {
    assert.ok(readFileSync(path, "utf8").split("\n").length <= maximum, `${path} exceeds ${maximum} lines`);
  }
});
