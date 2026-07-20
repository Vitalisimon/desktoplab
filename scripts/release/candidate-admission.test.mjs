import assert from "node:assert/strict";
import test from "node:test";

import {
  admitCandidateSource,
  bindCandidatePayload,
  rejectCandidate,
  transitionCandidate,
  verifyCandidate,
} from "./candidate-admission-core.mjs";

const head = "a".repeat(40);
const source = {
  status: "pass",
  canonicalRepository: "github.com/vitalisimon/desktoplab",
  origin: "github.com/vitalisimon/desktoplab",
  head,
  treeState: "clean",
};
const lockfiles = [{ path: "Cargo.lock", sha256: "b".repeat(64) }];
const payload = { platform: "macos-aarch64", relativePath: "DesktopLab.app", sha256: "c".repeat(64), sizeBytes: 42 };

test("admits one deterministic source identity", () => {
  const first = admitCandidateSource({ source, version: "0.1.0", channel: "beta", lockfiles, createdAt: "one" });
  const second = admitCandidateSource({ source, version: "0.1.0", channel: "beta", lockfiles, createdAt: "two" });
  assert.equal(first.candidateId, second.candidateId);
  assert.equal(first.state, "source_admitted");
});

test("rejects failed or private source admission", () => {
  assert.throws(
    () => admitCandidateSource({ source: { ...source, status: "fail" }, version: "0.1.0", channel: "beta", lockfiles }),
    /public source gate did not pass/,
  );
});

test("binds one exact macOS payload", () => {
  const admitted = admitCandidateSource({ source, version: "0.1.0", channel: "beta", lockfiles });
  const bound = bindCandidatePayload(admitted, payload);
  assert.equal(bound.state, "payload_built");
  assert.deepEqual(bound.payload, payload);
  assert.throws(() => bindCandidatePayload(bound, payload), /only bind after source admission/);
});

test("verifies current source, lockfiles and payload", () => {
  const admitted = admitCandidateSource({ source, version: "0.1.0", channel: "beta", lockfiles });
  const bound = bindCandidatePayload(admitted, payload);
  assert.equal(verifyCandidate({ candidate: bound, source, lockfiles, payload }).status, "pass");
});

test("rejects source drift, lock drift and payload mutation", () => {
  const admitted = admitCandidateSource({ source, version: "0.1.0", channel: "beta", lockfiles });
  const bound = bindCandidatePayload(admitted, payload);
  const report = verifyCandidate({
    candidate: bound,
    source: { ...source, head: "d".repeat(40) },
    lockfiles: [{ ...lockfiles[0], sha256: "e".repeat(64) }],
    payload: { ...payload, sha256: "f".repeat(64) },
  });
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /source commit differs/);
  assert.match(report.failures.join("\n"), /lock hashes differ/);
  assert.match(report.failures.join("\n"), /payload mutated/);
});

test("advances only through the declared evidence-backed sequence", () => {
  const admitted = admitCandidateSource({ source, version: "0.1.0", channel: "beta", lockfiles });
  const bound = bindCandidatePayload(admitted, payload);
  const safeSigning = {
    kind: "desktoplab.safe-signing-regression",
    runs: [{ status: "pass", candidateId: bound.candidateId, preparedAppSha256: payload.sha256 }],
  };
  const preSign = transitionCandidate(bound, { to: "pre_sign_pass", evidence: safeSigning, transitionedAt: "now" });
  assert.equal(preSign.state, "pre_sign_pass");
  assert.equal(preSign.transitions.at(-1).evidenceKind, "desktoplab.safe-signing-regression");
  assert.throws(() => transitionCandidate(bound, { to: "signed", evidence: {} }), /not allowed/);
  assert.throws(() => transitionCandidate(preSign, { to: "pre_sign_pass", evidence: safeSigning }), /not allowed/);
});

test("rejection is terminal and preserves the reason", () => {
  const admitted = admitCandidateSource({ source, version: "0.1.0", channel: "beta", lockfiles });
  const rejected = rejectCandidate(admitted, { reason: "live matrix failed", transitionedAt: "now" });
  assert.equal(rejected.state, "rejected");
  assert.equal(rejected.rejection.reason, "live matrix failed");
  assert.throws(() => transitionCandidate(rejected, { to: "payload_built", evidence: {} }), /already terminal/);
});
