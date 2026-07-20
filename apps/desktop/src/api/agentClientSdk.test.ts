import { describe, expect, it } from "vitest";

import { DesktopLabApiClient } from "./client";
import type { ApiTransport, TransportRequest } from "./transport";

describe("agent client SDK integration", () => {
  it("routes UI session creation, reuse and cancellation through the stable SDK contract", async () => {
    const requests: TransportRequest[] = [];
    const transport: ApiTransport = { request: async (request) => {
      requests.push(request);
      return { status: 200, body: { sessionId: "session.1", workspaceId: "workspace.1", executionBackendId: "backend.ollama", owner: "desktoplab", state: "completed" } };
    }};
    const client = new DesktopLabApiClient({ authToken: "token", transport });
    await client.createSession({ workspaceId: "workspace.1", executionBackendId: "backend.ollama", initialPrompt: "inspect" });
    await client.continueSession("session.1", { workspaceId: "workspace.1", executionBackendId: "backend.ollama", prompt: "continue" });
    await client.sessionControl("session.1", { action: "cancel" });
    expect(requests.map((request) => request.path)).toEqual([
      "/v1/sessions",
      "/v1/sessions/session.1/messages",
      "/v1/sessions/session.1/control",
    ]);
  });
});
