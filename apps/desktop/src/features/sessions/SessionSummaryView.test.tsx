// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import type { AgentSessionSnapshot } from "../../api/types";
import { SessionSummaryView } from "./SessionSummaryView";

test("renders backend summary and save point references", () => {
  render(<SessionSummaryView session={{ ...session(), summary: "Tests passed.", checkpoints: ["checkpoint.1", "checkpoint.2"] }} />);

  expect(screen.getByText("Tests passed.")).toBeInTheDocument();
  expect(screen.getByText("checkpoint.1")).toBeInTheDocument();
  expect(screen.getByText("checkpoint.2")).toBeInTheDocument();
});

test("renders no summary state without invented results", () => {
  render(<SessionSummaryView session={session()} />);

  expect(screen.getByText("No summary yet.")).toBeInTheDocument();
  expect(screen.getByText("No save points yet.")).toBeInTheDocument();
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
