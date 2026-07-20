// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import type { DesktopLabApiClient } from "../api/client";
import type {
  AgentSessionSnapshot,
  AgentWorkspaceSnapshot,
  ApprovalResolveResponse,
  ApprovalSummary,
  ApprovalsListResponse,
  CatalogRefreshRequestResponse,
  CatalogRefreshStatusResponse,
  ApprovalModesResponse,
  ExternalBackendsResponse,
  HealthResponse,
  JobsListResponse,
  DiagnosticsSnapshot,
  LocalAuditTransparencySnapshot,
  ModelDownloadResponse,
  ModelsListResponse,
  ProvidersListResponse,
  ReadinessResponse,
  RoutePreference,
  RuntimesListResponse,
  SessionsListResponse,
  SetupAcceptanceResponse,
  SetupPlanPreview,
  VersionResponse,
  WorkspaceFilePreviewResponse,
  WorkspaceFileTreeResponse,
  WorkspaceSnapshot,
  TerminalCommandResponse,
  AppStateResponse,
} from "../api/types";
import { DesktopLabApiError } from "../api/types";
import type { GitOperationsSnapshot } from "../api/gitTypes";
import { App } from "./App";
import { AppProviders } from "./AppProviders";

const runUserTerminalCommandMock = vi.hoisted(() => vi.fn());
const chooseRepositoryFolderMock = vi.hoisted(() => vi.fn());

vi.mock("../features/terminal/runUserTerminalCommand", () => ({
  runUserTerminalCommand: runUserTerminalCommandMock,
}));
vi.mock("../features/workspaces/repositoryFolderPicker", () => ({
  chooseRepositoryFolder: chooseRepositoryFolderMock,
}));

beforeEach(() => {
  window.localStorage.clear();
  runUserTerminalCommandMock.mockReset();
  runUserTerminalCommandMock.mockResolvedValue(terminalCommandResponse("npm test", "ok"));
  chooseRepositoryFolderMock.mockReset();
  chooseRepositoryFolderMock.mockResolvedValue(null);
});

test("renders setup wizard as the first real product surface", async () => {
  renderApp();

  expect(screen.getByTestId("desktoplab-root")).toBeInTheDocument();
  expect(await screen.findByText("Apple M4 Pro")).toBeInTheDocument();
});

test("renders the base desktop frame with stable navigation chrome", async () => {
  renderApp();

  expect(screen.getByText("DesktopLab")).toBeInTheDocument();
  expect(screen.getByText("Local coding agents")).toBeInTheDocument();
  expect(screen.getByText("New chat")).toBeInTheDocument();
  expect(screen.queryByText("Search")).not.toBeInTheDocument();
  expect(screen.queryByText("Scheduled")).not.toBeInTheDocument();
  expect(screen.queryByText("Plugins")).not.toBeInTheDocument();
  expect(screen.getByText("Projects")).toBeInTheDocument();
  expect(screen.queryByText("Approvals")).not.toBeInTheDocument();
  expect(screen.getByText("Control center")).toBeInTheDocument();
  expect(screen.queryByText("Accounts")).not.toBeInTheDocument();
  expect(screen.queryByText("Local")).not.toBeInTheDocument();
});

test("keeps open project available while another project is active", async () => {
  const openWorkspace = vi.fn().mockResolvedValue({
    ...workspaceSnapshot("/Users/name/second-project"),
    workspaceId: "workspace.second-project",
    displayName: "second-project",
  });
  chooseRepositoryFolderMock.mockResolvedValue("/Users/name/second-project");
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: workspaceSnapshot("/Users/name/current-project"),
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 0,
      },
    }),
    openWorkspace,
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Open project" }));
  fireEvent.click(await screen.findByRole("button", { name: "Open Repository" }));

  await waitFor(() => expect(chooseRepositoryFolderMock).toHaveBeenCalledOnce());
  expect(openWorkspace).toHaveBeenCalledWith({ path: "/Users/name/second-project" });
  expect(await screen.findByText("/Users/name/second-project")).toBeInTheDocument();
});

test("pane controls use compact native-titlebar toolbar alignment by default", async () => {
  renderApp();

  const commandRow = screen.getByTestId("window-command-row");
  const chromeStart = screen.getByTestId("window-chrome-left-cluster");
  const left = screen.getByRole("button", { name: "Collapse left drawer" });

  expect(commandRow).toHaveClass("grid-cols-[40px_minmax(0,1fr)_auto]");
  expect(commandRow).toHaveClass("dl-ambient-bar");
  expect(commandRow).not.toHaveAttribute("data-tauri-drag-region");
  expect(chromeStart).toHaveClass("h-full");
  expect(chromeStart).toHaveClass("items-center");
  expect(chromeStart).toHaveClass("pl-3");
  expect(chromeStart).not.toHaveClass("pl-[92px]");
  expect(chromeStart).not.toHaveClass("-translate-y-[6px]");
  expect(chromeStart.className).not.toMatch(/translate-y/);
  expect(left).toHaveClass("h-7");
  expect(left).toHaveClass("w-7");
  expect(left).toHaveClass("rounded-[6px]");
  expect(left).toHaveClass("border-transparent");
  expect(left).toHaveClass("bg-transparent");
  expect(left).toHaveAttribute("data-chrome-control", "true");
});

test("left drawer can collapse and hides secondary labels", async () => {
  renderApp();

  expect(screen.getByTestId("window-command-row")).toContainElement(screen.getByRole("button", { name: "Collapse left drawer" }));
  fireEvent.click(screen.getByRole("button", { name: "Collapse left drawer" }));

  expect(screen.queryByText("New chat")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Expand left drawer" })).toBeInTheDocument();
  expect(screen.getByTestId("window-command-row")).toContainElement(screen.getByRole("button", { name: "Expand left drawer" }));
  expect(screen.getByTestId("app-drawer")).toHaveClass("h-full");
  expect(screen.getByTestId("app-drawer")).toHaveClass("overflow-hidden");
});

test("right workspace drawer stays closed by default after opening a project", async () => {
  renderReadyApp();

  fireEvent.click(await screen.findByRole("button", { name: "Open Repository" }));
  fireEvent.change(screen.getByLabelText("Repository path"), { target: { value: "/Users/name/project" } });
  fireEvent.click(screen.getByRole("button", { name: "Open Repository" }));

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  expect(screen.queryByText("Repository inspector")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Show inspector" })).toBeInTheDocument();
});

test("top command row owns workspace context local status and pane controls", async () => {
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: workspaceSnapshot("/Users/name/project"),
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 0,
      },
    }),
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  const commandRow = screen.getByTestId("window-command-row");

  expect(within(commandRow).getByText("project")).toBeInTheDocument();
  expect(within(commandRow).getByText("/Users/name/project")).toBeInTheDocument();
  expect(within(commandRow).queryByText("Local")).not.toBeInTheDocument();
  expect(within(commandRow).getByRole("button", { name: "Collapse left drawer" })).toHaveAttribute("title");
  expect(within(commandRow).getByRole("button", { name: "Show changes panel" })).toBeInTheDocument();
  expect(within(commandRow).getByRole("button", { name: "Open repository in" })).toBeInTheDocument();
  expect(within(commandRow).getByRole("button", { name: "Show terminal" })).toHaveAttribute("title");
  expect(within(commandRow).getByRole("button", { name: "Show inspector" })).toHaveAttribute("title");
  expect(within(commandRow).getByRole("button", { name: "Collapse left drawer" })).toHaveAttribute("aria-pressed", "true");
  expect(within(commandRow).getByRole("button", { name: "Show terminal" })).toHaveAttribute("aria-pressed", "false");
  expect(within(commandRow).getByRole("button", { name: "Show inspector" })).toHaveAttribute("aria-pressed", "false");
});

