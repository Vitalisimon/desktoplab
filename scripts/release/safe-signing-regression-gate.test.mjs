import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { aggregateRun, appendRun, runCommand } from "./regression-gate-core.mjs";

const runtimeCertificationArgs = [
  "--runtime-ollama-managed", "/evidence/runtime-ollama-managed.json",
  "--runtime-lm-studio-existing", "/evidence/runtime-lm-studio-existing.json",
  "--runtime-lm-studio-managed", "/evidence/runtime-lm-studio-managed.json",
  "--runtime-mlx-lm-managed", "/evidence/runtime-mlx-lm-managed.json",
];

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
    "--agent-evidence-mode",
    "fresh-recording",
    ...runtimeCertificationArgs,
    "--report",
    report,
  ]);

  const evidence = JSON.parse(readFileSync(report, "utf8"));
  const run = evidence.runs.at(-1);
  const betaFull = run.steps.find((step) => step.id === "beta-full");
  const candidatePayload = run.steps.find((step) => step.id === "candidate-payload");
  const runtimeMatrix = run.steps.find((step) => step.id === "runtime-certification-matrix");
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
  assert.ok(run.steps.indexOf(runtimeMatrix) < run.steps.indexOf(installedAgent));
  assert.match(runtimeMatrix.args.join(" "), /--ollama-managed \/evidence\/runtime-ollama-managed\.json/);
  assert.match(runtimeMatrix.args.join(" "), /--lm-studio-existing \/evidence\/runtime-lm-studio-existing\.json/);
  assert.match(runtimeMatrix.args.join(" "), /--lm-studio-managed \/evidence\/runtime-lm-studio-managed\.json/);
  assert.match(runtimeMatrix.args.join(" "), /--mlx-lm-managed \/evidence\/runtime-mlx-lm-managed\.json/);
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
  const uiDependencies = agentReleaseGates.args.flatMap((value, index, values) =>
    value === "--ui-driver-dependency" ? [values[index + 1]] : [],
  );
  assert.deepEqual(uiDependencies, [
    "scripts/product/drivers/macos-installed-agent-ui.mjs",
    "scripts/product/drivers/macos-installed-agent-ui-wait.mjs",
    "scripts/product/drivers/macos-installed-agent-reliability-run.mjs",
    "scripts/product/drivers/reliability-run-collector.mjs",
    "scripts/product/drivers/memory-pressure-helper.mjs",
  ]);
  const stableUi = run.steps.find((step) => step.id === "stable-ui");
  assert.match(stableUi.args.join(" "), /--candidate .*admission\.json/);
  assert.match(stableUi.args.join(" "), /--app .*DesktopLab\.app/);

  spawnSync(process.execPath, ["scripts/release/safe-signing-regression-gate.mjs", "--dry-run", "--agent-evidence-mode", "fresh-recording", ...runtimeCertificationArgs, "--report", report]);
  const retried = JSON.parse(readFileSync(report, "utf8"));
  const workspaces = retried.runs.slice(-2).map((entry) => entry.steps.find((step) => step.id === "installed-agent").args.join(" ").match(/--workspace ([^ ]+)/)[1]);
  assert.notEqual(workspaces[0], workspaces[1], "safe-signing retries must not reuse agent state");
});

test("verified agent evidence reuse never schedules a second canary or reliability recording", () => {
  const report = join(mkdtempSync(join(tmpdir(), "desktoplab-safe-signing-reuse-")), "report.json");
  const result = spawnSync(process.execPath, [
    "scripts/release/safe-signing-regression-gate.mjs",
    "--dry-run",
    "--agent-evidence-mode",
    "verified-reuse",
    "--reuse-agent-certification",
    "/evidence/installed-agent.json",
    "--reuse-agent-campaign",
    "/evidence/reliability.json",
    ...runtimeCertificationArgs,
    "--report",
    report,
  ]);
  const run = JSON.parse(readFileSync(report, "utf8")).runs.at(-1);
  const installedAgent = run.steps.find((step) => step.id === "installed-agent");
  assert.equal(result.status, 1, "dry-run remains non-certifying");
  assert.match(installedAgent.args.join(" "), /safe-signing-agent-evidence\.mjs/);
  assert.match(installedAgent.args.join(" "), /--certification \/evidence\/installed-agent\.json/);
  assert.match(installedAgent.args.join(" "), /--campaign \/evidence\/reliability\.json/);
  assert.match(installedAgent.args.join(" "), /--installed-ui-driver scripts\/product\/drivers\/macos-installed-agent-ui\.mjs/);
  assert.equal(run.steps.some((step) => step.id === "measured-agent-runtime"), false);
  assert.equal(run.steps.some((step) => step.id === "agent-reliability-recording"), false);
  assert.equal(run.steps.some((step) => step.id === "agent-reliability-campaign"), false);
  assert.equal(run.steps.some((step) => step.id === "agent-release-gates"), false);
});

test("safe signing refuses an implicit or incomplete agent evidence mode", () => {
  const implicit = spawnSync(process.execPath, [
    "scripts/release/safe-signing-regression-gate.mjs",
    "--dry-run",
  ], { encoding: "utf8" });
  assert.notEqual(implicit.status, 0);
  assert.match(implicit.stderr, /requires --agent-evidence-mode/);

  const incomplete = spawnSync(process.execPath, [
    "scripts/release/safe-signing-regression-gate.mjs",
    "--dry-run",
    "--agent-evidence-mode",
    "verified-reuse",
    "--reuse-agent-certification",
    "/evidence/installed-agent.json",
  ], { encoding: "utf8" });
  assert.notEqual(incomplete.status, 0);
  assert.match(incomplete.stderr, /requires both agent certification and campaign/);
});

test("safe signing refuses an incomplete live runtime certification matrix", () => {
  const incomplete = spawnSync(process.execPath, [
    "scripts/release/safe-signing-regression-gate.mjs",
    "--dry-run",
    "--agent-evidence-mode",
    "fresh-recording",
    "--runtime-ollama-managed",
    "/evidence/runtime-ollama-managed.json",
  ], { encoding: "utf8" });

  assert.notEqual(incomplete.status, 0);
  assert.match(incomplete.stderr, /requires all four live runtime certification reports/);
});
