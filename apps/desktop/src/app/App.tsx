import { AppFrame } from "../design/AppFrame";
import { AppRoutes } from "./AppRoutes";
import type { InitialRouteInput } from "./routes";
import { useAppController } from "./useAppController";

export function App({ routeInput }: { routeInput?: InitialRouteInput }) {
  const controller = useAppController(routeInput);
  const startNewChat = () => {
    controller.setSelectedSession(null);
    controller.setDraftThreadOpen(false);
    controller.setForcedRoute("workspaces");
  };
  const handleSessionStarted = controller.selectThread;

  return (
    <AppFrame
      activeSection={controller.route}
      activeWorkspace={controller.activeWorkspace}
      workspaces={controller.workspaces}
      selectedSessionId={controller.selectedSession?.sessionId ?? null}
      onNewChat={startNewChat}
      onSelectProject={controller.selectProject}
      onSelectThread={controller.selectThread}
      onArchiveWorkspace={controller.archiveWorkspace}
      onRelinkWorkspace={controller.relinkWorkspace}
      onArchiveThread={controller.archiveSession}
      onNavigate={controller.setForcedRoute}
    >
      <div data-testid="desktoplab-root" className="h-full min-h-0">
        <AppRoutes
          route={controller.route}
          activeWorkspace={controller.activeWorkspace}
          selectedSession={controller.selectedSession}
          forceEmptyThread={controller.draftThreadOpen && !controller.selectedSession}
          setActiveWorkspace={controller.applyOpenedWorkspace}
          setForcedRoute={controller.setForcedRoute}
          onSessionStarted={handleSessionStarted}
        />
      </div>
    </AppFrame>
  );
}
