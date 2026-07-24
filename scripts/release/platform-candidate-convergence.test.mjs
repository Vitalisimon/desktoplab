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
const releaseClaims = {
  schemaVersion: 1,
  binaryReleasePlatforms: ["macosAppleSilicon", "linuxX64"],
  platforms: {
    macosAppleSilicon: {
      publicAvailability: "candidate_not_public",
      evidenceClaim: "signed_notarized_exact_candidate",
    },
    linuxX64: {
      publicAvailability: "candidate_not_public",
      evidenceClaim: "sigstore_signed_exact_candidate",
    },
    windowsX64: {
      publicAvailability: "not_public",
      evidenceClaim: "test_signed_physical_host_development",
    },
  },
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

test("passes one trusted evidence set for each declared beta platform", () => {
  const report = assessPlatformCandidateConvergence({ candidate, evidence: evidence.slice(0, 2), releaseClaims });
  assert.equal(report.status, "pass");
  assert.deepEqual(report.requiredPlatforms, ["macos-aarch64", "linux-x64"]);
  assert.deepEqual(report.releaseScope, ["macosAppleSilicon", "linuxX64"]);
});

test("rejects missing scoped platforms and mixed commits", () => {
  const report = assessPlatformCandidateConvergence({
    candidate,
    evidence: [{ ...evidence[1], commit: "c".repeat(40) }],
    releaseClaims,
  });
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /macos-aarch64 evidence, found 0/);
  assert.match(report.failures.join("\n"), /linux-x64 commit differs/);
});

test("rejects historical, unsigned or wrong-state evidence", () => {
  const report = assessPlatformCandidateConvergence({
    candidate: { ...candidate, state: "signed" },
    evidence: [evidence[0], { ...evidence[1], publicTrust: false }],
    releaseClaims,
  });
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /post-sign/);
  assert.match(report.failures.join("\n"), /linux-x64 lacks passing public trust/);
});

test("rejects evidence outside the declared beta scope", () => {
  const report = assessPlatformCandidateConvergence({ candidate, evidence, releaseClaims });
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /windows-x64 is outside/);
});

test("stable releases cannot omit Windows public trust", () => {
  const report = assessPlatformCandidateConvergence({
    candidate: { ...candidate, release: { channel: "stable" } },
    evidence: evidence.slice(0, 2).map((entry) => ({ ...entry, channel: "stable", build: entry.build ? { ...entry.build, channel: "stable" } : undefined })),
    releaseClaims,
  });
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /stable release scope requires macOS, Linux and Windows/);
});
