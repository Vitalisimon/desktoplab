// @vitest-environment jsdom
import { fireEvent, render, renderHook, screen, waitFor } from "@testing-library/react";
import { AppProviders } from "../app/AppProviders";
import type { DesktopLabApiClient } from "../api/client";
import type { AgentSessionSnapshot, SessionsListResponse, WorkspaceSnapshot } from "../api/types";
import { ProjectTree } from "./AppDrawerProjects";
import { useDrawerProjectThreads } from "./useDrawerProjectThreads";

test("recovers drawer threads after transient control-plane failures", async () => {
  const listSessions = vi
    .fn<(workspaceId: string) => Promise<SessionsListResponse>>()
    .mockRejectedValueOnce(new Error("warming up"))
    .mockRejectedValueOnce(new Error("warming up"))
    .mockRejectedValueOnce(new Error("warming up"))
    .mockResolvedValue({ sessions: [session("session.18")] });
  const { result } = renderHook(() => useDrawerProjectThreads([workspace]), {
    wrapper: wrapper({ listSessions } as unknown as DesktopLabApiClient),
  });

  expect(result.current.statusByWorkspace[workspace.workspaceId]).toBe("loading");
  await waitFor(() => expect(result.current.statusByWorkspace[workspace.workspaceId]).toBe("ready"), { timeout: 2_000 });
  expect(result.current.byWorkspace[workspace.workspaceId]?.[0]?.sessionId).toBe("session.18");
  expect(listSessions).toHaveBeenCalledTimes(4);
});

test("does not present loading or failed thread requests as an empty project", () => {
  const props = {
    compact: false,
    workspace,
    threads: [],
    active: true,
    selectedSessionId: null,
    pinnedItems: [],
    onTogglePin: vi.fn(),
  };
  const { rerender } = render(<ProjectTree {...props} threadsStatus="loading" />);

  expect(screen.getByText("Loading threads...")).toBeInTheDocument();
  expect(screen.queryByText("No threads yet")).not.toBeInTheDocument();
  rerender(<ProjectTree {...props} threadsStatus="error" />);
  expect(screen.getByText("Threads unavailable")).toBeInTheDocument();
  rerender(<ProjectTree {...props} threadsStatus="ready" />);
  expect(screen.getByText("No threads yet")).toBeInTheDocument();
});

test("keeps known threads visible while their background refresh is loading", () => {
  render(
    <ProjectTree
      compact={false}
      workspace={workspace}
      threads={[session("session.18", "Persisted task")]}
      threadsStatus="loading"
      active={true}
      selectedSessionId="session.18"
      pinnedItems={[]}
      onTogglePin={vi.fn()}
    />,
  );

  expect(screen.getByRole("button", { name: "Persisted task" })).toBeVisible();
  expect(screen.queryByText("Loading threads...")).not.toBeInTheDocument();
});

test("keeps long thread histories scannable without hiding access to older threads", () => {
  render(
    <ProjectTree
      compact={false}
      workspace={workspace}
      threads={Array.from({ length: 8 }, (_, index) => session(`session.${index + 1}`, `Task ${index + 1}`))}
      threadsStatus="ready"
      active={true}
      selectedSessionId={null}
      pinnedItems={[]}
      onTogglePin={vi.fn()}
    />,
  );

  expect(screen.getByText("Task 6")).toBeInTheDocument();
  expect(screen.queryByText("Task 7")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Show 2 older threads" }));
  expect(screen.getByText("Task 8")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Show recent only" }));
  expect(screen.queryByText("Task 8")).not.toBeInTheDocument();
});

test("distinguishes running, failed and paused thread indicators", () => {
  const { container } = render(
    <ProjectTree
      compact={false}
      workspace={workspace}
      threads={[
        session("session.running", "Running task", "running"),
        session("session.failed", "Failed task", "failed"),
        session("session.input", "Input task", "blocked", "clarification_required:file_target"),
        session("session.approval", "Approval task", "blocked", "approval_requested", true),
        session("session.blocked", "Blocked task", "blocked"),
        session("session.completed", "Completed task", "completed"),
      ]}
      threadsStatus="ready"
      active={true}
      selectedSessionId={null}
      pinnedItems={[]}
      onTogglePin={vi.fn()}
    />,
  );

  expect(container.querySelector('[title="Running"]')).toHaveClass("bg-accent", "dl-running-dot");
  expect(container.querySelector('[title="Failed"] svg')).toHaveClass("text-danger");
  expect(container.querySelector('[title="Input required"] svg')).toHaveClass("text-accent");
  expect(container.querySelector('[title="Approval required"] svg')).toHaveClass("text-warning");
  expect(container.querySelector('[title="Blocked"] svg')).toHaveClass("text-muted");
  expect(container.querySelectorAll('[title="Failed"].rounded-full, [title="Input required"].rounded-full, [title="Approval required"].rounded-full, [title="Blocked"].rounded-full')).toHaveLength(0);
  expect(screen.getByRole("button", { name: "Completed task" }).querySelector("[title]")).toBeNull();
});

function wrapper(apiClient: DesktopLabApiClient) {
  return ({ children }: { children: React.ReactNode }) => <AppProviders apiClient={apiClient}>{children}</AppProviders>;
}

const workspace: WorkspaceSnapshot = {
  workspaceId: "workspace.desktoplab",
  rootPath: "/tmp/desktoplab",
  displayName: "DesktopLab",
  gitDirPath: "/tmp/desktoplab/.git",
  apiState: "clean",
  statusEntries: [],
  diffText: "",
  checkpointStatus: "ready",
  canCheckpointRiskyExecution: true,
};

function session(
  sessionId: string,
  prompt?: string,
  state: AgentSessionSnapshot["state"] = "completed",
  timelineMessage?: string,
  pendingApproval = false,
): AgentSessionSnapshot {
  return {
    sessionId,
    workspaceId: workspace.workspaceId,
    executionBackendId: "backend.ollama",
    owner: "desktoplab",
    state,
    plan: null,
    checkpoints: [],
    summary: null,
    transcript: prompt ? [{ sequence: 1, role: "user", content: prompt }] : undefined,
    timeline: timelineMessage ? [{ sequence: 1, kind: "blocked", message: timelineMessage, createdAt: "0" }] : [],
    pendingApprovals: pendingApproval ? [{ approvalId: "approval.1", sessionId, action: "filesystem.write", operationId: "filesystem.write:README.md", state: "pending", risk: "medium", title: "Write README.md", message: "Approve file write", requestedAt: "0" }] : [],
  };
}
