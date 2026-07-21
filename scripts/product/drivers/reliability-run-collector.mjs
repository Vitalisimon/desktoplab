import { join } from "node:path";

export async function collectReliabilityRuns({ descriptors, root, record, existingRuns = [], checkpoint = async () => {}, onProgress = () => {} }) {
  const runs = [];
  const existing = new Map(existingRuns.filter((run) => run.recordingStatus === "completed").map((run) => [run.runId, run]));
  for (const [index, descriptor] of descriptors.entries()) {
    if (existing.has(descriptor.runId)) {
      const run = existing.get(descriptor.runId);
      runs.push(run);
      onProgress({ index: index + 1, total: descriptors.length, run, resumed: true });
      continue;
    }
    let run;
    let failureError = null;
    try {
      run = { runId: descriptor.runId, recordingStatus: "completed", ...await record(descriptor) };
    } catch (error) {
      failureError = error;
      run = failedRun(descriptor, root, error);
    }
    runs.push(run);
    await checkpoint(run);
    onProgress({ index: index + 1, total: descriptors.length, run, resumed: false });
    if (errorAbortsCampaign(run, failureError)) throw failureError;
  }
  return runs;
}

function errorAbortsCampaign(run, error) {
  return run.recordingStatus === "failed"
    && error instanceof Error
    && error.reliabilityAbortCampaign === true;
}

function failedRun(descriptor, root, error) {
  const runRoot = join(root, descriptor.runId);
  const stopReason = boundedReason(error instanceof Error ? error.message : String(error));
  const context = error instanceof Error ? error.reliabilityRunContext ?? {} : {};
  return {
    runId: descriptor.runId,
    caseId: descriptor.caseId,
    seed: descriptor.seed,
    profileId: descriptor.profileId,
    repetition: descriptor.repetition,
    recordingStatus: "failed",
    outcomeStatus: failureStatus(stopReason),
    stopReason,
    workspaceId: context.workspaceId ?? null,
    workspacePath: context.workspacePath ?? join(runRoot, "workspace"),
    statePath: context.statePath ?? join(runRoot, "app-data", "desktoplab.sqlite"),
    sessionId: context.sessionId ?? null,
    diagnostics: error instanceof Error ? error.reliabilityDiagnostics ?? null : null,
  };
}

function failureStatus(reason) {
  if (/(?:timed? out|timeout|did not complete before)/i.test(reason)) return "timeout";
  if (/(?:model_failure|model_protocol_error|invalid_final_response|duplicate_tool_call):?/i.test(reason)) return "agent_failure";
  return "infrastructure_failure";
}

function boundedReason(value) {
  return String(value)
    .replace(/\/Users\/[^\s]+|[A-Za-z]:\\Users\\[^\s]+/g, "[PATH_REDACTED]")
    .replace(/(token|api[_-]?key|secret)=[^\s]+/gi, "$1=[REDACTED]")
    .slice(0, 240);
}
