import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const scriptPath = new URL("./prune-build-cache.mjs", import.meta.url);
const policyPath = new URL("./build-cache-policy.mjs", import.meta.url);

test("cache maintenance prunes known build caches over budget", () => {
  withFixture(({ root, marker }) => {
    writeCache(root, "target", 256);
    writeCache(root, "node_modules", 512);
    writeCache(root, "dist/public-publication/candidate-old/target", 512);
    writeCache(root, "dist/public-publication/candidate-next/node_modules", 512);
    const legacyBuildInfo = path.join(
      root,
      "dist/public-publication/candidate-next/apps/desktop/legacy.tsbuildinfo",
    );
    mkdirSync(path.dirname(legacyBuildInfo), { recursive: true });
    writeFileSync(legacyBuildInfo, Buffer.alloc(512));
    const evidence = path.join(
      root,
      "dist/public-publication/candidate-old/dist/release/evidence.json",
    );
    mkdirSync(path.dirname(evidence), { recursive: true });
    writeFileSync(evidence, "{}\n");
    const result = runMaintenance(root, marker, { budget: 1024 });

    assert.equal(result.status, 0, result.stdout + result.stderr);
    assert.equal(existsSync(path.join(root, "target")), false);
    assert.equal(existsSync(path.join(root, "node_modules", "cache.bin")), true);
    assert.equal(existsSync(path.join(root, "dist/public-publication/candidate-old/target")), false);
    assert.equal(existsSync(path.join(root, "dist/public-publication/candidate-next/node_modules", "cache.bin")), true);
    assert.equal(
      existsSync(legacyBuildInfo),
      false,
    );
    assert.equal(readFileSync(evidence, "utf8"), "{}\n");
    assert.equal(readMarker(marker).reason, "over_budget");
  });
});

test("TypeScript incremental metadata is routed into the frontend build cache", () => {
  for (const configPath of ["tsconfig.app.json", "tsconfig.node.json"]) {
    const config = JSON.parse(readFileSync(path.join("apps/desktop", configPath), "utf8"));
    assert.match(config.compilerOptions.tsBuildInfoFile, /^\.\/dist\/cache\//);
  }
});

test("cache maintenance removes archived expanded apps but preserves release evidence", () => {
  withFixture(({ root, marker }) => {
    const release = path.join(root, "dist", "release", "macos", "abc1234");
    mkdirSync(path.join(release, "DesktopLab.app", "Contents"), { recursive: true });
    writeFileSync(path.join(release, "DesktopLab.app", "Contents", "payload"), "app");
    writeFileSync(path.join(release, "DesktopLab_0.1.0_aarch64.dmg"), "archive");
    writeFileSync(path.join(release, "installed-agent-certification.json"), "{}\n");

    const result = runMaintenance(root, marker, { budget: 1024 });

    assert.equal(result.status, 0, result.stdout + result.stderr);
    assert.equal(existsSync(path.join(release, "DesktopLab.app")), false);
    assert.equal(existsSync(path.join(release, "DesktopLab_0.1.0_aarch64.dmg")), true);
    assert.equal(existsSync(path.join(release, "installed-agent-certification.json")), true);
    assert.equal(readMarker(marker).reason, "expanded_release_residue");
  });
});

test("cache maintenance retains an unarchived release app", () => {
  withFixture(({ root, marker }) => {
    const app = path.join(root, "dist", "release", "macos", "abc1234", "DesktopLab.app");
    mkdirSync(app, { recursive: true });
    writeFileSync(path.join(app, "payload"), "app");

    const result = runMaintenance(root, marker, { budget: 1024 });

    assert.equal(result.status, 0, result.stdout + result.stderr);
    assert.equal(existsSync(app), true);
  });
});

test("cache maintenance records a baseline without pruning a small cache", () => {
  withFixture(({ root, marker }) => {
    writeCache(root, "target", 512);
    const result = runMaintenance(root, marker, { budget: 1024 });

    assert.equal(result.status, 0, result.stdout + result.stderr);
    assert.equal(existsSync(path.join(root, "target", "cache.bin")), true);
    assert.equal(readMarker(marker).status, "observed");
  });
});

test("cache maintenance prunes after the configured interval", () => {
  withFixture(({ root, marker }) => {
    writeCache(root, "target", 512);
    mkdirSync(path.dirname(marker), { recursive: true });
    writeFileSync(marker, JSON.stringify({ lastPrunedAt: "2000-01-01T00:00:00.000Z" }));
    const result = runMaintenance(root, marker, { budget: 1024, intervalMs: 1000 });

    assert.equal(result.status, 0, result.stdout + result.stderr);
    assert.equal(existsSync(path.join(root, "target")), false);
    assert.equal(readMarker(marker).reason, "interval_elapsed");
  });
});

test("cache maintenance sources stay focused", () => {
  assert.ok(readFileSync(scriptPath, "utf8").split("\n").length <= 130);
  assert.ok(readFileSync(policyPath, "utf8").split("\n").length <= 80);
  assert.ok(readFileSync(new URL(import.meta.url), "utf8").split("\n").length <= 150);
});

function withFixture(run) {
  const root = mkdtempSync(path.join(tmpdir(), "desktoplab-cache-prune-"));
  const marker = path.join(root, "dist", "cache-maintenance.json");
  try {
    run({ root, marker });
  } finally {
    rmSync(root, { force: true, recursive: true });
  }
}

function writeCache(root, relativePath, bytes) {
  const target = path.join(root, relativePath);
  mkdirSync(target, { recursive: true });
  writeFileSync(path.join(target, "cache.bin"), Buffer.alloc(bytes));
}

function runMaintenance(root, marker, { budget, intervalMs = 7 * 24 * 60 * 60 * 1000 }) {
  return spawnSync(process.execPath, [scriptPath.pathname], {
    cwd: root,
    encoding: "utf8",
    env: {
      ...process.env,
      DESKTOPLAB_BUILD_CACHE_BUDGET_BYTES: String(budget),
      DESKTOPLAB_BUILD_CACHE_PRUNE_INTERVAL_MS: String(intervalMs),
      DESKTOPLAB_BUILD_CACHE_MARKER: marker,
    },
  });
}

function readMarker(marker) {
  return JSON.parse(readFileSync(marker, "utf8"));
}
