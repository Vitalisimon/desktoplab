import { DesktopLabApiClient } from "./client";
import { DesktopLabApiError } from "./types";
import type { ApiTransport, TransportRequest, TransportResponse } from "./transport";

test("allows unauthenticated local requests when the local server does not require a token", async () => {
  const requests: TransportRequest[] = [];
  const client = new DesktopLabApiClient({
    authToken: "",
    transport: {
      async request(request) {
        requests.push(request);
        return { status: 200, body: { status: "healthy" } };
      },
    },
  });

  await expect(client.health()).resolves.toEqual({ status: "healthy" });
  expect(requests[0].headers.authorization).toBeUndefined();
});

test("attaches bearer token and returns typed control plane responses", async () => {
  const requests: TransportRequest[] = [];
  const client = new DesktopLabApiClient({
    authToken: "local-test-token",
    transport: {
      async request(request) {
        requests.push(request);
        if (request.path === "/health") return { status: 200, body: { status: "healthy" } };
        if (request.path === "/v1/readiness") return { status: 200, body: { state: "ready" } };
        return {
          status: 200,
          body: { productVersion: "0.1.0", apiVersion: "v1" },
        };
      },
    },
  });

  await expect(client.health()).resolves.toEqual({ status: "healthy" });
  await expect(client.readiness()).resolves.toEqual({ state: "ready" });
  await expect(client.version()).resolves.toEqual({ productVersion: "0.1.0", apiVersion: "v1" });
  expect(requests.every((request) => request.headers.authorization === "Bearer local-test-token")).toBe(true);
});

test("opens a workspace through the local api boundary", async () => {
  const client = new DesktopLabApiClient({
    authToken: "local-test-token",
    transport: {
      async request(request) {
        expect(request.path).toBe("/v1/workspaces/open");
        expect(request.body).toEqual({ path: "/repo/desktoplab" });
        return {
          status: 200,
          body: {
            workspaceId: "workspace.desktoplab",
            displayName: "desktoplab",
            rootPath: "/repo/desktoplab",
            gitDirPath: "/repo/desktoplab/.git",
            apiState: "clean",
            statusEntries: [],
            diffText: "",
            checkpointStatus: "ready",
            canCheckpointRiskyExecution: true,
          },
        };
      },
    },
  });

  await expect(client.openWorkspace({ path: "/repo/desktoplab" })).resolves.toMatchObject({
    workspaceId: "workspace.desktoplab",
    apiState: "clean",
  });
});

test("relinks a read-only workspace through the local api boundary", async () => {
  const requests: TransportRequest[] = [];
  const client = new DesktopLabApiClient({
    authToken: "local-test-token",
    transport: {
      async request(request) {
        requests.push(request);
        return {
          status: 200,
          body: {
            workspaceId: "workspace.desktoplab",
            displayName: "desktoplab",
            rootPath: "/new/desktoplab",
            rootExists: true,
            readOnly: false,
            gitDirPath: "/new/desktoplab/.git",
            apiState: "clean",
            statusEntries: [],
            diffText: "",
            checkpointStatus: "ready",
            canCheckpointRiskyExecution: true,
          },
        };
      },
    },
  });

  await expect(
    client.relinkWorkspace("workspace.desktoplab", { path: "/new/desktoplab" }),
  ).resolves.toMatchObject({
    workspaceId: "workspace.desktoplab",
    rootPath: "/new/desktoplab",
    readOnly: false,
  });
  expect(requests[0]).toMatchObject({
    method: "POST",
    path: "/v1/workspaces/workspace.desktoplab/relink",
    body: { path: "/new/desktoplab" },
  });
});

test("lists jobs and retries retryable jobs through the local api boundary", async () => {
  const requests: TransportRequest[] = [];
  const client = new DesktopLabApiClient({
    authToken: "local-test-token",
    transport: {
      async request(request) {
        requests.push(request);
        if (request.path === "/v1/jobs") {
          return {
            status: 200,
            body: {
              jobs: [
                {
                  jobId: "job.1",
                  kind: "model.download",
                  state: "failed",
                  progressPercent: 48,
                  retryClass: "retryable",
                  updatedAt: "2026-06-25T19:55:00Z",
                  failureReason: "network interrupted",
                },
              ],
            },
          };
        }
        return { status: 200, body: { accepted: true } };
      },
    },
  });

  await expect(client.listJobs()).resolves.toMatchObject({ jobs: [{ jobId: "job.1", retryClass: "retryable" }] });
  await expect(client.retryJob("job.1")).resolves.toEqual({ accepted: true });
  expect(requests.map((request) => `${request.method} ${request.path}`)).toEqual([
    "GET /v1/jobs",
    "POST /v1/jobs/job.1/retry",
  ]);
});

