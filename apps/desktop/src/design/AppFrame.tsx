import { useEffect, useRef, useState, type ReactNode } from "react";
import type { AppRoute } from "../app/routes";
import type { AgentSessionSnapshot, WorkspaceSnapshot } from "../api/types";
import { useApiClient } from "../api/ApiProvider";
import { AppDrawer } from "./AppDrawer";
import { RepositoryInspector } from "./RepositoryInspector";
import { ResizeHandle, startResize } from "./ResizeHandle";
import { ToolbarChangesPopover } from "./ToolbarChangesPopover";
import { WindowCommandRow } from "./WindowCommandRow";
import { useRepositoryOpenTargets } from "./useRepositoryOpenTargets";
import { TerminalDrawer } from "../features/terminal/TerminalDrawer";
import { TerminalResizeHandle } from "../features/terminal/TerminalResizeHandle";
import { useTerminalReplay } from "../features/terminal/useTerminalReplay";
import { clamp, drawerWidth, paneStorageKeys, readStoredPaneSize, writeStoredPaneSize } from "./paneSizing";
import { useDrawerProjectThreads } from "./useDrawerProjectThreads";

type AppFrameProps = {
  children: ReactNode;
  title?: string;
  status?: "local" | "ready" | "degraded" | "blocked";
  activeSection?: AppRoute;
  activeWorkspace?: WorkspaceSnapshot | null;
  workspaces?: WorkspaceSnapshot[];
  selectedSessionId?: string | null;
  onNewChat?: () => void;
  onSelectProject?: (workspace: WorkspaceSnapshot) => void;
  onSelectThread?: (session: AgentSessionSnapshot) => void;
  onArchiveWorkspace?: (workspaceId: string) => Promise<void> | void;
  onRelinkWorkspace?: (workspace: WorkspaceSnapshot) => Promise<void> | void;
  onArchiveThread?: (sessionId: string) => Promise<void> | void;
  onNavigate?: (section: AppRoute) => void;
};

