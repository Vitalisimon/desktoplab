// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { AgentSessionSnapshot, SessionsListResponse } from "../../api/types";
import { SessionsFeature } from "./SessionsFeature";

test("loads workspace sessions and renders active session evidence", async () => {
  const apiClient = clientFor({
    listSessions: vi.fn<(workspaceId: string) => Promise<SessionsListResponse>>().mockResolvedValue({
      sessions: [
        {
          ...session("session.1", "planning"),
          plan: "Inspect repository boundaries.",
          timeline: [{ sequence: 1, kind: "planning.started", message: "Planning started", createdAt: "2026-06-25T20:10:00Z" }],
        },
      ],
    }),
  });

  render(
    <AppProviders apiClient={apiClient}>
      <SessionsFeature workspaceId="workspace.desktoplab" executionBackends={["backend.ollama"]} />
    </AppProviders>,
  );

  expect(await screen.findByText("session.1")).toBeInTheDocument();
  expect(screen.getByText("Inspect repository boundaries.")).toBeInTheDocument();
  expect(screen.getByText("Planning started")).toBeInTheDocument();
});

test("creates a session and selects the backend response", async () => {
  const createSession = vi.fn<DesktopLabApiClient["createSession"]>().mockResolvedValue({
    ...session("session.2", "created"),
    summary: "Ready to start working.",
  });
  const apiClient = clientFor({
    listSessions: vi.fn<(workspaceId: string) => Promise<SessionsListResponse>>().mockResolvedValue({ sessions: [] }),
    createSession,
  });

  render(
    <AppProviders apiClient={apiClient}>
      <SessionsFeature workspaceId="workspace.desktoplab" executionBackends={["backend.ollama"]} />
    </AppProviders>,
  );

  fireEvent.change(await screen.findByLabelText("Prompt"), { target: { value: "Inspect the repository" } });
  fireEvent.click(screen.getByRole("button", { name: /start session/i }));

  await waitFor(() => expect(createSession).toHaveBeenCalledWith({
    workspaceId: "workspace.desktoplab",
    executionBackendId: "backend.ollama",
    initialPrompt: "Inspect the repository",
  }));
  expect(await screen.findByText("session.2")).toBeInTheDocument();
  expect(screen.getByText("Ready to start working.")).toBeInTheDocument();
});

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
