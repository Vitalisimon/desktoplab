import { expect, test } from "@playwright/test";
import { desktopOnly, localApi, openWorkspaceThroughUi } from "./auditHelpers";

test("24.5 product: first prompt creates backend-owned session and reports real outcome", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);
  await openWorkspaceThroughUi(page, request);

  const prompt = "Inspect this repository before editing";
  await page.getByRole("textbox", { name: "Prompt" }).fill(prompt);
  await page.getByRole("button", { name: "Send prompt" }).click();

  await expect(page.getByText(prompt).first()).toBeVisible();

  const state = await localApi(request, "GET", "/v1/app/state");
  const sessions = await localApi(request, "GET", `/v1/sessions?workspace_id=${state.currentWorkspace.workspaceId}`);
  const session = sessionForPrompt(sessions.sessions, prompt);
  expect(["completed", "failed"]).toContain(session.state);
  expect(session.timeline[0].message).toContain(prompt);

  const events = await localApi(request, "GET", "/v1/events/replay");
  const eventPayload = JSON.stringify(events.frames);
  expect(eventPayload).toContain("agent.prompt.accepted");
  if (session.state === "failed") {
    expect(session.timeline[1].message).toBe("local_inference_failed");
    await expect(page.getByText("Local inference failed before the agent could continue.")).toBeVisible();
    expect(eventPayload).toContain("agent.step.failed");
  } else {
    expect(session.timeline.some((event) => event.kind === "completed")).toBe(true);
    expect(eventPayload).toContain("agent.step.completed");
  }
});

function sessionForPrompt(sessions: Array<{ plan: string; state: string; timeline: Array<{ kind: string; message: string }> }>, prompt: string) {
  const session = [...sessions]
    .reverse()
    .find((candidate) => candidate.plan.includes(prompt) || candidate.timeline.some((event) => event.message.includes(prompt)));
  expect(session, `session for prompt "${prompt}"`).toBeTruthy();
  return session!;
}
