import assert from "node:assert/strict";
import test from "node:test";

import { collectReliabilityRuns } from "./reliability-run-collector.mjs";

test("recorder preserves a failed run and continues through the full matrix", async () => {
  const descriptors = [descriptor("one"), descriptor("two"), descriptor("three")];
  const visited = [];
  const progress = [];
  const runs = await collectReliabilityRuns({
    descriptors,
    root: "/tmp/reliability",
    record: async (entry) => {
      visited.push(entry.runId);
      if (entry.runId === "run-two") {
        const error = new Error("installed UI case inspect requested an unexpected approval");
        error.reliabilityDiagnostics = { accessibility: { buttons: ["Working"] } };
        throw error;
      }
      return { caseId: entry.caseId, seed: entry.seed, profileId: entry.profileId, repetition: 1 };
    },
    onProgress: (entry) => progress.push(entry.run.recordingStatus),
  });

  assert.deepEqual(visited, ["run-one", "run-two", "run-three"]);
  assert.deepEqual(progress, ["completed", "failed", "completed"]);
  assert.equal(runs[1].operationalStatus, "infrastructure_failure");
  assert.deepEqual(runs[1].diagnostics, { accessibility: { buttons: ["Working"] } });
  assert.match(runs[1].workspacePath, /run-two\/workspace$/);
});

test("recorder keeps timeout outcomes explicit without stopping later runs", async () => {
  const descriptors = [descriptor("slow"), descriptor("after")];
  const runs = await collectReliabilityRuns({
    descriptors,
    root: "/tmp/reliability",
    record: async (entry) => {
      if (entry.runId === "run-slow") throw new Error("case did not complete before timeout");
      return { caseId: entry.caseId, seed: entry.seed, profileId: entry.profileId, repetition: 1 };
    },
  });
  assert.equal(runs[0].operationalStatus, "timeout");
  assert.equal(runs[1].recordingStatus, "completed");
});

test("recorder resumes matching checkpointed runs without executing them again", async () => {
  const descriptors = [descriptor("saved"), descriptor("new")];
  const visited = [];
  const checkpointed = [];
  const saved = { ...descriptors[0], recordingStatus: "completed" };
  const runs = await collectReliabilityRuns({
    descriptors,
    root: "/tmp/reliability",
    existingRuns: [saved],
    record: async (entry) => { visited.push(entry.runId); return entry; },
    checkpoint: async (run) => checkpointed.push(run.runId),
  });
  assert.deepEqual(runs.map((run) => run.runId), ["run-saved", "run-new"]);
  assert.deepEqual(visited, ["run-new"]);
  assert.deepEqual(checkpointed, ["run-new"]);
});

function descriptor(id) {
  return { runId: `run-${id}`, caseId: "inspect", seed: id.length, profileId: "medium", repetition: 1 };
}
