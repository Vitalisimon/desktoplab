// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { GitOperationsSnapshot } from "../../api/types";
import { GitOperationsFeature } from "./GitOperationsFeature";

test("git actions create backend approvals and never construct approved state in the component", async () => {
  const createApproval = vi.fn().mockResolvedValue({ approvalId: "approval.git.1", state: "pending" });
  const commitSession = vi.fn().mockResolvedValue({ status: "blocked", reason: "approval_pending" });
  renderGit({
    createApproval,
    commitSession,
  });

  fireEvent.click(await screen.findByRole("button", { name: "Commit approved work" }));

  await waitFor(() => {
    expect(createApproval).toHaveBeenCalledWith({
      sessionId: "session.1",
      action: "git.commit",
      operationId: "git.commit",
      payload: {
        sessionId: "session.1",
        message: "agent change",
        changeFingerprint: "sha256:test-diff",
        changedFiles: ["a.rs"],
      },
    });
  });
  expect(commitSession).toHaveBeenCalledWith({
    workspaceId: "workspace.desktoplab",
    sessionId: "session.1",
    message: "agent change",
    changeFingerprint: "sha256:test-diff",
    changedFiles: ["a.rs"],
    approvalId: "approval.git.1",
  });
  expect(JSON.stringify(commitSession.mock.calls)).not.toContain('"approval":"approved"');
});

test("denied approval responses leave git operation blocked", async () => {
  const createApproval = vi.fn().mockResolvedValue({ approvalId: "approval.git.2", state: "pending" });
  const pushBranch = vi.fn().mockResolvedValue({ status: "blocked", reason: "approval_denied" });
  renderGit({
    createApproval,
    pushBranch,
  });

  fireEvent.click(await screen.findByRole("button", { name: "Push with approval" }));

  await waitFor(() => expect(pushBranch).toHaveBeenCalled());
  expect(pushBranch).toHaveBeenCalledWith({
    workspaceId: "workspace.desktoplab",
    remote: "origin",
    branch: "main",
    approvalId: "approval.git.2",
  });
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
    warnings: [],
    changedFiles: ["a.rs"],
    savePoints: [],
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
      preview: "Push requires approval",
      requiresApproval: true,
      normalizedReason: "remote_available",
    },
    worktrees: [],
  };
}
