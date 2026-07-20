export const SDK_VERSION = "1";

const terminalStates = new Set(["completed", "failed", "cancelled", "blocked"]);

export class DesktopLabAgentClient {
  #authToken;
  #transport;

  constructor({ authToken, transport }) {
    if (!transport || typeof transport.request !== "function") throw new TypeError("transport.request is required");
    this.#authToken = authToken;
    this.#transport = transport;
  }

  async run(request) {
    const body = request.sessionId
      ? {
          workspaceId: request.workspaceId,
          executionBackendId: request.executionBackendId,
          prompt: request.prompt,
          ...(request.contextPaths ? { contextPaths: request.contextPaths } : {}),
          ...(request.externalAttachments ? { externalAttachments: request.externalAttachments } : {}),
          ...(request.approvalId ? { approvalId: request.approvalId } : {}),
        }
      : {
          workspaceId: request.workspaceId,
          executionBackendId: request.executionBackendId,
          initialPrompt: request.prompt,
          ...(request.contextPaths ? { contextPaths: request.contextPaths } : {}),
          ...(request.externalAttachments ? { externalAttachments: request.externalAttachments } : {}),
          ...(request.approvalId ? { approvalId: request.approvalId } : {}),
          ...(request.newChat ? { newChat: true } : {}),
        };
    const path = request.sessionId
      ? `/v1/sessions/${encodeURIComponent(request.sessionId)}/messages`
      : "/v1/sessions";
    return resultEnvelope(await this.#request("POST", path, body));
  }

  async stream(request) {
    if (request.sessionId) return this.run(request);
    const response = await this.#request("POST", "/v1/sessions", {
      workspaceId: request.workspaceId,
      executionBackendId: request.executionBackendId,
      initialPrompt: request.prompt,
      ...(request.contextPaths ? { contextPaths: request.contextPaths } : {}),
      ...(request.externalAttachments ? { externalAttachments: request.externalAttachments } : {}),
      ...(request.approvalId ? { approvalId: request.approvalId } : {}),
      stream: true,
    });
    return resultEnvelope(response);
  }

  async wait({ sessionId, workspaceId, timeoutMs = 30_000, pollIntervalMs = 250 }) {
    const deadline = Date.now() + timeoutMs;
    while (Date.now() <= deadline) {
      const response = await this.#request("GET", `/v1/sessions?workspace_id=${encodeURIComponent(workspaceId)}`);
      const session = response.sessions?.find((candidate) => candidate.sessionId === sessionId);
      if (!session) throw new Error(`session not found: ${sessionId}`);
      if (terminalStates.has(session.state)) return resultEnvelope(session);
      await delay(pollIntervalMs);
    }
    throw new Error(`wait timed out for ${sessionId}`);
  }

  async cancel(sessionId) {
    return resultEnvelope(await this.#request("POST", `/v1/sessions/${encodeURIComponent(sessionId)}/control`, { action: "cancel" }));
  }

  async modelStatus() {
    const status = await this.#request("GET", "/v1/runtime/inspect");
    return { kind: "desktoplab.model-status", schemaVersion: 1, ...status };
  }

  async #request(method, path, body) {
    const response = await this.#transport.request({
      method,
      path,
      headers: { authorization: `Bearer ${this.#authToken}` },
      ...(body === undefined ? {} : { body }),
    });
    if (!response || !Number.isInteger(response.status)) throw new Error("transport response invalid");
    if (response.status < 200 || response.status >= 300) {
      const message = response.body?.message ?? `DesktopLab request failed with ${response.status}`;
      throw new Error(message);
    }
    return response.body ?? {};
  }
}

export class FetchTransport {
  constructor(baseUrl) { this.baseUrl = baseUrl.replace(/\/$/, ""); }
  async request(request) {
    const response = await fetch(`${this.baseUrl}${request.path}`, {
      method: request.method,
      headers: { "content-type": "application/json", ...request.headers },
      body: request.body === undefined ? undefined : JSON.stringify(request.body),
    });
    const text = await response.text();
    return { status: response.status, body: text ? JSON.parse(text) : {} };
  }
}

function resultEnvelope(session) {
  return {
    kind: "desktoplab.agent-run-result",
    schemaVersion: 1,
    sessionId: session.sessionId ?? null,
    workspaceId: session.workspaceId ?? null,
    state: session.state ?? "unknown",
    session,
  };
}

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
