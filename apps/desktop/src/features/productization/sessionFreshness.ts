import type { AgentSessionSnapshot } from "../../api/types";

export function latestSessionSnapshot(
  selected: AgentSessionSnapshot | null,
  workspace: AgentSessionSnapshot | null,
): AgentSessionSnapshot | null {
  if (!selected) return workspace;
  if (!workspace || selected.sessionId !== workspace.sessionId) return selected;
  if (isTerminal(workspace.state) && !isTerminal(selected.state)) return workspace;
  if (isTerminal(selected.state) && !isTerminal(workspace.state)) return selected;
  return evidenceSequence(workspace) > evidenceSequence(selected) ? workspace : selected;
}

export function shouldRefreshSession(session: AgentSessionSnapshot | null): boolean {
  return Boolean(session && !isTerminal(session.state));
}

function evidenceSequence(session: AgentSessionSnapshot): number {
  return Math.max(
    0,
    ...session.timeline.map((event) => event.sequence),
    ...(session.transcript ?? []).map((turn) => turn.sequence),
  );
}

function isTerminal(state: AgentSessionSnapshot["state"]): boolean {
  return state === "completed" || state === "failed" || state === "cancelled";
}