test("top command row opens a compact changes panel without navigating to the large changes route", async () => {
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: { ...workspaceSnapshot("/Users/name/project"), apiState: "dirty", statusEntries: ["modified: src/main.rs"] },
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 0,
      },
    }),
    gitOperations: vi.fn<() => Promise<GitOperationsSnapshot>>().mockResolvedValue({
      ...gitOperations(),
      workspaceState: "dirty",
      warnings: ["Dirty worktree"],
      changedFiles: ["src/main.rs", "candidate-proof.md"],
      statusEntries: [" M src/main.rs", "?? candidate-proof.md"],
      diffPreview: "diff --git a/src/main.rs b/src/main.rs",
    }),
  });

  await screen.findByRole("heading", { name: "Agent" });
  fireEvent.click(screen.getByRole("button", { name: "Show changes panel" }));

  const panel = await screen.findByRole("complementary", { name: "Toolbar changes" });
  expect(within(panel).getByText("Dirty worktree")).toBeInTheDocument();
  expect(within(panel).getByRole("button", { name: /src\/main\.rs/ })).toBeInTheDocument();
  expect(await within(panel).findByRole("button", { name: /candidate-proof\.md/ })).toHaveTextContent("?candidate-proof.md");
  expect(screen.queryByRole("heading", { name: "Changes" })).not.toBeInTheDocument();
});

test("changes popover and repository inspector do not overlap", async () => {
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: { ...workspaceSnapshot("/Users/name/project"), apiState: "dirty", statusEntries: ["modified: src/main.rs"] },
      routeInput: { readiness: "ready", hasWorkspace: true, activeApprovalCount: 0, activeSessionCount: 0 },
    }),
  });

  await screen.findByRole("heading", { name: "Agent" });
  fireEvent.click(screen.getByRole("button", { name: "Show inspector" }));
  expect(screen.getByRole("complementary", { name: "Repository inspector" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Show changes panel" }));
  expect(await screen.findByRole("complementary", { name: "Toolbar changes" })).toBeInTheDocument();
  expect(screen.queryByRole("complementary", { name: "Repository inspector" })).not.toBeInTheDocument();
});

test("left drawer keeps user pins separate from project threads", async () => {
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: workspaceSnapshot("/Users/name/project"),
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 1,
      },
    }),
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  expect(screen.queryByText("Pinned project")).not.toBeInTheDocument();
  expect(screen.getByText("Pinned")).toBeInTheDocument();
  expect(screen.getByText("No pinned items")).toBeInTheDocument();
  expect(screen.getByText("Projects")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "project" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Inspect repository boundaries." })).toBeInTheDocument();
});

test("left drawer pins projects and threads only by explicit user choice", async () => {
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: workspaceSnapshot("/Users/name/project"),
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 1,
      },
    }),
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  expect(screen.getByText("No pinned items")).toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: "Project actions for project" }));
  fireEvent.click(screen.getByRole("menuitem", { name: "Pin project" }));
  fireEvent.click(screen.getByRole("button", { name: "Thread actions for Inspect repository boundaries." }));
  fireEvent.click(screen.getByRole("menuitem", { name: "Pin thread" }));

  const pinned = screen.getByLabelText("Pinned items");
  expect(within(pinned).getByRole("button", { name: "project" })).not.toHaveClass("bg-elevated");
  expect(within(pinned).getByRole("button", { name: "Inspect repository boundaries." })).not.toHaveClass("bg-elevated");
  fireEvent.click(screen.getByRole("button", { name: "Project actions for project" }));
  expect(screen.getByRole("menuitem", { name: "Unpin project" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Thread actions for Inspect repository boundaries." }));
  expect(screen.getByRole("menuitem", { name: "Unpin thread" })).toBeInTheDocument();

  fireEvent.click(screen.getByRole("menuitem", { name: "Unpin project" }));
  expect(within(pinned).queryByRole("button", { name: "project" })).not.toBeInTheDocument();
  expect(within(pinned).getByRole("button", { name: "Inspect repository boundaries." })).toBeInTheDocument();
});

test("left drawer pinned items persist locally", async () => {
  const appState = vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
    readiness: { state: "ready" },
    currentWorkspace: workspaceSnapshot("/Users/name/project"),
    routeInput: {
      readiness: "ready",
      hasWorkspace: true,
      activeApprovalCount: 0,
      activeSessionCount: 1,
    },
  });

  const { unmount } = renderApp(<App />, { appState });
  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Project actions for project" }));
  fireEvent.click(screen.getByRole("menuitem", { name: "Pin project" }));
  unmount();

  renderApp(<App />, { appState });
  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  const pinned = screen.getByLabelText("Pinned items");
  expect(within(pinned).getByRole("button", { name: "project" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Project actions for project" }));
  expect(screen.getByRole("menuitem", { name: "Unpin project" })).toBeInTheDocument();
});

test("clicking a pinned thread reopens that exact conversation", async () => {
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: workspaceSnapshot("/Users/name/project"),
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 2,
      },
    }),
    listSessions: vi.fn<(workspaceId: string) => Promise<SessionsListResponse>>().mockResolvedValue({
      sessions: [
        session({ sessionId: "session.1", plan: "Inspect repository boundaries." }),
        session({
          sessionId: "session.2",
          plan: "Explain the contact importer.",
          timeline: [{ sequence: 1, kind: "assistant", message: "This importer reads spreadsheets and creates contacts.", createdAt: "2026-06-30T08:00:00Z" }],
        }),
      ],
    }),
    agentWorkspace: vi.fn<(workspaceId: string) => Promise<AgentWorkspaceSnapshot>>().mockResolvedValue({
      route: {
        status: "selected",
        backendId: "backend.ollama",
        backendDisplayName: "Local runner",
        backendKind: "local",
        summary: "Runs on this machine",
        reasons: [],
        requiredCapabilities: ["Chat"],
        needsFallbackApproval: false,
      },
      context: null,
      session: null,
    }),
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Thread actions for Explain the contact importer." }));
  fireEvent.click(screen.getByRole("menuitem", { name: "Pin thread" }));
  fireEvent.click(screen.getByRole("button", { name: "Inspect repository boundaries." }));
  expect(screen.queryByText("This importer reads spreadsheets and creates contacts.")).not.toBeInTheDocument();

  const pinned = screen.getByLabelText("Pinned items");
  fireEvent.click(within(pinned).getByRole("button", { name: "Explain the contact importer." }));

  expect(await screen.findByText("This importer reads spreadsheets and creates contacts.")).toBeInTheDocument();
});

test("project threads are nested under their repository and open the selected session", async () => {
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: workspaceSnapshot("/Users/name/project"),
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 2,
      },
    }),
    listSessions: vi.fn<(workspaceId: string) => Promise<SessionsListResponse>>().mockResolvedValue({
      sessions: [
        session({ sessionId: "session.1", plan: "Inspect repository boundaries." }),
        session({
          sessionId: "session.2",
          plan: "Explain the contact importer.",
          timeline: [{ sequence: 1, kind: "assistant", message: "This importer reads spreadsheets and creates contacts.", createdAt: "2026-06-30T08:00:00Z" }],
        }),
      ],
    }),
    agentWorkspace: vi.fn<(workspaceId: string) => Promise<AgentWorkspaceSnapshot>>().mockResolvedValue({
      route: {
        status: "selected",
        backendId: "backend.ollama",
        backendDisplayName: "Local runner",
        backendKind: "local",
        summary: "Runs on this machine",
        reasons: [],
        requiredCapabilities: ["Chat"],
        needsFallbackApproval: false,
      },
      context: null,
      session: null,
    }),
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  const projects = screen.getByLabelText("Projects");
  expect(within(projects).getByRole("button", { name: "project" })).toBeInTheDocument();
  expect(within(projects).getByRole("button", { name: "Inspect repository boundaries." })).toBeInTheDocument();
  fireEvent.click(within(projects).getByRole("button", { name: "Explain the contact importer." }));

  expect(await screen.findByText("This importer reads spreadsheets and creates contacts.")).toBeInTheDocument();
  expect(within(projects).getByRole("button", { name: "Explain the contact importer." })).toHaveClass("bg-elevated");
  expect(screen.getByText("No pinned items")).toBeInTheDocument();
});

