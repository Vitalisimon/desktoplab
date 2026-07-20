import { expect, test } from "@playwright/test";
import { desktopOnly, openWorkspaceThroughUi } from "./auditHelpers";

test("24.5 product: workbench empty states stay conversation-first and setup blocks clearly", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);

  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Setup" })).toBeVisible();
  await expect(page.getByText(/driver_probe_|gpu_probe_|vram_probe_/)).toHaveCount(0);

  await openWorkspaceThroughUi(page, request);
  await expect(page.getByRole("heading", { name: "Agent" })).toBeVisible();
  await expect(page.getByText("Ask DesktopLab what to change, inspect, or verify in this repository.")).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Prompt" })).toBeVisible();
  await expect(page.getByText("Local runner")).toHaveCount(0);
  await expect(page.getByText("Repository context")).toHaveCount(0);
  await expect(page.getByRole("complementary", { name: "Repository inspector" })).toHaveCount(0);
  await expect(page.getByRole("complementary", { name: "Terminal" })).toHaveCount(0);
});
