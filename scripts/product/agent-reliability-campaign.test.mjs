import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { buildRunDescriptors, runReliabilityCampaign } from "./agent-reliability-campaign-core.mjs";
import { passingExecutableCase } from "./test-fixtures/agent-trace-score-fixture.mjs";
import { agentConfiguration } from "./test-fixtures/agent-configuration-fixture.mjs";
import { moduleSourceBundleDigest } from "./versioned-module-bundle.mjs";

const isolationRoot = mkdtempSync(join(tmpdir(), "desktoplab-agent-reliability-"));

test("campaign is reproducible and reports reliability statistics", async () => {
  const manifest = campaignManifest();
  const first = await runCampaign(manifest, isolatedPassingExecutor);
  const second = await runCampaign(manifest, isolatedPassingExecutor);

  assert.equal(first.status, "pass");
  assert.equal(first.plannedRunCount, 8);
  assert.equal(first.metrics.passRate, 1);
  assert.equal(first.metrics.passAll, true);
  assert.equal(first.metrics.passPowerK, 1);
  assert.equal(first.metrics.worstOfN, 1);
  assert.equal(first.metrics.scoreDispersion, 0);
  assert.ok(first.metrics.passRateConfidence95.low < 1);
  assert.equal(first.manifestDigest, second.manifestDigest);
  assert.deepEqual(first.runs.map((run) => run.runId), second.runs.map((run) => run.runId));
  assert.ok(!JSON.stringify(first).includes("private-host"));
});

test("candidate identity namespaces every campaign run", () => {
  const first = buildRunDescriptors(campaignManifest({ candidateId: `sha256:${"a".repeat(64)}` }));
  const second = buildRunDescriptors(campaignManifest({ candidateId: `sha256:${"b".repeat(64)}` }));
  assert.ok(first.every((run) => run.candidateId === `sha256:${"a".repeat(64)}`));
  assert.notDeepEqual(first.map((run) => run.runId), second.map((run) => run.runId));
});

test("campaign fails closed without a versioned executor identity", async () => {
  const report = await runReliabilityCampaign(campaignManifest(), { executor: isolatedPassingExecutor });
  assert.equal(report.status, "blocked");
  assert.match(report.failures.join("\n"), /executor provenance/);
});

test("campaign fails closed without a transitive executor bundle", async () => {
  const report = await runReliabilityCampaign(campaignManifest(), {
    executor: isolatedPassingExecutor,
    executorProvenance: {
      kind: "versioned_external_driver",
      schemaVersion: 2,
      id: "fixture-driver.mjs",
      sha256: `sha256:${"4".repeat(64)}`,
    },
  });

  assert.equal(report.status, "blocked");
  assert.match(report.failures.join("\n"), /executor bundle/);
});

test("campaign rejects state contamination across runs", async () => {
  const report = await runCampaign(campaignManifest(), async (descriptor) => ({
      ...passingExecutableCase(descriptor.caseId),
      isolation: { workspaceId: "shared", sessionId: "shared", statePath: "/tmp/shared" },
    }));

  assert.equal(report.status, "fail");
  assert.ok(report.failures.some((failure) => failure.includes("reused")));
});

test("campaign permits database-local workspace and session identifiers", async () => {
  const report = await runCampaign(campaignManifest(), async (descriptor) => {
    const actual = passingExecutableCase(descriptor.caseId);
    const localIsolation = isolation(descriptor);
    relabelTrace(actual.trace, "session.1");
    actual.trace.events[0].correlationId = descriptor.runId;
    return {
      ...actual,
      provenance: provenance(descriptor),
      isolation: { ...localIsolation, workspaceId: "workspace.1", sessionId: "session.1" },
    };
  });

  assert.equal(report.status, "pass", report.failures.join("; "));
  assert.equal(report.failures.length, 0);
});

test("campaign rejects relabeled traces and nonexistent isolation paths", async () => {
  const report = await runCampaign(campaignManifest({ cases: ["inspect"], seeds: [1], repetitions: 1 }), async (descriptor) => ({
      ...passingExecutableCase(descriptor.caseId),
      provenance: provenance(descriptor),
      isolation: {
        workspaceId: `workspace-${descriptor.runId}`,
        workspacePath: join(isolationRoot, "missing-workspace"),
        sessionId: `relabeled-${descriptor.runId}`,
        statePath: join(isolationRoot, "missing-state.sqlite"),
      },
    }));

  assert.equal(report.status, "fail");
  assert.equal(report.runs[0].status, "blocked");
  assert.match(report.runs[0].failures.join("\n"), /trace session does not match|path is not real/);
});

test("campaign rejects duplicate trace evidence behind unique declared ids", async () => {
  const shared = passingExecutableCase("inspect");
  const report = await runCampaign(campaignManifest({ cases: ["inspect"], seeds: [1, 2], repetitions: 1 }), async (descriptor) => ({
      ...structuredClone(shared),
      provenance: provenance(descriptor),
      isolation: isolation(descriptor),
    }));

  assert.equal(report.status, "fail");
  assert.match(report.failures.join("\n"), /trace evidence reused/);
});

test("timeouts cancellations and infrastructure failures remain explicit", async () => {
  let ordinal = 0;
  const report = await runCampaign(campaignManifest({ cases: ["inspect"], seeds: [1, 2, 3], repetitions: 1 }), async (descriptor) => {
      ordinal += 1;
      const status = ["timeout", "cancelled", "infrastructure_failure"][ordinal - 1];
      return { status, reason: `${status} detail`, isolation: isolation(descriptor) };
    });

  assert.equal(report.status, "fail");
  assert.equal(report.metrics.outcomes.timeout, 1);
  assert.equal(report.metrics.outcomes.cancelled, 1);
  assert.equal(report.metrics.outcomes.infrastructure_failure, 1);
  assert.equal(report.metrics.meanScore, null);
});

