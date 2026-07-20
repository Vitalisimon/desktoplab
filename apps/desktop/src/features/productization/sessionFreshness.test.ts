import { describe, expect, test } from "vitest";
import type { AgentSessionSnapshot } from "../../api/types";
import { latestSessionSnapshot, shouldRefreshSession } from "./sessionFreshness";

describe("session snapshot freshness", () => {
  test("replaces a stale blocked drawer snapshot with the terminal control-plane result", () => {
    const selected = session("blocked", 1);
    const workspace = session("failed", 2);

    expect(latestSessionSnapshot(selected, workspace)).toBe(workspace);
  });

  test("does not replace an explicitly selected different thread", () => {
    const selected = { ...session("completed", 4), sessionId: "session.selected" };
    const workspace = { ...session("failed", 5), sessionId: "session.latest" };

    expect(latestSessionSnapshot(selected, workspace)).toBe(selected);
  });

  test("refreshes non-terminal sessions and stops after a terminal result", () => {
    expect(shouldRefreshSession(session("blocked", 2))).toBe(true);
    expect(shouldRefreshSession(session("failed", 3))).toBe(false);
  });
});

function session(state: AgentSessionSnapshot["state"], sequence: number): AgentSessionSnapshot {
  return {
    sessionId: "session.1",
    workspaceId: "workspace.1",
    executionBackendId: "backend.local",
    owner: "desktoplab",
    state,
    plan: "Inspect repository",
    checkpoints: [],
    summary: null,
    timeline: [{ sequence, kind: state, message: state, createdAt: "2026-07-10T12:00:00Z" }],
  };
}
