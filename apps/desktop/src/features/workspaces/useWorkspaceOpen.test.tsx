// @vitest-environment jsdom
import { renderHook, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { WorkspaceOpenRequest, WorkspaceSnapshot } from "../../api/types";
import { useWorkspaceOpen } from "./useWorkspaceOpen";

test("opens a workspace through the api client and exposes the snapshot", async () => {
  const openWorkspace = vi.fn<(request: WorkspaceOpenRequest) => Promise<WorkspaceSnapshot>>().mockResolvedValue(snapshot());
  const { result } = renderHook(() => useWorkspaceOpen(), {
    wrapper: ({ children }) => (
      <AppProviders apiClient={{ openWorkspace } as unknown as DesktopLabApiClient}>{children}</AppProviders>
    ),
  });

  result.current.open.mutate({ path: "/repo/desktoplab" });

  await waitFor(() => expect(result.current.workspace).toMatchObject({ workspaceId: "workspace.desktoplab" }));
  expect(openWorkspace).toHaveBeenCalledWith({ path: "/repo/desktoplab" });
});

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
