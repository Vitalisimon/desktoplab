import { expect, test } from "@playwright/test";
import path from "node:path";
import { desktopOnly, localApi, openWorkspaceThroughUi } from "./auditHelpers";

test("24.5 product: backend approval appears in UI and deny preserves blocked state", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);
  const workspaceRoot = await openWorkspaceThroughUi(page, request);
  const workspaceId = `workspace.${path.basename(workspaceRoot)}`;
  await localApi(request, "POST", `/v1/workspaces/${workspaceId}/terminal/commands`, {
    command: "printf approval-proof",
    cwd: ".",
    approvalRequired: true,
  });

  await page.getByRole("button", { name: "Show terminal" }).click();
  await expect(page.getByRole("complementary", { name: "Terminal" })).toBeVisible();
  await expect(page.getByText("Terminal command `printf approval-proof` in `.` requires approval.")).toBeVisible();

  await page.getByRole("button", { name: "Deny command" }).click();

  await expect(page.getByText("Denied", { exact: true })).toBeVisible();
  const approvals = await localApi(request, "GET", "/v1/approvals");
  expect(approvals.approvals).toContainEqual(expect.objectContaining({ action: "terminal.command", state: "denied" }));
});
