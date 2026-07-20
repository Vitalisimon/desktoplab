// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import type { AgentSessionSnapshot } from "../../api/types";
import { SessionTimelineView } from "./SessionTimelineView";

test("renders the backend plan and ordered timeline events", () => {
  render(
    <SessionTimelineView
      session={{
        ...session(),
        plan: "Inspect architecture, edit narrowly, run tests.",
        timeline: [
          event(1, "planning.started", "Planning started"),
          event(2, "execution.started", "Execution started"),
        ],
      }}
    />,
  );

  expect(screen.getByText("Inspect architecture, edit narrowly, run tests.")).toBeInTheDocument();
  expect(screen.getByText("Planning started")).toBeInTheDocument();
  expect(screen.getByText("Execution started")).toBeInTheDocument();
  expect(screen.getByText("planning.started")).toBeInTheDocument();
});

test("renders redacted evidence test output and result summary", () => {
  render(
    <SessionTimelineView
      session={{
        ...session(),
        summary: "2 files changed",
        timeline: [
          {
            ...event(1, "tool", "Terminal command"),
            evidence: { title: "Command output", body: "npm run check PASS", redacted: true },
          },
          {
            ...event(2, "test", "Tests passed"),
            test: { state: "passed", command: "npm run check", output: "PASS" },
          },
        ],
      }}
    />,
  );

  expect(screen.getByText("Command output")).toBeInTheDocument();
  expect(screen.getAllByText(/npm run check/).length).toBeGreaterThan(0);
  expect(screen.getAllByText("Tests passed").length).toBeGreaterThan(0);
  expect(screen.getByText("2 files changed")).toBeInTheDocument();
});

test("renders a truthful empty timeline state", () => {
  render(<SessionTimelineView session={session()} />);

  expect(screen.getByText("No session events yet.")).toBeInTheDocument();
  expect(screen.queryByText(/Execution started/i)).not.toBeInTheDocument();
});

function session(): AgentSessionSnapshot {
  return {
    sessionId: "session.1",
    workspaceId: "workspace.desktoplab",
    executionBackendId: "backend.ollama",
    owner: "desktoplab",
    state: "created",
    plan: null,
    checkpoints: [],
    summary: null,
    timeline: [],
  };
}

function event(sequence: number, kind: string, message: string) {
  return {
    sequence,
    kind,
    message,
    createdAt: "2026-06-25T20:10:00Z",
  };
}
