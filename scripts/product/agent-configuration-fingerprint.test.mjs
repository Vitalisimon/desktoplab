import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { compareAgentConfigurations, fingerprintAgentConfiguration } from "./agent-configuration-fingerprint.mjs";
import { agentConfiguration as configuration, sha256 as sha } from "./test-fixtures/agent-configuration-fixture.mjs";

test("same configuration produces a stable redacted fingerprint", () => {
  const first = fingerprintAgentConfiguration(configuration());
  const second = fingerprintAgentConfiguration({ ...configuration(), hostname: "private-host", workspacePath: "/Users/private/repo" });

  assert.equal(first.status, "pass");
  assert.equal(first.fingerprint, second.fingerprint);
  assert.ok(!JSON.stringify(first).includes("private-host"));
  assert.ok(!JSON.stringify(first).includes("/Users/private"));
});

test("plugin order is canonical and malformed capability data is blocked", () => {
  const first = { id: "plugin.first", version: "1", digest: sha("e") };
  const second = { id: "plugin.second", version: "1", digest: sha("f") };
  assert.equal(
    fingerprintAgentConfiguration(configuration({ plugins: [first, second] })).fingerprint,
    fingerprintAgentConfiguration(configuration({ plugins: [second, first] })).fingerprint,
  );
  assert.equal(fingerprintAgentConfiguration(configuration({ plugins: [first, first] })).status, "blocked");
  assert.equal(fingerprintAgentConfiguration(configuration({
    hostCapabilities: { ...configuration().hostCapabilities, unifiedMemory: "yes" },
  })).status, "blocked");
});

test("one changed factor is controlled A/B and multiple factors are non-comparable", () => {
  const baseline = configuration();
  const approvalVariant = configuration({ approvalMode: "workspace_write" });
  const controlled = compareAgentConfigurations(baseline, approvalVariant);
  assert.equal(controlled.comparison, "controlled_ab");
  assert.deepEqual(controlled.changedFactors, ["approvalMode"]);

  const drifted = configuration({
    approvalMode: "workspace_write",
    runtime: { ...baseline.runtime, version: "0.7.0" },
  });
  const drift = compareAgentConfigurations(baseline, drifted);
  assert.equal(drift.comparison, "non_comparable_drift");
  assert.deepEqual(drift.changedFactors, ["approvalMode", "runtime"]);
});

test("tool schema and runtime drift invalidate direct comparison", () => {
  const baseline = configuration();
  const toolDrift = compareAgentConfigurations(baseline, configuration({ toolSchemaDigest: sha("b") }));
  assert.equal(toolDrift.comparison, "controlled_ab");
  assert.deepEqual(toolDrift.changedFactors, ["toolSchema"]);

  const runtimeDrift = compareAgentConfigurations(baseline, configuration({
    runtime: { ...baseline.runtime, backendVersion: "2" },
  }));
  assert.deepEqual(runtimeDrift.changedFactors, ["runtime"]);
});

test("adaptive context and timeout policy are fingerprinted", () => {
  const baseline = configuration();
  const changed = configuration({ adaptivePolicy: { ...baseline.adaptivePolicy, requestTimeoutSeconds: 600 } });
  const comparison = compareAgentConfigurations(baseline, changed);
  assert.equal(comparison.comparison, "controlled_ab");
  assert.deepEqual(comparison.changedFactors, ["adaptivePolicy"]);
});

test("hardware evidence is truthful across macOS Linux and Windows", () => {
  for (const os of ["darwin", "linux", "win32"]) {
    const result = fingerprintAgentConfiguration(configuration({
      hostCapabilities: { ...configuration().hostCapabilities, os },
    }));
    assert.equal(result.status, "pass", `${os}: ${result.failures.join("; ")}`);
    assert.equal(result.canonical.hostCapabilities.os, os);
  }
  const invalid = fingerprintAgentConfiguration(configuration({
    hostCapabilities: { ...configuration().hostCapabilities, os: "macOS" },
  }));
  assert.equal(invalid.status, "blocked");
});

test("fingerprint source stays below line guard", () => {
  const logical = readFileSync("scripts/product/agent-configuration-fingerprint.mjs", "utf8")
    .split("\n")
    .filter((line) => line.trim()).length;
  assert.ok(logical <= 240, `agent-configuration-fingerprint.mjs has ${logical} logical lines`);
});