test("drawer titles threads from the user task and orders newest activity first", async () => {
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: workspaceSnapshot("/Users/name/project"),
      routeInput: { readiness: "ready", hasWorkspace: true, activeApprovalCount: 0, activeSessionCount: 2 },
    }),
    listSessions: vi.fn<(workspaceId: string) => Promise<SessionsListResponse>>().mockResolvedValue({
      sessions: [
        session({ sessionId: "session.2", plan: "Older task" }),
        session({
          sessionId: "session.9",
          plan: "Agent loop is waiting for the next decision.",
          summary: "agent loop completed",
          transcript: [{ sequence: 1, role: "user", content: "Refactor the parser without changing behavior" }],
        }),
      ],
    }),
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  const projects = screen.getByLabelText("Projects");
  const newest = within(projects).getByRole("button", { name: "Refactor the parser without changing beha..." });
  const older = within(projects).getByRole("button", { name: "Older task" });
  expect(newest).toHaveAttribute("title", "Refactor the parser without changing beha...");
  expect(Boolean(newest.compareDocumentPosition(older) & Node.DOCUMENT_POSITION_FOLLOWING)).toBe(true);
  expect(within(projects).queryByText(/Agent loop is waiting/)).not.toBeInTheDocument();
});

test("project row returns to an empty thread surface instead of behaving like a thread", async () => {
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: workspaceSnapshot("/Users/name/project"),
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 2,
      },
    }),
    listSessions: vi.fn<(workspaceId: string) => Promise<SessionsListResponse>>().mockResolvedValue({
      sessions: [
        session({ sessionId: "session.1", plan: "Inspect repository boundaries." }),
        session({
          sessionId: "session.2",
          plan: "Explain the contact importer.",
          timeline: [{ sequence: 1, kind: "assistant", message: "This importer reads spreadsheets and creates contacts.", createdAt: "2026-06-30T08:00:00Z" }],
        }),
      ],
    }),
    agentWorkspace: vi.fn<(workspaceId: string) => Promise<AgentWorkspaceSnapshot>>().mockResolvedValue({
      route: {
        status: "selected",
        backendId: "backend.ollama",
        backendDisplayName: "Local runner",
        backendKind: "local",
        summary: "Runs on this machine",
        reasons: [],
        requiredCapabilities: ["Chat"],
        needsFallbackApproval: false,
      },
      context: {
        workspaceId: "workspace.desktoplab",
        languages: ["Python"],
        frameworks: [],
        testCommands: [],
        protectedSummary: [".env is excluded"],
        stale: false,
        refreshSupported: true,
      },
      session: session({
        sessionId: "session.latest",
        plan: "Old latest thread.",
        timeline: [{ sequence: 1, kind: "assistant", message: "Old latest answer.", createdAt: "2026-06-30T08:00:00Z" }],
      }),
    }),
  });

  expect(await screen.findByText("Old latest answer.")).toBeInTheDocument();
  const projects = screen.getByLabelText("Projects");
  fireEvent.click(within(projects).getByRole("button", { name: "Explain the contact importer." }));
  expect(await screen.findByText("This importer reads spreadsheets and creates contacts.")).toBeInTheDocument();

  fireEvent.click(within(projects).getByRole("button", { name: "project" }));

  expect(await screen.findByText("Ask DesktopLab what to change, inspect, or verify in this repository.")).toBeInTheDocument();
  expect(screen.queryByText("Old latest answer.")).not.toBeInTheDocument();
  expect(screen.queryByText("This importer reads spreadsheets and creates contacts.")).not.toBeInTheDocument();
});

test("new chat starts from the project chooser even when a workspace already exists", async () => {
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: workspaceSnapshot("/Users/name/project"),
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 1,
      },
    }),
    agentWorkspace: vi.fn<(workspaceId: string) => Promise<AgentWorkspaceSnapshot>>().mockResolvedValue({
      route: {
        status: "selected",
        backendId: "backend.ollama",
        backendDisplayName: "Local runner",
        backendKind: "local",
        summary: "Runs on this machine",
        reasons: [],
        requiredCapabilities: ["Chat"],
        needsFallbackApproval: false,
      },
      context: {
        workspaceId: "workspace.desktoplab",
        languages: ["TypeScript"],
        frameworks: ["React"],
        testCommands: [],
        protectedSummary: [".env is excluded"],
        stale: false,
        refreshSupported: true,
      },
      session: session({
        sessionId: "session.latest",
        plan: "Old latest thread.",
        timeline: [{ sequence: 1, kind: "assistant", message: "Old latest answer.", createdAt: "2026-06-30T08:00:00Z" }],
      }),
    }),
  });

  expect(await screen.findByText("Old latest answer.")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "New chat" }));

  expect(await screen.findByRole("heading", { name: "Open a project folder" })).toBeInTheDocument();
  expect(screen.getByLabelText("Repository path")).toBeInTheDocument();
  expect(screen.queryByText("Old latest answer.")).not.toBeInTheDocument();
});

test("project row recovers a failed agent workspace read before opening the empty thread", async () => {
  const agentWorkspace = vi.fn<(workspaceId: string) => Promise<AgentWorkspaceSnapshot>>()
    .mockRejectedValueOnce(new Error("transient workspace read failure"))
    .mockRejectedValueOnce(new Error("transient workspace read retry failure"))
    .mockRejectedValueOnce(new Error("transient workspace read retry failure"))
    .mockRejectedValueOnce(new Error("transient workspace read retry failure"))
    .mockResolvedValue({
      route: {
        status: "selected",
        backendId: "backend.ollama",
        backendDisplayName: "Local runner",
        backendKind: "local",
        summary: "Runs on this machine",
        reasons: [],
        requiredCapabilities: ["Chat"],
        needsFallbackApproval: false,
      },
      context: {
        workspaceId: "workspace.desktoplab",
        languages: ["TypeScript"],
        frameworks: ["React"],
        testCommands: [],
        protectedSummary: [".env is excluded"],
        stale: false,
        refreshSupported: true,
      },
      session: session({
        sessionId: "session.latest",
        plan: "Old latest thread.",
        timeline: [{ sequence: 1, kind: "assistant", message: "Old latest answer.", createdAt: "2026-06-30T08:00:00Z" }],
      }),
    });
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: workspaceSnapshot("/Users/name/project"),
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 1,
      },
    }),
    agentWorkspace,
  });

  await waitFor(() => expect(agentWorkspace).toHaveBeenCalledTimes(4), { timeout: 3_000 });
  expect(screen.getByText("DesktopLab could not read the agent workspace right now.")).toBeInTheDocument();
  const projects = screen.getByLabelText("Projects");
  fireEvent.click(within(projects).getByRole("button", { name: "project" }));

  expect(await screen.findByText("Ask DesktopLab what to change, inspect, or verify in this repository.")).toBeInTheDocument();
  expect(screen.queryByText("Agent unavailable")).not.toBeInTheDocument();
  expect(screen.queryByText("Old latest answer.")).not.toBeInTheDocument();
  expect(agentWorkspace).toHaveBeenCalledTimes(5);
});

test("agent workspace recovers transient boot read failures before blocking the workbench", async () => {
  const agentWorkspace = vi.fn<(workspaceId: string) => Promise<AgentWorkspaceSnapshot>>()
    .mockRejectedValueOnce(new Error("workspace warming up"))
    .mockRejectedValueOnce(new Error("workspace still warming up"))
    .mockRejectedValueOnce(new Error("workspace final warmup"))
    .mockResolvedValue({
      route: {
        status: "selected",
        backendId: "backend.ollama",
        backendDisplayName: "Local runner",
        backendKind: "local",
        summary: "Runs on this machine",
        reasons: [],
        requiredCapabilities: ["Chat"],
        needsFallbackApproval: false,
      },
      context: {
        workspaceId: "workspace.desktoplab",
        languages: ["TypeScript"],
        frameworks: ["React"],
        testCommands: [],
        protectedSummary: [".env is excluded"],
        stale: false,
        refreshSupported: true,
      },
      session: null,
    });
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: workspaceSnapshot("/Users/name/project"),
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 0,
      },
    }),
    agentWorkspace,
  });

  expect(await screen.findByText("Ask DesktopLab what to change, inspect, or verify in this repository.", {}, { timeout: 3_000 })).toBeInTheDocument();
  expect(screen.queryByText("Agent unavailable")).not.toBeInTheDocument();
  expect(agentWorkspace).toHaveBeenCalledTimes(4);
});

