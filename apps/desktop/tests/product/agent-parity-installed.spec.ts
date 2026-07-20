import { expect, test, type Page } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { apiBase, desktopOnly, localApi, markSetupReady } from "./auditHelpers";

test("agent parity: desktop UI can inspect, edit, validate and show diff through the native loop", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);
  const workspaceRoot = createAgentParityFixture();
  await markSetupReady(request);
  const openedWorkspace = await openWorkspaceForParity(request, workspaceRoot);
  const workspaceId = openedWorkspace.workspaceId;
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Agent" })).toBeVisible();

  await setNativeAgentBackend(request, [
    toolCall("list-1", "desktoplab.list_files", {}),
    completeCall("Repository files include README.md, AGENTS.md and src/lib.rs.", "answered", ["list-1"]),
  ]);
  await sendPrompt(page, "spiega questa repo");
  await expect(page.getByText("Repository files include README.md, AGENTS.md and src/lib.rs.")).toBeVisible();

  await setNativeAgentBackend(request, [
    toolCall("search-1", "desktoplab.search_text", { query: "AgentComposer", path: "." }),
    completeCall(
      "Il composer vive in apps/desktop/src/features/productization/AgentComposer.tsx.",
      "answered",
      ["search-1"],
    ),
  ]);
  await sendPrompt(page, "trova dove viene gestito il composer");
  await expect(
    page.getByText("Il composer vive in apps/desktop/src/features/productization/AgentComposer.tsx.", { exact: true }),
  ).toBeVisible();

  await setNativeAgentBackend(request, [
    toolCall("write-prova", "desktoplab.write_file", {
      path: "prova.md",
      content: "# Prova\n\nNota iniziale.\n",
    }),
    toolCall("read-written-prova", "desktoplab.read_file", { path: "prova.md" }),
    completeCall("Creato prova.md.", "changed", ["write-prova", "read-written-prova"]),
  ]);
  await sendPrompt(page, "crea prova.md con una nota");
  await approveThreadAction(page, request, workspaceId);
  await expect.poll(() => localApi(request, "GET", `/v1/workspaces/${workspaceId}/files/preview?path=prova.md`)).toMatchObject({
    state: "text",
  });

  await setNativeAgentBackend(request, [
    toolCall("write-shortcuts", "desktoplab.write_file", {
      path: "manuale-tastiera.md",
      content: "# Scorciatoie da tastiera\n\n- Invio: invia il prompt.\n",
    }),
    toolCall("read-written-shortcuts", "desktoplab.read_file", { path: "manuale-tastiera.md" }),
    completeCall("Creato manuale-tastiera.md.", "changed", ["write-shortcuts", "read-written-shortcuts"]),
  ]);
  await sendPrompt(page, "prova a creare un nuovo file doc, in cui descrivi le scorciatoie da tastiera");
  await approveThreadAction(page, request, workspaceId);
  const shortcutsPreview = await localApi(
    request,
    "GET",
    `/v1/workspaces/${workspaceId}/files/preview?path=manuale-tastiera.md`,
  );
  expect(shortcutsPreview.text).toContain("Scorciatoie da tastiera");
  await expect(page.getByText("clarification_required:file_target")).toHaveCount(0);

  await setNativeAgentBackend(request, [
    toolCall("read-notes-before-patch", "desktoplab.read_file", { path: "notes.md" }),
    toolCall("patch-notes", "desktoplab.patch_file", {
      path: "notes.md",
      expected: "beta\n",
      replacement: "beta updated\n",
    }),
    completeCall("Aggiornato notes.md.", "changed", ["read-notes-before-patch", "patch-notes"]),
  ]);
  await sendPrompt(page, "modifica notes.md aggiornando beta in beta updated");
  await approveThreadAction(page, request, workspaceId);
  const notesPreview = await localApi(request, "GET", `/v1/workspaces/${workspaceId}/files/preview?path=notes.md`);
  expect(notesPreview.state).toBe("text");
  expect(readFileSync(rootPath(workspaceRoot, "notes.md"), "utf8")).toBe("alpha\nbeta updated\ngamma\n");

  await setNativeAgentBackend(request, [
    toolCall("read-prova-before-patch", "desktoplab.read_file", { path: "prova.md" }),
    toolCall("patch-prova", "desktoplab.patch_file", {
      path: "prova.md",
      expected: "# Prova\n\nNota iniziale.\n",
      replacement: "# Prova\n\nNota iniziale.\n\n## Seconda sezione\n\nAggiunta.\n",
    }),
    completeCall("Modificato prova.md.", "changed", ["read-prova-before-patch", "patch-prova"]),
  ]);
  await sendPrompt(page, "modifica prova.md aggiungendo una sezione");
  await approveThreadAction(page, request, workspaceId);
  const editedPreview = await localApi(request, "GET", `/v1/workspaces/${workspaceId}/files/preview?path=prova.md`);
  expect(editedPreview.text).toContain("Seconda sezione");

  await setNativeAgentBackend(request, [completeCall("Eseguo i test mirati.", "answered", [])]);
  await sendPrompt(page, "spiegami quali test mirati useresti");
  await expect(page.getByText("Eseguo i test mirati.")).toBeVisible();

  await setNativeAgentBackend(request, [
    completeCall("Validation retry: nessuna correzione necessaria dopo il pass.", "answered", []),
  ]);
  await sendPrompt(page, "correggi il test fallito");
  await expect(page.getByText("validation", { exact: false }).or(page.getByText("corretto", { exact: false }))).toBeVisible();

  await setNativeAgentBackend(request, [
    toolCall("diff-1", "desktoplab.git_diff", {}),
    completeCall("Git diff shows tracked changes in notes.md.", "answered", ["diff-1"]),
  ]);
  await sendPrompt(page, "mostrami il diff");
  const diffEvidence = page.getByLabel("Agent diff and validation evidence");
  const latestTrackedDiff = diffEvidence.locator("summary").filter({ hasText: "Changed notes.md" }).last();
  await expect(latestTrackedDiff).toBeVisible();
  await latestTrackedDiff.click();
  await expect(diffEvidence.locator("pre").filter({ hasText: "diff --git a/notes.md b/notes.md" }).last()).toBeVisible();
  const createdFilePatch = diffEvidence.locator("summary").filter({ hasText: "Changed prova.md" }).last();
  await expect(createdFilePatch).toBeVisible();
  await createdFilePatch.click();
  await expect(diffEvidence.locator("pre").filter({ hasText: "diff --git a/prova.md b/prova.md" }).last()).toBeVisible();
});

