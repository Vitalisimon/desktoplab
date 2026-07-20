import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { expect, test, type APIRequestContext } from "@playwright/test";
import { markSetupReady, resetProductState } from "./product/auditHelpers";

const artifactDir = "test-artifacts/frontend";

test.beforeEach(() => {
  mkdirSync(artifactDir, { recursive: true });
});

test("desktop shell exercises critical product routes through the real local api", async ({ page, request }, testInfo) => {
  test.skip(testInfo.project.name !== "desktop", "critical local API product smoke mutates shared backend state");

  await page.goto("/", { waitUntil: "domcontentloaded" });
  await resetProductState(request);

  await expect(page.getByTestId("desktoplab-root")).toBeVisible();
  const workspaceRoot = createWorkspaceFixture();

  const setup = await localApi(request, "GET", "/v1/setup/preview");
  expect(setup.registryState).toBe("ready");
  expect(setup.runtimeRecommendations[0].manifestId).toBe("runtime.ollama");
  expect(setup.modelRecommendations[0].manifestId).toMatch(/^model\./);

  const rejectedWorkspace = await localApi(request, "POST", "/v1/workspaces/open", { path: workspaceRoot }, 400);
  expect(rejectedWorkspace.blockedReason).toBe("setup_not_ready");

  const selection = await markSetupReady(request);

  const workspace = await localApi(request, "POST", "/v1/workspaces/open", { path: workspaceRoot });
  expect(workspace.workspaceId).toContain("workspace.desktoplab-smoke-");

  const agent = await localApi(request, "GET", `/v1/agent/workspace?workspace_id=${workspace.workspaceId}`);
  expect(agent.route.source).toBe("service_backed");
  expect(["selected", "blocked"]).toContain(agent.route.status);

  const files = await localApi(request, "GET", `/v1/workspaces/${workspace.workspaceId}/files`);
  expect(files.entries.length).toBeGreaterThan(0);
  const preview = await localApi(
    request,
    "GET",
    `/v1/workspaces/${workspace.workspaceId}/files/preview?path=${encodeURIComponent("AGENTS.md")}`,
  );
  expect(preview.state).toBe("text");
  expect(preview.text).toContain("DesktopLab");

  const terminal = await localApi(request, "POST", `/v1/workspaces/${workspace.workspaceId}/terminal/commands`, {
    command: "printf local-api-product",
    cwd: ".",
  });
  expect(terminal.state).toBe("completed");
  expect(terminal.events[0].stdout).toContain("local-api-product");

  const runtime = await localApi(request, "POST", "/v1/runtimes/runtime.ollama/install", {
    setupAccepted: true,
    networkAvailable: true,
    diskAvailableGb: 64,
  });
  expect(["running", "completed", "blocked", "external_guided", "failed"]).toContain(runtime.state);

  const model = await localApi(request, "POST", `/v1/models/${selection.modelId}/download`, {
    setupAccepted: true,
    networkAvailable: true,
    diskAvailableMb: 100_000,
  });
  expect(["running", "ready", "completed", "blocked"]).toContain(model.state);

  await page.screenshot({ path: `${artifactDir}/local-api-product-${testInfo.project.name}.png`, fullPage: true });
});

function createWorkspaceFixture() {
  const root = mkdtempSync(path.join(tmpdir(), "desktoplab-smoke-"));
  writeFileSync(path.join(root, "AGENTS.md"), "# DesktopLab smoke workspace\n\nDesktopLab real local API fixture.\n");
  writeFileSync(path.join(root, "package.json"), JSON.stringify({ name: "desktoplab-smoke", private: true }, null, 2));
  execFileSync("git", ["init"], { cwd: root, stdio: "ignore" });
  return root;
}

async function localApi(
  request: APIRequestContext,
  method: "GET" | "POST",
  path: string,
  body?: unknown,
  expectedStatus = 200,
) {
  const response = await request.fetch(`http://127.0.0.1:1421${path}`, {
    method,
    data: body,
  });
  expect(response.status(), `${method} ${path} status`).toBe(expectedStatus);
  return response.json();
}
