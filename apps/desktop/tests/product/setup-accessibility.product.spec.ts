import { expect, test } from "@playwright/test";
import { desktopOnly, localApi, openWorkspaceThroughUi, resetProductState, selectSetup } from "./auditHelpers";

test("24.5 product: setup primary flow has accessible actions and no raw backend codes", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);
  await resetProductState(request);

  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Setup" })).toBeVisible();
  await expect(page.getByRole("button", { name: /start setup/i })).toBeVisible();
  await expect(page.getByTestId("desktoplab-root").getByRole("button", { name: "Open Repository" })).toBeDisabled();
  await expect(page.getByText(/runtime_not_ready|runtime_not_verified|model_not_verified|backend_readiness_not_verified/)).toHaveCount(0);

  await selectSetup(request);
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByText("Setup progress")).toBeVisible();
  await expect(page.getByText("Runtime install")).toBeVisible();
  await expect(page.getByText(/runtime_not_ready|runtime_not_verified|non_retryable|user_action/)).toHaveCount(0);

  await localApi(request, "POST", "/v1/setup/complete", {});
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByText("Setup needs attention before coding.")).toBeVisible();
  await expect(page.getByText("Verify the local runner and model before opening a repository.").first()).toBeVisible();
  await expect(page.getByTestId("desktoplab-root").getByRole("button", { name: "Open Repository" })).toBeDisabled();
});

test("24.6 product: workbench controls are keyboard reachable and close transient drawers with escape", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);

  await openWorkspaceThroughUi(page, request);
  await page.keyboard.press("Tab");
  await expect(page.getByRole("button", { name: "Collapse left drawer" })).toBeFocused();
  await expect(page.getByRole("button", { name: "Collapse left drawer" })).toHaveCSS("outline-style", "solid");

  await page.getByRole("button", { name: "Show terminal" }).focus();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("complementary", { name: "Terminal" })).toBeVisible();

  await page.getByRole("button", { name: "Show inspector" }).focus();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("complementary", { name: "Repository inspector" })).toBeVisible();
  await expect(page.getByRole("separator", { name: "Resize right drawer" })).toBeVisible();

  const firstFile = page.getByRole("button", { name: "AGENTS.md" });
  await firstFile.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByText("# DesktopLab 24.5 audit")).toBeVisible();

  await page.getByRole("button", { name: "Show repository tree" }).focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("repository-tree-subdrawer")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("repository-tree-subdrawer")).toBeHidden();
  await expect(page.getByRole("complementary", { name: "Repository inspector" })).toBeVisible();
});
