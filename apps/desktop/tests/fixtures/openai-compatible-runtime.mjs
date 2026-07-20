import { createServer } from "node:http";

const port = Number(process.env.DESKTOPLAB_FRONTIER_FIXTURE_PORT ?? 18000);

createServer((request, response) => {
  if (request.method === "GET" && request.url === "/v1/models") {
    return json(response, 200, { data: [{ id: "frontier-test-model-600b" }] });
  }
  if (request.method === "GET" && request.url === "/health") {
    return json(response, 200, { tokenizerReady: true, gpuMemoryPressurePercent: 72, queueDepth: 0 });
  }
  if (request.method === "POST" && request.url === "/v1/chat/completions") {
    return json(response, 200, { choices: [{ message: { content: "Fixture response", tool_calls: [] } }] });
  }
  return json(response, 404, { error: "not_found" });
}).listen(port, "127.0.0.1");

function json(response, status, body) {
  const payload = JSON.stringify(body);
  response.writeHead(status, {
    "content-type": "application/json",
    "content-length": Buffer.byteLength(payload),
  });
  response.end(payload);
}
