import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { defaultRepositoryVisibilityMode } from "./repository-visibility-mode.mjs";

test("repository visibility defaults to public only for the canonical public repository", () => {
  assert.equal(defaultRepositoryVisibilityMode("https://github.com/Vitalisimon/desktoplab.git"), "public-export");
  assert.equal(defaultRepositoryVisibilityMode("git@github.com:Vitalisimon/desktoplab.git"), "public-export");
  assert.equal(defaultRepositoryVisibilityMode("https://github.com/Vitalisimon/desktoplab-private-history.git"), "internal");
  assert.equal(defaultRepositoryVisibilityMode("https://github.com/someone/desktoplab.git"), "internal");
  assert.equal(defaultRepositoryVisibilityMode(""), "internal");
});

test("prebuilt candidate mode never rebuilds the installed signing candidate", () => {
  const report = join(mkdtempSync(join(tmpdir(), "desktoplab-beta-gauntlet-")), "report.json");
  execFileSync(process.execPath, [
    "scripts/product/beta-gauntlet.mjs",
    "--profile",
    "full",
    "--mode",
    "internal",
    "--prebuilt-candidate",
    "--dry-run",
    "--report",
    report,
  ]);

  const evidence = JSON.parse(readFileSync(report, "utf8"));
  const ids = evidence.steps.map((step) => step.id);
  assert.equal(ids[0], "build-cache-maintenance");
  assert.equal(evidence.prebuiltCandidate, true);
  assert.equal(ids.includes("desktop-package-dev"), false);
  assert.equal(ids.includes("packaging-provenance-after-build"), false);
  assert.equal(ids.includes("packaging-provenance"), true);
  assert.equal(ids.includes("product-truth-real"), true);
  assert.equal(ids.includes("visual-product-audit"), true);
  assert.deepEqual(ids.slice(-2), ["build-cache-maintenance-final", "artifact-budget-final"]);
});

test("public export uses the public external-reference contract", () => {
  const report = join(mkdtempSync(join(tmpdir(), "desktoplab-beta-gauntlet-")), "report.json");
  execFileSync(process.execPath, [
    "scripts/product/beta-gauntlet.mjs",
    "--profile",
    "quick",
    "--mode",
    "public-export",
    "--dry-run",
    "--report",
    report,
  ]);

  const evidence = JSON.parse(readFileSync(report, "utf8"));
  const referenceStep = evidence.steps.find((step) => step.id === "external-reference-guard");
  assert.match(referenceStep.command, /external-reference-guard\.mjs --mode public-export$/);
  assert.deepEqual(
    evidence.steps.slice(-2).map((step) => step.id),
    ["build-cache-maintenance-final", "artifact-budget-final"],
  );
});