test("new chat without a workspace opens the repository chooser", async () => {
  renderApp(
    <App
      routeInput={{
        readiness: "ready",
        hasWorkspace: false,
        activeApprovalCount: 0,
        activeSessionCount: 0,
      }}
    />,
  );

  fireEvent.click(screen.getByRole("button", { name: "New chat" }));

  expect(screen.getByRole("heading", { name: "Open a project folder" })).toBeInTheDocument();
  expect(screen.getByLabelText("Repository path")).toBeInTheDocument();
});

test("project groups can collapse to a single repository row", async () => {
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: workspaceSnapshot("/Users/name/project"),
      workspaces: [workspaceSnapshot("/Users/name/project")],
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 1,
      },
    }),
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  const projects = screen.getByLabelText("Projects");
  expect(within(projects).getByRole("button", { name: "Collapse project project" })).toBeInTheDocument();
  expect(within(projects).getByRole("button", { name: "Inspect repository boundaries." })).toBeInTheDocument();

  fireEvent.click(within(projects).getByRole("button", { name: "Collapse project project" }));

  expect(within(projects).queryByRole("button", { name: "Inspect repository boundaries." })).not.toBeInTheDocument();
  expect(within(projects).getByRole("button", { name: "Expand project project" })).toBeInTheDocument();
});

test("left drawer keeps every opened project until the user archives it", async () => {
  const archiveWorkspace = vi.fn().mockResolvedValue({ archived: true });
  const appState = vi.fn<() => Promise<AppStateResponse>>()
    .mockResolvedValueOnce({
      readiness: { state: "ready" },
      currentWorkspace: { ...workspaceSnapshot("/Users/name/Second"), workspaceId: "workspace.Second", displayName: "Second" },
      workspaces: [
        { ...workspaceSnapshot("/Users/name/First"), workspaceId: "workspace.First", displayName: "First" },
        { ...workspaceSnapshot("/Users/name/Second"), workspaceId: "workspace.Second", displayName: "Second" },
      ],
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 0,
      },
    })
    .mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: { ...workspaceSnapshot("/Users/name/Second"), workspaceId: "workspace.Second", displayName: "Second" },
      workspaces: [{ ...workspaceSnapshot("/Users/name/Second"), workspaceId: "workspace.Second", displayName: "Second" }],
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 0,
      },
    });
  renderApp(<App />, {
    appState,
    archiveWorkspace,
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  const projects = screen.getByLabelText("Projects");
  const firstProject = within(projects).getByRole("button", { name: "First" });
  const secondProject = within(projects).getByRole("button", { name: "Second" });
  expect(firstProject).toBeInTheDocument();
  expect(secondProject).toBeInTheDocument();
  expect(Boolean(secondProject.compareDocumentPosition(firstProject) & Node.DOCUMENT_POSITION_FOLLOWING)).toBe(true);

  fireEvent.click(within(projects).getByRole("button", { name: "Project actions for First" }));
  fireEvent.click(screen.getByRole("menuitem", { name: "Archive project" }));

  await waitFor(() => expect(archiveWorkspace).toHaveBeenCalledWith("workspace.First"));
  await waitFor(() => expect(within(projects).queryByRole("button", { name: "First" })).not.toBeInTheDocument());
  expect(within(projects).getByRole("button", { name: "Second" })).toBeInTheDocument();
});

test("left drawer keeps inactive project threads visible under their project", async () => {
  const first = { ...workspaceSnapshot("/Users/name/First"), workspaceId: "workspace.First", displayName: "First" };
  const second = { ...workspaceSnapshot("/Users/name/Second"), workspaceId: "workspace.Second", displayName: "Second" };
  const listSessions = vi.fn<(workspaceId: string) => Promise<SessionsListResponse>>().mockImplementation(async (workspaceId) => ({
    sessions:
      workspaceId === "workspace.First"
        ? [session({ sessionId: "session.first", workspaceId, plan: "First project history." })]
        : [session({ sessionId: "session.second", workspaceId, plan: "Second project current thread." })],
  }));
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: second,
      workspaces: [first, second],
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 2,
      },
    }),
    listSessions,
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  const projects = screen.getByLabelText("Projects");
  expect(within(projects).getByRole("button", { name: "First project history." })).toBeInTheDocument();
  expect(within(projects).getByRole("button", { name: "Second project current thread." })).toBeInTheDocument();
  expect(listSessions).toHaveBeenCalledWith("workspace.First");
  expect(listSessions).toHaveBeenCalledWith("workspace.Second");
});

test("selecting an existing project activates it through the backend before starting a new thread", async () => {
  const first = { ...workspaceSnapshot("/Users/name/First"), workspaceId: "workspace.First", displayName: "First" };
  const second = { ...workspaceSnapshot("/Users/name/Second"), workspaceId: "workspace.Second", displayName: "Second" };
  const openWorkspace = vi.fn().mockResolvedValue(first);
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: second,
      workspaces: [first, second],
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 0,
      },
    }),
    openWorkspace,
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  const projects = screen.getByLabelText("Projects");
  fireEvent.click(within(projects).getByRole("button", { name: "First" }));

  await waitFor(() => expect(openWorkspace).toHaveBeenCalledWith({ path: "/Users/name/First" }));
  expect(await screen.findByText("/Users/name/First")).toBeInTheDocument();
});

test("left drawer relinks a read-only project through the backend", async () => {
  const readOnlyWorkspace = {
    ...workspaceSnapshot("/Users/name/missing-project"),
    displayName: "",
    rootExists: false,
    readOnly: true,
    stale: true,
    blockedReason: "workspace_root_missing",
  };
  const relinkWorkspace = vi.fn().mockResolvedValue({
    ...workspaceSnapshot("/Users/name/relinked-project"),
    displayName: "",
    readOnly: false,
    rootExists: true,
  });
  const relinkedWorkspace = {
    ...workspaceSnapshot("/Users/name/relinked-project"),
    displayName: "",
    readOnly: false,
    rootExists: true,
  };
  chooseRepositoryFolderMock.mockResolvedValue("/Users/name/relinked-project");
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>()
      .mockResolvedValueOnce({
        readiness: { state: "ready" },
        currentWorkspace: readOnlyWorkspace,
        workspaces: [readOnlyWorkspace],
        routeInput: {
          readiness: "ready",
          hasWorkspace: true,
          activeApprovalCount: 0,
          activeSessionCount: 0,
        },
      })
      .mockResolvedValue({
        readiness: { state: "ready" },
        currentWorkspace: relinkedWorkspace,
        workspaces: [relinkedWorkspace],
        routeInput: {
          readiness: "ready",
          hasWorkspace: true,
          activeApprovalCount: 0,
          activeSessionCount: 0,
        },
      }),
    relinkWorkspace: relinkWorkspace.mockResolvedValue(relinkedWorkspace),
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  const projects = screen.getByLabelText("Projects");
  fireEvent.click(within(projects).getByRole("button", { name: "Project actions for missing-project" }));
  fireEvent.click(screen.getByRole("menuitem", { name: "Relink project" }));

  await waitFor(() =>
    expect(relinkWorkspace).toHaveBeenCalledWith("workspace.desktoplab", {
      path: "/Users/name/relinked-project",
    }),
  );
  expect(await screen.findByRole("button", { name: "relinked-project" })).toBeInTheDocument();
});

