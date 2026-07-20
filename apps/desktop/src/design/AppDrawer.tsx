import { useState } from "react";
import {
  ChevronRight,
  FolderGit2,
  LayoutList,
  MessageSquarePlus,
} from "./icons";
import type { AppRoute } from "../app/routes";
import { readDrawerPinnedItems, toggleDrawerPinnedItem, writeDrawerPinnedItems, type DrawerPinnedItem } from "../api/appState";
import type { AgentSessionSnapshot, WorkspaceSnapshot } from "../api/types";
import { PinnedItems, ProjectTree } from "./AppDrawerProjects";
import type { DrawerThreadsStatus } from "./useDrawerProjectThreads";
import { ControlCenter, DrawerSection, NavItem, isSupportRoute } from "./AppDrawerNavigation";

type AppDrawerProps = {
  title: string;
  open: boolean;
  activeSection: AppRoute;
  activeWorkspace: WorkspaceSnapshot | null;
  workspaces?: WorkspaceSnapshot[];
  projectThreads?: AgentSessionSnapshot[];
  projectThreadsByWorkspace?: Record<string, AgentSessionSnapshot[]>;
  projectThreadsStatusByWorkspace?: Record<string, DrawerThreadsStatus>;
  selectedSessionId?: string | null;
  onNewChat?: () => void;
  onSelectProject?: (workspace: WorkspaceSnapshot) => void;
  onSelectThread?: (session: AgentSessionSnapshot) => void;
  onArchiveWorkspace?: (workspaceId: string) => Promise<void> | void;
  onRelinkWorkspace?: (workspace: WorkspaceSnapshot) => Promise<void> | void;
  onArchiveThread?: (sessionId: string) => Promise<void> | void;
  onNavigate?: (section: AppRoute) => void;
};

export function AppDrawer({
  title,
  open,
  activeSection,
  activeWorkspace,
  workspaces = activeWorkspace ? [activeWorkspace] : [],
  projectThreads = [],
  projectThreadsByWorkspace = {},
  projectThreadsStatusByWorkspace = {},
  selectedSessionId = null,
  onNewChat,
  onSelectProject,
  onSelectThread,
  onArchiveWorkspace,
  onRelinkWorkspace,
  onArchiveThread,
  onNavigate,
}: AppDrawerProps) {
  const [pinnedItems, setPinnedItems] = useState<DrawerPinnedItem[]>(() => readDrawerPinnedItems());
  const setPins = (items: DrawerPinnedItem[]) => {
    setPinnedItems(items);
    writeDrawerPinnedItems(items);
  };
  const togglePinnedItem = (item: DrawerPinnedItem) => setPins(toggleDrawerPinnedItem(pinnedItems, item));
  const recentWorkspaces = [...workspaces].sort((left, right) =>
    workspaceRecency(projectThreadsByWorkspace[right.workspaceId] ?? []) - workspaceRecency(projectThreadsByWorkspace[left.workspaceId] ?? []),
  );
  const orderedWorkspaces = activeWorkspace
    ? [
        activeWorkspace,
        ...recentWorkspaces.filter((workspace) => workspace.workspaceId !== activeWorkspace.workspaceId),
      ]
    : recentWorkspaces;

  return (
    <aside data-testid="app-drawer" className={`flex h-full overflow-hidden flex-col border-r border-line ${open ? "px-3" : "px-2"} py-3 dl-panel dl-pane-motion`}>
      <DrawerHeader title={title} open={open} />

      <div className="min-h-0 flex-1 overflow-auto">
        <nav aria-label="Primary" className="mt-7 space-y-1">
          <NavItem compact={!open} icon={<MessageSquarePlus size={16} />} label="New chat" active={activeSection === "agent"} onClick={onNewChat} />
          <NavItem compact={!open} icon={<FolderGit2 size={16} />} label="Open project" active={activeSection === "workspaces"} onClick={() => onNavigate?.("workspaces")} />
        </nav>

        <DrawerSection label="Pinned" open={open}>
          <PinnedItems compact={!open} items={pinnedItems} threads={projectThreads} onNavigate={onNavigate} onSelectThread={onSelectThread} onTogglePin={togglePinnedItem} />
        </DrawerSection>

        <DrawerSection label="Projects" open={open}>
          {orderedWorkspaces.length > 0 ? (
            orderedWorkspaces.map((workspace) => (
              <ProjectTree
                key={workspace.workspaceId}
                compact={!open}
                workspace={workspace}
                threads={projectThreadsByWorkspace[workspace.workspaceId] ?? []}
                threadsStatus={projectThreadsStatusByWorkspace[workspace.workspaceId] ?? "loading"}
                active={activeSection === "agent" && workspace.workspaceId === activeWorkspace?.workspaceId}
                selectedSessionId={selectedSessionId}
                pinnedItems={pinnedItems}
                onTogglePin={togglePinnedItem}
                onSelectProject={onSelectProject}
                onSelectThread={onSelectThread}
                onArchiveProject={onArchiveWorkspace}
                onRelinkProject={onRelinkWorkspace}
                onArchiveThread={onArchiveThread}
                onNavigate={onNavigate}
              />
            ))
          ) : (
            open ? <p className="px-2 py-2 text-xs text-muted/70">No projects yet</p> : null
          )}
        </DrawerSection>
      </div>

      <details className="mt-3 shrink-0" open={open && isSupportRoute(activeSection) ? true : undefined}>
        <summary className="flex cursor-pointer list-none items-center gap-2 rounded-desktop px-2 py-2 text-sm font-medium text-muted transition-colors duration-150 hover:bg-elevated/65 hover:text-ink">
          <LayoutList size={16} />
          {open ? (
            <>
              <span className="flex-1">Control center</span>
              <ChevronRight size={14} />
            </>
          ) : null}
        </summary>
        {open ? <ControlCenter activeSection={activeSection} onNavigate={onNavigate} /> : null}
      </details>
    </aside>
  );
}

function workspaceRecency(threads: AgentSessionSnapshot[]): number {
  return threads.reduce((latest, thread) => {
    const value = Number(thread.sessionId.split(".").at(-1));
    return Number.isFinite(value) ? Math.max(latest, value) : latest;
  }, 0);
}

function DrawerHeader({ title, open }: { title: string; open: boolean }) {
  if (!open) {
    return (
      <div className="flex flex-col items-center gap-2">
        <LogoMark />
      </div>
    );
  }

  return (
    <div className="flex h-10 items-center gap-2">
      <LogoMark />
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-semibold">{title}</div>
        <div className="text-xs text-muted">Local coding agents</div>
      </div>
    </div>
  );
}

function LogoMark() {
  return (
    <div className="grid h-8 w-8 shrink-0 place-items-center rounded-desktop bg-[linear-gradient(135deg,rgb(var(--dl-color-accent)),rgb(123_109_240))] text-[13px] font-semibold text-white shadow-[var(--dl-accent-glow)]">
      DL
    </div>
  );
}
