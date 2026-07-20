import type { AgentSessionSnapshot, WorkspaceSnapshot } from "../api/types";
import { ApprovalsFeature } from "../features/approvals/ApprovalsFeature";
import { ChangesFeature } from "../features/git/ChangesFeature";
import { JobsFeature } from "../features/jobs/JobsFeature";
import { DiagnosticsFeature } from "../features/productization/DiagnosticsFeature";
import { ExtensionsFeature } from "../features/productization/ExtensionsFeature";
import { ProvidersFeature } from "../features/productization/ProvidersFeature";
import { RuntimeModelFeature } from "../features/productization/RuntimeModelFeature";
import { WorkspaceContextFeature } from "../features/productization/WorkspaceContextFeature";
import { SessionsFeature } from "../features/sessions/SessionsFeature";
import { SetupWizard } from "../features/setup/SetupWizard";
import { SettingsFeature } from "../features/settings/SettingsFeature";
import { WorkspaceFeature } from "../features/workspaces/WorkspaceFeature";
import { AgentRoute } from "./AgentRoute";
import { RouteBoundary } from "./RouteBoundary";
import type { AppRoute } from "./routes";

type AppRoutesProps = {
  route: AppRoute;
  activeWorkspace: WorkspaceSnapshot | null;
  selectedSession: AgentSessionSnapshot | null;
  forceEmptyThread: boolean;
  setActiveWorkspace: (workspace: WorkspaceSnapshot) => void;
  setForcedRoute: (route: AppRoute) => void;
  onSessionStarted: (session: AgentSessionSnapshot) => void;
};

export function AppRoutes({ route, activeWorkspace, selectedSession, forceEmptyThread, setActiveWorkspace, setForcedRoute, onSessionStarted }: AppRoutesProps) {
  const openWorkspace = (workspace: WorkspaceSnapshot) => {
    setActiveWorkspace(workspace);
    setForcedRoute("agent");
  };

  if (route === "setup") return <SetupWizard hasActiveWorkspace={Boolean(activeWorkspace)} onOpenRepository={() => setForcedRoute(activeWorkspace ? "agent" : "workspaces")} />;
  if (route === "workspaces") return <WorkspaceFeature onWorkspaceOpened={openWorkspace} />;
  if (route === "changes") return <ChangesFeature workspace={activeWorkspace} />;
  if (route === "jobs") return <JobsFeature />;
  if (route === "sessions") {
    return activeWorkspace ? <SessionsFeature workspaceId={activeWorkspace.workspaceId} executionBackends={["backend.ollama"]} /> : <OpenWorkspaceFirst />;
  }
  if (route === "approvals") return <ApprovalsFeature />;
  if (route === "providers") return <ProvidersFeature />;
  if (route === "models") return <RuntimeModelFeature />;
  if (route === "agent") {
    return activeWorkspace ? (
      <AgentRoute
        workspace={activeWorkspace}
        selectedSession={selectedSession}
        forceEmptyThread={forceEmptyThread}
        onSessionStarted={onSessionStarted}
        onOpenChanges={() => setForcedRoute("changes")}
        onOpenApprovals={() => setForcedRoute("approvals")}
        onOpenSetup={() => setForcedRoute("setup")}
      />
    ) : (
      <OpenWorkspaceFirst />
    );
  }
  if (route === "context") return activeWorkspace ? <WorkspaceContextFeature workspaceId={activeWorkspace.workspaceId} /> : <OpenWorkspaceFirst />;
  if (route === "extensions") return <ExtensionsFeature />;
  if (route === "diagnostics") return <DiagnosticsFeature />;
  if (route === "settings") return <SettingsFeature />;
  return <RouteBoundary route={route} />;
}

function OpenWorkspaceFirst() {
  return (
    <section className="mx-auto max-w-3xl rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <h1 className="text-2xl font-semibold">Open a project folder</h1>
      <p className="mt-2 text-sm leading-6 text-muted">Open a code folder before starting or reviewing agent sessions.</p>
    </section>
  );
}
