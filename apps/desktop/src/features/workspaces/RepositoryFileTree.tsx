import { useQuery } from "@tanstack/react-query";
import { ChevronDown, ChevronRight, FileText, Folder, Lock } from "../../design/icons";
import { useState } from "react";
import { useApiClient } from "../../api/ApiProvider";
import type { WorkspaceFileTreeEntry } from "../../api/types";
import { FilePreviewPanel } from "./FilePreviewPanel";

type RepositoryFileTreeProps = {
  workspaceId: string;
};

type RepositoryTreeNode = {
  entry: WorkspaceFileTreeEntry;
  name: string;
  children: RepositoryTreeNode[];
};

export function RepositoryFileTree({ workspaceId }: RepositoryFileTreeProps) {
  const api = useApiClient();
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [treeOpen, setTreeOpen] = useState(true);
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(() => new Set());
  const tree = useQuery({
    queryKey: ["workspace-files", workspaceId],
    queryFn: () => api.listWorkspaceFiles(workspaceId),
    enabled: workspaceId.length > 0,
  });
  const preview = useQuery({
    queryKey: ["workspace-file-preview", workspaceId, selectedPath],
    queryFn: () => api.previewWorkspaceFile(workspaceId, selectedPath ?? ""),
    enabled: Boolean(selectedPath),
  });

  if (tree.isLoading) return <p className="text-sm text-muted">Reading repository...</p>;
  if (tree.isError || !tree.data) return <p className="text-sm text-muted">Repository files unavailable.</p>;
  const selectFile = (path: string) => {
    setSelectedPath(path);
    setTreeOpen(false);
  };
  const treeNodes = buildRepositoryTree(tree.data.entries);
  const toggleFolder = (path: string) => {
    setExpandedPaths((previous) => {
      const next = new Set(previous);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <FilePreviewPanel path={selectedPath} preview={preview.data} loading={preview.isFetching} onClose={() => setSelectedPath(null)} />
      <section
        data-testid="repository-tree-panel"
        className={`shrink-0 overflow-hidden rounded-desktop border border-line bg-panel/70 ${treeOpen ? "h-64" : ""}`}
        onKeyDown={(event) => {
          if (event.key === "Escape" && treeOpen) {
            event.stopPropagation();
            setTreeOpen(false);
          }
        }}
      >
        <button type="button" className="flex h-10 w-full items-center gap-2 px-3 text-left text-xs font-semibold text-ink" aria-label={treeOpen ? "Hide repository tree" : "Show repository tree"} onClick={() => setTreeOpen((open) => !open)}>
          {treeOpen ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          <span className="flex-1">Repository tree</span>
        </button>
        <div
          data-testid="repository-tree-subdrawer"
          aria-hidden={!treeOpen}
          className={treeOpen ? "h-[calc(16rem-2.5rem)] overflow-auto px-2 pb-2" : "hidden"}
        >
          <div className="grid gap-1">
            {treeNodes.map((node) => (
              <RepositoryFileNode
                key={node.entry.path}
                node={node}
                depth={0}
                expandedPaths={expandedPaths}
                selectedPath={selectedPath}
                onSelect={selectFile}
                onToggle={toggleFolder}
              />
            ))}
          </div>
          {tree.data.degraded ? <p className="mt-3 text-xs leading-5 text-muted">Large repository scan limited.</p> : null}
        </div>
      </section>
    </div>
  );
}

function RepositoryFileNode({
  node,
  depth,
  expandedPaths,
  selectedPath,
  onSelect,
  onToggle,
}: {
  node: RepositoryTreeNode;
  depth: number;
  expandedPaths: Set<string>;
  selectedPath: string | null;
  onSelect: (path: string) => void;
  onToggle: (path: string) => void;
}) {
  const { entry } = node;
  const protectedEntry = entry.protection === "protected";
  const fileEntry = entry.kind !== "directory";
  const expanded = expandedPaths.has(entry.path);
  const selected = entry.path === selectedPath;
  const label = node.name;
  const indent = { paddingLeft: `${8 + depth * 14}px` };
  const icon = fileEntry ? protectedEntry ? <Lock size={14} /> : <FileText size={14} /> : <Folder size={14} />;
  if (!fileEntry) {
    return (
      <>
        <button
          type="button"
          className="flex h-8 items-center gap-2 rounded-md pr-2 text-left text-sm text-muted transition-colors hover:bg-elevated/70 hover:text-ink"
          style={indent}
          aria-expanded={expanded}
          onClick={() => onToggle(entry.path)}
        >
          <span className="shrink-0">{expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}</span>
          <span className="shrink-0">{icon}</span>
          <span className="min-w-0 flex-1 truncate">{label}</span>
        </button>
        {expanded
          ? node.children.map((child) => (
              <RepositoryFileNode
                key={child.entry.path}
                node={child}
                depth={depth + 1}
                expandedPaths={expandedPaths}
                selectedPath={selectedPath}
                onSelect={onSelect}
                onToggle={onToggle}
              />
            ))
          : null}
      </>
    );
  }
  if (protectedEntry) {
    return (
      <div className="flex h-8 items-center gap-2 rounded-md pr-2 text-sm text-muted" style={indent}>
        <span className="shrink-0">{icon}</span>
        <span className="min-w-0 flex-1 truncate">{label}</span>
        <span className="text-[11px] font-medium text-muted">Protected</span>
      </div>
    );
  }

  return (
    <button
      type="button"
      className={`flex h-8 items-center gap-2 rounded-md pr-2 text-left text-sm transition-colors ${selected ? "bg-elevated text-ink" : "text-muted hover:bg-elevated/70 hover:text-ink"}`}
      style={indent}
      onClick={() => onSelect(entry.path)}
      onDoubleClick={() => onSelect(entry.path)}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onSelect(entry.path);
        }
      }}
    >
      <span className="shrink-0">{icon}</span>
      <span className="min-w-0 flex-1 truncate">{label}</span>
    </button>
  );
}

