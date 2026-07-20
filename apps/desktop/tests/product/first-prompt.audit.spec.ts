import { expect, test } from "@playwright/test";
import { auditScreenshotPath, auditThemes, desktopOnly, localApi, openWorkspaceThroughUi, setAuditTheme } from "./auditHelpers";

for (const theme of auditThemes) {
  test(`24.6 audit: first prompt creates backend-owned session evidence in ${theme} theme`, async ({ page, request }, testInfo) => {
    desktopOnly(testInfo);
    await setAuditTheme(page, theme);
    await openWorkspaceThroughUi(page, request);
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await page.getByRole("textbox", { name: "Prompt" }).fill("Audit the first prompt path");
    await page.getByRole("button", { name: "Send prompt" }).click();
    await expect(page.getByText("Audit the first prompt path").first()).toBeVisible();
    await page.screenshot({ path: auditScreenshotPath(theme, "first-prompt.png"), fullPage: true });

    const state = await localApi(request, "GET", "/v1/app/state");
    const sessions = await localApi(request, "GET", `/v1/sessions?workspace_id=${state.currentWorkspace.workspaceId}`);
    expect(sessions.sessions.length).toBeGreaterThan(0);
    expect(sessions.sessions[0].timeline.length).toBeGreaterThan(0);
  });
}