test("agent parity: desktop UI keeps read patch test loop in one native session", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);
  const workspaceRoot = createAgentParityFixture();
  await markSetupReady(request);
  const openedWorkspace = await openWorkspaceForParity(request, workspaceRoot);
  const workspaceId = openedWorkspace.workspaceId;
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Agent" })).toBeVisible();

  await setNativeAgentBackend(request, [
    toolCall("read-readme", "desktoplab.read_file", { path: "README.md" }),
    toolCall("read-answer-before-patch", "desktoplab.read_file", { path: "src/lib.rs" }),
    toolCall("patch-answer", "desktoplab.patch_file", {
      path: "src/lib.rs",
      expected: "pub fn answer() -> i32 { 41 }\n",
      replacement: "pub fn answer() -> i32 { 42 }\n",
    }),
    toolCall("test-answer", "desktoplab.run_tests", { command: "node test.js" }),
    completeCall("Summary: README letto, src/lib.rs corretto, node test.js passato.", "verified", [
      "read-readme",
      "read-answer-before-patch",
      "patch-answer",
      "test-answer",
    ]),
  ]);
  await sendPrompt(page, "Leggi README.md, correggi answer ed esegui i test mirati");
  const sessionId = await latestSessionId(request);
  const patched = await approveThreadAction(page, request, workspaceId);
  expect(patched.sessionId).toBe(sessionId);
  expect(readFileSync(rootPath(workspaceRoot, "src/lib.rs"), "utf8")).toContain("42");
  const summarized = await approveThreadAction(page, request, workspaceId);
  expect(summarized.sessionId).toBe(sessionId);
  const timeline = timelineText(summarized);
  expect(timeline).toContain("Read README.md:");
  expect(timeline).toContain("canonical=desktoplab.patch_file");
  expect(timeline).toContain("Test command `node test.js`");
  expect(timeline).toContain("agent parity test ok");
});

