import assert from "node:assert/strict";
import { closeSync, ftruncateSync, mkdirSync, mkdtempSync, openSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const FIVE_GIB = 5 * 1024 * 1024 * 1024;
const guardPath = new URL("./artifact-budget-guard.mjs", import.meta.url);

test("build cache guard fails closed above five GiB", () => {
  const result = runGuardWithSparseCache(FIVE_GIB + 1);

  assert.equal(result.status, 1, result.stdout + result.stderr);
  assert.match(result.stderr, /build caches use 5\.0GB, budget is 5\.0GB/);
});

test("build cache guard allows exactly five GiB", () => {
  const result = runGuardWithSparseCache(FIVE_GIB);

  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stdout, /5\.0GB build cache/);
});

test("build cache guard includes every staged public checkout", () => {
  const result = runGuardWithSparseCache(
    FIVE_GIB + 1,
    "dist/public-publication/candidate-next/target",
  );

  assert.equal(result.status, 1, result.stdout + result.stderr);
  assert.match(result.stderr, /build caches use 5\.0GB, budget is 5\.0GB/);
});

test("build cache guard includes legacy TypeScript build metadata", () => {
  const result = runGuardWithSparseCache(
    FIVE_GIB + 1,
    "dist/public-publication/candidate-next/apps/desktop",
    "legacy.tsbuildinfo",
  );

  assert.equal(result.status, 1, result.stdout + result.stderr);
});

test("artifact budget guard tests stay focused", () => {
  assert.ok(readFileSync(guardPath, "utf8").split("\n").length <= 130);
  assert.ok(readFileSync(new URL(import.meta.url), "utf8").split("\n").length <= 100);
});

function runGuardWithSparseCache(bytes, relativePath = "target", filename = "cache.bin") {
  const root = mkdtempSync(path.join(tmpdir(), "desktoplab-cache-budget-"));
  const target = path.join(root, relativePath);
  mkdirSync(target, { recursive: true });
  const descriptor = openSync(path.join(target, filename), "w");
  ftruncateSync(descriptor, bytes);
  closeSync(descriptor);

  try {
    return spawnSync(process.execPath, [guardPath.pathname], {
      cwd: root,
      encoding: "utf8",
      env: { ...process.env, DESKTOPLAB_BUILD_CACHE_BUDGET_BYTES: "" },
    });
  } finally {
    rmSync(root, { force: true, recursive: true });
  }
}
