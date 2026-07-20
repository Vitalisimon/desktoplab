import assert from "node:assert/strict";
import test from "node:test";
import { createHash } from "node:crypto";

import { assessStableUiEvidence } from "./stable-ui-certification.mjs";

test("stable UI certification requires two matching semantic runs and manual installed review", () => {
  const content = Buffer.from("png");
  const sha256 = `sha256:${createHash("sha256").update(content).digest("hex")}`;
  const screenshots = [];
  for (const run of [1, 2]) for (const theme of ["light", "dark"]) for (const filename of filenames) {
    screenshots.push({ run, theme, filename, path: `/tmp/${run}-${theme}-${filename}`, route: "agent", state: state(filename), sha256, viewport: { width: 1280, height: 820 } });
  }
  const manifest = stableManifest(screenshots);
  const blocked = assessStableUiEvidence({ manifest, manualReview: null, exists: () => true, read: () => content });
  assert.equal(blocked.status, "blocked");
  assert.ok(blocked.failures.includes("installed-app manual review not accepted"));

  const accepted = assessStableUiEvidence({
    manifest,
    manualReview: acceptedReview(screenshots),
    candidate,
    appHash,
    exists: () => true,
    read: () => content,
  });
  assert.equal(accepted.status, "pass");
});

test("stable UI certification rejects stale candidate and unrelated review evidence", () => {
  const content = Buffer.from("png");
  const screenshots = [];
  for (const run of [1, 2]) for (const theme of ["light", "dark"]) for (const filename of filenames) {
    screenshots.push({ run, theme, filename, path: `/tmp/${run}-${theme}-${filename}`, route: "agent", state: state(filename), sha256: hash, viewport: { width: 1280, height: 820 } });
  }
  const review = acceptedReview(screenshots);
  review.appHash = `sha256:${"f".repeat(64)}`;
  review.reviewedScreenshotHashes = Array(12).fill(`sha256:${"e".repeat(64)}`);
  const report = assessStableUiEvidence({
    manifest: stableManifest(screenshots),
    manualReview: review,
    candidate,
    appHash,
    exists: () => true,
    read: () => content,
  });

  assert.equal(report.status, "blocked");
  assert.match(report.failures.join("\n"), /app payload|reviewed screenshot set/);
});

test("stable UI certification rejects route drift between runs", () => {
  const screenshots = [
    { run: 1, theme: "light", filename: "01-idle-composer.png", path: "/tmp/a", route: "agent", state: "idle", sha256: hash, viewport: { width: 980, height: 720 } },
    { run: 2, theme: "light", filename: "01-idle-composer.png", path: "/tmp/b", route: "setup", state: "recommendation", sha256: hash, viewport: { width: 980, height: 720 } },
  ];
  const report = assessStableUiEvidence({
    manifest: { kind: "desktoplab.stable-ui-captures", evidenceKind: "dev_server_ui_with_test_controls", installedAppClaim: false, screenshots },
    exists: () => true,
    read: () => Buffer.from("png"),
  });
  assert.ok(report.failures.includes("consecutive visual runs captured different semantic state sets"));
});

const filenames = ["01-idle-composer.png", "02-approval.png", "03-completed-summary.png", "04-terminal-output.png", "05-failed-test.png", "06-diff-review.png"];
const hash = `sha256:${createHash("sha256").update(Buffer.from("png")).digest("hex")}`;
const appHash = `sha256:${"a".repeat(64)}`;
const candidate = {
  kind: "desktoplab.release-candidate",
  schemaVersion: 1,
  candidateId: `sha256:${"b".repeat(64)}`,
  state: "payload_built",
  source: { commit: "c".repeat(40), treeState: "clean" },
  payload: { sha256: appHash.slice(7) },
};

function stableManifest(screenshots) {
  return {
    kind: "desktoplab.stable-ui-captures",
    schemaVersion: 2,
    evidenceKind: "dev_server_ui_with_test_controls",
    installedAppClaim: false,
    testControlsUsed: true,
    sourceCommit: candidate.source.commit,
    sourceTreeState: "clean",
    screenshots,
  };
}

function acceptedReview(screenshots) {
  return {
    kind: "desktoplab.installed-ui-manual-review",
    schemaVersion: 2,
    status: "accepted",
    candidateId: candidate.candidateId,
    sourceCommit: candidate.source.commit,
    appHash,
    installedAppScreenshot: { path: "/tmp/installed.png", sha256: hash },
    reviewedScreenshotHashes: screenshots.filter((entry) => entry.run === 1).map((entry) => entry.sha256),
    findings: [],
  };
}

function state(filename) {
  if (filename.includes("idle")) return "idle";
  if (filename.includes("approval")) return "approval";
  return "completion";
}
