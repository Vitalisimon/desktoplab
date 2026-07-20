import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflow = readFileSync(".github/workflows/packaging.yml", "utf8");
const devBuild = readFileSync("scripts/packaging/build-dev.sh", "utf8");

test("packaging workflow pins every external action to an immutable commit", () => {
  assert.doesNotMatch(workflow, /uses: [^\n]+@(?![a-f0-9]{40}\b)/);
});

test("dev packaging preserves verbose native bundler diagnostics", () => {
  assert.equal(devBuild.match(/npm exec tauri -- build --verbose/g)?.length, 2);
  assert.match(devBuild, /linux-packaging-path\.sh/);
});

test("packaging workflow validates all targets without linking every test binary", () => {
  assert.equal(matches(/cargo check --locked --workspace --all-targets/g), 3);
  assert.equal(
    matches(/cargo check --locked --manifest-path apps\/desktop\/src-tauri\/Cargo\.toml --all-targets/g),
    3,
  );
  assert.doesNotMatch(workflow, /cargo test --locked --workspace --no-run/);
});

test("packaging workflow bounds CI artifact growth", () => {
  assert.match(workflow, /CARGO_INCREMENTAL: "0"/);
  assert.match(workflow, /CARGO_PROFILE_DEV_DEBUG: "0"/);
  assert.match(workflow, /CARGO_PROFILE_TEST_DEBUG: "0"/);
});

test("packaging workflow retains native bundles with their evidence", () => {
  assert.equal(matches(/dist\/desktoplab-packaging\/\*\*/g), 3);
  assert.equal(
    matches(/apps\/desktop\/src-tauri\/target\/debug\/bundle\/\*\*/g),
    3,
  );
});

test("Windows packaging initializes the x64 MSVC environment", () => {
  const windowsStart = workflow.indexOf("  package-windows:");
  const linuxStart = workflow.indexOf("  package-linux:");
  const windowsJob = workflow.slice(windowsStart, linuxStart);
  const setup = "ilammy/msvc-dev-cmd@0b201ec74fa43914dc39ae48a89fd1d8cb592756";

  assert.equal(matches(new RegExp(setup, "g")), 1);
  assert.match(windowsJob, /Set up MSVC x64 environment/);
  assert.match(windowsJob, /arch: x64/);
  assert.ok(windowsJob.indexOf(setup) < windowsJob.indexOf("Validate locked dependency graphs"));
});

test("Windows packaging reports source drift without weakening provenance verification", () => {
  const windowsStart = workflow.indexOf("  package-windows:");
  const linuxStart = workflow.indexOf("  package-linux:");
  const windowsJob = workflow.slice(windowsStart, linuxStart);

  assert.match(windowsJob, /Verify current-head artifact manifest and checksums/);
  assert.match(windowsJob, /Report source drift after a failed Windows package/);
  assert.match(windowsJob, /if: failure\(\) && inputs\.channel == 'dev'/);
  assert.match(windowsJob, /git status --short/);
  assert.match(windowsJob, /git diff -- apps\/desktop\/src-tauri\/Cargo\.toml/);
});

test("Linux packaging installs native Tauri dependencies before validation", () => {
  const linuxJob = workflow.slice(workflow.indexOf("  package-linux:"));
  assert.equal(matches(/Install Linux build prerequisites/g), 1);
  assert.equal(matches(/Verify self-hosted Linux build prerequisites/g), 1);
  assert.ok(linuxJob.indexOf("Install Linux build prerequisites") >= 0);
  assert.ok(
    linuxJob.indexOf("Install Linux build prerequisites")
      < linuxJob.indexOf("Validate locked dependency graphs"),
  );
  assert.match(linuxJob, /if: inputs\.runner_profile == 'github-hosted'/);
  assert.match(linuxJob, /if: inputs\.runner_profile == 'self-hosted'/);
  assert.match(linuxJob, /bash scripts\/packaging\/check-linux-build-prereqs\.sh/);
  for (const dependency of [
    "libayatana-appindicator3-dev",
    "librsvg2-dev",
    "libwebkit2gtk-4.1-dev",
    "patchelf",
    "rpm",
  ]) {
    assert.match(linuxJob, new RegExp(`\\b${dependency}\\b`));
  }
});

function matches(pattern) {
  return workflow.match(pattern)?.length ?? 0;
}