test("worst-of-n and dispersion expose partial and failed runs", async () => {
  let ordinal = 0;
  const report = await runCampaign(campaignManifest({ cases: ["create"], seeds: [11, 12], repetitions: 1, minimumPassRate: 0.4 }), async (descriptor) => {
      ordinal += 1;
      const actual = await isolatedPassingExecutor(descriptor);
      if (ordinal === 2) {
        actual.trace.events = actual.trace.events.filter((event) => event.kind !== "approval_resolved");
        relink(actual.trace.events);
      }
      return actual;
    });

  assert.equal(report.metrics.passRate, 0.5);
  assert.equal(report.metrics.outcomes.partial, 1);
  assert.ok(report.metrics.worstOfN < 1);
  assert.ok(report.metrics.scoreDispersion > 0);
});

test("configuration comparison permits one controlled factor and rejects multi-factor drift", async () => {
  const baseline = agentConfiguration();
  const controlled = await runCampaign(campaignManifest({
    baselineConfiguration: baseline,
    configuration: agentConfiguration({ approvalMode: "workspace_write" }),
  }), isolatedPassingExecutor);
  assert.equal(controlled.status, "pass");
  assert.equal(controlled.configurationComparison.comparison, "controlled_ab");
  assert.deepEqual(controlled.configurationComparison.changedFactors, ["approvalMode"]);

  const drifted = await runCampaign(campaignManifest({
    baselineConfiguration: baseline,
    configuration: agentConfiguration({
      approvalMode: "workspace_write",
      runtime: { ...baseline.runtime, version: "0.7.0" },
    }),
  }), isolatedPassingExecutor);
  assert.equal(drifted.status, "fail");
  assert.equal(drifted.configurationComparison.comparison, "non_comparable_drift");
});

test("campaign modules stay below line guards", () => {
  for (const [path, limit] of [
    ["scripts/product/agent-reliability-campaign-core.mjs", 300],
    ["scripts/product/agent-reliability-evidence.mjs", 110],
    ["scripts/product/agent-reliability-campaign.mjs", 120],
    ["scripts/product/agent-reliability-campaign.test.mjs", 240],
    ["scripts/product/versioned-module-bundle.mjs", 90],
    ["scripts/product/versioned-module-bundle.test.mjs", 70],
  ]) {
    const logical = readFileSync(path, "utf8").split("\n").filter((line) => line.trim()).length;
    assert.ok(logical <= limit, `${path} has ${logical} logical lines, limit ${limit}`);
  }
});

function campaignManifest(overrides = {}) {
  return {
    kind: "desktoplab.agent-reliability-manifest",
    schemaVersion: 1,
    campaignId: "local-agent-smoke",
    candidateId: `sha256:${"e".repeat(64)}`,
    appHash: `sha256:${"f".repeat(64)}`,
    cases: ["inspect", "create"],
    seeds: [7, 19],
    repetitions: 2,
    timeoutMs: 60_000,
    minimumPassRate: 1,
    configuration: agentConfiguration({ hostname: "private-host", workspacePath: "/private/repo" }),
    ...overrides,
  };
}

async function isolatedPassingExecutor(descriptor) {
  const actual = passingExecutableCase(descriptor.caseId);
  const isolated = isolation(descriptor);
  relabelTrace(actual.trace, isolated.sessionId);
  return { ...actual, provenance: provenance(descriptor), isolation: isolated };
}

function isolation(descriptor, { traceSessionId = null } = {}) {
  const workspacePath = join(isolationRoot, `workspace-${descriptor.runId}`);
  const statePath = join(isolationRoot, `${descriptor.runId}.sqlite`);
  mkdirSync(join(workspacePath, ".git"), { recursive: true });
  writeFileSync(statePath, descriptor.runId);
  return {
    workspaceId: `workspace-${descriptor.runId}`,
    workspacePath,
    sessionId: traceSessionId ?? `session-${descriptor.runId}`,
    statePath,
  };
}

function provenance(descriptor) {
  return {
    executionKind: "installed_app_ui",
    candidateId: descriptor.candidateId,
    appHash: descriptor.appHash,
    modelRequestCount: 1,
    testControlRequests: 0,
    uiDriverSha256: `sha256:${"1".repeat(64)}`,
    interactionSha256: `sha256:${"2".repeat(64)}`,
    screenshotSha256: `sha256:${"3".repeat(64)}`,
  };
}

function runCampaign(manifest, executor) {
  const source = { path: "fixture-driver.mjs", sha256: `sha256:${"4".repeat(64)}` };
  return runReliabilityCampaign(manifest, {
    executor,
    executorProvenance: {
      kind: "versioned_external_driver",
      schemaVersion: 2,
      id: "fixture-driver.mjs",
      sha256: `sha256:${"4".repeat(64)}`,
      bundleSha256: moduleSourceBundleDigest([source]),
      sourceCount: 1,
      sources: [source],
    },
  });
}

function relabelTrace(trace, sessionId) {
  trace.sessionId = sessionId;
  trace.events.forEach((event, index) => {
    event.eventId = `${sessionId}:trace:${index + 1}`;
    event.parentEventId = index === 0 ? null : `${sessionId}:trace:${index}`;
  });
}

function relink(events) {
  events.forEach((event, index) => {
    event.sequence = index + 1;
    event.parentEventId = index === 0 ? null : events[index - 1].eventId;
  });
}
