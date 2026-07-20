import { expect, test } from "@playwright/test";
import { auditScreenshotPath, auditThemes, desktopOnly, openWorkspaceThroughUi, setAuditTheme } from "./auditHelpers";

for (const theme of auditThemes) {
  test(`24.6 audit: terminal drawer opens as backend-owned surface in ${theme} theme`, async ({ page, request }, testInfo) => {
    desktopOnly(testInfo);
    await setAuditTheme(page, theme);
    await openWorkspaceThroughUi(page, request);
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await expect(page.getByRole("complementary", { name: "Terminal" })).toHaveCount(0);
    await page.getByRole("button", { name: "Show terminal" }).click();
    await expect(page.getByRole("complementary", { name: "Terminal" })).toBeVisible();
    await expect(page.getByRole("textbox", { name: "Terminal input" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Copy terminal output" })).toHaveCount(0);
    await page.screenshot({ path: auditScreenshotPath(theme, "terminal-drawer.png"), fullPage: true });
  });
}
