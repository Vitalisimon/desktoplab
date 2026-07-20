import type { AgentSessionSnapshot, WorkspaceSnapshot } from "../api/types";
import { AgentWorkspaceFeature } from "../features/productization/AgentWorkspaceFeature";

type AgentRouteProps = {
  workspace: WorkspaceSnapshot;
  selectedSession: AgentSessionSnapshot | null;
  forceEmptyThread: boolean;
  onSessionStarted: (session: AgentSessionSnapshot) => void;
  onOpenChanges: () => void;
  onOpenApprovals: () => void;
  onOpenSetup: () => void;
};

export function AgentRoute({ workspace, selectedSession, forceEmptyThread, onSessionStarted, onOpenChanges, onOpenApprovals, onOpenSetup }: AgentRouteProps) {
  return (
    <AgentWorkspaceFeature
      workspaceId={workspace.workspaceId}
      workspaceName={workspace.displayName}
      selectedSession={selectedSession}
      forceEmptyThread={forceEmptyThread}
      onSessionStarted={onSessionStarted}
      onOpenChanges={onOpenChanges}
      onOpenApprovals={onOpenApprovals}
      onOpenSetup={onOpenSetup}
    />
  );
}
