import assert from "node:assert/strict";
import test from "node:test";

import { installedAgentUiDriverEvidence } from "./macos-installed-agent-ui.mjs";
import { reliabilityUiDriverEvidence } from "./macos-installed-agent-reliability-ui.mjs";
import { installedAgentUiWaitModulePath, latestTerminalTurn, waitForActiveUi } from "./macos-installed-agent-ui-wait.mjs";

test("foreground UI probes reactivate DesktopLab while a control is pending", async () => {
  const activations = [];
  const observations = [false, false, true];
  const times = [0, 1_000, 6_000];
  const ui = { activate: () => activations.push(times[0]), hasButton: () => observations.shift() };
  const wait = async (probe) => {
    while (true) {
      const value = probe();
      times.shift();
      if (value) return value;
    }
  };
  const visible = await waitForActiveUi(ui, () => ui.hasButton("Deny"), 30_000, "approval", {
    wait,
    now: () => times[0],
  });
  assert.equal(visible, true);
  assert.equal(activations.length, 2);
});

test("terminal turn failures retain the persisted backend reason", () => {
  const session = {
    events: [{ kind: "created", sessionId: "session.1" }, { kind: "failed", reason: "model_failure:model_protocol_error" }],
    trace: [
      { kind: "completed", recordedAtUnixMs: 100 },
      { kind: "failed", recordedAtUnixMs: 300 },
    ],
  };
  assert.deepEqual(latestTerminalTurn(session, 200), {
    kind: "failed",
    reason: "model_failure:model_protocol_error",
  });
  assert.equal(latestTerminalTurn(session, 400), null);
});

test("both installed driver bundles bind the foreground wait helper", () => {
  for (const evidence of [installedAgentUiDriverEvidence(), reliabilityUiDriverEvidence()]) {
    assert.ok(evidence.dependencies.some((dependency) => dependency.path === installedAgentUiWaitModulePath));
  }
});
