import { expect, test, type Locator, type Page } from "@playwright/test";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import {
  apiBase,
  auditThemes,
  localApi,
  markSetupReady,
  resetProductState,
  setAuditTheme,
  type AuditTheme,
} from "./auditHelpers";

const appArtifact = "/Applications/DesktopLab.app";
const repoRoot = path.resolve(process.cwd(), "../..");
const qaArtifactRoot = path.join(repoRoot, "dist", "product", "stable-ui-qa");

test("stable desktop UI QA covers composer approvals terminal validation and diff", async ({ page, request }, testInfo) => {
  test.setTimeout(120_000);
  const qaArtifactDir = path.join(qaArtifactRoot, testInfo.project.name);
  mkdirSync(qaArtifactDir, { recursive: true });
  const screenshots: ScreenshotRecord[] = [];

  for (const run of [1, 2]) for (const theme of auditThemes) {
    await resetProductState(request);
    await setTheme(page, theme);
    await markSetupReady(request);
    const workspace = await localApi(request, "POST", "/v1/workspaces/open", {
      path: createVisualQaWorkspace(),
    });
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("agent-composer")).toBeVisible();
    await assertComposer(page);
    screenshots.push(await capture(page, qaArtifactDir, run, theme, "01-idle-composer.png", "agent", "idle"));

    await setNativeAgentBackend(request, [
      toolCall("write-visual-qa", "desktoplab.write_file", {
        path: "VISUAL_QA.md",
        content: "# Visual QA\n",
      }),
      toolCall("read-written-visual-qa", "desktoplab.read_file", { path: "VISUAL_QA.md" }),
      completeCall("Created VISUAL_QA.md.", "changed", ["write-visual-qa", "read-written-visual-qa"]),
    ]);
    await sendPrompt(page, "crea VISUAL_QA.md", "keyboard");
    await expect(page.getByRole("group", { name: "Thread approval required" })).toBeVisible();
    await assertApprovalContrast(page);
    screenshots.push(await capture(page, qaArtifactDir, run, theme, "02-approval.png", "agent", "approval"));
    await approveLatest(page, request, workspace.workspaceId);
    await expect.poll(() => filePreviewText(request, workspace.workspaceId, "VISUAL_QA.md")).toContain("# Visual QA");
    await expect(page.getByRole("button", { name: "crea VISUAL_QA.md", exact: true })).toBeVisible();
    screenshots.push(await capture(page, qaArtifactDir, run, theme, "03-completed-summary.png", "agent", "completion"));

    await page.getByRole("button", { name: "Show terminal" }).click();
    await expect(page.getByRole("complementary", { name: "Terminal" })).toBeVisible();
    screenshots.push(await capture(page, qaArtifactDir, run, theme, "04-terminal-output.png", "agent", "completion"));
    await page.getByRole("button", { name: "Hide terminal" }).click();

    await setNativeAgentBackend(request, [
      toolCall("visual-test-failure", "desktoplab.run_tests", { command: "node test.js" }),
    ]);
    await sendPrompt(page, "esegui un test che fallisce");
    await expect(page.getByRole("group", { name: "Thread approval required" })).toBeVisible();
    await approveLatest(page, request, workspace.workspaceId);
    const failedValidation = page.locator('[data-evidence-state="validation-failed"]');
    await expect(failedValidation).toBeVisible();
    await expect(page.getByText("The latest validation command failed. Review the output, repair the issue, and run it again.")).toBeVisible();
    await expect(failedValidation).not.toContainText(/tests_failed|error=|status exited/);
    await expect(page.getByText("Waiting for approval.")).toHaveCount(0);
    await expect(page.getByText("Created VISUAL_QA.md.", { exact: true })).toHaveCount(1);
    await expect(page.getByRole("button", { name: "crea VISUAL_QA.md", exact: true })).toBeVisible();
    await expect(page.getByText("Loading threads...")).toHaveCount(0);
    await expectInConversationViewport(page, failedValidation);
    screenshots.push(await capture(page, qaArtifactDir, run, theme, "05-failed-test.png", "agent", "failure"));

    await setNativeAgentBackend(request, [
      toolCall("read-visual-qa-before-patch", "desktoplab.read_file", { path: "VISUAL_QA.md" }),
      toolCall("patch-visual-qa", "desktoplab.patch_file", {
        path: "VISUAL_QA.md",
        expected: "# Visual QA\n",
        replacement: "# Visual QA\n\nPatch evidence.\n",
      }),
      completeCall("Updated VISUAL_QA.md.", "changed", ["read-visual-qa-before-patch", "patch-visual-qa"]),
    ]);
    await sendPrompt(page, "modifica VISUAL_QA.md aggiungendo evidence");
    await approveLatest(page, request, workspace.workspaceId);
    await expect.poll(() => filePreviewText(request, workspace.workspaceId, "VISUAL_QA.md")).toContain("Patch evidence.");
    const diffRegion = page.getByLabel("Agent diff and validation evidence");
    const diffSummary = diffRegion.locator("summary").filter({ hasText: /changed file|Changed / }).last();
    await expect(diffSummary).toBeVisible();
    await diffSummary.click();
    const diffEvidence = diffRegion.locator("pre").filter({ hasText: /diff --git a\/VISUAL_QA.md b\/VISUAL_QA.md/ }).last();
    await expect(diffEvidence).toBeVisible();
    await expect(page.getByRole("button", { name: "crea VISUAL_QA.md", exact: true })).toBeVisible();
    await expect(page.getByText("Loading threads...")).toHaveCount(0);
    await expectInConversationViewport(page, diffSummary);
    screenshots.push(await capture(page, qaArtifactDir, run, theme, "06-diff-review.png", "agent", "completion"));
  }

  expect(stateSet(screenshots, 1)).toEqual(stateSet(screenshots, 2));
  mkdirSync(qaArtifactDir, { recursive: true });
  writeFileSync(
    path.join(qaArtifactDir, "manifest.json"),
    `${JSON.stringify({
      kind: "desktoplab.stable-ui-captures",
      schemaVersion: 2,
      evidenceKind: "dev_server_ui_with_test_controls",
      installedAppClaim: false,
      testControlsUsed: true,
      sourceCommit: git(["rev-parse", "HEAD"]),
      sourceTreeState: git(["status", "--porcelain=v1"]) ? "dirty" : "clean",
      generatedAt: new Date().toISOString(),
      screenshots,
      appArtifact,
      appBundle: appBundleMetadata(),
    }, null, 2)}\n`,
  );
});