test("left drawer archives threads through the backend instead of deleting the project", async () => {
  const archiveSession = vi.fn().mockResolvedValue({ archived: true });
  const listSessions = vi.fn<(workspaceId: string) => Promise<SessionsListResponse>>()
    .mockResolvedValueOnce({ sessions: [session({ sessionId: "session.1", plan: "Inspect repository boundaries." })] })
    .mockResolvedValue({ sessions: [] });
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: workspaceSnapshot("/Users/name/project"),
      workspaces: [workspaceSnapshot("/Users/name/project")],
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 1,
      },
    }),
    archiveSession,
    listSessions,
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  const projects = screen.getByLabelText("Projects");
  expect(within(projects).getByRole("button", { name: "Inspect repository boundaries." })).toBeInTheDocument();

  fireEvent.click(within(projects).getByRole("button", { name: "Thread actions for Inspect repository boundaries." }));
  fireEvent.click(within(projects).getByRole("menuitem", { name: "Archive thread" }));

  await waitFor(() => expect(archiveSession).toHaveBeenCalledWith("session.1"));
  await waitFor(() => expect(within(projects).queryByRole("button", { name: "Inspect repository boundaries." })).not.toBeInTheDocument());
  expect(within(projects).getByRole("button", { name: "project" })).toBeInTheDocument();
});

test("project drawer falls back to repository folder name when backend display name is empty", async () => {
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: { ...workspaceSnapshot("/Users/name/ContactCreator"), displayName: "" },
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 0,
      },
    }),
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  const projects = screen.getByLabelText("Projects");
  expect(within(projects).getByRole("button", { name: "ContactCreator" })).toBeInTheDocument();
  expect(within(projects).getByRole("button", { name: "Collapse project ContactCreator" })).toBeInTheDocument();
});

test("control center demotes support surfaces without displacing projects", async () => {
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: workspaceSnapshot("/Users/name/project"),
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 1,
        activeSessionCount: 1,
      },
    }),
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  expect(screen.getByText("New chat")).toBeInTheDocument();
  expect(screen.queryByText("Scheduled")).not.toBeInTheDocument();
  expect(screen.queryByText("Plugins")).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Approvals" })).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "project" })).toBeInTheDocument();

  expect(screen.queryByText("Accounts")).not.toBeInTheDocument();
  openSupport();

  const drawer = screen.getByTestId("app-drawer");
  expect(drawer).toContainElement(screen.getByRole("navigation", { name: "Control center" }));
  expect(screen.getByText("Setup")).toBeVisible();
  expect(screen.getByText("Diagnostics")).toBeVisible();
  expect(screen.getByText("Settings")).toBeVisible();
  expect(screen.queryByText("Accounts")).not.toBeInTheDocument();
  expect(screen.queryByText("Models")).not.toBeInTheDocument();
  expect(screen.queryByText("Sessions")).not.toBeInTheDocument();
  expect(screen.queryByText("Context")).not.toBeInTheDocument();
  expect(screen.queryByText("Changes")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "project" })).toBeInTheDocument();
});

test("resets the main scroll position when navigating between product sections", async () => {
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: workspaceSnapshot("/Users/name/project"),
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 0,
      },
    }),
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  const workbench = screen.getByTestId("workbench-scroll-region");
  workbench.scrollTop = 320;
  openSupport();
  fireEvent.click(screen.getByRole("button", { name: "Settings" }));

  expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
  expect(workbench.scrollTop).toBe(0);
});

