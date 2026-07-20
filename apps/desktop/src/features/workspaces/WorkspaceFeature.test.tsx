// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { HealthResponse, ReadinessResponse, WorkspaceOpenRequest, WorkspaceSnapshot } from "../../api/types";
import { WorkspaceFeature } from "./WorkspaceFeature";

test("routes from workspace open to workspace home after backend snapshot", async () => {
  const openWorkspace = vi.fn<(request: WorkspaceOpenRequest) => Promise<WorkspaceSnapshot>>().mockResolvedValue(snapshot());

  render(
    <AppProviders apiClient={clientFor(openWorkspace)}>
      <WorkspaceFeature />
    </AppProviders>,
  );

  fireEvent.change(screen.getByLabelText("Repository path"), { target: { value: "/repo/desktoplab" } });
  fireEvent.click(screen.getByRole("button", { name: /open repository/i }));

  await waitFor(() => expect(screen.getByText("Repository changes")).toBeInTheDocument());
  expect(screen.getByText("desktoplab")).toBeInTheDocument();
});

function clientFor(openWorkspace: (request: WorkspaceOpenRequest) => Promise<WorkspaceSnapshot>): DesktopLabApiClient {
  return {
    health: vi.fn<() => Promise<HealthResponse>>().mockResolvedValue({ status: "healthy" }),
    readiness: vi.fn<() => Promise<ReadinessResponse>>().mockResolvedValue({ state: "ready" }),
    openWorkspace,
  } as unknown as DesktopLabApiClient;
}

function snapshot(): WorkspaceSnapshot {
  return {
    workspaceId: "workspace.desktoplab",
    displayName: "desktoplab",
    rootPath: "/repo/desktoplab",
    gitDirPath: "/repo/desktoplab/.git",
    apiState: "clean",
    statusEntries: [],
    diffText: "",
    checkpointStatus: "ready",
    canCheckpointRiskyExecution: true,
  };
}
