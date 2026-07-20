import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  assessPublicReleaseSource,
  normalizeRepositoryOrigin,
} from "./public-release-source-core.mjs";

const head = "a".repeat(40);

test("accepts clean HTTPS and SSH checkouts of the canonical public repository", () => {
  for (const origin of [
    "https://github.com/Vitalisimon/desktoplab.git",
    "git@github.com:Vitalisimon/desktoplab.git",
  ]) {
    const report = assessPublicReleaseSource({ origin, head, publishedHead: head, treeState: "clean", trackedPaths: ["Cargo.lock"] });
    assert.equal(report.status, "pass");
  }
});

test("rejects the private history repository and private tracked paths", () => {
  const report = assessPublicReleaseSource({
    origin: "https://github.com/Vitalisimon/desktoplab-private-history.git",
    head,
    publishedHead: head,
    treeState: "clean",
    trackedPaths: ["AGENTS.md", "docs/private.md"],
  });
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /not canonical public repository/);
  assert.deepEqual(report.privatePaths, ["AGENTS.md", "docs/private.md"]);
});

test("rejects a tracked transport manifest from canonical release source", () => {
  const report = assessPublicReleaseSource({
    origin: "https://github.com/Vitalisimon/desktoplab.git",
    head,
    publishedHead: head,
    treeState: "clean",
    trackedPaths: ["Cargo.lock", "PUBLIC_EXPORT_MANIFEST.json"],
  });
  assert.equal(report.status, "fail");
  assert.deepEqual(report.privatePaths, ["PUBLIC_EXPORT_MANIFEST.json"]);
});

test("historyless export regenerates instead of copying the transport manifest", () => {
  const source = readFileSync("scripts/product/create-public-export.mjs", "utf8");
  assert.match(source, /excludedExact[\s\S]*PUBLIC_EXPORT_MANIFEST\.json/);
});

test("rejects a clean canonical checkout whose HEAD is not published", () => {
  const report = assessPublicReleaseSource({
    origin: "https://github.com/Vitalisimon/desktoplab.git",
    head,
    publishedHead: "b".repeat(40),
    treeState: "clean",
    trackedPaths: ["Cargo.lock"],
  });
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /not the published canonical main HEAD/);
});

test("rejects dirty or malformed source state", () => {
  const report = assessPublicReleaseSource({
    origin: "https://github.com/Vitalisimon/desktoplab.git",
    head: "short",
    publishedHead: null,
    treeState: "dirty",
  });
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /HEAD is missing or invalid/);
  assert.match(report.failures.join("\n"), /tree is not clean/);
});

test("normalizes supported repository origins", () => {
  assert.equal(normalizeRepositoryOrigin("https://github.com/Vitalisimon/desktoplab.git"), "github.com/vitalisimon/desktoplab");
  assert.equal(normalizeRepositoryOrigin("git@github.com:Vitalisimon/desktoplab.git"), "github.com/vitalisimon/desktoplab");
});

test("release-producing lanes invoke the canonical source verifier", () => {
  for (const path of [
    "scripts/packaging/prepare-macos-candidate.sh",
    "scripts/packaging/promote-macos-candidate.sh",
    ".github/workflows/linux-release-signing.yml",
    ".github/workflows/windows-signpath.yml",
    ".github/workflows/release-draft.yml",
  ]) {
    assert.match(readFileSync(path, "utf8"), /release:verify-public-source/, `${path} bypasses canonical source verification`);
  }
});

test("the legacy monolithic macOS command is inert instead of a release lane", () => {
  const source = readFileSync("scripts/packaging/build-macos-release.sh", "utf8");
  assert.match(source, /monolithic macOS release command is disabled/);
  assert.match(source, /exit 1/);
  assert.doesNotMatch(source, /tauri build|codesign|notarytool/);
});
