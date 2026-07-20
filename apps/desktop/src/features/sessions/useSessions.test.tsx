// @vitest-environment jsdom
import { renderHook, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { AgentSessionSnapshot, SessionsListResponse } from "../../api/types";
import { useSessions } from "./useSessions";

test("loads sessions for the active workspace through the api client", async () => {
  const apiClient = clientFor({
    listSessions: vi.fn<(workspaceId: string) => Promise<SessionsListResponse>>().mockResolvedValue({
      sessions: [session("session.1", "planning")],
    }),
  });

  const { result } = renderHook(() => useSessions("workspace.desktoplab"), { wrapper: wrapper(apiClient) });

  await waitFor(() => expect(result.current.sessions).toHaveLength(1));
  expect(result.current.sessions[0].sessionId).toBe("session.1");
  expect(apiClient.listSessions).toHaveBeenCalledWith("workspace.desktoplab");
});

test("creates a DesktopLab-owned session through the api client", async () => {
  const createSession = vi
    .fn<DesktopLabApiClient["createSession"]>()
    .mockResolvedValue(session("session.2", "created"));
  const apiClient = clientFor({
    listSessions: vi.fn<(workspaceId: string) => Promise<SessionsListResponse>>().mockResolvedValue({ sessions: [] }),
    createSession,
  });

  const { result } = renderHook(() => useSessions("workspace.desktoplab"), { wrapper: wrapper(apiClient) });
  result.current.create.mutate({
    workspaceId: "workspace.desktoplab",
    executionBackendId: "backend.ollama",
    initialPrompt: "Inspect the repository",
  });

  await waitFor(() =>
    expect(createSession).toHaveBeenCalledWith({
      workspaceId: "workspace.desktoplab",
      executionBackendId: "backend.ollama",
      initialPrompt: "Inspect the repository",
    }),
  );
});

function wrapper(apiClient: DesktopLabApiClient) {
  return ({ children }: { children: React.ReactNode }) => <AppProviders apiClient={apiClient}>{children}</AppProviders>;
}

function clientFor(methods: Partial<DesktopLabApiClient>): DesktopLabApiClient {
  return methods as DesktopLabApiClient;
}

function session(sessionId: string, state: AgentSessionSnapshot["state"]): AgentSessionSnapshot {
  return {
    sessionId,
    workspaceId: "workspace.desktoplab",
    executionBackendId: "backend.ollama",
    owner: "desktoplab",
    state,
    plan: null,
    checkpoints: [],
    summary: null,
    timeline: [],
  };
}
