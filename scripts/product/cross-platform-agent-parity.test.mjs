import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  cargoTestExecutables,
  crossPlatformAgentParityCommands,
  releaseAgentSurfaceFailures,
} from "./cross-platform-agent-parity-core.mjs";

test("cross-platform gate covers native execution and durable agent state", () => {
  const commands = crossPlatformAgentParityCommands();
  const serialized = JSON.stringify(commands);

  for (const required of [
    "desktoplab-tool-gateway",
    "runtime_install_execution",
    "durable_local_state",
    "root_capability",
    "context_planner",
    "canonical_agent_tool_executor",
    "canonical_agent_process_tools",
    "local_api_native_iterative_loop",
    "local_api_native_iterative_restart",
    "local_api_agent_lifecycle_guards",
    "local_api_agent_model_concurrency",
    "local_api_agent_filesystem_mutations",
    "local_api_agent_managed_process",
    "local_api_agent_mcp_execution",
    "local_api_native_action_chain",
    "local_api_approval_modes",
    "local_api_subagent_lifecycle",
    "local_api_codex_agent_execution",
  ]) {
    assert.match(serialized, new RegExp(required));
  }
  assert.ok(commands.every((step) => /^cargo(?:\.exe)?$/.test(step.command)));
  assert.ok(commands.every((step) => step.args.every((arg) => !/[;&|]/.test(arg))));
  assert.ok(commands.some((step) => step.kind === "build" && step.args.includes("--release")));
  assert.ok(commands.some((step) => step.kind === "build" && step.args.includes("desktoplab-local-api")));
  for (const legacyOnly of [
    "local_api_agent_action_contract",
    "local_api_agent_multi_tool_continuation",
    "local_api_agent_provider_tool_contract",
    "local_api_agent_tool_approvals",
  ]) {
    assert.doesNotMatch(serialized, new RegExp(legacyOnly));
  }
});

test("release agent surface rejects linked debug and legacy execution controls", () => {
  assert.deepEqual(
    releaseAgentSurfaceFailures(
      Buffer.from("desktoplab.complete PLANNED_TOOL_TEST_HARNESS_ONLY native iterative runtime"),
    ),
    [],
  );
  assert.deepEqual(
    releaseAgentSurfaceFailures(
      Buffer.from("desktoplab.complete PLANNED_TOOL_TEST_HARNESS_ONLY provider_output_recovery:"),
    ),
    ["release binary contains debug/legacy agent surface: provider_output_recovery:"],
  );
});

test("cargo JSON output yields each concrete test executable once", () => {
  const output = [
    JSON.stringify({ reason: "compiler-artifact", profile: { test: true }, executable: "a.exe" }),
    JSON.stringify({ reason: "compiler-artifact", profile: { test: true }, executable: "a.exe" }),
    JSON.stringify({ reason: "compiler-artifact", profile: { test: false }, executable: "lib.dll" }),
    "not json",
  ].join("\n");

  assert.deepEqual(cargoTestExecutables(output), ["a.exe"]);
});

test("Windows test signing uses the certified PowerShell 7 host", () => {
  const runner = readFileSync("scripts/product/cross-platform-agent-parity.mjs", "utf8");
  assert.match(runner, /"pwsh\.exe"/);
  assert.match(runner, /windows-sign\.ps1/);
  assert.match(runner, /WINDOWS_TRUST_SETTLE_MS = 5_000/);
  assert.doesNotMatch(runner, /Set-AuthenticodeSignature/);
  assert.doesNotMatch(runner, /"powershell\.exe"/);
});
