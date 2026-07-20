import { useQuery } from "@tanstack/react-query";
import { X } from "./icons";
import { useApiClient } from "../api/ApiProvider";
import type { WorkspaceSnapshot } from "../api/types";

type ToolbarChangesPopoverProps = {
  workspace: WorkspaceSnapshot;
  onClose: () => void;
  onOpenChanges: () => void;
};

export function ToolbarChangesPopover({ workspace, onClose, onOpenChanges }: ToolbarChangesPopoverProps) {
  const api = useApiClient();
  const git = useQuery({
    queryKey: ["toolbar-changes", workspace.workspaceId, workspace.rootPath],
    queryFn: () => api.gitOperations(workspace.workspaceId),
  });
  const statusEntries = git.data?.statusEntries ?? workspace.statusEntries;
  const changedFiles = git.data?.changedFiles ?? statusFiles(workspace.statusEntries);
  const title = git.data?.warnings[0] ?? stateLabel(git.data?.workspaceState ?? workspace.apiState);
  const stats = diffStats(git.data?.diffPreview ?? workspace.diffText);

  return (
    <aside
      aria-label="Toolbar changes"
      className="absolute right-3 top-11 z-40 w-80 rounded-desktop border border-line bg-panel/95 p-3 text-sm shadow-panel backdrop-blur"
      role="complementary"
    >
      <div className="mb-3 flex items-start justify-between gap-3">
        <div>
          <p className="text-xs font-semibold text-muted">Changes</p>
          <p className="font-semibold text-ink">{title}</p>
        </div>
        <button
          type="button"
          aria-label="Close changes panel"
          className="grid h-7 w-7 place-items-center rounded-desktop text-muted hover:bg-elevated hover:text-ink"
          onClick={onClose}
        >
          <X size={14} />
        </button>
      </div>
      {changedFiles.length > 0 ? (
        <ul className="grid max-h-52 gap-1 overflow-auto">
          {changedFiles.slice(0, 8).map((file) => (
            <li key={file}>
              <button type="button" className="flex w-full items-center gap-2 rounded-[6px] bg-elevated/60 px-2 py-1.5 text-left text-xs text-ink hover:bg-elevated" onClick={onOpenChanges}>
                <span className="w-4 shrink-0 font-semibold text-warning">{fileStatus(file, statusEntries)}</span>
                <span className="min-w-0 flex-1 truncate">{file}</span>
              </button>
            </li>
          ))}
        </ul>
      ) : (
        <p className="text-xs text-muted">No local changes.</p>
      )}
      {stats ? <p className="mt-2 text-xs text-muted"><span className="text-success">+{stats.added}</span> · <span className="text-danger">-{stats.removed}</span></p> : null}
    </aside>
  );
}

function fileStatus(file: string, entries: string[]): string {
  const entry = entries.find((value) => value.endsWith(file))?.trimStart().toLowerCase() ?? "modified";
  if (entry.startsWith("??") || entry.startsWith("untracked")) return "?";
  const code = entry.slice(0, 2);
  if (entry.startsWith("added") || code.includes("a")) return "A";
  if (entry.startsWith("deleted") || code.includes("d")) return "D";
  if (entry.startsWith("renamed") || code.includes("r")) return "R";
  return "M";
}

function diffStats(diff: string | undefined): { added: number; removed: number } | null {
  if (!diff) return null;
  const lines = diff.split("\n");
  return {
    added: lines.filter((line) => line.startsWith("+") && !line.startsWith("+++")).length,
    removed: lines.filter((line) => line.startsWith("-") && !line.startsWith("---")).length,
  };
}

function statusFiles(entries: string[]) {
  return entries.map((entry) => entry.replace(/^[^:]+:\s*/, "")).filter(Boolean);
}

function stateLabel(state: WorkspaceSnapshot["apiState"] | "conflicted") {
  if (state === "dirty") return "Dirty worktree";
  if (state === "conflicted") return "Conflicts detected";
  return "Clean worktree";
}
