import type { QueryClient } from "@tanstack/react-query";
import type { AgentSessionSnapshot, SessionsListResponse } from "../../api/types";

export function cacheDrawerSession(
  queryClient: QueryClient,
  workspaceId: string,
  session: AgentSessionSnapshot,
) {
  queryClient.setQueryData<SessionsListResponse>(["drawer-project-threads", workspaceId], (current) => {
    const sessions = current?.sessions ?? [];
    const existing = sessions.findIndex((candidate) => candidate.sessionId === session.sessionId);
    if (existing === -1) return { sessions: [...sessions, session] };
    return {
      sessions: sessions.map((candidate, index) => index === existing ? session : candidate),
    };
  });
}
