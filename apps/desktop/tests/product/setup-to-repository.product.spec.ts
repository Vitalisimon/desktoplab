import { expect, test } from "@playwright/test";
import { desktopOnly, localApi, createWorkspaceFixture, markSetupReady } from "./auditHelpers";

test("24.5 product: setup ready unlocks repository open and primary workbench", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);
  const workspaceRoot = createWorkspaceFixture("desktoplab-setup-to-repo-");

  await markSetupReady(request);
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Open a project folder" })).toBeVisible();

  await page.getByLabel("Repository path").fill(workspaceRoot);
  await page.getByRole("button", { name: "Open Repository" }).click();
  await expect(page.getByRole("heading", { name: "Agent" })).toBeVisible();

  const state = await localApi(request, "GET", "/v1/app/state");
  expect(state.setup.state).toBe("ready");
  expect(state.setupPipeline.state).toBe("ready");
  expect(state.currentWorkspace.rootPath).toBe(workspaceRoot);
  expect(state.currentWorkspace.workspaceId).toBeTruthy();

  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Agent" })).toBeVisible();
});
