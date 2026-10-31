import assert from "node:assert/strict";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";
import test from "node:test";

import { certifiedReliabilityProtocol, deriveReliabilityConfiguration } from "./agent-reliability-configuration.mjs";

test("reliability provenance uses the certified protocol and fails closed on stale evidence", () => {
  const fingerprint = `sha256:${"a".repeat(64)}`;
  const capabilities = { fingerprint, toolProtocolCertification: { state: "certified", protocol: "native_tools.v1", capabilityFingerprint: fingerprint } };
  const freeze = { runtime: { protocol: "native_tools.v1" }, model: { capabilityFingerprint: fingerprint } };
  assert.equal(certifiedReliabilityProtocol(capabilities, freeze), "native_tools.v1");
  const constrained = { ...capabilities, toolProtocolCertification: { ...capabilities.toolProtocolCertification, protocol: "constrained_json.v1" } };
  assert.equal(certifiedReliabilityProtocol(constrained, { ...freeze, runtime: { protocol: "constrained_json.v1" } }), "constrained_json.v1");
  assert.throws(() => certifiedReliabilityProtocol(constrained, freeze), /protocol/);
  assert.throws(() => certifiedReliabilityProtocol({ ...capabilities, toolProtocolCertification: null }, freeze), /protocol/);
  assert.throws(() => certifiedReliabilityProtocol({ ...capabilities, toolProtocolCertification: { ...capabilities.toolProtocolCertification, state: "failed" } }, freeze), /protocol/);
  assert.throws(() => certifiedReliabilityProtocol({ ...capabilities, toolProtocolCertification: { ...capabilities.toolProtocolCertification, capabilityFingerprint: `sha256:${"b".repeat(64)}` } }, freeze), /protocol/);
  assert.throws(() => certifiedReliabilityProtocol({ ...capabilities, fingerprint: undefined }, { ...freeze, model: {} }), /protocol/);
});

test("derived configuration binds the certified protocol instead of a hardcoded label", () => {
  const repoRoot = process.cwd();
  const freeze = JSON.parse(readFileSync(join(repoRoot, "evaluation/release-candidate-agent-freeze.json"), "utf8"));
  const model = { modelId: freeze.model.runtimeRef, digest: freeze.model.digest, quantization: freeze.model.quantization };
  const fingerprint = freeze.model.capabilityFingerprint;
  const statePath = join(mkdtempSync(join(tmpdir(), "desktoplab-protocol-test-")), "state.sqlite");
  const database = new DatabaseSync(statePath);
  database.exec("create table productization_state (kind text, subject_id text, payload text); create table settings (key text, value text)");
  const readiness = { state: "ready", modelCapabilities: { modelId: model.modelId, fingerprint, toolProtocolCertification: { state: "certified", protocol: freeze.runtime.protocol, capabilityFingerprint: fingerprint } } };
  database.prepare("insert into productization_state values (?, ?, ?)").run("backend_readiness", "local", JSON.stringify(readiness));
  database.close();
  const actual = deriveReliabilityConfiguration({ statePath, repoRoot, model, freeze, versionOutput: () => `ollama version is ${freeze.runtime.version}` });
  assert.equal(actual.runtime.backendVersion, freeze.runtime.protocol);
});