type ScreenshotRecord = { run: number; theme: AuditTheme; filename: string; path: string; route: string; state: string; sha256: string; viewport: { width: number; height: number } };

async function setTheme(page: Page, theme: AuditTheme) {
  await setAuditTheme(page, theme);
  await page.addInitScript((selectedTheme) => {
    window.localStorage.setItem("desktoplab.themePreference", selectedTheme);
  }, theme);
}

async function assertComposer(page: Page) {
  const input = page.getByRole("textbox", { name: "Prompt" });
  await expect(input).toBeVisible();
  await input.focus();
  await page.keyboard.type("caret proof");
  await expect(input).toHaveValue("caret proof");
  await page.keyboard.press("Meta+A");
  await page.keyboard.press("Backspace");
  await expect(page.getByRole("button", { name: "Send prompt" })).toBeDisabled();
}

async function assertApprovalContrast(page: Page) {
  const approve = page.getByRole("button", { name: "Approve" });
  const deny = page.getByRole("button", { name: "Deny" });
  await expect(approve).toBeVisible();
  await expect(deny).toBeVisible();
  expect(await contrastRatio(approve)).toBeGreaterThanOrEqual(4.5);
  expect(await contrastRatio(deny)).toBeGreaterThanOrEqual(3);
}

async function contrastRatio(locator: Locator) {
  return locator.evaluate((element) => {
    const rgb = (value: string) => {
      const match = value.match(/\d+/g)?.slice(0, 3).map(Number);
      if (!match || match.length !== 3) return [0, 0, 0];
      return match;
    };
    const style = window.getComputedStyle(element);
    const fg = rgb(style.color);
    const bg = rgb(style.backgroundColor);
    const luminance = (color: number[]) => {
      const [r, g, b] = color.map((channel) => {
        const value = channel / 255;
        return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
      });
      return 0.2126 * r + 0.7152 * g + 0.0722 * b;
    };
    const high = Math.max(luminance(fg), luminance(bg));
    const low = Math.min(luminance(fg), luminance(bg));
    return (high + 0.05) / (low + 0.05);
  });
}

async function sendPrompt(page: Page, prompt: string, submit: "button" | "keyboard" = "button") {
  await page.getByRole("textbox", { name: "Prompt" }).fill(prompt);
  if (submit === "keyboard") {
    await page.keyboard.press("Enter");
  } else {
    await page.getByRole("button", { name: "Send prompt" }).click();
  }
  await expect(page.getByText(prompt).first()).toBeVisible();
}

