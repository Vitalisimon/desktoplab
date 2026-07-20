export function crossPlatformAgentParityCommands() {
  return [
    cargo("test", "-p", "desktoplab-agent-engine", "--test", "context_planner"),
    cargo("test", "-p", "desktoplab-runtime", "--test", "runtime_install_execution"),
    cargo("test", "-p", "desktoplab-storage", "--test", "durable_local_state"),
    cargo(
      "test",
      "-p",
      "desktoplab-tool-gateway",
      "--test",
      "filesystem_patch",
      "--test",
      "filesystem_tool_execution",
      "--test",
      "git_tool_execution",
      "--test",
      "path_containment",
      "--test",
      "root_capability",
      "--test",
      "terminal_process_adapter",
      "--test",
      "terminal_tool_execution",
      "--test",
      "test_command_selection",
      "--test",
      "test_runner",
      "--test",
      "tool_gateway",
      "--test",
      "tool_policy_snapshot",
    ),
    cargo(
      "test",
      "-p",
      "desktoplab-control-plane",
      "--lib",
    ),
    cargo(
      "test",
      "-p",
      "desktoplab-control-plane",
      "--test",
      "canonical_agent_tool_executor",
      "--test",
      "canonical_agent_process_tools",
      "--test",
      "local_api_native_iterative_loop",
      "--test",
      "local_api_native_iterative_restart",
      "--test",
      "local_api_agent_lifecycle_guards",
      "--test",
      "local_api_agent_model_concurrency",
      "--test",
      "local_api_agent_filesystem_mutations",
      "--test",
      "local_api_agent_managed_process",
      "--test",
      "local_api_agent_mcp_execution",
      "--test",
      "local_api_native_action_chain",
      "--test",
      "local_api_approval_modes",
      "--test",
      "local_api_subagent_lifecycle",
      "--test",
      "local_api_codex_agent_execution",
    ),
    cargoBuild(
      "build",
      "--release",
      "-p",
      "desktoplab-control-plane",
      "--bin",
      "desktoplab-local-api",
    ),
  ];
}

export function releaseAgentSurfaceFailures(binary) {
  const contents = binary.toString("latin1");
  const forbidden = [
    "provider_output_recovery:",
    "DeterministicForTest",
    "DeterministicSequenceForTest",
    "NativeIterativeSequenceForTest",
    "legacy_agent_test_harness",
    "initial_malformed_retry",
  ];
  const failures = forbidden
    .filter((term) => contents.includes(term))
    .map((term) => `release binary contains debug/legacy agent surface: ${term}`);
  for (const required of ["desktoplab.complete", "PLANNED_TOOL_TEST_HARNESS_ONLY"]) {
    if (!contents.includes(required)) failures.push(`release binary is missing fail-closed agent marker: ${required}`);
  }
  return failures;
}

export function cargoTestExecutables(jsonLines) {
  const executables = new Set();
  for (const line of jsonLines.split(/\r?\n/)) {
    if (!line.trim().startsWith("{")) continue;
    let message;
    try {
      message = JSON.parse(line);
    } catch {
      continue;
    }
    if (
      message.reason === "compiler-artifact"
      && message.profile?.test === true
      && typeof message.executable === "string"
    ) {
      executables.add(message.executable);
    }
  }
  return [...executables];
}

function cargo(...args) {
  return { kind: "test", command: process.platform === "win32" ? "cargo.exe" : "cargo", args };
}

function cargoBuild(...args) {
  return { kind: "build", command: process.platform === "win32" ? "cargo.exe" : "cargo", args };
}
