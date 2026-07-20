import { useState, type KeyboardEvent } from "react";
import type { RepositoryOpenTarget } from "./repositoryOpen";
import { ChevronDown, Code2, FolderOpen } from "./icons";

export function RepositoryOpenMenu({ targets, onOpen }: { targets: RepositoryOpenTarget[]; onOpen: (targetId: string) => void }) {
  const [open, setOpen] = useState(false);
  const closeOnEscape = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") setOpen(false);
  };
  return (
    <div className="relative" onKeyDown={closeOnEscape}>
      <button
        type="button"
        aria-label="Open repository in"
        aria-expanded={open}
        aria-haspopup="menu"
        title="Open repository in"
        data-chrome-control="true"
        className="inline-flex h-7 items-center gap-1.5 rounded-[6px] px-2 text-xs font-medium text-muted transition-colors duration-150 hover:bg-elevated/70 hover:text-ink disabled:opacity-50"
        disabled={targets.length === 0}
        onClick={() => setOpen((value) => !value)}
      >
        <FolderOpen size={14} />
        <span>Open in</span>
        <ChevronDown size={12} className={`transition-transform ${open ? "rotate-180" : ""}`} />
      </button>
      {open ? (
        <div role="menu" aria-label="Open repository in" className="absolute right-0 top-8 z-40 w-52 rounded-desktop border border-line bg-elevated p-1 shadow-[0_18px_48px_rgba(15,23,42,0.18)]">
          {targets.map((target) => (
            <button key={target.id} type="button" role="menuitem" className="flex w-full items-center gap-2 rounded-[6px] px-3 py-2 text-left text-sm text-ink hover:bg-muted/10" onClick={() => { onOpen(target.id); setOpen(false); }}>
              {target.kind === "ide" ? <Code2 size={14} /> : <FolderOpen size={14} />}
              <span>{target.label}</span>
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}
