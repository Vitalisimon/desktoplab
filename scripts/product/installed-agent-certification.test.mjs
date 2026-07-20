import assert from "node:assert/strict";
import test from "node:test";

import { assessInstalledAgentEvidence, installedAgentCases, installedAgentDriverArgs, installedAgentDriverTimeoutMs } from "./installed-agent-certification.mjs";
import { passingExecutableCase } from "./test-fixtures/agent-trace-score-fixture.mjs";

const app = "/Applications/DesktopLab.app";
const workspace = "/tmp/repo";
const evidence = "/tmp/evidence.json";
const build = (commitSha = "head") => ({
  commitSha,
  channel: "beta",
  architecture: "arm64",
  lockfiles: [{ path: "Cargo.lock", sha256: "a".repeat(64) }],
});
const contract = (overrides = {}) => ({
  kind: "desktoplab.installed-agent-evidence",
  schemaVersion: 2,
  appHash: "sha256:app",
  commit: "head",
  appBuild: build(),
  modelId: "qwen2.5-coder:7b",
  quantization: "Q4_K_M",
  host: "test-host",
  ...overrides,
});
const verifiedRecording = (cases, metrics = { localModelRequestCount: 7, realToolExecutionCount: 6, testControlRequests: 0 }) => ({
  status: "pass",
  failures: [],
  cases,
  metrics,
});

test("installed driver receives the exact candidate admission path", () => {
  const args = installedAgentDriverArgs({ appPath: app, workspacePath: workspace, evidencePath: evidence, candidatePath: "/tmp/admission.json" });
  assert.deepEqual(args.slice(-2), ["--candidate", "/tmp/admission.json"]);
  assert.ok(installedAgentDriverTimeoutMs >= 5 * 12 * 60 * 1000);
});

test("installed harness rejects missing or stale app evidence", () => {
  const missing = assessInstalledAgentEvidence({
    appPath: app,
    workspacePath: workspace,
    evidencePath: evidence,
    exists: () => false,
  });
  assert.equal(missing.status, "blocked");

  const stale = assessInstalledAgentEvidence({
    appPath: app,
    workspacePath: workspace,
    evidencePath: evidence,
    head: "current",
    exists: () => true,
    hash: () => "sha256:current",
    readBuild: () => build("current"),
    readEvidence: () => JSON.stringify({ appHash: "sha256:current", commit: "stale", appBuild: build("current"), cases: [] }),
  });
  assert.equal(stale.status, "fail");
  assert.ok(stale.failures.includes("installed evidence commit is stale"));
});

test("deterministic or incomplete evidence cannot certify installed agent", () => {
  const incomplete = assessInstalledAgentEvidence({
    appPath: app,
    workspacePath: workspace,
    evidencePath: evidence,
    head: "head",
    exists: () => true,
    hash: () => "sha256:app",
    readBuild: () => build(),
    readEvidence: () => JSON.stringify({
      appHash: "sha256:app",
      commit: "head",
      appBuild: build(),
      executionKind: "deterministic-dev",
      localModelRequestCount: 0,
      realToolExecutionCount: 0,
      testControlRequests: 1,
      cases: [],
    }),
  });
  assert.equal(incomplete.liveClaim, false);
  assert.equal(incomplete.deterministicEvidenceAccepted, false);
  assert.ok(incomplete.failures.includes("installed evidence contract is invalid or obsolete"));
});

test("only current installed UI evidence with real model and tools passes", () => {
  const cases = installedAgentCases.map((expected) => ({
    ...passingExecutableCase(expected.id),
    id: expected.id,
    promptEntered: true,
    sendClicked: true,
    sessionContinuous: true,
    approvalClicked: expected.approval,
    evidence: Object.fromEntries(expected.evidence.map((key) => [key, true])),
  }));
  const report = assessInstalledAgentEvidence({
    appPath: app,
    workspacePath: workspace,
    evidencePath: evidence,
    head: "head",
    exists: () => true,
    hash: () => "sha256:app",
    readBuild: () => build(),
    readEvidence: () => JSON.stringify(contract()),
    verifyRecording: () => verifiedRecording(cases),
  });
  assert.equal(report.status, "pass");
  assert.equal(report.schemaVersion, 3);
  assert.equal(report.liveClaim, true);
});

