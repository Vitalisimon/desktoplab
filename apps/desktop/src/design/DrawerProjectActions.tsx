import { useState, type ReactNode } from "react";
import type { DrawerPinnedItem } from "../api/appState";
import { Archive, FolderOpen, MoreHorizontal, Pin, PinOff } from "./icons";

export function DrawerProjectActions({
  item,
  pinned,
  relinkAvailable,
  onTogglePin,
  onRelink,
  onArchive,
}: {
  item: DrawerPinnedItem;
  pinned: boolean;
  relinkAvailable: boolean;
  onTogglePin: (item: DrawerPinnedItem) => void;
  onRelink: () => void;
  onArchive: () => void;
}) {
  const [open, setOpen] = useState(false);
  return (
    <div className="relative">
      <button type="button" aria-label={`Project actions for ${item.label}`} aria-expanded={open} aria-haspopup="menu" className="grid h-7 w-7 place-items-center rounded-desktop text-muted hover:bg-elevated hover:text-ink" onClick={(event) => { event.stopPropagation(); setOpen((value) => !value); }}>
        <MoreHorizontal size={14} />
      </button>
      {open ? (
        <div role="menu" aria-label={`Actions for ${item.label}`} className="absolute right-0 top-8 z-30 w-40 rounded-desktop border border-line bg-elevated p-1 shadow-panel">
          <Action icon={pinned ? <PinOff size={14} /> : <Pin size={14} />} label={pinned ? "Unpin project" : "Pin project"} onClick={() => { onTogglePin(item); setOpen(false); }} />
          {relinkAvailable ? <Action icon={<FolderOpen size={14} />} label="Relink project" onClick={() => { onRelink(); setOpen(false); }} /> : null}
          <Action icon={<Archive size={14} />} label="Archive project" onClick={() => { onArchive(); setOpen(false); }} />
        </div>
      ) : null}
    </div>
  );
}

function Action({ icon, label, onClick }: { icon: ReactNode; label: string; onClick: () => void }) {
  return <button type="button" role="menuitem" className="flex w-full items-center gap-2 rounded-[6px] px-2 py-2 text-left text-xs text-ink hover:bg-muted/10" onClick={(event) => { event.stopPropagation(); onClick(); }}>{icon}{label}</button>;
}