test("creates and lists agent sessions through the local api boundary", async () => {
  const requests: TransportRequest[] = [];
  const client = new DesktopLabApiClient({
    authToken: "local-test-token",
    transport: {
      async request(request) {
        requests.push(request);
        if (request.method === "GET" && request.path.startsWith("/v1/sessions?")) {
          return {
            status: 200,
            body: {
              sessions: [
                {
                  sessionId: "session.1",
                  workspaceId: "workspace.desktoplab",
                  executionBackendId: "backend.ollama",
                  owner: "desktoplab",
                  state: "planning",
                  plan: "Inspect repository",
                  checkpoints: [],
                  summary: null,
                  timeline: [],
                },
              ],
            },
          };
        }
        return {
          status: 200,
          body: {
            sessionId: "session.2",
            workspaceId: "workspace.desktoplab",
            executionBackendId: "backend.ollama",
            owner: "desktoplab",
            state: "created",
            plan: null,
            checkpoints: [],
            summary: null,
            timeline: [],
          },
        };
      },
    },
  });

  await expect(client.listSessions("workspace.desktoplab")).resolves.toMatchObject({
    sessions: [{ sessionId: "session.1", state: "planning" }],
  });
  await expect(
    client.createSession({
      workspaceId: "workspace.desktoplab",
      executionBackendId: "backend.ollama",
      initialPrompt: "Inspect the repository",
    }),
  ).resolves.toMatchObject({ sessionId: "session.2", owner: "desktoplab" });

  expect(requests[0].path).toBe("/v1/sessions?workspace_id=workspace.desktoplab");
  expect(requests[1].body).toEqual({
    workspaceId: "workspace.desktoplab",
    executionBackendId: "backend.ollama",
    initialPrompt: "Inspect the repository",
  });
});

test("lists and resolves approvals through the local api boundary", async () => {
  const requests: TransportRequest[] = [];
  const client = new DesktopLabApiClient({
    authToken: "local-test-token",
    transport: {
      async request(request) {
        requests.push(request);
        if (request.method === "POST" && request.path === "/v1/approvals") {
          return {
            status: 200,
            body: {
              approvalId: "approval.1",
              sessionId: "session.1",
              action: "terminal.command",
              operationId: "terminal.local",
              state: "pending",
            },
          };
        }
        if (request.method === "GET" && request.path === "/v1/approvals") {
          return {
            status: 200,
            body: {
              approvals: [
                {
                  approvalId: "approval.1",
                  sessionId: "session.1",
                  action: "filesystem.write",
                  state: "pending",
                  risk: "medium",
                  title: "Review file change",
                  message: "The agent wants to edit files in the active repository.",
                  requestedAt: "2026-06-25T20:30:00Z",
                  policyReason: "Filesystem writes need confirmation.",
                },
              ],
            },
          };
        }
        return {
          status: 200,
          body: {
            approvalId: "approval.1",
            state: "approved",
          },
        };
      },
    },
  });

  await expect(
    client.createApproval({
      sessionId: "session.1",
      action: "terminal.command",
      operationId: "terminal.local",
    }),
  ).resolves.toMatchObject({
    approvalId: "approval.1",
    operationId: "terminal.local",
    state: "pending",
  });
  await expect(client.listApprovals()).resolves.toMatchObject({
    approvals: [{ approvalId: "approval.1", state: "pending", title: "Review file change" }],
  });
  await expect(client.resolveApproval("approval.1", { resolution: "approve" })).resolves.toEqual({
    approvalId: "approval.1",
    state: "approved",
  });

  expect(requests.map((request) => `${request.method} ${request.path}`)).toEqual([
    "POST /v1/approvals",
    "GET /v1/approvals",
    "POST /v1/approvals/approval.1/resolve",
  ]);
  expect(requests[0].body).toEqual({
    sessionId: "session.1",
    action: "terminal.command",
    operationId: "terminal.local",
  });
  expect(requests[2].body).toEqual({ resolution: "approve" });
});

test("reads and updates approval modes through the local api boundary", async () => {
  const requests: TransportRequest[] = [];
  const client = new DesktopLabApiClient({
    authToken: "local-test-token",
    transport: {
      async request(request) {
        requests.push(request);
        return {
          status: 200,
          body: {
            modes: [
              {
                mode: "require_approval",
                label: "Ask for approval",
                description: "DesktopLab asks before writes and commands.",
              },
              {
                mode: "approve_for_me",
                label: "Approve routine actions",
                description: "DesktopLab can approve low-risk routine actions.",
              },
              {
                mode: "approve_workspace_writes_for_session",
                label: "Allow workspace writes",
                description: "DesktopLab can approve workspace file writes for this session.",
              },
              {
                mode: "full_access",
                label: "Full local access",
                description: "DesktopLab can work with fewer prompts while hard blocks remain.",
              },
            ],
            defaultMode: "require_approval",
            sessionMode: "approve_for_me",
          },
        };
      },
    },
  });

  await expect(client.approvalModes()).resolves.toMatchObject({
    defaultMode: "require_approval",
    sessionMode: "approve_for_me",
    modes: [
      { mode: "require_approval" },
      { mode: "approve_for_me" },
      { mode: "approve_workspace_writes_for_session" },
      { mode: "full_access" },
    ],
  });
  await client.updateDefaultApprovalMode({ mode: "full_access" });
  await client.updateSessionApprovalMode({ mode: "approve_for_me" });

  expect(requests.map((request) => `${request.method} ${request.path}`)).toEqual([
    "GET /v1/approval-modes",
    "POST /v1/approval-modes/default",
    "POST /v1/approval-modes/session",
  ]);
  expect(requests[1].body).toEqual({ mode: "full_access" });
  expect(requests[2].body).toEqual({ mode: "approve_for_me" });
});

function responding(response: TransportResponse): ApiTransport {
  return {
    async request() {
      return response;
    },
  };
}
