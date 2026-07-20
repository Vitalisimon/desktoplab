// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { GitOperationsSnapshot } from "../../api/types";
import { GitOperationsFeature } from "./GitOperationsFeature";

test("renders save points rollback commit push and worktree isolation from backend snapshot", async () => {
  renderGit();

  expect(await screen.findByRole("heading", { name: "Save points" })).toBeInTheDocument();
  expect(screen.getByText("Before agent edit")).toBeInTheDocument();
  expect(screen.getByText("Dirty worktree")).toBeInTheDocument();
  expect(screen.getByText("Commit requires approval")).toBeInTheDocument();
  expect(screen.getByText("No remote configured")).toBeInTheDocument();
  expect(screen.getByRole("heading", { name: "Patch review" })).toBeInTheDocument();
  expect(screen.getByText("Changed files: a.rs, b.rs")).toBeInTheDocument();
  expect(screen.getByText("diff --git a/a.rs b/a.rs")).toBeInTheDocument();
  expect(screen.getByText("write-capable work is isolated")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Rollback" })).toBeEnabled();
  expect(screen.getByRole("button", { name: "Clean up user worktree" })).toBeDisabled();
});

test("approval-gated git actions call backend commands", async () => {
  const createApproval = vi
    .fn()
    .mockResolvedValueOnce({ approvalId: "approval.rollback", state: "pending" })
    .mockResolvedValueOnce({ approvalId: "approval.commit", state: "pending" })
    .mockResolvedValueOnce({ approvalId: "approval.push", state: "pending" });
  const rollbackSavePoint = vi.fn().mockResolvedValue({ status: "restored" });
  const commitSession = vi.fn().mockResolvedValue({ status: "committed", commitHash: "abc123" });
  const pushBranch = vi.fn().mockResolvedValue({ status: "denied", reason: "no_remote" });
  renderGit({ createApproval, rollbackSavePoint, commitSession, pushBranch });

  await screen.findByRole("heading", { name: "Save points" });
  fireEvent.click(screen.getByRole("button", { name: "Rollback" }));
  fireEvent.click(screen.getByRole("button", { name: "Approve rollback" }));
  fireEvent.click(screen.getByRole("button", { name: "Commit approved work" }));
  fireEvent.click(screen.getByRole("button", { name: "Push with approval" }));

  await waitFor(() => expect(rollbackSavePoint).toHaveBeenCalledWith("savepoint.1", { approvalId: "approval.rollback" }));
  expect(createApproval).toHaveBeenCalledWith({ sessionId: "session.1", action: "git.rollback", operationId: "savepoint.1" });
  expect(createApproval).toHaveBeenCalledWith({
    sessionId: "session.1",
    action: "git.commit",
    operationId: "git.commit",
    payload: {
      sessionId: "session.1",
      message: "agent change",
      changeFingerprint: "sha256:test-diff",
      changedFiles: ["a.rs", "b.rs"],
    },
  });
  expect(createApproval).toHaveBeenCalledWith({ sessionId: "session.1", action: "git.push", operationId: "git.push" });
  expect(commitSession).toHaveBeenCalledWith({
    workspaceId: "workspace.desktoplab",
    sessionId: "session.1",
    message: "agent change",
    changeFingerprint: "sha256:test-diff",
    changedFiles: ["a.rs", "b.rs"],
    approvalId: "approval.commit",
  });
  expect(pushBranch).toHaveBeenCalledWith({ workspaceId: "workspace.desktoplab", remote: "origin", branch: "main", approvalId: "approval.push" });
});

function renderGit(overrides: Partial<DesktopLabApiClient> = {}) {
  const client = {
    gitOperations: vi.fn().mockResolvedValue(gitSnapshot()),
    createApproval: vi.fn().mockResolvedValue({ approvalId: "approval.1", state: "pending" }),
    rollbackSavePoint: vi.fn(),
    commitSession: vi.fn(),
    pushBranch: vi.fn(),
    cleanupWorktree: vi.fn(),
    ...overrides,
  } as unknown as DesktopLabApiClient;

  return render(
    <AppProviders apiClient={client}>
      <GitOperationsFeature workspaceId="workspace.desktoplab" workspaceRootPath="/repo/desktoplab" />
    </AppProviders>,
  );
}

function gitSnapshot(): GitOperationsSnapshot {
  return {
    workspaceId: "workspace.desktoplab",
    workspaceState: "dirty",
    warnings: ["Dirty worktree"],
    changedFiles: ["a.rs", "b.rs"],
    diffPreview: "diff --git a/a.rs b/a.rs",
    savePoints: [
      {
        savePointId: "savepoint.1",
        title: "Before agent edit",
        sessionId: "session.1",
        createdAt: "2026-06-26T09:00:00Z",
        rollbackSupported: true,
        rollbackPreview: "2 files would be restored",
      },
    ],
    commit: {
      supported: true,
      sessionId: "session.1",
      message: "agent change",
      preview: "Commit requires approval",
      changeFingerprint: "sha256:test-diff",
      requiresApproval: true,
    },
    push: {
      supported: true,
      remote: "origin",
      branch: "main",
      preview: "No remote configured",
      requiresApproval: true,
      normalizedReason: "no_remote",
    },
    worktrees: [
      {
        worktreeId: "worktree.1",
        label: "Agent worktree",
        path: "/repo/.worktrees/session.1",
        sessionId: "session.1",
        cleanupSupported: true,
        userOwned: false,
        isolationReason: "write-capable work is isolated",
      },
      {
        worktreeId: "worktree.user",
        label: "User worktree",
        path: "/repo/manual",
        sessionId: null,
        cleanupSupported: false,
        userOwned: true,
        isolationReason: "unknown user worktree",
      },
    ],
  };
}