test("legacy claimed outputs cannot certify an installed agent", () => {
  const cases = installedAgentCases.map((expected) => ({
    id: expected.id,
    status: "pass",
    promptEntered: true,
    sendClicked: true,
    sessionContinuous: true,
    approvalClicked: expected.approval,
    evidence: Object.fromEntries(expected.evidence.map((key) => [key, true])),
  }));
  const report = assessInstalledAgentEvidence({
    appPath: app,
    workspacePath: workspace,
    evidencePath: evidence,
    head: "head",
    exists: () => true,
    hash: () => "sha256:app",
    readBuild: () => build(),
    readEvidence: () => JSON.stringify({
      appHash: "sha256:app",
      commit: "head",
      appBuild: build(),
      executionKind: "installed_app_ui",
      localModelRequestCount: 5,
      realToolExecutionCount: 5,
      testControlRequests: 0,
      modelId: "qwen2.5-coder:7b",
      quantization: "Q4_K_M",
      host: "test-host",
      cases,
    }),
  });

  assert.equal(report.status, "fail");
  assert.ok(report.failures.includes("installed evidence contract is invalid or obsolete"));
});

test("operator evidence cannot hide a stale installed app", () => {
  const report = assessInstalledAgentEvidence({
    appPath: app,
    workspacePath: workspace,
    evidencePath: evidence,
    head: "current",
    exists: () => true,
    hash: () => "sha256:app",
    readBuild: () => build("stale"),
    readEvidence: () => JSON.stringify({
      appHash: "sha256:app",
      commit: "current",
      appBuild: build("current"),
      cases: [],
    }),
  });
  assert.equal(report.status, "fail");
  assert.ok(report.failures.includes("installed app embedded commit differs from current source"));
  assert.ok(report.failures.includes("installed evidence commit differs from app metadata"));
});

test("candidate certification binds the exact embedded build and app hash", () => {
  const cases = installedAgentCases.map((expected) => ({
    ...passingExecutableCase(expected.id),
    id: expected.id,
    promptEntered: true,
    sendClicked: true,
    sessionContinuous: true,
    approvalClicked: expected.approval,
  }));
  const candidate = {
    kind: "desktoplab.release-candidate",
    schemaVersion: 1,
    candidateId: `sha256:${"b".repeat(64)}`,
    state: "payload_built",
    source: { commit: "head" },
    release: { channel: "beta" },
    lockfiles: build().lockfiles,
    payload: { sha256: "app" },
  };
  const base = contract();
  const pass = assessInstalledAgentEvidence({
    appPath: app,
    workspacePath: workspace,
    evidencePath: evidence,
    head: "head",
    exists: () => true,
    hash: () => "sha256:app",
    readBuild: () => build(),
    readEvidence: () => JSON.stringify(base),
    candidate,
    verifyRecording: () => verifiedRecording(cases),
  });
  assert.equal(pass.status, "pass");
  assert.equal(pass.provenance.candidateId, candidate.candidateId);

  const fail = assessInstalledAgentEvidence({
    appPath: app,
    workspacePath: workspace,
    evidencePath: evidence,
    head: "head",
    exists: () => true,
    hash: () => "sha256:different",
    readBuild: () => build(),
    readEvidence: () => JSON.stringify({ ...base, appHash: "sha256:different" }),
    candidate,
    verifyRecording: () => verifiedRecording(cases),
  });
  assert.equal(fail.status, "fail");
  assert.ok(fail.failures.includes("candidate payload hash differs from installed app"));
});
