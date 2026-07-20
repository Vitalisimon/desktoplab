import { expect, test } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { markSetupReady, resetProductState } from "./product/auditHelpers";

const apiBase = "http://127.0.0.1:1421";

test("fresh desktop shell reaches first workbench through live backend state", async ({ page, request }, testInfo) => {
  test.skip(testInfo.project.name !== "desktop", "fresh product flow mutates shared backend state");
  const repoRoot = createWorkspaceFixture();
  await resetProductState(request);

  await page.goto("/", { waitUntil: "domcontentloaded" });

  await expect(page.getByRole("heading", { name: "Setup" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Agent" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Open Repository" })).toHaveCount(0);

  const setup = await request.get(`${apiBase}/v1/setup/preview`);
  expect(setup.status()).toBe(200);
  const preview = await setup.json();
  expect(preview.registryState).toBe("ready");

  await markSetupReady(request);

  await page.reload();
  await expect(page.getByRole("heading", { name: "Open a project folder" })).toBeVisible();
  await page.getByLabel("Repository path").fill(repoRoot);
  await page.getByRole("button", { name: "Open Repository" }).click();
  await expect(page.getByRole("heading", { name: "Agent" })).toBeVisible();

  await page.getByRole("textbox", { name: "Prompt" }).fill("Inspect this repository and propose the first verification step");
  await page.getByRole("button", { name: "Send prompt" }).click();
  await expect(page.getByText("Inspect this repository and propose the first verification step").first()).toBeVisible();

  const state = await (await request.get(`${apiBase}/v1/app/state`)).json();
  const workspaceId = state.currentWorkspace.workspaceId;
  const sessions = await (await request.get(`${apiBase}/v1/sessions?workspace_id=${workspaceId}`)).json();
  expect(sessions.sessions.length).toBeGreaterThan(0);
  expect(sessions.sessions[0].timeline.length).toBeGreaterThan(1);
  const files = await (await request.get(`${apiBase}/v1/workspaces/${workspaceId}/files`)).json();
  expect(files.entries.length).toBeGreaterThan(0);

  await page.reload();
  await expect(page.getByRole("heading", { name: "Agent" })).toBeVisible();
  await expect(page.getByText("Inspect this repository and propose the first verification step").first()).toBeVisible();
  const resumed = await (await request.get(`${apiBase}/v1/app/state`)).json();
  expect(resumed.setup.state).toBe("ready");
  expect(resumed.currentWorkspace.workspaceId).toBe(workspaceId);

  const terminal = await request.post(`${apiBase}/v1/workspaces/${workspaceId}/terminal/commands`, {
    data: { command: "printf fresh-flow", cwd: "." },
  });
  const terminalBody = await terminal.json();
  expect(terminalBody.state).toBe("completed");

  await expect(page.getByText("Projects")).toBeVisible();
  await expect(page.getByText("Scheduled")).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Background work" })).toHaveCount(0);
});

function createWorkspaceFixture() {
  const root = mkdtempSync(path.join(tmpdir(), "desktoplab-fresh-flow-"));
  writeFileSync(path.join(root, "AGENTS.md"), "# DesktopLab fresh flow\n\nReal git fixture for first prompt proof.\n");
  writeFileSync(path.join(root, "package.json"), JSON.stringify({ name: "desktoplab-fresh-flow", private: true }, null, 2));
  execFileSync("git", ["init"], { cwd: root, stdio: "ignore" });
  return root;
}
