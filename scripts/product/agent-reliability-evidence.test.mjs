import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { normalizeIsolation, sanitizedProvenance, validateRunEvidence } from "./agent-reliability-evidence.mjs";

const digest = (character) => `sha256:${character.repeat(64)}`;

test("run provenance preserves the complete versioned UI driver identity", () => {
  const provenance = validProvenance();
  assert.equal(sanitizedProvenance(provenance).uiDriverBundleSha256, provenance.uiDriverBundleSha256);
});

test("run evidence rejects a missing UI driver dependency bundle", () => {
  const root = mkdtempSync(join(tmpdir(), "desktoplab-reliability-evidence-"));
  const workspacePath = join(root, "workspace");
  const statePath = join(root, "state.sqlite");
  mkdirSync(join(workspacePath, ".git"), { recursive: true });
  writeFileSync(statePath, "state");
  const descriptor = { candidateId: digest("a"), appHash: digest("b") };
  const isolation = normalizeIsolation({ workspaceId: "workspace.1", workspacePath, sessionId: "session.1", statePath });
  const output = { trace: { sessionId: "session.1" }, provenance: validProvenance(descriptor) };
  assert.deepEqual(validateRunEvidence(descriptor, output, isolation), []);
  delete output.provenance.uiDriverBundleSha256;
  assert.match(validateRunEvidence(descriptor, output, isolation).join("\n"), /dependency bundle/);
});

function validProvenance(descriptor = { candidateId: digest("a"), appHash: digest("b") }) {
  return {
    executionKind: "installed_app_ui",
    candidateId: descriptor.candidateId,
    appHash: descriptor.appHash,
    modelRequestCount: 1,
    testControlRequests: 0,
    uiDriverSha256: digest("c"),
    uiDriverBundleSha256: digest("d"),
    interactionSha256: digest("e"),
    screenshotSha256: digest("f"),
  };
}
