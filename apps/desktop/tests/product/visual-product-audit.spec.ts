import { expect, test, type Page } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import {
  auditThemes,
  desktopOnly,
  localApi,
  openWorkspaceThroughUi,
  resetProductState,
  setAuditTheme,
  visualProductArtifactDir,
  visualProductScreenshotPath,
  type AuditTheme,
} from "./auditHelpers";

test("24.6 visual product audit captures complete proof pack in light and dark", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);
  mkdirSync(visualProductArtifactDir, { recursive: true });

  for (const theme of auditThemes) {
    await resetProductState(request);
    await setTheme(page, theme);
    const preview = await localApi(request, "GET", "/v1/setup/preview");
    expect(preview.modelRecommendations.length).toBeGreaterThan(0);
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await expect(page.getByRole("heading", { name: "Setup" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Recommended setup" })).toBeVisible();
    await expect(page.getByText("Install the recommended local runner and coding model, or keep what is already installed.")).toBeVisible();
    await expect(page.getByText(/driver_probe_|gpu_probe_|vram_probe_/)).toHaveCount(0);
    await expect(page.getByTestId("window-command-row")).not.toContainText(/Runtime install|Model download|Recommended setup/);
    await page.screenshot({ path: visualProductScreenshotPath(theme, "01-setup.png"), fullPage: true });

    await expect(page.getByText(preview.modelRecommendations[0].displayName).first()).toBeVisible();
    await expect(page.getByText(/GB memory class|GB on disk/).first()).toBeVisible();
    await page.screenshot({ path: visualProductScreenshotPath(theme, "02-catalog.png"), fullPage: true });
  }

  for (const theme of auditThemes) {
    await setTheme(page, theme);
    await openWorkspaceThroughUi(page, request);
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await assertWorkbenchPrinciples(page);
    await expect(page.getByText("Ask DesktopLab what to change, inspect, or verify in this repository.")).toBeVisible();
    await assertComposerControls(page);
    await page.screenshot({ path: visualProductScreenshotPath(theme, "03-workbench-empty.png"), fullPage: true });

    await page.getByRole("textbox", { name: "Prompt" }).fill("Summarize this repository");
    await page.getByRole("button", { name: "Send prompt" }).click();
    await expect(page.getByText("Summarize this repository").first()).toBeVisible();
    await expect(page.getByText("DesktopLab is working...")).toHaveCount(0);
    const state = await localApi(request, "GET", "/v1/app/state");
    const sessions = await localApi(request, "GET", `/v1/sessions?workspace_id=${state.currentWorkspace.workspaceId}`);
    expect(sessions.sessions.length).toBeGreaterThan(0);
    expect(sessions.sessions[0].timeline.length).toBeGreaterThan(0);
    await assertWorkbenchPrinciples(page);
    await page.screenshot({ path: visualProductScreenshotPath(theme, "04-first-prompt.png"), fullPage: true });

    await page.getByRole("button", { name: "Show inspector" }).click();
    await expect(page.getByRole("complementary", { name: "Repository inspector" })).toBeVisible();
    await page.getByRole("button", { name: "AGENTS.md" }).click();
    await expect(page.getByText("# DesktopLab 24.5 audit")).toBeVisible();
    await expect(page.getByRole("button", { name: "Show repository tree" })).toBeVisible();
    await page.screenshot({ path: visualProductScreenshotPath(theme, "05-file-preview.png"), fullPage: true });

    await page.getByRole("button", { name: "Show terminal" }).click();
    await expect(page.getByRole("complementary", { name: "Terminal" })).toBeVisible();
    await expect(page.getByRole("textbox", { name: "Terminal input" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Copy terminal output" })).toHaveCount(0);
    await page.screenshot({ path: visualProductScreenshotPath(theme, "06-terminal.png"), fullPage: true });

    await page.getByRole("button", { name: "Hide terminal" }).click();
    await page.getByRole("button", { name: "Hide inspector" }).click();
    await openControlCenterItem(page, "Settings");
    await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Appearance" })).toBeVisible();
    await expect(page.getByRole("radio", { name: "System" })).toBeVisible();
    await expect(page.getByRole("radio", { name: "Light" })).toBeVisible();
    await expect(page.getByRole("radio", { name: "Dark" })).toBeVisible();
    await page.getByRole("button", { name: "Providers" }).click();
    await expect(page.getByRole("heading", { name: "Accounts" })).toBeVisible();
    await expect(page.getByText("Cloud accounts are optional. Local models stay the default route until you choose otherwise.")).toBeVisible();
    await expect(page.getByRole("heading", { name: "Connect account" })).toBeVisible();
    await page.getByRole("button", { name: "Safety and approvals" }).click();
    await expect(page.getByRole("heading", { name: "Safety & Approvals" })).toBeVisible();
    await page.screenshot({ path: visualProductScreenshotPath(theme, "08-settings-appearance.png"), fullPage: true });
  }

  writeVisualPrinciplesReport();
});

async function setTheme(page: Page, theme: AuditTheme) {
  await setAuditTheme(page, theme);
  if (page.url() !== "about:blank") {
    await page.evaluate((selectedTheme) => {
      window.localStorage.setItem("desktoplab.themePreference", selectedTheme);
    }, theme);
  }
}

async function assertWorkbenchPrinciples(page: Page) {
  await expect(page.getByTestId("window-command-row")).toBeVisible();
  await expect(page.getByTestId("window-command-row")).not.toContainText(/Runtime install|Model download|Recommended setup|Diagnostics/);
  await expect(page.getByTestId("agent-composer")).toBeVisible();
  await expect(page.getByRole("button", { name: "Start agent" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Accounts" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Settings" })).toHaveCount(0);
  await expect(page.getByText(/driver_probe_|gpu_probe_|vram_probe_|runtime_not_ready|backend_readiness_not_verified/)).toHaveCount(0);
}

async function assertComposerControls(page: Page) {
  const composer = page.getByTestId("agent-composer");
  await expect(composer).toBeVisible();
  await expect(composer.getByRole("button", { name: "Attach external files" })).toBeVisible();
  await expect(composer.getByRole("button", { name: /Approval:/ })).toBeVisible();
  await expect(composer.getByRole("button", { name: /Selected model/ })).toBeVisible();
  await expect(composer.getByRole("button", { name: "Send prompt" })).toBeVisible();

  await composer.getByRole("button", { name: /Approval:/ }).click();
  await expect(page.getByRole("menu", { name: "Approval mode" })).toBeVisible();
  await expect(page.getByRole("menuitemradio", { name: "Ask for approval" })).toBeVisible();
  await expect(page.getByRole("menuitemradio", { name: "Approve routine actions" })).toBeVisible();
  await expect(page.getByRole("menuitemradio", { name: "Full local access" })).toBeVisible();
  await page.keyboard.press("Escape");

  const modelButton = composer.getByRole("button", { name: /Selected model/ });
  if (await modelButton.isDisabled()) {
    await expect(modelButton).toBeDisabled();
    await expect(page.getByRole("menu", { name: "Execution route" })).toHaveCount(0);
  } else {
    await modelButton.click();
    await expect(page.getByRole("menu", { name: "Execution route" })).toBeVisible();
    await expect(page.getByRole("menuitemradio").first()).toBeVisible();
    await page.keyboard.press("Escape");
  }
}

async function openControlCenterItem(page: Page, label: "Settings") {
  const item = page.getByRole("button", { name: label });
  if (!(await item.isVisible())) {
    await page.getByText("Control center").click();
  }
  await item.click();
}

function writeVisualPrinciplesReport() {
  const reportPath = path.join(visualProductArtifactDir, "visual-principles-report.md");
  writeFileSync(
    reportPath,
    [
      "# DesktopLab 24.6 Visual Product Audit",
      "",
      "Status: PASS when this Playwright spec passes.",
      "",
      "Compared principles:",
      "",
      "- Quiet top bar: verified by absence of runtime/model/setup diagnostics in `window-command-row`.",
      "- Compact composer: verified by `agent-composer` visibility and no legacy `Start agent` button.",
      "- Real composer controls: verified by external file attach affordance plus approval and model menus without route interception.",
      "- No thread-center diagnostics: verified by absence of raw probe/readiness codes on setup and workbench surfaces.",
      "- Drawer-contained support surfaces: verified by Accounts and Settings being absent from workbench center until opened from Control Center.",
      "- Preview-first right inspector: verified by selecting `AGENTS.md` and rendering file content in the right drawer.",
      "- Stable bottom terminal: verified by opening Terminal as a complementary pane with compact empty state and no copy control before output exists.",
      "- Safety settings: verified by the Settings surface exposing Safety & Approvals in both themes.",
      "- Light and dark coverage: verified by screenshots under `light/` and `dark/`.",
      "",
      "Screenshot matrix:",
      "",
      ...auditThemes.flatMap((theme) => [
        `- ${theme}/01-setup.png`,
        `- ${theme}/02-catalog.png`,
        `- ${theme}/03-workbench-empty.png`,
        `- ${theme}/04-first-prompt.png`,
        `- ${theme}/05-file-preview.png`,
        `- ${theme}/06-terminal.png`,
        `- ${theme}/08-settings-appearance.png`,
      ]),
      "",
    ].join("\n"),
  );
}
