import { Activity, FileText, FolderGit2, GitPullRequestArrow, Network, X } from "./icons";
import { useState } from "react";
import type { AppRoute } from "../app/routes";
import type { WorkspaceSnapshot } from "../api/types";
import { RepositoryFileTree } from "../features/workspaces/RepositoryFileTree";
import { InspectorEmptyState } from "./InspectorEmptyState";

type RepositoryInspectorProps = {
  workspace: WorkspaceSnapshot;
  onNavigate?: (section: AppRoute) => void;
  onClose?: () => void;
};

type InspectorMode = "files" | "context" | "changes" | "activity";

const inspectorModes: Array<{ id: InspectorMode; label: string; icon: typeof FileText }> = [
  { id: "files", label: "Files", icon: FileText },
  { id: "context", label: "Context", icon: Network },
  { id: "changes", label: "Changes", icon: GitPullRequestArrow },
  { id: "activity", label: "Activity", icon: Activity },
];

export function RepositoryInspector({ workspace, onNavigate, onClose }: RepositoryInspectorProps) {
  const [mode, setMode] = useState<InspectorMode>("files");
  const statusLabel = workspace.apiState === "clean" ? "Clean" : "Changes found";
  return (
    <aside className="flex h-full min-h-0 flex-col overflow-hidden border-l border-line bg-panel px-4 py-4" aria-label="Repository inspector">
      <div className="flex shrink-0 items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold">Repository inspector</h2>
          <p className="mt-1 text-xs text-muted">{statusLabel}</p>
        </div>
        <div className="flex items-center gap-2">
          <FolderGit2 size={17} className="text-muted" />
          <button type="button" aria-label="Close repository inspector" className="grid h-8 w-8 place-items-center rounded-desktop border border-line text-muted hover:text-ink" onClick={onClose}>
            <X size={15} />
          </button>
        </div>
      </div>
      <nav aria-label="Inspector modes" className="mt-4 grid shrink-0 grid-cols-4 gap-1 rounded-desktop border border-line bg-canvas/50 p-1">
        {inspectorModes.map((item) => {
          const Icon = item.icon;
          const selected = mode === item.id;
          return (
            <button
              key={item.id}
              type="button"
              aria-pressed={selected}
              className={`flex h-8 items-center justify-center gap-1.5 rounded-md text-xs font-medium transition-colors ${selected ? "bg-elevated text-ink shadow-sm" : "text-muted hover:bg-elevated/70 hover:text-ink"}`}
              onClick={() => setMode(item.id)}
            >
              <Icon size={13} />
              <span>{item.label}</span>
            </button>
          );
        })}
      </nav>
      <div data-testid="repository-inspector-scroll-region" className="mt-4 min-h-0 flex-1 overflow-auto">
        <div className={mode === "files" ? "h-full min-h-0" : "hidden"}>
          <RepositoryFileTree workspaceId={workspace.workspaceId} />
        </div>
        {mode === "context" ? (
          <InspectorEmptyState
            title="Workspace context"
            description="Review repository languages, package managers, protected paths and refresh status."
            actionLabel="View context"
            onAction={() => onNavigate?.("context")}
          />
        ) : null}
        {mode === "changes" ? (
          <InspectorEmptyState
            title="Repository changes"
            description="Review current file changes and diffs for this repository."
            actionLabel="Review changes"
            onAction={() => onNavigate?.("changes")}
          />
        ) : null}
        {mode === "activity" ? (
          <InspectorEmptyState
            title="Workspace activity"
            description="Agent and terminal activity will appear here."
          />
        ) : null}
      </div>
    </aside>
  );
}
