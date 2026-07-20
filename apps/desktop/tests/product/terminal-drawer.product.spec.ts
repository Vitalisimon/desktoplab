import { expect, test } from "@playwright/test";
import path from "node:path";
import { desktopOnly, localApi, openWorkspaceThroughUi } from "./auditHelpers";

test("24.5 product: terminal drawer replays explicitly requested approval state", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);
  const workspaceRoot = await openWorkspaceThroughUi(page, request);
  const workspaceId = `workspace.${path.basename(workspaceRoot)}`;

  await localApi(request, "POST", `/v1/workspaces/${workspaceId}/terminal/commands`, {
    command: "printf terminal-proof",
    cwd: ".",
    approvalRequired: true,
  });

  await expect(page.getByRole("complementary", { name: "Terminal" })).toHaveCount(0);
  await page.getByRole("button", { name: "Show terminal" }).click();

  await expect(page.getByRole("complementary", { name: "Terminal" })).toBeVisible();
  await expect(page.getByText("Terminal command `printf terminal-proof` in `.` requires approval.")).toBeVisible();
  await expect(page.getByText(/% printf terminal-proof/)).toBeVisible();
  await expect(page.getByRole("button", { name: "Approve command" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Deny command" })).toBeVisible();
});

test("24.6 product: user typed terminal commands execute without approval prompts", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);
  await openWorkspaceThroughUi(page, request);

  await page.getByRole("button", { name: "Show terminal" }).click();
  const terminalInput = page.getByRole("textbox", { name: "Terminal input" });
  await terminalInput.fill("printf user-terminal-proof");
  await terminalInput.press("Enter");

  await expect(page.getByText("user-terminal-proof", { exact: true })).toBeVisible();
  await expect(page.getByText(/requires approval/i)).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Approve command" })).toHaveCount(0);
});