export function AppFrame({
  children,
  title = "DesktopLab",
  status = "local",
  activeSection = "setup",
  activeWorkspace = null,
  workspaces = activeWorkspace ? [activeWorkspace] : [],
  selectedSessionId = null,
  onNewChat,
  onSelectProject,
  onSelectThread,
  onArchiveWorkspace,
  onRelinkWorkspace,
  onArchiveThread,
  onNavigate,
}: AppFrameProps) {
  const api = useApiClient();
  const [leftOpen, setLeftOpen] = useState(true);
  const [rightOpen, setRightOpen] = useState(false);
  const [terminalOpen, setTerminalOpen] = useState(false);
  const [changesOpen, setChangesOpen] = useState(false);
  const [terminalRefreshKey, setTerminalRefreshKey] = useState(0);
  const workbenchRef = useRef<HTMLElement>(null);
  const [leftWidthValue, setLeftWidthValue] = useState<number>(() =>
    readStoredPaneSize(paneStorageKeys.leftWidth, drawerWidth.leftDefault, drawerWidth.leftMin, drawerWidth.leftMax),
  );
  const [rightWidthValue, setRightWidthValue] = useState<number>(() =>
    readStoredPaneSize(paneStorageKeys.rightWidth, drawerWidth.rightDefault, drawerWidth.rightMin, drawerWidth.rightMax),
  );
  const [terminalHeightValue, setTerminalHeightValue] = useState<number>(() =>
    readStoredPaneSize(paneStorageKeys.terminalHeight, drawerWidth.terminalDefault, drawerWidth.terminalMin, drawerWidth.terminalMax),
  );
  const setLeftWidth = (width: number) => {
    const nextWidth = clamp(width, drawerWidth.leftMin, drawerWidth.leftMax);
    setLeftWidthValue(nextWidth);
    writeStoredPaneSize(paneStorageKeys.leftWidth, nextWidth);
  };
  const setRightWidth = (width: number) => {
    const nextWidth = clamp(width, drawerWidth.rightMin, drawerWidth.rightMax);
    setRightWidthValue(nextWidth);
    writeStoredPaneSize(paneStorageKeys.rightWidth, nextWidth);
  };
  const setTerminalHeight = (height: number) => {
    const nextHeight = clamp(height, drawerWidth.terminalMin, drawerWidth.terminalMax);
    setTerminalHeightValue(nextHeight);
    writeStoredPaneSize(paneStorageKeys.terminalHeight, nextHeight);
  };
  const leftWidth = leftWidthValue;
  const rightWidth = rightWidthValue;
  const terminalHeight = terminalHeightValue;
  const showInspector = Boolean(activeWorkspace && rightOpen);
  const columns = `${leftOpen ? `${leftWidth}px ${drawerWidth.handle}px` : `${drawerWidth.leftCollapsed}px`} minmax(0,1fr)${showInspector ? ` ${drawerWidth.handle}px ${rightWidth}px` : ""}`;
  const terminalReplay = useTerminalReplay({
    open: terminalOpen,
    enabled: Boolean(activeWorkspace),
    refreshKey: terminalRefreshKey,
  });
  const repositoryOpen = useRepositoryOpenTargets(activeWorkspace);
  const resolveTerminalApproval = async (approvalId: string, resolution: "approve" | "deny") => {
    await api.resolveApproval(approvalId, { resolution });
    if (activeWorkspace && terminalReplay.response) {
      await api.createTerminalCommand(activeWorkspace.workspaceId, {
        command: terminalReplay.response.command,
        cwd: terminalReplay.response.cwd,
        approvalId,
        approvalRequired: true,
      });
    }
    setTerminalRefreshKey((key) => key + 1);
  };
  const drawerWorkspaces = workspaces.length > 0 ? workspaces : activeWorkspace ? [activeWorkspace] : [];
  const { byWorkspace: projectThreadsByWorkspace, statusByWorkspace: projectThreadsStatusByWorkspace, pinnedThreads } =
    useDrawerProjectThreads(drawerWorkspaces);

  useEffect(() => {
    if (workbenchRef.current) workbenchRef.current.scrollTop = 0;
  }, [activeSection, activeWorkspace?.workspaceId, selectedSessionId]);

  return (
    <div className="relative flex h-screen min-w-[980px] flex-col overflow-hidden bg-canvas text-ink antialiased">
      <WindowCommandRow
        leftOpen={leftOpen}
        rightOpen={rightOpen}
        terminalOpen={terminalOpen}
        changesOpen={changesOpen}
        hasWorkspace={Boolean(activeWorkspace)}
        workspace={activeWorkspace}
        onToggleLeft={() => setLeftOpen((open) => !open)}
        onToggleRight={() => { const next = !rightOpen; setRightOpen(next); if (next) setChangesOpen(false); }}
        onToggleTerminal={() => setTerminalOpen((open) => !open)}
        onToggleChanges={() => { const next = !changesOpen; setChangesOpen(next); if (next) setRightOpen(false); }}
        openTargets={repositoryOpen.targets}
        onOpenTarget={repositoryOpen.openTarget}
      />
      {activeWorkspace && changesOpen ? <ToolbarChangesPopover workspace={activeWorkspace} onClose={() => setChangesOpen(false)} onOpenChanges={() => { setChangesOpen(false); onNavigate?.("changes"); }} /> : null}
      <div className="grid min-h-0 flex-1 overflow-hidden dl-shell-motion" data-testid="desktoplab-shell" style={{ gridTemplateColumns: columns }}>
        <AppDrawer
          title={title}
          open={leftOpen}
          activeSection={activeSection}
          activeWorkspace={activeWorkspace}
          workspaces={workspaces}
          projectThreads={pinnedThreads}
          projectThreadsByWorkspace={projectThreadsByWorkspace}
          projectThreadsStatusByWorkspace={projectThreadsStatusByWorkspace}
          selectedSessionId={selectedSessionId}
          onNewChat={onNewChat}
          onSelectProject={onSelectProject}
          onSelectThread={onSelectThread}
          onArchiveWorkspace={onArchiveWorkspace}
          onRelinkWorkspace={onRelinkWorkspace}
          onArchiveThread={onArchiveThread}
          onNavigate={onNavigate}
        />
        {leftOpen ? (
          <ResizeHandle
            label="Resize left drawer"
            onKeyStep={(delta) => setLeftWidth(leftWidth + delta)}
            onStart={(clientX) => startResize("left", clientX, leftWidth, setLeftWidth)}
          />
        ) : null}
        <main aria-label="Workspace" data-testid="desktoplab-main" className="flex h-full min-w-0 flex-col overflow-hidden">
          <section ref={workbenchRef} data-testid="workbench-scroll-region" className="min-h-0 flex-1 overflow-auto px-6 py-6 dl-content-enter">{children}</section>
          {activeWorkspace && terminalOpen ? <TerminalResizeHandle startHeight={terminalHeight} setHeight={setTerminalHeight} /> : null}
          {activeWorkspace ? (
            <TerminalDrawer
              open={terminalOpen}
              height={terminalHeight}
              response={terminalReplay.response}
              eventFrames={terminalReplay.frames}
              workspacePath={activeWorkspace.rootPath}
              sessionLabel={selectedSessionId ? "Agent session" : null}
              onRunCommand={(request) => api.createTerminalCommand(activeWorkspace.workspaceId, request)}
              onApprove={(approvalId) => void resolveTerminalApproval(approvalId, "approve")}
              onDeny={(approvalId) => void resolveTerminalApproval(approvalId, "deny")}
              onClose={() => setTerminalOpen(false)}
            />
          ) : null}
        </main>
        {showInspector ? (
          <ResizeHandle
            label="Resize right drawer"
            onKeyStep={(delta) => setRightWidth(rightWidth - delta)}
            onStart={(clientX) => startResize("right", clientX, rightWidth, setRightWidth)}
          />
        ) : null}
        {showInspector ? <div className="h-full min-w-0 overflow-hidden dl-pane-enter"><RepositoryInspector workspace={activeWorkspace!} onNavigate={onNavigate} onClose={() => setRightOpen(false)} /></div> : null}
      </div>
    </div>
  );
}
