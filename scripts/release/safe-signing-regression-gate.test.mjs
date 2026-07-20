import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { aggregateRun, appendRun, runCommand } from "./regression-gate-core.mjs";

test("a narrow pass cannot override any required failure", () => {
  const result = aggregateRun([
    { id: "narrow", status: "passed" },
    { id: "full", status: "failed" },
  ]);
  assert.equal(result.status, "blocked");
  assert.equal(result.blocked, 1);
});

test("retries append evidence instead of replacing the failed run", () => {
  const failed = { runId: "run-1", status: "blocked", steps: [{ status: "failed" }] };
  const passed = { runId: "run-2", status: "pass", steps: [{ status: "passed" }] };
  const first = appendRun(null, failed);
  const second = appendRun(first, passed);
  assert.deepEqual(second.runs.map((run) => run.runId), ["run-1", "run-2"]);
  assert.equal(second.status, "pass");
});

test("clean-tree style checks fail when a successful command emits output", () => {
  const result = runCommand({ id: "clean", command: "printf", args: ["dirty"], rejectOutput: true });
  assert.equal(result.status, "failed");
});

test("gate commands use the boolean CI value expected by Tauri", () => {
  const result = runCommand(
    { id: "ci", command: process.execPath, args: ["-e", "console.log(process.env.CI + ':' + process.env.RELEASE_SECRET)"] , env: { RELEASE_SECRET: "available-to-child" } },
    { env: {} },
  );
  assert.equal(result.status, "passed");
  assert.equal(result.outputTail.trim(), "true:available-to-child");
  assert.equal("env" in result, false, "step environment must not be serialized into reports");
});

test("safe signing validates a prebuilt candidate without rebuilding it", () => {
  const report = join(mkdtempSync(join(tmpdir(), "desktoplab-safe-signing-")), "report.json");
  const result = spawnSync(process.execPath, [
    "scripts/release/safe-signing-regression-gate.mjs",
    "--dry-run",
    "--report",
    report,
  ]);

  const evidence = JSON.parse(readFileSync(report, "utf8"));
  const run = evidence.runs.at(-1);
  const betaFull = run.steps.find((step) => step.id === "beta-full");
  const candidatePayload = run.steps.find((step) => step.id === "candidate-payload");
  const installedAgent = run.steps.find((step) => step.id === "installed-agent");
  const reliabilityRecording = run.steps.find((step) => step.id === "agent-reliability-recording");
  const reliabilityCampaign = run.steps.find((step) => step.id === "agent-reliability-campaign");
  const agentReleaseGates = run.steps.find((step) => step.id === "agent-release-gates");
  assert.equal(result.status, 1, "dry-run must remain non-certifying");
  assert.deepEqual(betaFull.args.slice(0, 4), [
    "scripts/product/beta-gauntlet.mjs",
    "--profile",
    "full",
    "--prebuilt-candidate",
  ]);
  assert.match(candidatePayload.args.join(" "), /dist\/release\/candidate\/admission\.json/);
  assert.match(installedAgent.args.join(" "), /--candidate .*dist\/release\/candidate\/admission\.json/);
  assert.match(installedAgent.args.join(" "), /--app \/Applications\/DesktopLab\.app/);
  assert.match(installedAgent.args.join(" "), /--driver scripts\/product\/drivers\/macos-installed-agent-ui\.mjs/);
  assert.ok(installedAgent.timeoutMs >= 90 * 60 * 1000);
  assert.match(reliabilityRecording.args.join(" "), /macos-installed-agent-reliability-ui\.mjs/);
  assert.match(reliabilityRecording.args.join(" "), /reliability-recording.*manifest\.json/);
  assert.match(reliabilityCampaign.args.join(" "), /recorded-agent-reliability-driver\.mjs/);
  assert.equal("env" in reliabilityCampaign, false);
  assert.ok(run.steps.indexOf(reliabilityCampaign) < run.steps.indexOf(agentReleaseGates));
  assert.match(agentReleaseGates.args.join(" "), /agent-reliability-campaign\.json/);
  assert.match(agentReleaseGates.args.join(" "), /--executor .*recorded-agent-reliability-driver\.mjs/);
  assert.match(agentReleaseGates.args.join(" "), /--ui-driver .*macos-installed-agent-reliability-ui\.mjs/);
  assert.match(agentReleaseGates.args.join(" "), /--ui-driver-dependency .*macos-installed-agent-ui\.mjs/);
  const stableUi = run.steps.find((step) => step.id === "stable-ui");
  assert.match(stableUi.args.join(" "), /--candidate .*admission\.json/);
  assert.match(stableUi.args.join(" "), /--app .*DesktopLab\.app/);

  spawnSync(process.execPath, ["scripts/release/safe-signing-regression-gate.mjs", "--dry-run", "--report", report]);
  const retried = JSON.parse(readFileSync(report, "utf8"));
  const workspaces = retried.runs.slice(-2).map((entry) => entry.steps.find((step) => step.id === "installed-agent").args.join(" ").match(/--workspace ([^ ]+)/)[1]);
  assert.notEqual(workspaces[0], workspaces[1], "safe-signing retries must not reuse agent state");
});