function buildRepositoryTree(entries: WorkspaceFileTreeEntry[]): RepositoryTreeNode[] {
  const byPath = new Map<string, RepositoryTreeNode>();
  const roots: RepositoryTreeNode[] = [];
  const ensureDirectory = (path: string): RepositoryTreeNode => {
    const existing = byPath.get(path);
    if (existing) return existing;
    const node: RepositoryTreeNode = {
      entry: { path, kind: "directory", protection: "readable" },
      name: basename(path),
      children: [],
    };
    byPath.set(path, node);
    const parent = parentPath(path);
    if (parent) ensureDirectory(parent).children.push(node);
    else roots.push(node);
    return node;
  };
  for (const entry of entries) {
    const parent = parentPath(entry.path);
    if (parent) ensureDirectory(parent);
    const existing = byPath.get(entry.path);
    if (existing) existing.entry = entry;
    else {
      const node = { entry, name: basename(entry.path), children: [] };
      byPath.set(entry.path, node);
      if (parent) ensureDirectory(parent).children.push(node);
      else roots.push(node);
    }
  }
  sortNodes(roots);
  return roots;
}

function sortNodes(nodes: RepositoryTreeNode[]) {
  nodes.sort((left, right) => {
    if (left.entry.kind === "directory" && right.entry.kind !== "directory") return -1;
    if (left.entry.kind !== "directory" && right.entry.kind === "directory") return 1;
    return left.name.localeCompare(right.name);
  });
  nodes.forEach((node) => sortNodes(node.children));
}

function basename(path: string): string {
  return path.split("/").filter(Boolean).at(-1) ?? path;
}

function parentPath(path: string): string | null {
  const parts = path.split("/").filter(Boolean);
  if (parts.length <= 1) return null;
  return parts.slice(0, -1).join("/");
}