test("right workspace drawer can open and close after a project is active", async () => {
  renderReadyApp();

  fireEvent.click(await screen.findByRole("button", { name: "Open Repository" }));
  fireEvent.change(screen.getByLabelText("Repository path"), { target: { value: "/Users/name/project" } });
  fireEvent.click(screen.getByRole("button", { name: "Open Repository" }));

  await screen.findByRole("heading", { name: "Agent" });
  fireEvent.click(screen.getByRole("button", { name: "Show inspector" }));
  expect(await screen.findByText("Repository inspector")).toBeInTheDocument();
  fireEvent.click(await screen.findByRole("button", { name: "src" }));
  expect(await screen.findByRole("button", { name: "main.rs" })).toBeInTheDocument();
  expect(screen.getByText("Protected")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Hide inspector" }));

  expect(screen.queryByText("Repository inspector")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Show inspector" })).toBeInTheDocument();
});

test("right inspector exposes stable work modes without losing file preview state", async () => {
  renderReadyApp();

  fireEvent.click(await screen.findByRole("button", { name: "Open Repository" }));
  fireEvent.change(screen.getByLabelText("Repository path"), { target: { value: "/Users/name/project" } });
  fireEvent.click(screen.getByRole("button", { name: "Open Repository" }));
  fireEvent.click(await screen.findByRole("button", { name: "Show inspector" }));

  const inspector = await screen.findByRole("complementary", { name: "Repository inspector" });
  expect(within(inspector).getByRole("button", { name: "Files" })).toHaveAttribute("aria-pressed", "true");
  expect(within(inspector).getByRole("button", { name: "Context" })).toBeInTheDocument();
  expect(within(inspector).getByRole("button", { name: "Changes" })).toBeInTheDocument();
  expect(within(inspector).getByRole("button", { name: "Activity" })).toBeInTheDocument();

  fireEvent.click(await within(inspector).findByRole("button", { name: "src" }));
  fireEvent.click(await within(inspector).findByRole("button", { name: "main.rs" }));
  expect(await within(inspector).findByText("fn main() {}")).toBeInTheDocument();

  fireEvent.click(within(inspector).getByRole("button", { name: "Context" }));
  expect(within(inspector).getByRole("heading", { name: "Workspace context" })).toBeInTheDocument();
  expect(within(inspector).getByText("Review repository languages, package managers, protected paths and refresh status.")).toBeInTheDocument();
  expect(within(inspector).getByRole("button", { name: "View context" })).toBeInTheDocument();
  expect(screen.getByRole("complementary", { name: "Repository inspector" })).toBeInTheDocument();

  fireEvent.click(within(inspector).getByRole("button", { name: "Changes" }));
  expect(within(inspector).getByRole("heading", { name: "Repository changes" })).toBeInTheDocument();
  expect(within(inspector).getByRole("button", { name: "Review changes" })).toBeInTheDocument();

  fireEvent.click(within(inspector).getByRole("button", { name: "Activity" }));
  expect(within(inspector).getByRole("heading", { name: "Workspace activity" })).toBeInTheDocument();
  expect(within(inspector).getByText("Agent and terminal activity will appear here.")).toBeInTheDocument();

  fireEvent.click(within(inspector).getByRole("button", { name: "Files" }));
  expect(within(inspector).getByRole("button", { name: "Files" })).toHaveAttribute("aria-pressed", "true");
  expect(within(inspector).getByText("fn main() {}")).toBeInTheDocument();
});

test("escape closes the repository tree subdrawer without closing the inspector", async () => {
  renderReadyApp();

  fireEvent.click(await screen.findByRole("button", { name: "Open Repository" }));
  fireEvent.change(screen.getByLabelText("Repository path"), { target: { value: "/Users/name/project" } });
  fireEvent.click(screen.getByRole("button", { name: "Open Repository" }));
  fireEvent.click(await screen.findByRole("button", { name: "Show inspector" }));

  const inspector = await screen.findByRole("complementary", { name: "Repository inspector" });
  fireEvent.click(await within(inspector).findByRole("button", { name: "src" }));
  fireEvent.click(await within(inspector).findByRole("button", { name: "main.rs" }));
  expect(await within(inspector).findByText("fn main() {}")).toBeInTheDocument();
  fireEvent.click(within(inspector).getByRole("button", { name: "Show repository tree" }));
  expect(within(inspector).getByTestId("repository-tree-subdrawer")).not.toHaveClass("hidden");

  fireEvent.keyDown(within(inspector).getByTestId("repository-tree-panel"), { key: "Escape" });

  expect(within(inspector).getByTestId("repository-tree-subdrawer")).toHaveClass("hidden");
  expect(screen.getByRole("complementary", { name: "Repository inspector" })).toBeInTheDocument();
});

test("bottom terminal drawer stays closed by default and can open resize and close", async () => {
  renderReadyApp();

  fireEvent.click(await screen.findByRole("button", { name: "Open Repository" }));
  fireEvent.change(screen.getByLabelText("Repository path"), { target: { value: "/Users/name/project" } });
  fireEvent.click(screen.getByRole("button", { name: "Open Repository" }));

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  expect(screen.queryByRole("complementary", { name: "Terminal" })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Show terminal" }));

  const terminal = await screen.findByRole("complementary", { name: "Terminal" });
  expect(screen.getByTestId("desktoplab-main")).toHaveClass("h-full");
  expect(screen.getByTestId("workbench-scroll-region")).toHaveClass("min-h-0");
  expect(screen.getByTestId("workbench-scroll-region")).toHaveClass("overflow-auto");
  expect(terminal.parentElement).toHaveAttribute("data-testid", "desktoplab-main");
  expect(terminal).toHaveStyle({ height: "132px" });
  expect(screen.getByText("project %")).toBeInTheDocument();
  expect(screen.queryByText("Run a workspace command")).not.toBeInTheDocument();
  fireEvent.mouseDown(screen.getByRole("separator", { name: "Resize terminal drawer" }), { clientY: 700 });
  fireEvent.mouseMove(window, { clientY: 100 });
  fireEvent.mouseUp(window);

  expect(terminal).toHaveStyle({ height: "420px" });
  fireEvent.click(screen.getByRole("button", { name: "Hide terminal" }));
  expect(screen.queryByRole("complementary", { name: "Terminal" })).not.toBeInTheDocument();
});

test("bottom terminal submits typed commands through the native workspace terminal", async () => {
  const createTerminalCommand = vi.fn().mockResolvedValue(terminalCommandResponse("npm test", "ok"));
  renderReadyApp({ createTerminalCommand });

  fireEvent.click(await screen.findByRole("button", { name: "Open Repository" }));
  fireEvent.change(screen.getByLabelText("Repository path"), { target: { value: "/Users/name/project" } });
  fireEvent.click(screen.getByRole("button", { name: "Open Repository" }));
  await screen.findByRole("heading", { name: "Agent" });
  fireEvent.click(screen.getByRole("button", { name: "Show terminal" }));

  const input = await screen.findByRole("textbox", { name: "Terminal input" });
  fireEvent.change(input, { target: { value: "npm test" } });
  fireEvent.keyDown(input, { key: "Enter" });

  await waitFor(() =>
    expect(createTerminalCommand).toHaveBeenCalledWith("workspace.desktoplab", {
      command: "npm test",
      cwd: ".",
    }),
  );
  expect(runUserTerminalCommandMock).not.toHaveBeenCalled();
  expect(await screen.findByText("project % npm test")).toBeInTheDocument();
  expect(screen.queryByText("Approval required")).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Run terminal command" })).not.toBeInTheDocument();
  expect(input).toHaveValue("");
});

test("drawer inspector thread and terminal own independent scroll regions", async () => {
  renderReadyApp();

  fireEvent.click(await screen.findByRole("button", { name: "Open Repository" }));
  fireEvent.change(screen.getByLabelText("Repository path"), { target: { value: "/Users/name/project" } });
  fireEvent.click(screen.getByRole("button", { name: "Open Repository" }));
  await screen.findByRole("heading", { name: "Agent" });
  await screen.findByRole("button", { name: "Inspect repository boundaries." });
  fireEvent.click(screen.getByRole("button", { name: "Show inspector" }));
  fireEvent.click(screen.getByRole("button", { name: "Show terminal" }));
  await waitFor(() => expect(screen.getByTestId("terminal-scroll-region")).toBeInTheDocument());

  expect(screen.getByTestId("app-drawer")).toHaveClass("overflow-hidden");
  expect(screen.getByTestId("workbench-scroll-region")).toHaveClass("overflow-auto");
  expect(screen.getByTestId("repository-inspector-scroll-region")).toHaveClass("overflow-auto");
  expect(screen.getByTestId("terminal-scroll-region")).toHaveClass("overflow-auto");
  expect(screen.getByRole("complementary", { name: "Terminal" }).parentElement).toHaveAttribute(
    "data-testid",
    "desktoplab-main",
  );
});

test("open drawers can be resized within hard min and max widths", async () => {
  renderReadyApp();

  fireEvent.click(await screen.findByRole("button", { name: "Open Repository" }));
  fireEvent.change(screen.getByLabelText("Repository path"), { target: { value: "/Users/name/project" } });
  fireEvent.click(screen.getByRole("button", { name: "Open Repository" }));
  fireEvent.click(await screen.findByRole("button", { name: "Show inspector" }));
  await screen.findByText("Repository inspector");

  const root = screen.getByTestId("desktoplab-shell");
  fireEvent.mouseDown(screen.getByRole("separator", { name: "Resize left drawer" }), { clientX: 276 });
  fireEvent.mouseMove(window, { clientX: 900 });
  fireEvent.mouseUp(window);

  expect(root).toHaveStyle({ gridTemplateColumns: "360px 4px minmax(0,1fr) 4px 420px" });

  fireEvent.mouseDown(screen.getByRole("separator", { name: "Resize right drawer" }), { clientX: 980 });
  fireEvent.mouseMove(window, { clientX: 300 });
  fireEvent.mouseUp(window);

  expect(root).toHaveStyle({ gridTemplateColumns: "360px 4px minmax(0,1fr) 4px 720px" });
});

test("pane resize handles support keyboard controls and persist sizes", async () => {
  const appState = vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
    readiness: { state: "ready" },
    currentWorkspace: workspaceSnapshot("/Users/name/project"),
    routeInput: {
      readiness: "ready",
      hasWorkspace: true,
      activeApprovalCount: 0,
      activeSessionCount: 0,
    },
  });
  const { unmount } = renderApp(<App />, { appState });
  await screen.findByRole("heading", { name: "Agent" });
  fireEvent.click(screen.getByRole("button", { name: "Show inspector" }));
  fireEvent.click(screen.getByRole("button", { name: "Show terminal" }));
  await waitFor(() => expect(screen.getByTestId("terminal-scroll-region")).toBeInTheDocument());

  const shell = screen.getByTestId("desktoplab-shell");
  const leftHandle = screen.getByRole("separator", { name: "Resize left drawer" });
  expect(leftHandle).toHaveAttribute("tabindex", "0");
  fireEvent.keyDown(leftHandle, { key: "ArrowRight" });
  expect(shell).toHaveStyle({ gridTemplateColumns: "292px 4px minmax(0,1fr) 4px 420px" });

  const rightHandle = screen.getByRole("separator", { name: "Resize right drawer" });
  expect(rightHandle).toHaveAttribute("tabindex", "0");
  fireEvent.keyDown(rightHandle, { key: "ArrowLeft" });
  expect(shell).toHaveStyle({ gridTemplateColumns: "292px 4px minmax(0,1fr) 4px 436px" });

  const terminalHandle = screen.getByRole("separator", { name: "Resize terminal drawer" });
  expect(terminalHandle).toHaveAttribute("tabindex", "0");
  fireEvent.keyDown(terminalHandle, { key: "ArrowUp" });
  expect(screen.getByRole("complementary", { name: "Terminal" })).toHaveStyle({ height: "148px" });

  unmount();
  renderApp(<App />, { appState });
  await screen.findByRole("heading", { name: "Agent" });
  fireEvent.click(screen.getByRole("button", { name: "Show inspector" }));
  fireEvent.click(screen.getByRole("button", { name: "Show terminal" }));
  await waitFor(() => expect(screen.getByTestId("terminal-scroll-region")).toBeInTheDocument());

  expect(screen.getByTestId("desktoplab-shell")).toHaveStyle({
    gridTemplateColumns: "292px 4px minmax(0,1fr) 4px 436px",
  });
  expect(screen.getByRole("complementary", { name: "Terminal" })).toHaveStyle({ height: "148px" });
});

test("renders workspace open feature when backend route selects workspaces", () => {
  renderApp(
    <App
      routeInput={{
        readiness: "ready",
        hasWorkspace: false,
        activeApprovalCount: 0,
        activeSessionCount: 0,
      }}
    />,
  );

  expect(screen.getByRole("heading", { name: "Open a project folder" })).toBeInTheDocument();
  expect(screen.getByLabelText("Repository path")).toBeInTheDocument();
});

test("operational navigation returns to setup until backend setup is ready", async () => {
  renderApp();

  fireEvent.click(screen.getByText("Projects"));

  expect(await screen.findByText("Apple M4 Pro")).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Open a project folder" })).not.toBeInTheDocument();
});

test("workspace open shows the backend setup-required reason", async () => {
  renderReadyApp({
    openWorkspace: vi.fn().mockRejectedValue(
      new DesktopLabApiError(
        "backend_error",
        "Setup must finish before opening a repository.",
        400,
      ),
    ),
  });

  fireEvent.change(await screen.findByLabelText("Repository path"), { target: { value: "/Users/name/project" } });
  fireEvent.click(screen.getByRole("button", { name: "Open Repository" }));

  expect(await screen.findByText("Setup must finish before opening a repository.")).toBeInTheDocument();
});

test("hides internal background work from primary app navigation", async () => {
  renderApp();

  expect(await screen.findByText("Apple M4 Pro")).toBeInTheDocument();
  const drawer = screen.getByTestId("app-drawer");
  expect(within(drawer).queryByText("Scheduled")).not.toBeInTheDocument();
  expect(within(drawer).queryByText("Background work")).not.toBeInTheDocument();
  expect(within(drawer).queryByText("Download coding model")).not.toBeInTheDocument();
});

test("opens settings diagnostics from the app navigation", async () => {
  renderApp();

  openSupport();
  fireEvent.click(screen.getByText("Settings"));

  expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
  expect(screen.getByRole("heading", { name: "Current setup" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Updates" })).toBeInTheDocument();
});

test("opens diagnostics repair center from the app navigation", async () => {
  renderApp();

  openSupport();
  fireEvent.click(screen.getByText("Diagnostics"));

  expect(await screen.findByRole("heading", { name: "Diagnostics" })).toBeInTheDocument();
  expect(screen.getByText("Restart local runner")).toBeInTheDocument();
});

test("keeps session changes out of the thread until toolbar changes is opened", async () => {
  renderReadyApp({
    agentWorkspace: vi.fn<(workspaceId: string) => Promise<AgentWorkspaceSnapshot>>().mockResolvedValue({
      route: {
        status: "selected",
        backendId: "backend.ollama",
        backendDisplayName: "Local runner",
        backendKind: "local",
        summary: "Runs on this machine",
        reasons: ["No provider access required"],
        requiredCapabilities: ["Chat"],
        needsFallbackApproval: false,
      },
      context: null,
      session: session({ state: "completed", summary: "1 file changed", checkpoints: ["checkpoint.1"] }),
    }),
  });

  fireEvent.click(screen.getByText("Projects"));
  fireEvent.change(screen.getByLabelText("Repository path"), { target: { value: "/Users/name/project" } });
  fireEvent.click(screen.getByRole("button", { name: "Open Repository" }));
  await screen.findByRole("heading", { name: "Agent" });

  const composer = screen.getByTestId("agent-composer");
  expect(within(composer).queryByRole("button", { name: "Review changes" })).not.toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Changes" })).not.toBeInTheDocument();
  expect(screen.queryByRole("complementary", { name: "Session changes" })).not.toBeInTheDocument();
});

test("opens the agent workbench after a repository is opened", async () => {
  renderApp(
    <App
      routeInput={{
        readiness: "ready",
        hasWorkspace: false,
        activeApprovalCount: 0,
        activeSessionCount: 0,
      }}
    />,
  );

  fireEvent.change(screen.getByLabelText("Repository path"), { target: { value: "/Users/name/project" } });
  fireEvent.click(screen.getByRole("button", { name: "Open Repository" }));

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  expect(screen.getByTestId("agent-thread-surface")).toBeInTheDocument();
  expect(screen.queryByLabelText("Workbench readiness")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: /Selected model/ })).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Agent route" })).not.toBeInTheDocument();
});

test("keeps session management out of primary drawer navigation", async () => {
  renderReadyApp();

  fireEvent.click(screen.getByText("Projects"));
  fireEvent.change(screen.getByLabelText("Repository path"), { target: { value: "/Users/name/project" } });
  fireEvent.click(screen.getByRole("button", { name: "Open Repository" }));
  await screen.findByRole("heading", { name: "Agent" });

  openSupport();
  expect(screen.queryByText("Sessions")).not.toBeInTheDocument();
  expect(screen.getByRole("heading", { name: "Agent" })).toBeInTheDocument();
});

test("active approvals do not displace the agent workbench", async () => {
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: workspaceSnapshot("/Users/name/project"),
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 1,
        activeSessionCount: 0,
      },
    }),
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Approvals" })).not.toBeInTheDocument();
});

