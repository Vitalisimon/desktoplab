import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useApiClient } from "../api/ApiProvider";
import { appStateWithOpenedWorkspace, defaultAppState } from "../api/appState";
import type { AgentSessionSnapshot, AppStateResponse, WorkspaceSnapshot } from "../api/types";
import { chooseRepositoryFolder } from "../features/workspaces/repositoryFolderPicker";
import { guardRouteForSetup, selectInitialRoute, type AppRoute, type InitialRouteInput } from "./routes";

export function useAppController(routeInput?: InitialRouteInput) {
  const api = useApiClient();
  const queryClient = useQueryClient();
  const appState = useQuery({ queryKey: ["app-state"], queryFn: () => api.appState(), enabled: !routeInput });
  const [forcedRoute, setForcedRoute] = useState<AppRoute | null>(null);
  const [selectedSession, setSelectedSession] = useState<AgentSessionSnapshot | null>(null);
  const [draftThreadOpen, setDraftThreadOpen] = useState(false);
  const backendState = appState.data ?? defaultAppState;
  const activeWorkspace = backendState.currentWorkspace;
  const route = guardRouteForSetup(forcedRoute ?? selectInitialRoute(routeInput ?? backendState.routeInput), routeInput ?? backendState.routeInput);

  const applyOpenedWorkspace = (workspace: WorkspaceSnapshot) => {
    setSelectedSession(null);
    setDraftThreadOpen(true);
    queryClient.setQueryData<AppStateResponse>(["app-state"], (current) => appStateWithOpenedWorkspace(current, workspace));
  };
  const showProject = (workspace: WorkspaceSnapshot) => {
    applyOpenedWorkspace(workspace);
    setForcedRoute("agent");
    const queryKey = ["agent-workspace", workspace.workspaceId];
    void queryClient.resetQueries({ exact: true, queryKey });
    void queryClient.refetchQueries({ exact: true, queryKey, type: "active" });
  };
  const selectProject = (workspace: WorkspaceSnapshot) => {
    if (workspace.readOnly === true || workspace.rootExists === false) {
      showProject(workspace);
      return;
    }
    void api.openWorkspace({ path: workspace.rootPath }).then(showProject, () => {
      void queryClient.invalidateQueries({ queryKey: ["app-state"] });
    });
  };
  const selectThread = (session: AgentSessionSnapshot) => {
    setDraftThreadOpen(false);
    setSelectedSession(session);
    setForcedRoute("agent");
  };
  const archiveWorkspace = async (workspaceId: string) => {
    await api.archiveWorkspace(workspaceId);
    queryClient.setQueryData<AppStateResponse>(["app-state"], (current) => {
      const base = current ?? backendState;
      const workspaces = (base.workspaces ?? []).filter((workspace) => workspace.workspaceId !== workspaceId);
      const currentWorkspace =
        base.currentWorkspace?.workspaceId === workspaceId ? (workspaces[0] ?? null) : base.currentWorkspace;
      return {
        ...base,
        currentWorkspace,
        workspaces,
        routeInput: {
          ...base.routeInput,
          hasWorkspace: Boolean(currentWorkspace),
        },
      };
    });
    if (activeWorkspace?.workspaceId === workspaceId) {
      setSelectedSession(null);
      setDraftThreadOpen(false);
    }
    await queryClient.invalidateQueries({ queryKey: ["app-state"] });
  };
  const archiveSession = async (sessionId: string) => {
    await api.archiveSession(sessionId);
    if (selectedSession?.sessionId === sessionId) setSelectedSession(null);
    await queryClient.invalidateQueries({ queryKey: ["drawer-project-threads", activeWorkspace?.workspaceId] });
  };
  const relinkWorkspace = async (workspace: WorkspaceSnapshot) => {
    const path = await chooseRepositoryFolder();
    if (!path) return;
    const relinked = await api.relinkWorkspace(workspace.workspaceId, { path });
    applyOpenedWorkspace(relinked);
    setForcedRoute("agent");
    await queryClient.invalidateQueries({ queryKey: ["app-state"] });
    await queryClient.invalidateQueries({ queryKey: ["drawer-project-threads", workspace.workspaceId] });
  };

  return {
    activeWorkspace,
    applyOpenedWorkspace,
    archiveSession,
    archiveWorkspace,
    draftThreadOpen,
    route,
    relinkWorkspace,
    selectProject,
    selectedSession,
    selectThread,
    setForcedRoute,
    setSelectedSession,
    setDraftThreadOpen,
    workspaces: backendState.workspaces?.length ? backendState.workspaces : activeWorkspace ? [activeWorkspace] : [],
  };
}
