import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { parse } from "yaml";

const workflow = readFileSync(".github/workflows/release-draft.yml", "utf8");

test("draft workflow is valid YAML", () => {
  assert.doesNotThrow(() => parse(workflow));
});

test("draft workflow uses an existing exact tag and matching successful runs", () => {
  assert.match(workflow, /release_ref:/);
  assert.match(workflow, /git rev-parse "\$RELEASE_REF\^\{commit\}"/);
  assert.match(workflow, /test "\$GITHUB_REF" = "\$RELEASE_REF"/);
  assert.match(workflow, /environment: release-draft/);
  assert.match(workflow, /headSha/);
  assert.match(workflow, /conclusion/);
  assert.match(workflow, /candidate_state_artifact/);
  assert.match(workflow, /release:verify-tag/);
  assert.match(workflow, /release:verify-platform-convergence/);
  assert.match(workflow, /cross_platform_pass/);
  assert.match(workflow, /--candidate "\$CANDIDATE_STATE"/);
  assert.match(workflow, /--verify-tag/);
  assert.doesNotMatch(workflow, /RELEASE_TAG: v\$\{\{/);
});

test("draft workflow assembles verified metadata and never publishes", () => {
  assert.match(workflow, /release:sbom/);
  assert.match(workflow, /packaging:verify:updater-disabled/);
  assert.match(workflow, /prepare-release-assembly\.mjs/);
  assert.match(workflow, /release-files\.txt/);
  assert.doesNotMatch(workflow, /desktoplab-packaging\/\*\*/);
  assert.doesNotMatch(workflow, /--draft=false|--latest|gh release edit/);
});

test("release assembly implementation stays reviewable", () => {
  const logical = readFileSync("scripts/release/prepare-release-assembly.mjs", "utf8").split("\n").filter((line) => line.trim() && !line.trim().startsWith("//")).length;
  assert.ok(logical <= 220, `prepare-release-assembly has ${logical} logical lines, limit 220`);
});
