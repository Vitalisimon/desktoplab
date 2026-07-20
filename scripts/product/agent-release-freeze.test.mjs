import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { verifyAgentReleaseFreeze } from "./agent-release-freeze-core.mjs";

test("release freeze binds source, model, runtime and adaptive policy", () => {
  const root = mkdtempSync(join(tmpdir(), "desktoplab-agent-freeze-"));
  writeFileSync(join(root, "agent.rs"), "frozen\n");
  const freeze = fixture({ sources: [{ path: "agent.rs", sha256: sha("frozen\n") }] });
  const report = verifyAgentReleaseFreeze(freeze, { repoRoot: root, observed: observation() });
  assert.equal(report.status, "pass", report.failures.join("; "));
  assert.equal(report.sourceCount, 1);
});

test("source or runtime drift fails closed", () => {
  const root = mkdtempSync(join(tmpdir(), "desktoplab-agent-freeze-drift-"));
  writeFileSync(join(root, "agent.rs"), "changed\n");
  const freeze = fixture({ sources: [{ path: "agent.rs", sha256: sha("frozen\n") }] });
  const observed = observation();
  observed.runtime.version = "different";
  const report = verifyAgentReleaseFreeze(freeze, { repoRoot: root, observed });
  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /source drifted/);
  assert.match(report.failures.join("\n"), /runtime.version differs/);
});

test("committed release freeze verifies against the current source tree", () => {
  const freeze = JSON.parse(readFileSync("evaluation/release-candidate-agent-freeze.json", "utf8"));
  const report = verifyAgentReleaseFreeze(freeze, { repoRoot: process.cwd() });
  assert.equal(report.status, "pass", report.failures.join("; "));
  assert.ok(report.sourceCount >= 30);
  const frozenPaths = new Set(report.sources.map((source) => source.path));
  for (const path of [
    "crates/desktoplab-control-plane/src/agent_model_adapter.rs",
    "crates/desktoplab-control-plane/src/agent_completion_grounding.rs",
    "crates/desktoplab-control-plane/src/agent_execution_obligations.rs",
  ]) {
    assert.ok(frozenPaths.has(path), "required protocol source must be frozen");
  }
});

function fixture(overrides = {}) {
  return {
    kind: "desktoplab.agent-release-freeze",
    schemaVersion: 1,
    state: "frozen_for_reliability",
    model: observation().model,
    runtime: observation().runtime,
    adaptivePolicy: observation().adaptivePolicy,
    sources: [],
    ...overrides,
  };
}

function observation() {
  return {
    model: { id: "model.gemma4-12b-q4", runtimeRef: "gemma4:12b", digest: sha("model"), capabilityFingerprint: sha("capability"), quantization: "Q4_K_M" },
    runtime: { id: "runtime.ollama", version: "0.32.1", protocol: "native_tools.v1" },
    adaptivePolicy: { contextWindowTokens: 32768, requestTimeoutSeconds: 300, modelMaximumTokens: 262144, hostMemoryGb: 24 },
  };
}

function sha(value) {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
}
