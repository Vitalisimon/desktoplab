import { useState, type ReactNode } from "react";
import { ChevronDown, ChevronRight, FolderGit2, MessageSquare, ShieldQuestion, XCircle } from "./icons";
import type { AppRoute } from "../app/routes";
import { isDrawerPinned, type DrawerPinnedItem } from "../api/appState";
import type { AgentSessionSnapshot, WorkspaceSnapshot } from "../api/types";
import { NavItem, PinToggleButton } from "./AppDrawerNavigation";
import type { DrawerThreadsStatus } from "./useDrawerProjectThreads";
import { DrawerThreadActions } from "./DrawerThreadActions";
import { DrawerProjectActions } from "./DrawerProjectActions";

export function PinnedItems({
  compact,
  items,
  threads,
  onNavigate,
  onSelectThread,
  onTogglePin,
}: {
  compact: boolean;
  items: DrawerPinnedItem[];
  threads: AgentSessionSnapshot[];
  onNavigate?: (section: AppRoute) => void;
  onSelectThread?: (session: AgentSessionSnapshot) => void;
  onTogglePin: (item: DrawerPinnedItem) => void;
}) {
  if (items.length === 0) return compact ? null : <p className="px-2 py-2 text-xs text-muted/70">No pinned items</p>;
  return (
    <nav aria-label="Pinned items" className="space-y-1">
      {items.map((item) => (
        <NavItem
          key={item.id}
          compact={compact}
          icon={item.type === "project" ? <FolderGit2 size={16} /> : <MessageSquare size={16} />}
          label={item.label}
          onClick={() => {
            if (item.type === "thread") {
              const session = threads.find((thread) => `thread:${thread.sessionId}` === item.id);
              if (session) onSelectThread?.(session);
            }
            onNavigate?.("agent");
          }}
          trailingAction={compact ? null : <PinToggleButton pinned={true} item={item} labelPrefix="Remove pinned" onTogglePin={onTogglePin} />}
        />
      ))}
    </nav>
  );
}