test("agent parity: desktop UI repairs a failing test and reruns native validation", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);
  const workspaceRoot = createAgentParityFixture();
  await markSetupReady(request);
  const openedWorkspace = await openWorkspaceForParity(request, workspaceRoot);
  const workspaceId = openedWorkspace.workspaceId;
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Agent" })).toBeVisible();

  await setNativeAgentBackend(request, [
    toolCall("test-failing", "desktoplab.run_tests", { command: "node test.js" }),
    toolCall("read-answer-after-failure", "desktoplab.read_file", { path: "src/lib.rs" }),
    toolCall("patch-from-failure", "desktoplab.patch_file", {
      path: "src/lib.rs",
      expected: "pub fn answer() -> i32 { 41 }\n",
      replacement: "pub fn answer() -> i32 { 42 }\n",
    }),
    toolCall("test-passing", "desktoplab.run_tests", { command: "node test.js" }),
    completeCall(
      "Summary: first node test.js failed with expected answer 42; patched src/lib.rs; rerun node test.js passed with agent parity test ok.",
      "verified",
      ["patch-from-failure", "test-passing"],
    ),
  ]);
  await sendPrompt(page, "Correggi il test fallito e rilancia la validazione");
  const sessionId = await latestSessionId(request);
  const failed = await approveThreadAction(page, request, workspaceId);
  expect(failed.sessionId).toBe(sessionId);
  expect(timelineText(failed)).toContain("expected answer 42");
  const patched = await approveThreadAction(page, request, workspaceId);
  expect(patched.sessionId).toBe(sessionId);
  expect(readFileSync(rootPath(workspaceRoot, "src/lib.rs"), "utf8")).toContain("42");
  const summarized = await approveThreadAction(page, request, workspaceId);
  const timeline = timelineText(summarized);
  expect(timeline).toContain("expected answer 42");
  expect(timeline).toContain("agent parity test ok");
  expect(timeline).toContain("first node test.js failed");
  expect(timeline).toContain("rerun node test.js passed");
});

async function sendPrompt(page: Page, prompt: string) {
  await page.getByRole("textbox", { name: "Prompt" }).fill(prompt);
  await page.getByRole("button", { name: "Send prompt" }).click();
  await expect(page.getByText(prompt).first()).toBeVisible();
}

async function approveThreadAction(page: Page, request: Parameters<typeof localApi>[0], workspaceId: string) {
  const listed = await localApi(request, "GET", "/v1/approvals");
  const approval = [...listed.approvals].reverse().find((candidate) => candidate.state === "pending" && candidate.consumed !== true);
  expect(approval, "latest approval").toBeTruthy();
  await localApi(request, "POST", `/v1/approvals/${approval.approvalId}/resolve`, { resolution: "approve" });
  const completed = await localApi(request, "POST", `/v1/sessions/${approval.sessionId}/messages`, {
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
    const nextApprovalReady = (session.pendingApprovals ?? []).some(
      (candidate: { approvalId: string }) => candidate.approvalId !== approval.approvalId,
    );
    return !originalStillPending && (session.state === "completed" || session.state === "failed" || nextApprovalReady)
      ? session
      : null;
  }).not.toBeNull();
  const current = await localApi(request, "GET", "/v1/agent/workspace");
  await page.reload({ waitUntil: "domcontentloaded" });
  return current.session ?? completed;
}

