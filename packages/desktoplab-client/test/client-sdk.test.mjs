import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import test from "node:test";

import { DesktopLabAgentClient, SDK_VERSION } from "../src/index.mjs";
import { InMemoryTransport } from "../src/testing.mjs";

test("run reuse cancel wait and model status share one transport contract", async () => {
  const sessions = [{ sessionId: "session.1", workspaceId: "workspace.1", state: "completed" }];
  const transport = new InMemoryTransport({ testOnly: true, responder: (request) => {
    if (request.path === "/v1/sessions") return { status: 200, body: sessions[0] };
    if (request.path.endsWith("/messages")) return { status: 200, body: { ...sessions[0], summary: request.body.prompt } };
    if (request.path.endsWith("/control")) return { status: 200, body: { ...sessions[0], state: "cancelled" } };
    if (request.path.startsWith("/v1/sessions?")) return { status: 200, body: { sessions } };
    if (request.path === "/v1/runtime/inspect") return { status: 200, body: { inspectState: "ready" } };
    return { status: 404, body: { message: "missing" } };
  }});
  const client = new DesktopLabAgentClient({ authToken: "local", transport });
  assert.equal(SDK_VERSION, "1");
  assert.equal((await client.run(runRequest())).sessionId, "session.1");
  assert.equal((await client.run({ ...runRequest(), sessionId: "session.1", prompt: "continue" })).session.summary, "continue");
  assert.equal((await client.wait({ sessionId: "session.1", workspaceId: "workspace.1" })).state, "completed");
  assert.equal((await client.cancel("session.1")).state, "cancelled");
  assert.equal((await client.modelStatus()).inspectState, "ready");
});

test("in-memory transport cannot be constructed as production routing", () => {
  assert.throws(() => new InMemoryTransport({ responder: () => ({ status: 200, body: {} }) }), /test routing/);
  const desktopSources = collectFiles(new URL("../../../apps/desktop/src", import.meta.url).pathname);
  assert.ok(desktopSources.every((file) => !readFileSync(file, "utf8").includes("@desktoplab/client-sdk/testing")));
});

test("run and stream preserve attachments and payload-bound egress approval", async () => {
  const requests = [];
  const transport = new InMemoryTransport({ testOnly: true, responder: (request) => {
    requests.push(request);
    return { status: 200, body: { sessionId: "session.1", workspaceId: "workspace.1", state: "completed" } };
  }});
  const client = new DesktopLabAgentClient({ authToken: "local", transport });
  const externalAttachments = [{ name: "brief.txt", size: 5, mediaType: "text/plain", contentText: "notes" }];
  const request = { ...runRequest(), contextPaths: ["README.md"], externalAttachments, approvalId: "approval.egress.1" };

  await client.run(request);
  await client.run({ ...request, sessionId: "session.1" });
  await client.stream(request);

  for (const { body } of requests) {
    assert.deepEqual(body.contextPaths, ["README.md"]);
    assert.deepEqual(body.externalAttachments, externalAttachments);
    assert.equal(body.approvalId, "approval.egress.1");
  }
});

test("package exports only stable main and explicit testing surfaces", async () => {
  const manifest = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
  assert.deepEqual(Object.keys(manifest.exports), [".", "./testing"]);
  assert.deepEqual(Object.keys(await import("../src/index.mjs")).sort(), ["DesktopLabAgentClient", "FetchTransport", "SDK_VERSION"]);
  for (const [path, limit] of [["src/index.mjs", 180], ["test/client-sdk.test.mjs", 120]]) {
    const logical = readFileSync(new URL(`../${path}`, import.meta.url), "utf8").split("\n").filter((line) => line.trim()).length;
    assert.ok(logical <= limit, `${path} has ${logical} logical lines`);
  }
});

function runRequest() {
  return { workspaceId: "workspace.1", executionBackendId: "backend.ollama", prompt: "inspect" };
}

function collectFiles(root) {
  return readdirSync(root, { withFileTypes: true }).flatMap((entry) => {
    const child = path.join(root, entry.name);
    if (entry.isDirectory()) return collectFiles(child);
    return /\.(ts|tsx)$/.test(entry.name) ? [child] : [];
  });
}