async function approveLatest(page: Page, request: Parameters<typeof localApi>[0], workspaceId: string) {
  const findPendingApproval = async () => {
    const listed = await localApi(request, "GET", "/v1/approvals");
    return [...listed.approvals]
      .reverse()
      .find((candidate) => candidate.state === "pending" && candidate.consumed !== true);
  };
  await expect.poll(async () => Boolean(await findPendingApproval())).toBe(true);
  const approval = await findPendingApproval();
  expect(approval).toBeTruthy();
  await localApi(request, "POST", `/v1/approvals/${approval.approvalId}/resolve`, { resolution: "approve" });
  await localApi(request, "POST", `/v1/sessions/${approval.sessionId}/messages`, {
    workspaceId,
    executionBackendId: "backend.ollama",
    prompt: "continue approved action",
    approvalId: approval.approvalId,
  });
  await expect.poll(async () => {
    const current = await localApi(request, "GET", "/v1/agent/workspace");
    const session = current.session;
    const originalStillPending = (session.pendingApprovals ?? []).some(
      (candidate: { approvalId: string }) => candidate.approvalId === approval.approvalId,
    );
    return !originalStillPending && session.state !== "running" ? session.state : null;
  }).not.toBeNull();
  await page.reload({ waitUntil: "domcontentloaded" });
}

async function filePreviewText(request: Parameters<typeof localApi>[0], workspaceId: string, relativePath: string) {
  const route = `/v1/workspaces/${workspaceId}/files/preview?path=${relativePath}`;
  const response = await request.get(`${apiBase}${route}`);
  if (!response.ok()) return `preview status ${response.status()}`;
  const preview = await response.json();
  return preview.state === "text" ? preview.text : `preview state ${preview.state}`;
}

async function setNativeAgentBackend(request: Parameters<typeof localApi>[0], outputs: string[]) {
  await localApi(request, "POST", "/v1/test/agent-backend", { mode: "native_iterative", outputs });
}

function toolCall(id: string, tool: string, args: Record<string, unknown>) {
  return JSON.stringify({ id, tool, arguments: args });
}

function completeCall(message: string, outcome: "answered" | "changed", evidenceCallIds: string[]) {
  return JSON.stringify({ tool: "desktoplab.complete", arguments: { message, outcome, evidenceCallIds } });
}

async function expectInConversationViewport(page: Page, target: Locator) {
  const region = page.getByTestId("agent-conversation-scroll-region");
  await expect.poll(async () => {
    const [regionBox, targetBox] = await Promise.all([region.boundingBox(), target.boundingBox()]);
    if (!regionBox || !targetBox) return false;
    return targetBox.y >= regionBox.y && targetBox.y + targetBox.height <= regionBox.y + regionBox.height;
  }).toBe(true);
}

async function capture(page: Page, qaArtifactDir: string, run: number, theme: AuditTheme, filename: string, route: string, state: string): Promise<ScreenshotRecord> {
  const marker = page.locator(`[data-ui-route="${route}"][data-ui-state="${state}"]`);
  await expect(marker).toBeVisible();
  await expect(page.getByText(/Checking local setup|Loading accounts|Loading models/)).toHaveCount(0);
  const target = path.join(qaArtifactDir, `run-${run}`, theme, filename);
  mkdirSync(path.dirname(target), { recursive: true });
  await page.screenshot({ path: target, fullPage: true });
  const viewport = page.viewportSize();
  if (!viewport) throw new Error("visual capture requires a fixed viewport");
  return {
    run,
    theme,
    filename,
    path: target,
    route,
    state,
    sha256: `sha256:${createHash("sha256").update(readFileSync(target)).digest("hex")}`,
    viewport,
  };
}

function stateSet(records: ScreenshotRecord[], run: number) {
  return records.filter((record) => record.run === run).map((record) => `${record.theme}:${record.filename}:${record.route}:${record.state}`).sort();
}

function appBundleMetadata() {
  if (!existsSync(appArtifact)) return { exists: false };
  const stat = statSync(appArtifact);
  return {
    exists: true,
    modifiedAt: stat.mtime.toISOString(),
    sizeBytes: stat.size,
  };
}

function git(args: string[]) {
  return execFileSync("git", args, { cwd: repoRoot, encoding: "utf8" }).trim();
}

function createVisualQaWorkspace() {
  const root = path.join(
    process.cwd(),
    "test-artifacts",
    `installed-agent-ui-qa-workspace-${Date.now()}-${Math.random().toString(36).slice(2)}`,
  );
  mkdirSync(root, { recursive: true });
  writeFileSync(path.join(root, "AGENTS.md"), "# Visual QA fixture\n");
  writeFileSync(path.join(root, "test.js"), "process.exit(1)\n");
  execFileSync("git", ["init", "-b", "main"], { cwd: root, stdio: "ignore" });
  return root;
}
