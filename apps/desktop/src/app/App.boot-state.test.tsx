// @vitest-environment jsdom
import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/react";
import type { DesktopLabApiClient } from "../api/client";
import type { AgentWorkspaceSnapshot, AppStateResponse, ApprovalsListResponse, WorkspaceSnapshot } from "../api/types";
import { App } from "./App";
import { AppProviders } from "./AppProviders";

afterEach(() => {
  cleanup();
});

test("boots to repository open when backend has no current workspace", async () => {
  renderBootApp(appState(null, { readiness: "ready", setupState: "ready", hasWorkspace: false, activeApprovalCount: 0, activeSessionCount: 0 }));

  expect(await screen.findByRole("heading", { name: "Open a project folder" })).toBeInTheDocument();
});

test("boots to agent when backend returns current workspace", async () => {
  renderBootApp(appState(workspace(), { readiness: "ready", setupState: "ready", hasWorkspace: true, activeApprovalCount: 0, activeSessionCount: 0 }));

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
});

test("keeps the agent workbench central when backend reports pending approvals", async () => {
  renderBootApp(appState(workspace(), { readiness: "ready", setupState: "ready", hasWorkspace: true, activeApprovalCount: 1, activeSessionCount: 0 }));

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Approvals" })).not.toBeInTheDocument();
});

test("opening a repository updates backend app-state cache instead of keeping a separate active workspace", async () => {
  const client = clientFor(appState(null, { readiness: "ready", setupState: "ready", hasWorkspace: false, activeApprovalCount: 0, activeSessionCount: 0 }));
  render(<AppProviders apiClient={client}><App /></AppProviders>);

  fireEvent.change(await screen.findByLabelText("Repository path"), { target: { value: "/Users/name/project" } });
  fireEvent.click(screen.getByRole("button", { name: "Open Repository" }));

  await screen.findByRole("heading", { name: "Agent" });
  await waitFor(() => expect(client.agentWorkspace).toHaveBeenCalledWith("workspace.desktoplab"));
});

test("boots to setup when backend setup is not complete even with persisted workspace", async () => {
  renderBootApp(appState(workspace(), { readiness: "blocked", setupState: "in_progress", hasWorkspace: true, activeApprovalCount: 0, activeSessionCount: 0 }));

  expect(await screen.findByRole("heading", { name: "Finish local setup" })).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Agent" })).not.toBeInTheDocument();
});

function renderBootApp(state: AppStateResponse) {
  render(<AppProviders apiClient={clientFor(state)}><App /></AppProviders>);
}

function clientFor(state: AppStateResponse): DesktopLabApiClient {
  return {
    appState: vi.fn().mockResolvedValue(state),
    health: vi.fn().mockResolvedValue({ status: "healthy" }),
    readiness: vi.fn().mockResolvedValue(state.readiness),
    setupPreview: vi.fn().mockResolvedValue(setupPreview()),
    catalogRefreshStatus: vi.fn().mockResolvedValue({ state: "ready", lastKnownGoodAvailable: true, degradedReasons: [], manualRefresh: { available: true } }),
    openWorkspace: vi.fn().mockResolvedValue(workspace()),
    agentWorkspace: vi.fn().mockResolvedValue(agentWorkspace()),
    listApprovals: vi.fn().mockResolvedValue({ approvals: [] } satisfies ApprovalsListResponse),
  } as unknown as DesktopLabApiClient;
}

function setupPreview() {
  return {
    registryState: "ready",
    hardware: {
      cpu: { label: "CPU", value: "Apple M4 Pro", confidence: "confirmed" },
      ramGb: { label: "RAM", value: 48, confidence: "confirmed" },
      gpu: { label: "GPU", value: null, confidence: "unknown" },
      vramGb: { label: "VRAM", value: null, confidence: "unknown" },
      unifiedMemoryGb: { label: "Unified memory", value: 48, confidence: "confirmed" },
      operatingSystem: { label: "OS", value: "macOS", confidence: "confirmed" },
      architecture: { label: "Architecture", value: "arm64", confidence: "confirmed" },
      storageAvailableGb: { label: "Storage", value: 900, confidence: "confirmed" },
    },
    runtimeRecommendations: [{ manifestId: "runtime.ollama", displayName: "Ollama", channel: "stable" }],
    modelRecommendations: [{ manifestId: "model.qwen-coder", displayName: "Qwen Coder", channel: "stable" }],
    warnings: [],
    expectedLimitations: [],
    hiddenReasons: [],
  };
}

function appState(currentWorkspace: WorkspaceSnapshot | null, routeInput: AppStateResponse["routeInput"]): AppStateResponse {
  return { readiness: { state: routeInput.readiness }, setup: { state: routeInput.setupState ?? "ready" }, currentWorkspace, routeInput };
}

function workspace(): WorkspaceSnapshot {
  return {
    workspaceId: "workspace.desktoplab",
    displayName: "project",
    rootPath: "/Users/name/project",
    gitDirPath: "/Users/name/project/.git",
    apiState: "clean",
    statusEntries: [],
    diffText: "",
    checkpointStatus: "ready",
    canCheckpointRiskyExecution: true,
  };
}

function agentWorkspace(): AgentWorkspaceSnapshot {
  return {
    route: { status: "selected", backendId: "backend.ollama", backendDisplayName: "Local runner", backendKind: "local", summary: "Runs locally", reasons: [], requiredCapabilities: ["llm.chat"], needsFallbackApproval: false },
    context: { workspaceId: "workspace.desktoplab", languages: [], frameworks: [], testCommands: [], protectedSummary: [], stale: false, refreshSupported: true },
    session: null,
  };
}
