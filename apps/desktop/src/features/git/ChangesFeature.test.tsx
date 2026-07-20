// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { GitOperationsSnapshot, WorkspaceSnapshot } from "../../api/types";
import { ChangesFeature } from "./ChangesFeature";

test("renders repository changes from the backend workspace snapshot", () => {
  renderChanges(dirtyWorkspace());

  expect(screen.getByRole("heading", { name: "Changes" })).toBeInTheDocument();
  expect(screen.getByText("Changes found")).toBeInTheDocument();
  expect(screen.getByText("modified: apps/desktop/src/App.tsx")).toBeInTheDocument();
  expect(screen.getByLabelText("Change preview")).toHaveTextContent("diff --git");
});

test("shows dirty-worktree save point readiness through backend-owned actions", async () => {
  renderChanges(dirtyWorkspace());

  expect(screen.getByText("Save point ready")).toBeInTheDocument();
  expect(screen.getByText("DesktopLab can require a save point before risky agent work.")).toBeInTheDocument();
  expect(await screen.findByRole("heading", { name: "Save points" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Commit approved work" })).toBeInTheDocument();
});

test("renders clean repository empty state", () => {
  renderChanges(
    { ...dirtyWorkspace(), apiState: "clean", statusEntries: [], diffText: "", checkpointStatus: "ready", canCheckpointRiskyExecution: true },
    { ...gitSnapshot(), workspaceState: "clean", warnings: [] },
  );

  expect(screen.getByText("No changes")).toBeInTheDocument();
  expect(screen.getByText("No local changes reported.")).toBeInTheDocument();
  expect(screen.getByText("No file preview yet.")).toBeInTheDocument();
  expect(screen.getByText("Save point ready")).toBeInTheDocument();
});

test("replaces a stale clean workspace snapshot with live Git changes", async () => {
  renderChanges(
    { ...dirtyWorkspace(), apiState: "clean", statusEntries: [], diffText: "" },
    {
      ...gitSnapshot(),
      changedFiles: ["README.md", "candidate-proof.md"],
      statusEntries: [" M README.md", "?? candidate-proof.md"],
      diffPreview: "diff --git a/README.md b/README.md",
    },
  );

  expect(await screen.findByText("Changes found")).toBeInTheDocument();
  expect(screen.getByText("M README.md")).toBeInTheDocument();
  expect(screen.getByText("?? candidate-proof.md")).toBeInTheDocument();
  expect(screen.getByLabelText("Change preview")).toHaveTextContent("diff --git");
});

test("refreshes Git data when a workspace is relinked to a different root", async () => {
  const gitOperations = vi
    .fn<() => Promise<GitOperationsSnapshot>>()
    .mockResolvedValueOnce({
      ...gitSnapshot(),
      workspaceState: "clean",
      warnings: [],
      changedFiles: [],
      diffPreview: "",
    })
    .mockResolvedValueOnce({
      ...gitSnapshot(),
      changedFiles: ["new-root.md"],
      diffPreview: "diff --git a/new-root.md b/new-root.md",
    });
  const client = gitClient(gitOperations);
  const first = {
    ...dirtyWorkspace(),
    rootPath: "/first/repo",
    apiState: "clean" as const,
    statusEntries: [],
    diffText: "",
  };
  const view = renderChangesWithClient(first, client);

  expect(await screen.findByText("No changes")).toBeInTheDocument();
  view.rerender(
    <AppProviders apiClient={client}>
      <ChangesFeature workspace={{ ...first, rootPath: "/second/repo" }} />
    </AppProviders>,
  );

  expect(await screen.findByText("new-root.md")).toBeInTheDocument();
  expect(gitOperations).toHaveBeenCalledTimes(2);
});

test("asks for a repository before showing changes", () => {
  renderChanges(null);

  expect(screen.getByRole("heading", { name: "Open a project folder" })).toBeInTheDocument();
  expect(screen.getByText("Open a code folder before reviewing changes and save points.")).toBeInTheDocument();
});

test("keeps changes feature container small enough to split responsibilities early", () => {
  const source = readFileSync("src/features/git/ChangesFeature.tsx", "utf8");

  expect(source.split("\n").length).toBeLessThan(140);
});

function dirtyWorkspace(): WorkspaceSnapshot {
  return {
    workspaceId: "workspace.desktoplab",
    displayName: "desktoplab",
    rootPath: "/repo/desktoplab",
    gitDirPath: "/repo/desktoplab/.git",
    apiState: "dirty",
    statusEntries: ["modified: apps/desktop/src/App.tsx"],
    diffText: "diff --git a/apps/desktop/src/App.tsx b/apps/desktop/src/App.tsx",
    checkpointStatus: "ready",
    canCheckpointRiskyExecution: true,
  };
}

function renderChanges(workspace: WorkspaceSnapshot | null, snapshot = gitSnapshot()) {
  return renderChangesWithClient(
    workspace,
    gitClient(vi.fn<() => Promise<GitOperationsSnapshot>>().mockResolvedValue(snapshot)),
  );
}

function gitClient(gitOperations: () => Promise<GitOperationsSnapshot>) {
  return {
    gitOperations,
    rollbackSavePoint: vi.fn(),
    commitSession: vi.fn(),
    pushBranch: vi.fn(),
    cleanupWorktree: vi.fn(),
  } as unknown as DesktopLabApiClient;
}

function renderChangesWithClient(workspace: WorkspaceSnapshot | null, client: DesktopLabApiClient) {
  return render(
    <AppProviders apiClient={client}>
      <ChangesFeature workspace={workspace} />
    </AppProviders>,
  );
}

function gitSnapshot(): GitOperationsSnapshot {
  return {
    workspaceId: "workspace.desktoplab",
    workspaceState: "dirty",
    warnings: ["Dirty worktree"],
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
      supported: false,
      remote: "origin",
      branch: "main",
      preview: "No remote configured",
      requiresApproval: true,
    },
    worktrees: [],
  };
}