export function ProjectTree({
  compact,
  workspace,
  threads,
  threadsStatus,
  active,
  selectedSessionId,
  pinnedItems,
  onTogglePin,
  onSelectProject,
  onSelectThread,
  onArchiveProject,
  onRelinkProject,
  onArchiveThread,
  onNavigate,
}: {
  compact: boolean;
  workspace: WorkspaceSnapshot;
  threads: AgentSessionSnapshot[];
  threadsStatus: DrawerThreadsStatus;
  active: boolean;
  selectedSessionId: string | null;
  pinnedItems: DrawerPinnedItem[];
  onTogglePin: (item: DrawerPinnedItem) => void;
  onSelectProject?: (workspace: WorkspaceSnapshot) => void;
  onSelectThread?: (session: AgentSessionSnapshot) => void;
  onArchiveProject?: (workspaceId: string) => Promise<void> | void;
  onRelinkProject?: (workspace: WorkspaceSnapshot) => Promise<void> | void;
  onArchiveThread?: (sessionId: string) => Promise<void> | void;
  onNavigate?: (section: AppRoute) => void;
}) {
  const projectName = workspaceDisplayName(workspace);
  const projectItem = projectPinItem(workspace, projectName);
  const [expanded, setExpanded] = useState(true);
  return (
    <div>
      <div className="flex items-center gap-1">
        {compact ? null : (
          <button
            type="button"
            aria-label={`${expanded ? "Collapse" : "Expand"} project ${projectName}`}
            className="grid h-7 w-7 shrink-0 place-items-center rounded-md text-muted hover:bg-elevated hover:text-ink"
            onClick={() => setExpanded((open) => !open)}
          >
            {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          </button>
        )}
        <NavItem
          compact={compact}
          icon={<FolderGit2 size={16} />}
          label={projectName}
          active={active && !selectedSessionId}
          onClick={() => {
            onSelectProject?.(workspace);
            onNavigate?.("agent");
          }}
          trailingAction={
            compact ? null : (
              <DrawerProjectActions
                item={projectItem}
                pinned={isDrawerPinned(pinnedItems, projectItem.id)}
                relinkAvailable={Boolean(workspace.readOnly || workspace.stale || workspace.rootExists === false)}
                onTogglePin={onTogglePin}
                onRelink={() => onRelinkProject?.(workspace)}
                onArchive={() => onArchiveProject?.(workspace.workspaceId)}
              />
            )
          }
        />
      </div>
      {compact || !expanded ? null : (
        <ThreadList
          threads={threads}
          status={threadsStatus}
          selectedSessionId={selectedSessionId}
          pinnedItems={pinnedItems}
          onTogglePin={onTogglePin}
          onSelectThread={onSelectThread}
          onArchiveThread={onArchiveThread}
          onNavigate={onNavigate}
        />
      )}
    </div>
  );
}

function ThreadList({
  threads,
  status,
  selectedSessionId,
  pinnedItems,
  onTogglePin,
  onSelectThread,
  onArchiveThread,
  onNavigate,
}: {
  threads: AgentSessionSnapshot[];
  status: DrawerThreadsStatus;
  selectedSessionId: string | null;
  pinnedItems: DrawerPinnedItem[];
  onTogglePin: (item: DrawerPinnedItem) => void;
  onSelectThread?: (session: AgentSessionSnapshot) => void;
  onArchiveThread?: (sessionId: string) => Promise<void> | void;
  onNavigate?: (section: AppRoute) => void;
}) {
  const [showAll, setShowAll] = useState(false);
  if (threads.length === 0 && status === "loading") return <ThreadListStatus>Loading threads...</ThreadListStatus>;
  if (threads.length === 0 && status === "error") return <ThreadListStatus>Threads unavailable</ThreadListStatus>;
  if (threads.length === 0) return <ThreadListStatus>No threads yet</ThreadListStatus>;
  const visibleThreads = showAll ? threads : threads.slice(0, 6);
  const hiddenCount = threads.length - visibleThreads.length;
  return (
    <div className="ml-7 mt-1 space-y-1">
      {visibleThreads.map((thread) => {
        const threadItem = threadPinItem(thread);
        return (
          <NavItem
            key={thread.sessionId}
            compact={false}
            icon={<MessageSquare size={14} />}
            label={threadItem.label}
            active={selectedSessionId === thread.sessionId}
            status={threadStatus(thread)}
            onClick={() => {
              onSelectThread?.(thread);
              onNavigate?.("agent");
            }}
            trailingAction={
              <DrawerThreadActions item={threadItem} pinned={isDrawerPinned(pinnedItems, threadItem.id)} onTogglePin={onTogglePin} onArchive={() => onArchiveThread?.(thread.sessionId)} />
            }
          />
        );
      })}
      {hiddenCount > 0 ? (
        <button type="button" className="h-8 px-2 text-xs font-medium text-muted hover:text-ink" onClick={() => setShowAll(true)}>
          Show {hiddenCount} older {hiddenCount === 1 ? "thread" : "threads"}
        </button>
      ) : showAll && threads.length > 6 ? (
        <button type="button" className="h-8 px-2 text-xs font-medium text-muted hover:text-ink" onClick={() => setShowAll(false)}>
          Show recent only
        </button>
      ) : null}
    </div>
  );
}

function threadStatus(thread: AgentSessionSnapshot): { label: string; className?: string; active?: boolean; icon?: ReactNode } | null {
  if (thread.state === "created" || thread.state === "planning" || thread.state === "running") return { label: "Running", className: "bg-accent", active: true };
  if (thread.state === "blocked" && (thread.pendingApprovals?.length ?? 0) > 0) return { label: "Approval required", icon: <ShieldQuestion className="text-warning" size={14} /> };
  if (thread.state === "blocked" && thread.timeline.some(isClarificationEvent)) return { label: "Input required", icon: <MessageSquare className="text-accent" size={14} /> };
  if (thread.state === "blocked") return { label: "Blocked", icon: <XCircle className="text-muted" size={14} /> };
  if (thread.state === "failed") return { label: "Failed", icon: <XCircle className="text-danger" size={14} /> };
  return null;
}

function isClarificationEvent(event: AgentSessionSnapshot["timeline"][number]): boolean {
  return event.kind.toLowerCase().includes("clarif") || event.message.trim().toLowerCase().startsWith("clarification_required:");
}

function ThreadListStatus({ children }: { children: string }) {
  return <p className="ml-7 mt-1 px-2 py-1.5 text-xs text-muted/60">{children}</p>;
}

function projectPinItem(workspace: WorkspaceSnapshot, label = workspaceDisplayName(workspace)): DrawerPinnedItem {
  return { id: `project:${workspace.workspaceId}`, type: "project", label };
}

function threadPinItem(thread: AgentSessionSnapshot): DrawerPinnedItem {
  return { id: `thread:${thread.sessionId}`, type: "thread", label: threadTitle(thread) };
}

function threadTitle(thread: AgentSessionSnapshot): string {
  const userPrompt = thread.transcript?.find((turn) => turn.role === "user")?.content.trim();
  const candidate = userPrompt || thread.plan?.trim() || thread.summary?.trim();
  if (!candidate || /^(agent loop|response complete|working|waiting for approval)/i.test(candidate)) return "Untitled task";
  const firstLine = candidate.split(/\r?\n/)[0].replace(/\s+/g, " ").trim();
  const title = firstLine.length > 44 ? `${firstLine.slice(0, 41)}...` : firstLine;
  const time = threadTimeLabel(thread.sessionId);
  return time ? `${title} · ${time}` : title;
}

function threadTimeLabel(sessionId: string): string | null {
  const raw = Number(sessionId.split(".").at(-1));
  if (!Number.isFinite(raw) || raw < 1_000_000_000) return null;
  const date = new Date(raw < 10_000_000_000 ? raw * 1000 : raw);
  return Number.isNaN(date.getTime()) ? null : date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function workspaceDisplayName(workspace: WorkspaceSnapshot): string {
  const explicit = workspace.displayName.trim();
  if (explicit.length > 0) return explicit;
  const normalized = workspace.rootPath.replace(/\\/g, "/").replace(/\/+$/, "");
  return normalized.split("/").filter(Boolean).pop() ?? "Repository";
}
