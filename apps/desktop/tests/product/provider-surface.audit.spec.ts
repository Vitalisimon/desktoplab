import { expect, test } from "@playwright/test";
import { auditScreenshotPath, auditThemes, desktopOnly, localApi, setAuditTheme } from "./auditHelpers";

for (const theme of auditThemes) {
  test(`24.6 audit: provider surface exposes backend-owned account modes in ${theme} theme`, async ({ page, request }, testInfo) => {
    desktopOnly(testInfo);
    await setAuditTheme(page, theme);
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    const providers = await localApi(request, "GET", "/v1/providers");
    expect(providers.source).toBe("service_backed");
    expect(providers.providers[0].supportedAccountModes).toContain("api_key_billing");

    await page.getByText("Control center").click();
    await page.getByRole("button", { name: "Settings" }).click();
    await page.getByRole("button", { name: "Providers" }).click();
    await expect(page.getByRole("heading", { name: "Accounts" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Connect account" })).toBeVisible();
    await expect(page.getByText(providers.providers[0].displayName).first()).toBeVisible();
    await page.screenshot({ path: auditScreenshotPath(theme, "provider-surface.png"), fullPage: true });
  });
}
