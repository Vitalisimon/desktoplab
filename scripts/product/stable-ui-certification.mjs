#!/usr/bin/env node
import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { hashArtifact } from "../packaging/artifact-provenance-core.mjs";

const requiredCaptures = ["01-idle-composer.png", "02-approval.png", "03-completed-summary.png", "04-terminal-output.png", "05-failed-test.png", "06-diff-review.png"];

export function assessStableUiEvidence({ manifest, manualReview, candidate, appHash, exists = existsSync, read = readFileSync } = {}) {
  const failures = [];
  if (!manifest || manifest.kind !== "desktoplab.stable-ui-captures" || manifest.schemaVersion !== 2) failures.push("stable UI manifest missing");
  if (manifest?.evidenceKind !== "dev_server_ui_with_test_controls" || manifest?.installedAppClaim !== false) {
    failures.push("deterministic UI evidence is not truthfully classified");
  }
  if (manifest?.testControlsUsed !== true) failures.push("deterministic UI test-control provenance missing");
  const candidateValid = candidate?.kind === "desktoplab.release-candidate"
    && candidate?.schemaVersion === 1
    && candidate?.state === "payload_built";
  if (!candidateValid) failures.push("release candidate binding missing");
  const expectedAppHash = candidate?.payload?.sha256 ? `sha256:${candidate.payload.sha256}` : null;
  if (!expectedAppHash || appHash !== expectedAppHash) failures.push("installed app hash differs from candidate payload");
  if (manifest?.sourceCommit !== candidate?.source?.commit || manifest?.sourceTreeState !== "clean") {
    failures.push("stable UI captures are not bound to clean candidate source");
  }
  const screenshots = manifest?.screenshots ?? [];
  for (const record of screenshots) {
    if (!record.path || !exists(record.path)) failures.push(`screenshot missing ${record.path ?? "unknown"}`);
    else if (fileHash(record.path, read) !== record.sha256) failures.push(`screenshot hash mismatch ${record.path}`);
    if (!record.route || !record.state || !record.theme || !record.viewport?.width || !record.viewport?.height) {
      failures.push(`semantic capture metadata incomplete ${record.filename ?? "unknown"}`);
    }
  }
  const first = stateSet(screenshots, 1);
  const second = stateSet(screenshots, 2);
  if (JSON.stringify(first) !== JSON.stringify(second)) failures.push("consecutive visual runs captured different semantic state sets");
  for (const theme of ["light", "dark"]) {
    for (const filename of requiredCaptures) {
      if (!screenshots.some((record) => record.run === 1 && record.theme === theme && record.filename === filename)) {
        failures.push(`required capture missing ${theme}/${filename}`);
      }
    }
  }
  if (manualReview?.kind !== "desktoplab.installed-ui-manual-review" || manualReview?.schemaVersion !== 2 || manualReview?.status !== "accepted") {
    failures.push("installed-app manual review not accepted");
  }
  if (manualReview?.candidateId !== candidate?.candidateId || manualReview?.sourceCommit !== candidate?.source?.commit) {
    failures.push("installed-app manual review belongs to another candidate");
  }
  if (manualReview?.appHash !== expectedAppHash || manualReview?.appHash !== appHash) {
    failures.push("installed-app manual review belongs to another app payload");
  }
  const expectedReviewed = screenshots
    .filter((record) => record.run === 1 && ["light", "dark"].includes(record.theme) && requiredCaptures.includes(record.filename))
    .map((record) => record.sha256)
    .sort();
  const actualReviewed = Array.isArray(manualReview?.reviewedScreenshotHashes)
    ? [...manualReview.reviewedScreenshotHashes].sort()
    : [];
  if (JSON.stringify(actualReviewed) !== JSON.stringify(expectedReviewed)) failures.push("reviewed screenshot set differs from required current captures");
  const installedScreenshot = manualReview?.installedAppScreenshot;
  if (!installedScreenshot?.path || !exists(installedScreenshot.path)) failures.push("installed-app review screenshot missing");
  else if (fileHash(installedScreenshot.path, read) !== installedScreenshot.sha256) failures.push("installed-app review screenshot hash mismatch");
  if (!Array.isArray(manualReview?.findings) || manualReview.findings.length > 0) failures.push("installed-app manual review has unresolved findings");
  return {
    kind: "desktoplab.stable-ui-certification",
    schemaVersion: 2,
    status: failures.length === 0 ? "pass" : "blocked",
    deterministicEvidenceAcceptedAsInstalledProof: false,
    provenance: { candidateId: candidate?.candidateId ?? null, appHash: expectedAppHash },
    stateSet: first,
    failures,
  };
}

function stateSet(records, run) {
  return records.filter((record) => record.run === run).map((record) => `${record.theme}:${record.filename}:${record.route}:${record.state}`).sort();
}

function fileHash(path, read) {
  return `sha256:${createHash("sha256").update(read(path)).digest("hex")}`;
}

function parseArgs(argv) {
  const args = { manifest: null, manualReview: null, candidate: null, app: null };
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === "--manifest") args.manifest = argv[++index];
    else if (argv[index] === "--manual-review") args.manualReview = argv[++index];
    else if (argv[index] === "--candidate") args.candidate = argv[++index];
    else if (argv[index] === "--app") args.app = argv[++index];
  }
  return args;
}

function readJson(path) {
  return path && existsSync(resolve(path)) ? JSON.parse(readFileSync(resolve(path), "utf8")) : null;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const args = parseArgs(process.argv.slice(2));
  const appPath = args.app ? resolve(args.app) : null;
  const report = assessStableUiEvidence({
    manifest: readJson(args.manifest),
    manualReview: readJson(args.manualReview),
    candidate: readJson(args.candidate),
    appHash: appPath && existsSync(appPath) ? `sha256:${hashArtifact(appPath).sha256}` : null,
  });
  console.log(JSON.stringify(report, null, 2));
  process.exitCode = report.status === "pass" ? 0 : 1;
}
