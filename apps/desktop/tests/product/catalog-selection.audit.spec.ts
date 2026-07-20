import { expect, test } from "@playwright/test";
import { auditScreenshotPath, auditThemes, desktopOnly, localApi, resetProductState, setAuditTheme } from "./auditHelpers";

for (const theme of auditThemes) {
  test(`24.6 audit: catalog recommendations are live in ${theme} theme`, async ({ page, request }, testInfo) => {
    desktopOnly(testInfo);
    await resetProductState(request);
    await setAuditTheme(page, theme);
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    const preview = await localApi(request, "GET", "/v1/setup/preview");
    expect(preview.modelRecommendations.length).toBeGreaterThan(0);
    await expect(page.getByText("Recommended setup")).toBeVisible();
    await expect(page.getByText(preview.modelRecommendations[0].displayName).first()).toBeVisible();
    await page.screenshot({ path: auditScreenshotPath(theme, "catalog-selection.png"), fullPage: true });
  });
}
