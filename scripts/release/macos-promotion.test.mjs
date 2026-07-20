import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { assessMacosPromotion } from "./macos-promotion-core.mjs";

const head = "a".repeat(40);
const hash = "b".repeat(64);
const lockfiles = [{ path: "Cargo.lock", sha256: "c".repeat(64) }];
const appBuild = { commitSha: head, channel: "beta", architecture: "arm64", lockfiles };
const candidate = {
  kind: "desktoplab.release-candidate",
  schemaVersion: 1,
  candidateId: `sha256:${"d".repeat(64)}`,
  state: "pre_sign_pass",
  source: { commit: head },
  release: { channel: "beta" },
  lockfiles,
  payload: { sha256: hash },
};
const certification = {
  kind: "desktoplab.installed-agent-certification",
  schemaVersion: 3,
  status: "pass",
  liveClaim: true,
  deterministicEvidenceAccepted: false,
  provenance: { candidateId: candidate.candidateId, appHash: `sha256:${hash}`, appBuild },
};
const safeSigning = {
  kind: "desktoplab.safe-signing-regression",
  runs: [{
    status: "pass",
    head,
    treeState: "clean",
    candidateId: candidate.candidateId,
    preparedAppSha256: hash,
    steps: [{ id: "installed-agent", status: "passed" }],
  }],
};

test("admits only the exact live-certified prepared app", () => {
  const report = assessMacosPromotion({ candidate, certification, safeSigning, appHash: hash, appBuild, currentHead: head });
  assert.equal(report.status, "pass");
});

test("rejects changed app bytes and stale certification", () => {
  const report = assessMacosPromotion({
    candidate,
    certification: { ...certification, provenance: { ...certification.provenance, candidateId: "other" } },
    safeSigning,
    appHash: "e".repeat(64),
    appBuild,
    currentHead: head,
  });
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /prepared payload/);
  assert.match(report.failures.join("\n"), /another candidate/);
});

test("rejects deterministic or failed certification", () => {
  const report = assessMacosPromotion({
    candidate,
    certification: { ...certification, status: "fail", liveClaim: false, deterministicEvidenceAccepted: true },
    safeSigning,
    appHash: hash,
    appBuild,
    currentHead: head,
  });
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /did not pass/);
  assert.match(report.failures.join("\n"), /cannot authorize/);
});

test("rejects missing or stale safe-signing evidence", () => {
  const report = assessMacosPromotion({
    candidate,
    certification,
    safeSigning: {
      ...safeSigning,
      runs: [{ ...safeSigning.runs[0], head: "e".repeat(40), preparedAppSha256: "f".repeat(64) }],
    },
    appHash: hash,
    appBuild,
    currentHead: head,
  });
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /stale or dirty/);
  assert.match(report.failures.join("\n"), /another candidate payload/);
});

test("prepare builds while promote never recompiles", () => {
  const prepare = readFileSync("scripts/packaging/prepare-macos-candidate.sh", "utf8");
  const promote = readFileSync("scripts/packaging/promote-macos-candidate.sh", "utf8");
  assert.match(prepare, /tauri -- build/);
  assert.match(prepare, /release:candidate -- bind-payload/);
  assert.doesNotMatch(prepare, /macos-sign\.sh|macos-notarize\.sh/);
  assert.match(promote, /verify-macos-promotion\.mjs/);
  assert.match(promote, /safe-signing/);
  assert.match(promote, /macos-sign\.sh/);
  assert.doesNotMatch(promote, /tauri -- build|cargo (?:build|run)|npm .* run build/);
  assert.ok(promote.indexOf("verify-macos-promotion.mjs") < promote.indexOf("macos-sign.sh"));
});