test("boots into agent workbench from backend current workspace state", async () => {
  renderApp(<App />, {
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "ready" },
      currentWorkspace: workspaceSnapshot("/Users/name/project"),
      routeInput: {
        readiness: "ready",
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 0,
      },
    }),
  });

  expect(await screen.findByRole("heading", { name: "Agent" })).toBeInTheDocument();
});

function renderApp(node = <App />, overrides: Partial<DesktopLabApiClient> = {}) {
  return render(<AppProviders apiClient={clientFor(overrides)}>{node}</AppProviders>);
}

function renderReadyApp(overrides: Partial<DesktopLabApiClient> = {}) {
  return renderApp(
    <App
      routeInput={{
        readiness: "ready",
        hasWorkspace: false,
        activeApprovalCount: 0,
        activeSessionCount: 0,
      }}
    />,
    overrides,
  );
}

function openSupport() {
  fireEvent.click(screen.getByText("Control center"));
}

function clientFor(overrides: Partial<DesktopLabApiClient> = {}): DesktopLabApiClient {
  return {
    health: vi.fn<() => Promise<HealthResponse>>().mockResolvedValue({ status: "healthy" }),
    readiness: vi.fn<() => Promise<ReadinessResponse>>().mockResolvedValue({ state: "ready" }),
    version: vi.fn<() => Promise<VersionResponse>>().mockResolvedValue({ productVersion: "0.1.0", apiVersion: "v1" }),
    appState: vi.fn<() => Promise<AppStateResponse>>().mockResolvedValue({
      readiness: { state: "degraded" },
      currentWorkspace: null,
      routeInput: {
        readiness: "degraded",
        hasWorkspace: false,
        activeApprovalCount: 0,
        activeSessionCount: 0,
      },
    }),
    setupPreview: vi.fn<() => Promise<SetupPlanPreview>>().mockResolvedValue(preview()),
    acceptSetupPlan: vi.fn<() => Promise<SetupAcceptanceResponse>>().mockResolvedValue({ startedJobIds: [] }),
    catalogRefreshStatus: vi.fn<() => Promise<CatalogRefreshStatusResponse>>().mockResolvedValue({
      state: "ready",
      lastKnownGoodAvailable: false,
      degradedReasons: [],
      manualRefresh: { available: true, jobId: "registry.refresh.manual" },
    }),
    startCatalogRefresh: vi.fn<() => Promise<CatalogRefreshRequestResponse>>().mockResolvedValue({ jobId: "registry.refresh.manual" }),
    listProviders: vi.fn<() => Promise<ProvidersListResponse>>().mockResolvedValue({ providers: [] }),
    routePreference: vi.fn<() => Promise<RoutePreference>>().mockResolvedValue({
      mode: "local_first",
      cloudAllowed: false,
      lockedByPolicy: true,
      explanation: "Local first.",
    }),
    listRuntimes: vi.fn<() => Promise<RuntimesListResponse>>().mockResolvedValue({
      runtimes: [
        {
          runtimeId: "runtime.ollama",
          displayName: "Ollama",
          ownership: "desktoplab_managed",
          status: "running",
          capabilities: ["Local chat"],
          install: { supported: true },
          repairActions: [],
        },
      ],
    }),
    listModels: vi.fn<() => Promise<ModelsListResponse>>().mockResolvedValue({
      models: [
        {
          modelId: "model.qwen-coder",
          displayName: "Qwen Coder",
          runtimeId: "runtime.ollama",
          channel: "stable",
          installState: "installed",
          compatibility: "ready",
          sizeGb: 8,
          recommended: true,
        },
      ],
    }),
    startModelDownload: vi.fn<() => Promise<ModelDownloadResponse>>().mockResolvedValue({
      jobId: "model.download.qwen-coder",
      modelId: "model.qwen-coder",
      runtimeId: "runtime.ollama",
      state: "downloading",
      retryClass: "retryable",
    }),
    listExternalBackends: vi.fn<() => Promise<ExternalBackendsResponse>>().mockResolvedValue({ backends: [] }),
    approvalModes: vi.fn<() => Promise<ApprovalModesResponse>>().mockResolvedValue({
      defaultMode: "require_approval",
      sessionMode: "require_approval",
      modes: [],
    }),
    listJobs: vi.fn<() => Promise<JobsListResponse>>().mockResolvedValue({
      jobs: [
        {
          jobId: "job.1",
          kind: "model.download",
          state: "running",
          progressPercent: 42,
          retryClass: "unknown",
          updatedAt: "2026-06-25T19:55:00Z",
        },
      ],
    }),
    retryJob: vi.fn(),
    listSessions: vi.fn<(workspaceId: string) => Promise<SessionsListResponse>>().mockResolvedValue({
      sessions: [session()],
    }),
    archiveWorkspace: vi.fn(),
    archiveSession: vi.fn(),
    createSession: vi.fn(),
    gitOperations: vi.fn<() => Promise<GitOperationsSnapshot>>().mockResolvedValue(gitOperations()),
    listApprovals: vi.fn<() => Promise<ApprovalsListResponse>>().mockResolvedValue({
      approvals: [approval()],
    }),
    resolveApproval: vi.fn<(approvalId: string, request: { resolution: "approve" | "deny" }) => Promise<ApprovalResolveResponse>>(),
    diagnostics: vi.fn<() => Promise<DiagnosticsSnapshot>>().mockResolvedValue(diagnostics()),
    localAuditTransparency: vi.fn<() => Promise<LocalAuditTransparencySnapshot>>().mockResolvedValue(localAudit()),
    runDiagnosticRepair: vi.fn(),
    openWorkspace: vi.fn<(request: { path: string }) => Promise<WorkspaceSnapshot>>().mockImplementation(async (request) => ({
      workspaceId: "workspace.desktoplab",
      displayName: "project",
      rootPath: request.path,
      gitDirPath: `${request.path}/.git`,
      apiState: "clean",
      statusEntries: [],
      diffText: "",
      checkpointStatus: "ready",
      canCheckpointRiskyExecution: true,
    })),
    relinkWorkspace: vi.fn<(workspaceId: string, request: { path: string }) => Promise<WorkspaceSnapshot>>().mockImplementation(async (_workspaceId, request) => ({
      workspaceId: "workspace.desktoplab",
      displayName: "project",
      rootPath: request.path,
      rootExists: true,
      readOnly: false,
      gitDirPath: `${request.path}/.git`,
      apiState: "clean",
      statusEntries: [],
      diffText: "",
      checkpointStatus: "ready",
      canCheckpointRiskyExecution: true,
    })),
    listWorkspaceFiles: vi.fn<() => Promise<WorkspaceFileTreeResponse>>().mockResolvedValue({
      workspaceId: "workspace.desktoplab",
      degraded: false,
      degradedReasons: [],
      limits: { maxEntries: 200, maxDepth: 8 },
      entries: [
        { path: "src", kind: "directory", protection: "readable" },
        { path: "src/main.rs", kind: "file", protection: "readable" },
        { path: ".env", kind: "hidden_file", protection: "protected" },
      ],
    }),
    previewWorkspaceFile: vi.fn<() => Promise<WorkspaceFilePreviewResponse>>().mockResolvedValue({
      workspaceId: "workspace.desktoplab",
      path: "src/main.rs",
      state: "text",
      text: "fn main() {}",
      deniedReason: null,
      originalBytes: 12,
      originalLines: 1,
      returnedLines: 1,
      truncated: false,
    }),
    createTerminalCommand: vi.fn<() => Promise<TerminalCommandResponse>>(),
    replayEvents: vi.fn().mockResolvedValue([]),
    agentWorkspace: vi.fn<(workspaceId: string) => Promise<AgentWorkspaceSnapshot>>().mockResolvedValue({
      route: {
        status: "selected",
        backendId: "backend.ollama",
        backendDisplayName: "Local runner",
        backendKind: "local",
        summary: "Runs on this machine",
        reasons: ["No provider access required"],
        requiredCapabilities: ["Chat"],
        needsFallbackApproval: false,
      },
      context: {
        workspaceId: "workspace.desktoplab",
        languages: ["TypeScript"],
        frameworks: ["React"],
        testCommands: [],
        protectedSummary: [".env is excluded"],
        stale: false,
        refreshSupported: true,
      },
      session: null,
    }),
    ...overrides,
  } as unknown as DesktopLabApiClient;
}

