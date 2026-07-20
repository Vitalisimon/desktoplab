import assert from "node:assert/strict";
import test from "node:test";

import { assessReleaseTag } from "./release-tag-policy-core.mjs";

const commit = "a".repeat(40);
const candidate = {
  kind: "desktoplab.release-candidate",
  schemaVersion: 1,
  candidateId: `sha256:${"b".repeat(64)}`,
  state: "pre_sign_pass",
  source: { commit },
  release: { version: "0.1.0", channel: "beta" },
};

test("accepts an annotated beta tag only after pre-sign acceptance", () => {
  const report = assessReleaseTag({ candidate, releaseRef: "refs/tags/v0.1.0-beta.9", objectType: "tag", tagCommit: commit });
  assert.equal(report.status, "pass");
});

test("rejects exploratory, lightweight and wrong-commit tags", () => {
  const report = assessReleaseTag({
    candidate: { ...candidate, state: "payload_built" },
    releaseRef: "refs/tags/v0.1.0-beta.9",
    objectType: "commit",
    tagCommit: "c".repeat(40),
  });
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /pre-sign/);
  assert.match(report.failures.join("\n"), /annotated/);
  assert.match(report.failures.join("\n"), /differs/);
});

test("keeps stable and beta tag shapes distinct", () => {
  const stable = { ...candidate, release: { version: "0.1.0", channel: "stable" } };
  assert.equal(assessReleaseTag({ candidate: stable, releaseRef: "refs/tags/v0.1.0", objectType: "tag", tagCommit: commit }).status, "pass");
  assert.equal(assessReleaseTag({ candidate: stable, releaseRef: "refs/tags/v0.1.0-beta.1", objectType: "tag", tagCommit: commit }).status, "fail");
});
