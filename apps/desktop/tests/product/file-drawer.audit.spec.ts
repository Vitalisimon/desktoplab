import { expect, test } from "@playwright/test";
import { auditScreenshotPath, auditThemes, desktopOnly, openWorkspaceThroughUi, setAuditTheme } from "./auditHelpers";

for (const theme of auditThemes) {
  test(`24.6 audit: repository inspector opens live file tree in ${theme} theme`, async ({ page, request }, testInfo) => {
    desktopOnly(testInfo);
    await setAuditTheme(page, theme);
    await openWorkspaceThroughUi(page, request);
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await expect(page.getByRole("complementary", { name: "Repository inspector" })).toHaveCount(0);
    await page.getByRole("button", { name: "Show inspector" }).click();
    const inspector = page.getByRole("complementary", { name: "Repository inspector" });
    await expect(inspector).toBeVisible();
    await expect(inspector).toHaveCSS("width", "420px");
    const rightHandle = page.getByRole("separator", { name: "Resize right drawer" });
    const box = await rightHandle.boundingBox();
    expect(box).not.toBeNull();
    await page.mouse.move(box!.x + box!.width / 2, box!.y + 20);
    await page.mouse.down();
    await page.mouse.move(box!.x - 300, box!.y + 20);
    await page.mouse.up();
    const expandedBox = await inspector.boundingBox();
    expect(expandedBox?.width).toBeGreaterThanOrEqual(650);
    await page.getByRole("button", { name: "AGENTS.md" }).dblclick();
    await expect(page.getByText("# DesktopLab 24.5 audit")).toBeVisible();
    await expect(page.getByRole("button", { name: "Show repository tree" })).toBeVisible();
    await page.screenshot({ path: auditScreenshotPath(theme, "file-drawer.png"), fullPage: true });
  });
}
