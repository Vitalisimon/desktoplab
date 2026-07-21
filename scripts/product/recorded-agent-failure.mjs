const failureStatuses = new Set(["timeout", "infrastructure_failure", "agent_failure"]);

export function recordedFailureOutcome(run) {
  if (run.recordingStatus !== "failed") return null;
  const status = run.outcomeStatus ?? run.operationalStatus;
  if (!failureStatuses.has(status)) throw new Error("recorded failure status is invalid");
  return {
    status,
    reason: run.stopReason,
    isolation: {
      workspaceId: run.workspaceId,
      workspacePath: run.workspacePath,
      sessionId: run.sessionId,
      statePath: run.statePath,
    },
  };
}
