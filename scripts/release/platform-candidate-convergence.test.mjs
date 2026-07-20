import assert from "node:assert/strict";
import test from "node:test";

import { assessPlatformCandidateConvergence } from "./platform-candidate-convergence-core.mjs";

const commit = "a".repeat(40);
const candidate = {
  kind: "desktoplab.release-candidate",
  schemaVersion: 1,
  candidateId: `sha256:${"b".repeat(64)}`,
  state: "post_sign_pass",
  source: { commit },
  release: { channel: "beta" },
};
const evidence = [
  {
    kind: "desktoplab.artifact-provenance",
    schemaVersion: 2,
    build: { commitSha: commit, channel: "beta" },
    entries: [
      { target: "macos-aarch64", kind: "app_bundle", signatureState: "notarized" },
      { target: "macos-aarch64", kind: "distribution_file", signatureState: "notarized" },
    ],
  },
  { kind: "desktoplab.linux-signed-release", status: "pass", publicTrust: true, platform: "linux-x64", commit, channel: "beta" },
  { kind: "desktoplab.windows-signpath-provenance", status: "pass", publicTrust: true, commit, channel: "beta" },
];

test("passes only one trusted evidence set on the candidate commit", () => {
  const report = assessPlatformCandidateConvergence({ candidate, evidence });
  assert.equal(report.status, "pass");
  assert.equal(report.platforms.length, 3);
});

test("rejects missing platforms and mixed commits", () => {
  const report = assessPlatformCandidateConvergence({
    candidate,
    evidence: [evidence[0], { ...evidence[1], commit: "c".repeat(40) }],
  });
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /windows-x64 evidence, found 0/);
  assert.match(report.failures.join("\n"), /linux-x64 commit differs/);
});

test("rejects historical, unsigned or wrong-state evidence", () => {
  const report = assessPlatformCandidateConvergence({
    candidate: { ...candidate, state: "signed" },
    evidence: [evidence[0], { ...evidence[1], publicTrust: false }, evidence[2]],
  });
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /post-sign/);
  assert.match(report.failures.join("\n"), /linux-x64 lacks passing public trust/);
});
