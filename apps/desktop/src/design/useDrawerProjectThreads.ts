import { useQueries } from "@tanstack/react-query";
import { useApiClient } from "../api/ApiProvider";
import type { AgentSessionSnapshot, WorkspaceSnapshot } from "../api/types";

export type DrawerThreadsStatus = "loading" | "ready" | "error";

export function useDrawerProjectThreads(workspaces: WorkspaceSnapshot[]) {
  const api = useApiClient();
  const queries = useQueries({
    queries: workspaces.map((workspace) => ({
      queryKey: ["drawer-project-threads", workspace.workspaceId],
      queryFn: () => api.listSessions(workspace.workspaceId),
      enabled: Boolean(workspace.workspaceId),
      retry: 3,
      retryDelay: 250,
      staleTime: 2_000,
      placeholderData: (previous: { sessions: AgentSessionSnapshot[] } | undefined) => previous,
    })),
  });
  const byWorkspace = Object.fromEntries(
    workspaces.map((workspace, index) => [
      workspace.workspaceId,
      orderedThreads(queries[index]?.data?.sessions ?? []),
    ]),
  ) as Record<string, AgentSessionSnapshot[]>;
  const statusByWorkspace = Object.fromEntries(
    workspaces.map((workspace, index) => {
      const query = queries[index];
      const status: DrawerThreadsStatus = query?.data ? "ready" : query?.isError ? "error" : "loading";
      return [workspace.workspaceId, status];
    }),
  ) as Record<string, DrawerThreadsStatus>;
  return {
    byWorkspace,
    statusByWorkspace,
    pinnedThreads: Object.values(byWorkspace).flat(),
  };
}

function orderedThreads(threads: AgentSessionSnapshot[]) {
  return [...threads].sort((left, right) => sessionNumber(right.sessionId) - sessionNumber(left.sessionId));
}

function sessionNumber(sessionId: string): number {
  const value = Number(sessionId.split(".").at(-1));
  return Number.isFinite(value) ? value : 0;
}
