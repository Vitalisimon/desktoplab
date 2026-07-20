import { expect, test } from "@playwright/test";
import { writeFileSync } from "node:fs";
import path from "node:path";
import { desktopOnly, openWorkspaceThroughUi } from "./auditHelpers";

test("24.5 product: file drawer reads backend tree, previews files and blocks protected files", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);
  const workspaceRoot = await openWorkspaceThroughUi(page, request);
  writeFileSync(path.join(workspaceRoot, ".env"), "SECRET_TOKEN=not-for-ui\n");

  await expect(page.getByRole("complementary", { name: "Repository inspector" })).toHaveCount(0);

  await page.getByRole("button", { name: "Show inspector" }).click();
  await expect(page.getByRole("complementary", { name: "Repository inspector" })).toBeVisible();
  await expect(page.getByRole("button", { name: "AGENTS.md" })).toBeVisible();

  await page.getByRole("button", { name: "AGENTS.md" }).click();
  await expect(page.getByText("Workspace fixture for product truth audit.")).toBeVisible();

  await page.getByRole("button", { name: "Show repository tree" }).click();
  await expect(page.getByText(".env")).toBeVisible();
  await expect(page.getByText("Protected").first()).toBeVisible();
  await expect(page.getByRole("button", { name: /\\.env/ })).toHaveCount(0);
});
