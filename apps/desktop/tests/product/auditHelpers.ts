import { expect, test, type APIRequestContext, type Page, type TestInfo } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { localApi, resetProductState } from "./localApiTestClient";
import { markSetupReady } from "./setupModelHelpers";

export { apiBase, localApi, resetProductState } from "./localApiTestClient";
export { markSetupReady, recommendedSetupModel, selectSetup, type SetupModelSelection } from "./setupModelHelpers";
export const artifactDir = "test-artifacts/24.5-product-audit";
export const visualProductArtifactDir = "test-artifacts/24.6-visual-product-audit";
export const auditThemes = ["light", "dark"] as const;
export type AuditTheme = (typeof auditThemes)[number];

test.beforeEach(async ({ request }, testInfo) => {
  if (testInfo.project.name === "desktop") {
    await resetProductState(request);
  }
});

export function desktopOnly(testInfo: TestInfo) {
  test.skip(testInfo.project.name !== "desktop", "24.5 audit mutates shared backend state");
  mkdirSync(artifactDir, { recursive: true });
}

export async function setAuditTheme(page: Page, theme: AuditTheme) {
  await page.addInitScript((selectedTheme) => {
    window.localStorage.setItem("desktoplab.themePreference", selectedTheme);
  }, theme);
}

export function auditScreenshotPath(theme: AuditTheme, filename: string) {
  const themeDir = path.join(artifactDir, theme);
  mkdirSync(themeDir, { recursive: true });
  return path.join(themeDir, filename);
}

export function visualProductScreenshotPath(theme: AuditTheme, filename: string) {
  const themeDir = path.join(visualProductArtifactDir, theme);
  mkdirSync(themeDir, { recursive: true });
  return path.join(themeDir, filename);
}

export function createWorkspaceFixture(prefix = "desktoplab-245-audit-") {
  const root = mkdtempSync(path.join(tmpdir(), prefix));
  writeFileSync(path.join(root, "AGENTS.md"), "# DesktopLab 24.5 audit\n\nWorkspace fixture for product truth audit.\n");
  writeFileSync(path.join(root, "package.json"), JSON.stringify({ name: "desktoplab-245-audit", private: true }, null, 2));
  execFileSync("git", ["init"], { cwd: root, stdio: "ignore" });
  return root;
}

export async function openWorkspaceThroughUi(page: Page, request: APIRequestContext) {
  const workspaceRoot = createWorkspaceFixture();
  await markSetupReady(request);
  await localApi(request, "POST", "/v1/workspaces/open", { path: workspaceRoot });
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Agent" })).toBeVisible();
  return workspaceRoot;
}
