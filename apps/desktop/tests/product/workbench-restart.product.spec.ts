import { expect, test } from "@playwright/test";
import { desktopOnly, localApi, openWorkspaceThroughUi } from "./auditHelpers";

test("24.5 product: workbench restores workspace and blocked first prompt after reload", async ({
  page,
  request,
}, testInfo) => {
  desktopOnly(testInfo);
  await openWorkspaceThroughUi(page, request);

  const prompt = "Keep this prompt after restart";
  await page.getByRole("textbox", { name: "Prompt" }).fill(prompt);
  await page.getByRole("button", { name: "Send prompt" }).click();
  await expect(page.getByText(prompt).first()).toBeVisible();

  const before = await localApi(request, "GET", "/v1/app/state");
  const workspaceId = before.currentWorkspace.workspaceId;
  const beforeSessions = await localApi(request, "GET", `/v1/sessions?workspace_id=${workspaceId}`);
  const beforeSession = sessionForPrompt(beforeSessions.sessions, prompt);
  expect(["completed", "failed"]).toContain(beforeSession.state);

  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Agent" })).toBeVisible();
  await expect(page.getByText(prompt).first()).toBeVisible();

  const after = await localApi(request, "GET", "/v1/app/state");
  expect(after.currentWorkspace.workspaceId).toBe(workspaceId);
  expect(after.routeInput.activeSessionCount).toBeGreaterThan(0);

  const afterSessions = await localApi(request, "GET", `/v1/sessions?workspace_id=${workspaceId}`);
  const afterSession = sessionForPrompt(afterSessions.sessions, prompt);
  expect(afterSession.state).toBe(beforeSession.state);
  expect(afterSession.timeline[0].message).toContain(prompt);
  if (afterSession.state === "failed") {
    expect(afterSession.timeline[1].message).toBe("local_inference_failed");
    await expect(page.getByText("Local inference failed before the agent could continue.")).toBeVisible();
  } else {
    expect(afterSession.timeline.some((event) => event.kind === "completed")).toBe(true);
  }
});

function sessionForPrompt(sessions: Array<{ plan: string; state: string; timeline: Array<{ kind: string; message: string }> }>, prompt: string) {
  const session = [...sessions]
    .reverse()
    .find((candidate) => candidate.plan.includes(prompt) || candidate.timeline.some((event) => event.message.includes(prompt)));
  expect(session, `session for prompt "${prompt}"`).toBeTruthy();
  return session!;
}
