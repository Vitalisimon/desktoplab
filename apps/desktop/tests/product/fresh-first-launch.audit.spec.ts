import { expect, test } from "@playwright/test";
import { auditScreenshotPath, auditThemes, desktopOnly, setAuditTheme } from "./auditHelpers";

for (const theme of auditThemes) {
  test(`24.6 audit: fresh launch enters setup in ${theme} theme`, async ({ page }, testInfo) => {
    desktopOnly(testInfo);
    await setAuditTheme(page, theme);
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await expect(page.getByRole("heading", { name: "Setup" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Agent" })).toHaveCount(0);
    await page.screenshot({ path: auditScreenshotPath(theme, "fresh-first-launch.png"), fullPage: true });
  });
}
