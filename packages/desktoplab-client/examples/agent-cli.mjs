#!/usr/bin/env node
import { DesktopLabAgentClient, FetchTransport } from "../src/index.mjs";

const [baseUrl, token, workspaceId, executionBackendId, ...promptParts] = process.argv.slice(2);
if (!baseUrl || !token || !workspaceId || !executionBackendId || promptParts.length === 0) {
  console.error("Usage: agent-cli <base-url> <token> <workspace-id> <backend-id> <prompt>");
  process.exit(2);
}
const client = new DesktopLabAgentClient({ authToken: token, transport: new FetchTransport(baseUrl) });
const started = await client.run({ workspaceId, executionBackendId, prompt: promptParts.join(" ") });
const result = started.state === "running"
  ? await client.wait({ sessionId: started.sessionId, workspaceId })
  : started;
process.stdout.write(`${JSON.stringify(result)}\n`);