async function setNativeAgentBackend(request: Parameters<typeof localApi>[0], outputs: string[]) {
  await localApi(request, "POST", "/v1/test/agent-backend", { mode: "native_iterative", outputs });
}

function toolCall(id: string, tool: string, args: Record<string, unknown>) {
  return JSON.stringify({ id, tool, arguments: args });
}

function completeCall(message: string, outcome: "answered" | "changed" | "verified", evidenceCallIds: string[]) {
  return JSON.stringify({ tool: "desktoplab.complete", arguments: { message, outcome, evidenceCallIds } });
}

async function latestSessionId(request: Parameters<typeof localApi>[0]) {
  const listed = await localApi(request, "GET", "/v1/sessions");
  const session = listed.sessions.at(-1);
  expect(session, "latest session").toBeTruthy();
  return session.sessionId;
}

function timelineText(session: { timeline?: Array<{ message?: string }> }) {
  return (session.timeline ?? []).map((event) => event.message ?? "").join("\n");
}

function createAgentParityFixture() {
  const root = mkdtempSync(path.join(tmpdir(), "desktoplab-agent-parity-"));
  mkdirSync(path.join(root, "src"), { recursive: true });
  mkdirSync(path.join(root, "apps/desktop/src/features/productization"), { recursive: true });
  writeFileSync(rootPath(root, "AGENTS.md"), "# DesktopLab Agent Parity\n\nWorkspace fixture for coding-agent parity.\n");
  writeFileSync(rootPath(root, "README.md"), "# Agent Parity Fixture\n\nRepository used for installed-app agent parity.\n");
  writeFileSync(rootPath(root, "notes.md"), "alpha\nbeta\ngamma\n");
  writeFileSync(rootPath(root, "src/lib.rs"), "pub fn answer() -> i32 { 41 }\n");
  writeFileSync(
    rootPath(root, "apps/desktop/src/features/productization/AgentComposer.tsx"),
    "export function AgentComposer() { return null; }\n",
  );
  writeFileSync(
    rootPath(root, "package.json"),
    JSON.stringify({ name: "desktoplab-agent-parity", private: true, scripts: { test: "node test.js" } }, null, 2),
  );
  writeFileSync(
    rootPath(root, "test.js"),
    "const { readFileSync } = require('fs');\nif (!readFileSync('src/lib.rs', 'utf8').includes('42')) { console.error('expected answer 42'); process.exit(1); }\nconsole.log('agent parity test ok');\n",
  );
  execFileSync("git", ["init", "-b", "main"], { cwd: root, stdio: "ignore" });
  execFileSync("git", ["config", "user.email", "desktoplab@example.test"], { cwd: root, stdio: "ignore" });
  execFileSync("git", ["config", "user.name", "DesktopLab Test"], { cwd: root, stdio: "ignore" });
  execFileSync("git", ["add", "."], { cwd: root, stdio: "ignore" });
  execFileSync("git", ["commit", "-m", "initial fixture"], { cwd: root, stdio: "ignore" });
  return root;
}

function rootPath(root: string, relativePath: string) {
  return path.join(root, relativePath);
}

async function openWorkspaceForParity(request: Parameters<typeof localApi>[0], workspaceRoot: string) {
  let response = await request.fetch(`${apiBase}/v1/workspaces/open`, {
    method: "POST",
    data: { path: workspaceRoot },
  });
  if (response.status() === 400) {
    const blocked = await response.json();
    if (blocked.blockedReason === "setup_not_ready") {
      await markSetupReady(request);
      response = await request.fetch(`${apiBase}/v1/workspaces/open`, {
        method: "POST",
        data: { path: workspaceRoot },
      });
    } else {
      expect(response.status(), `open workspace status: ${JSON.stringify(blocked)}`).toBe(200);
      return blocked;
    }
  }
  const body = await response.json();
  expect(response.status(), `open workspace status: ${JSON.stringify(body)}`).toBe(200);
  return body;
}
