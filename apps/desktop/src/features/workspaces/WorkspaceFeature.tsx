import { useState } from "react";
import { useControlPlaneStatus } from "../../app/useControlPlaneStatus";
import type { WorkspaceHomeSnapshot, WorkspaceSnapshot } from "../../api/types";
import { WorkspaceHomeView } from "./WorkspaceHomeView";
import { WorkspaceOpenView } from "./WorkspaceOpenView";

type WorkspaceFeatureProps = {
  onWorkspaceOpened?: (workspace: WorkspaceSnapshot) => void;
};

export function WorkspaceFeature({ onWorkspaceOpened }: WorkspaceFeatureProps) {
  const controlPlane = useControlPlaneStatus();
  const [workspace, setWorkspace] = useState<WorkspaceSnapshot | null>(null);

  if (!workspace) {
    return (
      <WorkspaceOpenView
        onOpened={(openedWorkspace) => {
          setWorkspace(openedWorkspace);
          onWorkspaceOpened?.(openedWorkspace);
        }}
      />
    );
  }

  const readiness = controlPlane.readiness.data ?? { state: "starting" as const };

  return (
    <WorkspaceHomeView
      home={{
        workspace,
        setupHealth: readiness,
        runtimeHealth: runtimeHealthFromReadiness(readiness.state),
        recentSessions: [],
      }}
    />
  );
}

function runtimeHealthFromReadiness(state: WorkspaceHomeSnapshot["setupHealth"]["state"]) {
  if (state === "blocked") return { state: "blocked" as const, label: "Local setup blocked" };
  if (state === "degraded") return { state: "degraded" as const, label: "Local setup limited" };
  if (state === "starting") return { state: "degraded" as const, label: "Local setup checking" };
  return { state: "ready" as const, label: "Local tools verified" };
}
