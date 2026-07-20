// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import type { AgentSessionSnapshot } from "../../api/types";
import { SessionStatusView } from "./SessionStatusView";

test("renders all backend session states with explicit labels", () => {
  const states: AgentSessionSnapshot["state"][] = [
    "created",
    "planning",
    "running",
    "paused",
    "blocked",
    "failed",
    "cancelled",
    "completed",
  ];

  render(
    <div>
      {states.map((state) => (
        <SessionStatusView key={state} session={{ ...session(), state }} />
      ))}
    </div>,
  );

  expect(screen.getByText("Created")).toBeInTheDocument();
  expect(screen.getByText("Planning")).toBeInTheDocument();
  expect(screen.getByText("Running")).toBeInTheDocument();
  expect(screen.getByText("Paused")).toBeInTheDocument();
  expect(screen.getByText("Blocked")).toBeInTheDocument();
  expect(screen.getByText("Failed")).toBeInTheDocument();
  expect(screen.getByText("Cancelled")).toBeInTheDocument();
  expect(screen.getByText("Completed")).toBeInTheDocument();
});

test("shows session owner runner and repository identity", () => {
  render(<SessionStatusView session={session()} />);

  expect(screen.getAllByText("desktoplab").length).toBeGreaterThan(0);
  expect(screen.getByText("Ollama local")).toBeInTheDocument();
  expect(screen.queryByText("workspace.desktoplab")).not.toBeInTheDocument();
});

function session(): AgentSessionSnapshot {
  return {
    sessionId: "session.1",
    workspaceId: "workspace.desktoplab",
    executionBackendId: "backend.ollama",
    owner: "desktoplab",
    state: "running",
    plan: null,
    checkpoints: [],
    summary: null,
    timeline: [],
  };
}
