import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  assessRuntimeCertification,
  runtimeCertificationTemplate,
} from "../product/runtime-certification-core.mjs";

const digest = (character) => `sha256:${character.repeat(64)}`;
const checks = [
  "install_or_connect",
  "restart_recovery",
  "stale_process_recovery",
  "first_model_response",
  "protocol_tool_canary",
  "desktoplab_tool_execution",
  "approval_boundary",
  "user_owned_process_preservation",
  "egress_observation",
  "cleanup_ownership",
];

test("live evidence passes only for the exact candidate runtime model and host", () => {
  const expected = scope();
  const report = assessRuntimeCertification(expected, evidence(expected));

  assert.equal(report.status, "pass");
  assert.equal(report.publicSupportClaim, true);
  assert.equal(report.scope.runtimeId, "runtime.mlx-lm");
  assert.equal(report.distinctions.agenticReliability, false);
});

test("deterministic evidence cannot authorize a public runtime claim", () => {
  const expected = scope();
  const deterministic = evidence(expected);
  deterministic.evidenceKind = "deterministic_adapter";

  const report = assessRuntimeCertification(expected, deterministic);

  assert.equal(report.status, "deterministic_pass");
  assert.equal(report.publicSupportClaim, false);
  assert.equal(report.distinctions.deterministicAdapter, true);
});

test("scope drift and non-loopback endpoints fail closed", () => {
  const expected = scope();
  const drifted = evidence(expected);
  drifted.appHash = digest("9");
  drifted.runtime.endpoint = `http://${["192", "168", "1", "10"].join(".")}:18080`;
  drifted.model.revision = "f".repeat(40);

  const report = assessRuntimeCertification(expected, drifted);

  assert.equal(report.status, "blocked");
  assert.match(report.failures.join("\n"), /appHash|loopback|modelRevision/);
});

test("every lifecycle safety check requires its own evidence digest", () => {
  const expected = scope();
  for (const id of checks) {
    const missing = evidence(expected);
    missing.checks[id] = { status: "not_run", evidenceSha256: "" };
    const report = assessRuntimeCertification(expected, missing);
    assert.equal(report.status, "blocked", id);
    assert.match(report.failures.join("\n"), new RegExp(id));
  }
});

test("template is inert and explicitly not run", () => {
  const template = runtimeCertificationTemplate(scope());
  assert.equal(template.evidenceKind, "live_installed_app");
  assert.ok(checks.every((id) => template.checks[id].status === "not_run"));
});

test("runtime certification modules stay focused", () => {
  for (const [path, maximum] of [
    ["../product/runtime-certification-core.mjs", 260],
    ["../product/runtime-certification.mjs", 70],
  ]) {
    const source = readFileSync(new URL(path, import.meta.url), "utf8");
    const logical = source.split("\n").filter((line) => line.trim()).length;
    assert.ok(logical <= maximum, `${path} has ${logical} lines, max ${maximum}`);
  }
});

function scope() {
  return {
    kind: "desktoplab.runtime-certification-expected",
    schemaVersion: 1,
    candidateId: digest("a"),
    appHash: digest("b"),
    sourceHead: "c".repeat(40),
    platform: "macos",
    architecture: "arm64",
    runtimeId: "runtime.mlx-lm",
    ownership: "desktoplab_managed",
    modelId: "mlx-community/SmolLM3-3B-4bit",
    modelRevision: "d3a7e0594d6642dbcfb7d149bed8b0bdf49f95ce",
  };
}

function evidence(expected) {
  return {
    ...runtimeCertificationTemplate(expected),
    generatedAt: "2026-07-30T12:00:00.000Z",
    host: {
      platform: expected.platform,
      architecture: expected.architecture,
      hostIdSha256: digest("d"),
    },
    runtime: {
      id: expected.runtimeId,
      version: "0.31.3",
      ownership: expected.ownership,
      source: "https://pypi.org/project/mlx-lm/0.31.3/",
      integrity: digest("e"),
      endpoint: "http://127.0.0.1:18080",
    },
    model: {
      id: expected.modelId,
      revision: expected.modelRevision,
      license: "apache-2.0",
      quantization: "4bit",
    },
    checks: Object.fromEntries(
      checks.map((id, index) => [
        id,
        { status: "pass", evidenceSha256: digest(String((index % 9) + 1)) },
      ]),
    ),
    egress: {
      observed: true,
      destinations: [
        { purpose: "model_acquisition", host: "huggingface.co" },
        { purpose: "bootstrap", host: "github.com" },
      ],
    },
    artifacts: [
      { kind: "runtime_report", path: "evidence/runtime.json", sha256: digest("f") },
    ],
  };
}
