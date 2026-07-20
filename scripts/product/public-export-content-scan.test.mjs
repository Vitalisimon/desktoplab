import assert from "node:assert/strict";
import test from "node:test";
import {
  buildDynamicForbiddenPatterns,
  decodeTextCandidate,
} from "./public-export-content-scan.mjs";

test("binary assets are excluded from private wording scans", () => {
  const pngLike = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x00, 0x6e, 0x35]);

  assert.equal(decodeTextCandidate(pngLike), null);
});

test("short machine hostnames do not create random export findings", () => {
  const patterns = buildDynamicForbiddenPatterns({
    home: "/home/desktoplab-runner",
    hostname: "ci",
    blocklist: "",
  });
  const packageLockFragment = '"integrity":"sha512-AbCciDef"';

  assert.equal(patterns.some((pattern) => pattern.test(packageLockFragment)), false);
});

test("distinctive hostnames and explicit values remain bounded findings", () => {
  const patterns = buildDynamicForbiddenPatterns({
    home: "/home/desktoplab-runner",
    hostname: "private-ci-host",
    blocklist: "ci,secret-host",
  });

  assert.equal(patterns.some((pattern) => pattern.test("host=private-ci-host")), true);
  assert.equal(patterns.some((pattern) => pattern.test("runner ci ready")), true);
  assert.equal(patterns.some((pattern) => pattern.test("sha512-AbCciDef")), false);
  assert.equal(patterns.some((pattern) => pattern.test("/home/desktoplab-runner/work")), true);
});
