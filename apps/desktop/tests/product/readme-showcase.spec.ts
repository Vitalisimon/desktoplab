import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";

import {
  localApi,
  markSetupReady,
  resetProductState,
  setAuditTheme,
} from "./auditHelpers";

const repoRoot = path.resolve(process.cwd(), "../..");
const screenshotRoot = path.join(repoRoot, "assets", "readme");
const workspaceRoot = path.join(
  process.cwd(),
  "test-artifacts",
  "readme-showcase",
  "atlas-notes",
);

test("capture the public README product showcase", async ({ page, request }, testInfo) => {
  test.skip(testInfo.project.name !== "desktop", "README captures use the desktop viewport");
  test.setTimeout(90_000);

  prepareWorkspace();
  mkdirSync(screenshotRoot, { recursive: true });
  await resetProductState(request);
  await setAuditTheme(page, "dark");

  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Finish local setup" })).toBeVisible();
  await expect(page.getByText("DesktopLab selected a local setup that fits this computer.")).toBeVisible();
  await captureShell(page, "automatic-setup.png");

  await markSetupReady(request);

  const workspace = await localApi(request, "POST", "/v1/workspaces/open", {
    path: workspaceRoot,
  });
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("agent-composer")).toBeVisible();
  await expect(page.getByText("atlas-notes", { exact: true }).first()).toBeVisible();

  await setNativeAgentBackend(request, [
    toolCall("read-health-module", "desktoplab.read_file", {
      path: "src/health.js",
    }),
    toolCall("update-health-module", "desktoplab.patch_file", {
      path: "src/health.js",
      expected: [
        "export function summarizeHealth(checks) {",
        "  const passing = checks.filter((check) => check.ok).length;",
        "  return {",
        "    passing,",
        "    total: checks.length,",
        '    status: passing === checks.length ? "healthy" : "degraded"',
        "  };",
        "}",
        "",
      ].join("\n"),
      replacement: [
        "export function summarizeHealth(checks) {",
        "  const passing = checks.filter((check) => check.ok).length;",
        "  const total = checks.length;",
        "  return {",
        "    passing,",
        "    total,",
        '    status: passing === total ? "healthy" : "degraded",',
        "    summary: `${passing}/${total} checks passing`",
        "  };",
        "}",
        "",
      ].join("\n"),
    }),
    completeCall(
      "Added a concise service-health summary. The change is ready for verification.",
      ["read-health-module", "update-health-module"],
    ),
  ]);

  const prompt = "Add a concise health summary and keep the change easy to verify.";
  await page.getByRole("textbox", { name: "Prompt" }).fill(prompt);
  await page.getByRole("button", { name: "Send prompt" }).click();
  await expect(page.getByRole("group", { name: "Thread approval required" })).toBeVisible();
  await captureShell(page, "approval.png");

  await approveLatest(request, workspace.workspaceId);
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByText("Added a concise service-health summary. The change is ready for verification.")).toBeVisible();

  const diffRegion = page.getByLabel("Agent diff and validation evidence");
  const diffSummary = diffRegion.locator("summary").filter({ hasText: /changed file|Changed / }).last();
  await expect(diffSummary).toBeVisible();
  await diffSummary.click();
  await expect(diffRegion.locator("pre").filter({ hasText: /diff --git a\/src\/health.js/ }).last()).toBeVisible();
  await captureShell(page, "agent-workbench.png");

  await page.getByRole("button", { name: "Show inspector" }).click();
  await expect(page.getByRole("complementary", { name: "Repository inspector" })).toBeVisible();
  await page.getByRole("button", { name: "src" }).click();
  await page.getByRole("button", { name: "health.js" }).click();
  const inspector = page.getByRole("complementary", { name: "Repository inspector" });
  await expect(inspector.getByText(/checks passing/)).toBeVisible();

  await page.getByRole("button", { name: "Show terminal" }).click();
  const terminalInput = page.getByRole("textbox", { name: "Terminal input" });
  await terminalInput.fill("npm test");
  await terminalInput.press("Enter");
  await expect(page.getByRole("complementary", { name: "Terminal" }).getByText(/pass 1/i)).toBeVisible();
  await page.getByTestId("terminal-scroll-region").evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  await captureShell(page, "workspace-tools.png");
});

function prepareWorkspace() {
  rmSync(workspaceRoot, { force: true, recursive: true });
  mkdirSync(path.join(workspaceRoot, "src"), { recursive: true });
  writeFileSync(
    path.join(workspaceRoot, "README.md"),
    [
      "# Atlas Notes",
      "",
      "A tiny local-first notes service used for the DesktopLab product showcase.",
      "",
    ].join("\n"),
  );
  writeFileSync(
    path.join(workspaceRoot, "package.json"),
    `${JSON.stringify(
      {
        name: "atlas-notes",
        private: true,
        type: "module",
        scripts: { test: "node --test" },
      },
      null,
      2,
    )}\n`,
  );
  writeFileSync(
    path.join(workspaceRoot, "src", "health.js"),
    [
      "export function summarizeHealth(checks) {",
      "  const passing = checks.filter((check) => check.ok).length;",
      "  return {",
      "    passing,",
      "    total: checks.length,",
      '    status: passing === checks.length ? "healthy" : "degraded"',
      "  };",
      "}",
      "",
    ].join("\n"),
  );
  writeFileSync(
    path.join(workspaceRoot, "src", "health.test.js"),
    [
      'import assert from "node:assert/strict";',
      'import test from "node:test";',
      "",
      'import { summarizeHealth } from "./health.js";',
      "",
      'test("summarizes healthy checks", () => {',
      "  assert.equal(",
      '    summarizeHealth([{ ok: true }, { ok: true }]).status, "healthy"',
      "  );",
      "});",
      "",
    ].join("\n"),
  );
  execFileSync("git", ["init", "-b", "main"], {
    cwd: workspaceRoot,
    stdio: "ignore",
  });
}

async function captureShell(page: Page, filename: string) {
  await page.getByTestId("desktoplab-shell").screenshot({
    path: path.join(screenshotRoot, filename),
  });
}

async function setNativeAgentBackend(request: APIRequestContext, outputs: string[]) {
  await localApi(request, "POST", "/v1/test/agent-backend", {
    mode: "native_iterative",
    outputs,
  });
}

function toolCall(id: string, tool: string, args: Record<string, unknown>) {
  return JSON.stringify({ id, tool, arguments: args });
}

function completeCall(message: string, evidenceCallIds: string[]) {
  return JSON.stringify({
    tool: "desktoplab.complete",
    arguments: { message, outcome: "changed", evidenceCallIds },
  });
}

async function approveLatest(request: APIRequestContext, workspaceId: string) {
  const pendingApproval = async () => {
    const listed = await localApi(request, "GET", "/v1/approvals");
    return [...listed.approvals]
      .reverse()
      .find((candidate) => candidate.state === "pending" && candidate.consumed !== true);
  };

  await expect.poll(async () => Boolean(await pendingApproval())).toBe(true);
  const approval = await pendingApproval();
  expect(approval).toBeTruthy();

  await localApi(request, "POST", `/v1/approvals/${approval.approvalId}/resolve`, {
    resolution: "approve",
  });
  await localApi(request, "POST", `/v1/sessions/${approval.sessionId}/messages`, {
    workspaceId,
    executionBackendId: "backend.ollama",
    prompt: "continue approved action",
    approvalId: approval.approvalId,
  });

  await expect.poll(async () => {
    const current = await localApi(request, "GET", "/v1/agent/workspace");
    return current.session.state === "running" ? null : current.session.state;
  }).not.toBeNull();
}
