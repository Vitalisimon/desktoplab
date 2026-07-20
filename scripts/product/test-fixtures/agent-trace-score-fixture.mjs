import { executableCaseSpecs } from "../agent-trace-score-core.mjs";

export function passingExecutableCase(id) {
  const events = eventTemplates(id).map((event, index) => ({
    eventId: `session-1:trace:${index + 1}`,
    parentEventId: index === 0 ? null : `session-1:trace:${index}`,
    sequence: index + 1,
    recordedAtUnixMs: 1000 + index,
    durationMs: null,
    correlationId: null,
    truncated: false,
    redacted: false,
    ...event,
  }));
  return {
    status: "pass",
    semanticJudge: { score: 1, rationale: "ignored sidecar" },
    verification: {
      kind: "desktoplab.deterministic-verification",
      schemaVersion: 1,
      status: "pass",
      checks: executableCaseSpecs[id].requiredChecks.map((check, index) => ({
        id: check,
        passed: true,
        source: verifierSource(check),
        evidenceId: `sha256:${String(index + 1).repeat(64).slice(0, 64)}`,
      })),
    },
    trace: {
      schemaVersion: 1,
      producer: "desktoplab-session-service/0.1.0",
      sessionId: "session-1",
      events,
    },
  };
}

function eventTemplates(id) {
  const prompt = event("prompt_recorded", "user");
  const complete = event("completed", "agent", false, true);
  const read = event("tool_observed", "desktoplab.read_file", false, true);
  const approve = event("approval_resolved", "policy", false, true);
  if (id === "inspect") return [prompt, read, complete];
  if (id === "create") {
    return [
      prompt,
      approve,
      event("tool_observed", "desktoplab.write_file", true, true),
      complete,
    ];
  }
  if (id === "patch") {
    return [
      prompt,
      read,
      approve,
      event("tool_observed", "desktoplab.patch_file", true, true),
      event("tool_observed", "desktoplab.git_diff", false, true),
      complete,
    ];
  }
  if (id === "test_repair") {
    return [
      prompt,
      read,
      event("terminal_observed", "desktoplab.run_tests", false, false),
      approve,
      event("tool_observed", "desktoplab.patch_file", true, true),
      event("terminal_observed", "desktoplab.run_tests", false, true),
      complete,
    ];
  }
  return [prompt, event("tool_observed", "desktoplab.git_diff", false, true), complete];
}

function event(kind, source, mutation = false, success = null) {
  return { kind, source, mutation, success, detail: `${kind} tool=${source}` };
}

function verifierSource(check) {
  if (check.includes("test") || check.includes("rerun")) return "process";
  if (check.includes("diff") || check.includes("push")) return "git";
  if (check.includes("grounded")) return "session";
  return "filesystem";
}
