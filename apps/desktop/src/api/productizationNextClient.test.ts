import { DesktopLabApiClient } from "./client";
import type { ApiTransport, TransportRequest } from "./transport";
import type { TerminalCommandResponse, WorkspaceFilePreviewResponse } from "./types";

test("maps git intelligence memory plugin and external backend methods to local api paths", async () => {
  const requests: TransportRequest[] = [];
  const client = new DesktopLabApiClient({
    authToken: "local-test-token",
    transport: transportFor(requests),
  });

  await client.gitOperations("workspace.desktoplab");
  await client.rollbackSavePoint("savepoint.1", { approvalId: "approval.1" });
  await client.commitSession({
    workspaceId: "workspace.desktoplab",
    sessionId: "session.1",
    message: "agent change",
    changeFingerprint: "sha256:reviewed-diff",
    changedFiles: ["README.md"],
    approvalId: "approval.2",
  });
  await client.pushBranch({ workspaceId: "workspace.desktoplab", remote: "origin", branch: "main", approvalId: "approval.3" });
  await client.cleanupWorktree("worktree.1");
  await client.workspaceIntelligence("workspace.desktoplab");
  await client.refreshWorkspaceScan("workspace.desktoplab");
  await client.listWorkspaceMemory("workspace.desktoplab");
  await client.deleteMemory("memory.1");
  await client.sessionContextPreview("workspace.desktoplab");
  await client.listPlugins();
  await client.trustPlugin("plugin.tools", { decision: "approve" });
  await client.listExternalBackends();
  await client.approveExternalBackendRoute("route.codex", { resolution: "deny" });

  expect(requests.map((request) => `${request.method} ${request.path}`)).toEqual([
    "GET /v1/git/operations?workspace_id=workspace.desktoplab",
    "POST /v1/git/savepoints/savepoint.1/rollback",
    "POST /v1/git/commit",
    "POST /v1/git/push",
    "POST /v1/git/worktrees/worktree.1/cleanup",
    "GET /v1/workspaces/workspace.desktoplab/intelligence",
    "POST /v1/workspaces/workspace.desktoplab/intelligence/refresh",
    "GET /v1/workspaces/workspace.desktoplab/memory",
    "POST /v1/workspaces/memory/memory.1/delete",
    "GET /v1/workspaces/workspace.desktoplab/context-preview",
    "GET /v1/plugins",
    "POST /v1/plugins/plugin.tools/trust",
    "GET /v1/external-backends",
    "POST /v1/external-backends/routes/route.codex/resolve",
  ]);
  expect(requests[1].body).toEqual({ approvalId: "approval.1" });
  expect(requests[2].body).toEqual({
    workspaceId: "workspace.desktoplab",
    sessionId: "session.1",
    message: "agent change",
    changeFingerprint: "sha256:reviewed-diff",
    changedFiles: ["README.md"],
    approvalId: "approval.2",
  });
  expect(requests[3].body).toEqual({ workspaceId: "workspace.desktoplab", remote: "origin", branch: "main", approvalId: "approval.3" });
  expect(requests[11].body).toEqual({ decision: "approve" });
  expect(requests[13].body).toEqual({ resolution: "deny" });
});

test("maps workspace file tree and preview methods to local api paths", async () => {
  const requests: TransportRequest[] = [];
  const client = new DesktopLabApiClient({
    authToken: "local-test-token",
    transport: transportFor(requests),
  });

  await client.listWorkspaceFiles("workspace.desktoplab");
  await client.previewWorkspaceFile("workspace.desktoplab", "src/main.rs");

  expect(requests.map((request) => `${request.method} ${request.path}`)).toEqual([
    "GET /v1/workspaces/workspace.desktoplab/files",
    "GET /v1/workspaces/workspace.desktoplab/files/preview?path=src%2Fmain.rs",
  ]);
});

test("maps terminal command method to local api path", async () => {
  const requests: TransportRequest[] = [];
  const client = new DesktopLabApiClient({
    authToken: "local-test-token",
    transport: transportFor(requests),
  });

  await client.createTerminalCommand("workspace.desktoplab", {
    command: "npm test",
    cwd: "apps/desktop",
  });
  await client.createTerminalCommand("workspace.desktoplab", {
    command: "printf terminal-ok",
    cwd: "",
    approvalId: "approval.terminal.1",
  });

  expect(requests.map((request) => `${request.method} ${request.path}`)).toEqual([
    "POST /v1/workspaces/workspace.desktoplab/terminal/commands",
    "POST /v1/workspaces/workspace.desktoplab/terminal/commands",
  ]);
  expect(requests[0].body).toEqual({ command: "npm test", cwd: "apps/desktop" });
  expect(requests[1].body).toEqual({ command: "printf terminal-ok", cwd: "", approvalId: "approval.terminal.1" });
});

test("terminal command response type separates approval and completed states", () => {
  const responses: TerminalCommandResponse[] = [
    {
      workspaceId: "workspace.desktoplab",
      terminalId: "terminal.local",
      state: "approval_required",
      command: "npm test",
      cwd: ".",
      approval: {
        approvalId: "approval.terminal.local",
        state: "pending",
        copy: "Terminal command `npm test` in `.` requires approval.",
      },
      events: [],
    },
    {
      workspaceId: "workspace.desktoplab",
      terminalId: "terminal.local",
      state: "completed",
      command: "printf ok",
      cwd: ".",
      approval: {
        approvalId: "approval.terminal.local",
        state: "approved",
        copy: "Approved",
      },
      events: [
        {
          eventId: "terminal.local.output",
          kind: "output",
          stdout: "ok",
          stderr: "",
          status: "exited",
          exitCode: 0,
          stdoutTruncated: false,
          redacted: false,
        },
      ],
    },
  ];

  expect(responses.map((response) => response.state)).toEqual(["approval_required", "completed"]);
});

test("workspace file preview type distinguishes text binary and denied states", () => {
  const previews: WorkspaceFilePreviewResponse[] = [
    {
      workspaceId: "workspace.desktoplab",
      path: "src/main.rs",
      state: "text",
      text: "fn main() {}",
      deniedReason: null,
      originalBytes: 12,
      originalLines: 1,
      returnedLines: 1,
      truncated: false,
    },
    {
      workspaceId: "workspace.desktoplab",
      path: "image.bin",
      state: "binary",
      text: null,
      deniedReason: null,
      originalBytes: 1024,
      originalLines: 0,
      returnedLines: 0,
      truncated: false,
    },
    {
      workspaceId: "workspace.desktoplab",
      path: ".env",
      state: "denied",
      text: null,
      deniedReason: "local_only_path",
      originalBytes: 0,
      originalLines: 0,
      returnedLines: 0,
      truncated: false,
    },
  ];

  expect(previews.map((preview) => preview.state)).toEqual(["text", "binary", "denied"]);
});

function transportFor(requests: TransportRequest[]): ApiTransport {
  return {
    async request(request) {
      requests.push(request);
      return { status: 200, body: {} };
    },
  };
}