function terminalCommandResponse(command: string, stdout: string): TerminalCommandResponse {
  return {
    workspaceId: "workspace.desktoplab",
    terminalId: "terminal.local",
    state: "completed",
    command,
    cwd: ".",
    approval: {
      approvalId: "",
      state: "approved",
      copy: "",
    },
    events: [
      {
        eventId: "terminal.local.output",
        kind: "output",
        status: "exited",
        exitCode: 0,
        stdout,
        stderr: "",
        stdoutTruncated: false,
        redacted: false,
      },
    ],
  };
}

function workspaceSnapshot(path: string): WorkspaceSnapshot {
  return {
    workspaceId: "workspace.desktoplab",
    displayName: "project",
    rootPath: path,
    gitDirPath: `${path}/.git`,
    apiState: "clean",
    statusEntries: [],
    diffText: "",
    checkpointStatus: "ready",
    canCheckpointRiskyExecution: true,
  };
}

function localAudit(): LocalAuditTransparencySnapshot {
  return {
    scope: "local_single_user",
    records: [],
    redactedExport: "",
  };
}

function gitOperations(): GitOperationsSnapshot {
  return {
    workspaceId: "workspace.desktoplab",
    workspaceState: "clean",
    warnings: [],
    changedFiles: [],
    diffPreview: "",
    savePoints: [],
    commit: {
      supported: false,
      sessionId: "session.1",
      message: "",
      preview: "",
      changeFingerprint: "sha256:empty",
      requiresApproval: true,
    },
    push: {
      supported: false,
      remote: "origin",
      branch: "main",
      preview: "",
      requiresApproval: true,
    },
    worktrees: [],
  };
}

function diagnostics(): DiagnosticsSnapshot {
  return {
    state: "degraded",
    services: [{ family: "runtime", label: "Runtime", state: "degraded", message: "Ollama is stopped" }],
    repairActions: [
      {
        repairId: "repair.runtime",
        family: "runtime",
        label: "Restart local runner",
        reason: "Ollama is stopped",
        mode: "executable",
      },
    ],
    bundlePreview: {
      summary: "Runtime stopped. token=[REDACTED]",
      sizeBytes: 9000,
      maxBytes: 64000,
      redacted: true,
    },
    updateStatus: {
      channel: "dev",
      currentVersion: "0.1.0",
      state: "disabled",
      message: "Update checks are prepared but public release updates are not enabled yet.",
      canInstall: false,
    },
  };
}

function approval(): ApprovalSummary {
  return {
    approvalId: "approval.1",
    sessionId: "session.1",
    action: "filesystem.write",
    state: "pending",
    risk: "medium",
    title: "Review file change",
    message: "The agent wants to edit files in the active repository.",
    requestedAt: "2026-06-25T20:30:00Z",
    policyReason: "Filesystem writes need confirmation.",
  };
}

function preview(): SetupPlanPreview {
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

function session(overrides: Partial<AgentSessionSnapshot> = {}): AgentSessionSnapshot {
  return {
    sessionId: "session.1",
    workspaceId: "workspace.desktoplab",
    executionBackendId: "backend.ollama",
    owner: "desktoplab",
    state: "planning",
    plan: "Inspect repository boundaries.",
    checkpoints: [],
    summary: null,
    timeline: [],
    ...overrides,
  };
}
