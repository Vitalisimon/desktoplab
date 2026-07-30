import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  assessRuntimeCertificationMatrix,
  requiredRuntimeRoutes,
} from "../product/runtime-certification-matrix-core.mjs";

const digest = (character) => `sha256:${character.repeat(64)}`;
const candidate = {
  kind: "desktoplab.release-candidate",
  schemaVersion: 1,
  candidateId: digest("a"),
  state: "payload_built",
  source: { commit: "b".repeat(40), treeState: "clean" },
  payload: { platform: "macos-aarch64", sha256: "c".repeat(64) },
};

test("requires four exact live runtime routes for the same candidate and app", () => {
  const result = assessRuntimeCertificationMatrix(matrixInput());

  assert.equal(result.status, "pass");
  assert.equal(result.publicSupportClaim, true);
  assert.deepEqual(result.routes.map(({ key }) => key), requiredRuntimeRoutes.map(({ key }) => key));
  assert.ok(result.routes.every((route) => route.status === "pass"));
});

test("does not mistake the 25-case campaign or deterministic evidence for runtime proof", () => {
  const input = matrixInput();
  input.reports.mlx_lm_managed.evidenceClass = "deterministic_adapter";
  input.reports.mlx_lm_managed.distinctions.liveRuntimeCertification = false;

  const result = assessRuntimeCertificationMatrix(input);

  assert.equal(result.status, "blocked");
  assert.match(result.failures.join("\n"), /mlx_lm_managed.*not live/);
});

test("rejects missing, swapped, stale or incomplete route evidence", () => {
  const input = matrixInput();
  input.reports.ollama_managed = null;
  input.reports.lm_studio_existing.scope.ownership = "desktoplab_managed";
  input.reports.lm_studio_managed.appHash = digest("9");
  input.reports.mlx_lm_managed.checks.pop();

  const result = assessRuntimeCertificationMatrix(input);

  assert.equal(result.status, "blocked");
  assert.match(result.failures.join("\n"), /ollama_managed.*invalid/);
  assert.match(result.failures.join("\n"), /lm_studio_existing.*ownership/);
  assert.match(result.failures.join("\n"), /lm_studio_managed.*appHash/);
  assert.match(result.failures.join("\n"), /mlx_lm_managed.*incomplete/);
});

test("rejects candidate, source and report-digest drift", () => {
  const input = matrixInput();
  input.appHash = digest("9");
  input.sourceHead = "d".repeat(40);
  input.reportDigests.lm_studio_existing = "invalid";

  const result = assessRuntimeCertificationMatrix(input);

  assert.equal(result.status, "blocked");
  assert.match(result.failures.join("\n"), /installed app|source HEAD|report digest/);
});

test("runtime matrix modules stay focused", () => {
  for (const [path, maximum] of [
    ["../product/runtime-certification-matrix-core.mjs", 180],
    ["../product/runtime-certification-matrix.mjs", 90],
  ]) {
    const source = readFileSync(new URL(path, import.meta.url), "utf8");
    const logical = source.split("\n").filter((line) => line.trim()).length;
    assert.ok(logical <= maximum, `${path} has ${logical} lines, max ${maximum}`);
  }
});

function matrixInput() {
  const reports = Object.fromEntries(requiredRuntimeRoutes.map((route) => [route.key, report(route)]));
  const reportDigests = Object.fromEntries(
    requiredRuntimeRoutes.map((route, index) => [route.key, digest(String(index + 1))]),
  );
  return {
    candidate: structuredClone(candidate),
    appHash: digest("c"),
    sourceHead: "b".repeat(40),
    reports,
    reportDigests,
  };
}

function report(route) {
  return {
    kind: "desktoplab.runtime-certification",
    schemaVersion: 1,
    status: "pass",
    publicSupportClaim: true,
    evidenceClass: "live_installed_app",
    candidateId: candidate.candidateId,
    appHash: digest("c"),
    sourceHead: candidate.source.commit,
    scope: {
      platform: "macos",
      architecture: "arm64",
      runtimeId: route.runtimeId,
      runtimeVersion: "1.2.3",
      ownership: route.ownership,
      modelId: `model/${route.key}`,
      modelRevision: "d".repeat(40),
    },
    checks: Array.from({ length: 10 }, (_, index) => ({ id: `check_${index}`, status: "pass" })),
    distinctions: { liveRuntimeCertification: true, agenticReliability: false },
    failures: [],
  };
}
