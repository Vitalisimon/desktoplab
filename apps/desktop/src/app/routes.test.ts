import { selectInitialRoute } from "./routes";

test("routes degraded or blocked setup state to setup boundary", () => {
  expect(
    selectInitialRoute({
      readiness: "degraded",
      hasWorkspace: false,
      activeApprovalCount: 0,
      activeSessionCount: 0,
    }),
  ).toBe("setup");
});

test("routes unfinished setup state to setup boundary even with workspace", () => {
  for (const setupState of ["not_started", "in_progress", "blocked"] as const) {
    expect(
      selectInitialRoute({
        readiness: "blocked",
        setupState,
        hasWorkspace: true,
        activeApprovalCount: 0,
        activeSessionCount: 0,
      }),
    ).toBe("setup");
  }
});

test("keeps active approvals inside the agent workbench route", () => {
  expect(
    selectInitialRoute({
      readiness: "ready",
      hasWorkspace: true,
      activeApprovalCount: 1,
      activeSessionCount: 1,
    }),
  ).toBe("agent");
});

test("routes ready app without workspace to workspace boundary", () => {
  expect(
    selectInitialRoute({
      readiness: "ready",
      hasWorkspace: false,
      activeApprovalCount: 0,
      activeSessionCount: 0,
    }),
  ).toBe("workspaces");
});

test("routes ready app with workspace to the agent workbench", () => {
  expect(
    selectInitialRoute({
      readiness: "ready",
      hasWorkspace: true,
      activeApprovalCount: 0,
      activeSessionCount: 0,
    }),
  ).toBe("agent");
});
