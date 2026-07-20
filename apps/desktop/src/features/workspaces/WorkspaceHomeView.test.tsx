// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import type { WorkspaceHomeSnapshot } from "../../api/types";
import { WorkspaceHomeView } from "./WorkspaceHomeView";

test("renders workspace status summary from backend snapshot", () => {
  render(<WorkspaceHomeView home={home()} />);

  expect(screen.getByText("desktoplab")).toBeInTheDocument();
  expect(screen.getAllByText("/repo/desktoplab").length).toBeGreaterThan(0);
  expect(screen.getByText("Dirty")).toBeInTheDocument();
  expect(screen.getByText("Save point ready")).toBeInTheDocument();
  expect(screen.getByText("modified: apps/desktop/src/App.tsx")).toBeInTheDocument();
});

test("renders recent sessions without exposing prompt entry", () => {
  render(<WorkspaceHomeView home={home()} />);

  expect(screen.getByText("Ollama local")).toBeInTheDocument();
  expect(screen.getByText("Complete")).toBeInTheDocument();
  expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
});

function home(): WorkspaceHomeSnapshot {
  return {
    workspace: {
      workspaceId: "workspace.desktoplab",
      displayName: "desktoplab",
      rootPath: "/repo/desktoplab",
      gitDirPath: "/repo/desktoplab/.git",
      apiState: "dirty",
      statusEntries: ["modified: apps/desktop/src/App.tsx"],
      diffText: "diff --git",
      checkpointStatus: "ready",
      canCheckpointRiskyExecution: true,
    },
    setupHealth: { state: "ready" },
    runtimeHealth: { state: "degraded", label: "Local setup limited" },
    recentSessions: [
      {
        sessionId: "session.1",
        backendId: "backend.ollama",
        state: "completed",
        updatedAt: "2026-06-25T19:48:00Z",
      },
    ],
  };
}
