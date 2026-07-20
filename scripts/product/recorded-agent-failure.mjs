const operationalStatuses = new Set(["timeout", "infrastructure_failure"]);

export function recordedFailureOutcome(run) {
  if (run.recordingStatus !== "failed") return null;
  if (!operationalStatuses.has(run.operationalStatus)) throw new Error("recorded failure status is invalid");
  return {
    status: run.operationalStatus,
    reason: run.stopReason,
    isolation: {
      workspaceId: run.workspaceId,
      workspacePath: run.workspacePath,
      sessionId: run.sessionId,
      statePath: run.statePath,
    },
  };
}
