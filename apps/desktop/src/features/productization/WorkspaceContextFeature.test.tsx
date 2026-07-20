// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { SessionContextPreview, WorkspaceIntelligenceSnapshot, WorkspaceMemoryResponse } from "../../api/types";
import { WorkspaceContextFeature } from "./WorkspaceContextFeature";

test("renders workspace intelligence memory and context preview without protected content", async () => {
  renderContext();

  expect(await screen.findByRole("heading", { name: "Context" })).toBeInTheDocument();
  expect(screen.getByText("Desktop app")).toBeInTheDocument();
  expect(screen.getByText("TypeScript")).toBeInTheDocument();
  expect(screen.getByText("probable")).toBeInTheDocument();
  expect(screen.getByText(".env and SSH keys excluded")).toBeInTheDocument();
  expect(screen.getByText("Decision: local-first routing")).toBeInTheDocument();
  expect(screen.getByText("Architecture decision record")).toBeInTheDocument();
  expect(screen.getByText("Source: ADR-0004")).toBeInTheDocument();
  expect(screen.getByText("Local-only memory; export is not wired yet")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Export local-only memory" })).not.toBeInTheDocument();
  expect(screen.getByText("Provider context excludes protected files")).toBeInTheDocument();
  expect(screen.queryByText("SECRET_VALUE")).not.toBeInTheDocument();
});

test("blocked refresh stays disabled while memory delete calls backend", async () => {
  const refreshWorkspaceScan = vi.fn().mockResolvedValue({ status: "blocked", reason: "workspace_scan_refresh_not_available" });
  const deleteMemory = vi.fn().mockResolvedValue({ status: "deleted", deletedMemoryId: "memory.1", workspaceId: "workspace.desktoplab" });
  renderContext({ refreshWorkspaceScan, deleteMemory });

  await screen.findByRole("heading", { name: "Context" });
  expect(screen.getByRole("button", { name: "Refresh scan" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Delete memory" }));

  expect(refreshWorkspaceScan).not.toHaveBeenCalled();
  await waitFor(() => expect(deleteMemory).toHaveBeenCalledWith("memory.1"));
});

function renderContext(overrides: Partial<DesktopLabApiClient> = {}) {
  const client = {
    workspaceIntelligence: vi.fn().mockResolvedValue(intelligence()),
    refreshWorkspaceScan: vi.fn(),
    listWorkspaceMemory: vi.fn().mockResolvedValue(memory()),
    deleteMemory: vi.fn(),
    sessionContextPreview: vi.fn().mockResolvedValue(contextPreview()),
    ...overrides,
  } as unknown as DesktopLabApiClient;

  return render(
    <AppProviders apiClient={client}>
      <WorkspaceContextFeature workspaceId="workspace.desktoplab" />
    </AppProviders>,
  );
}

function intelligence(): WorkspaceIntelligenceSnapshot {
  return {
    workspaceId: "workspace.desktoplab",
    projectType: "Desktop app",
    stale: true,
    refreshSupported: false,
    facts: [
      { label: "Language", value: "TypeScript", confidence: "confirmed" },
      { label: "Package manager", value: "npm", confidence: "probable" },
    ],
    testCommands: [{ command: "npm run check", confidence: "confirmed" }],
    protectedSummary: [".env and SSH keys excluded"],
    diagnosticsLink: "Workspace diagnostics",
  };
}

function memory(): WorkspaceMemoryResponse {
  return {
    memories: [
      {
        memoryId: "memory.1",
        title: "Decision: local-first routing",
        kind: "Architecture decision record",
        summary: "DesktopLab owns the session and routes through a local control plane.",
        decisions: ["DesktopLab owns the session"],
        source: "ADR-0004",
        createdAt: "2026-07-08T10:00:00Z",
        redactionStatus: "local_only",
      },
    ],
  };
}

function contextPreview(): SessionContextPreview {
  return {
    summary: "Provider context excludes protected files",
    sizeBudget: "42 KB of 256 KB",
    provenance: ["Workspace scan", "Memory store"],
    cloudEgressWarning: "Cloud route would require approval",
    excludedProtectedContent: [".env"],
  };
}
