import { ChevronDown, ShieldCheck } from "../../design/icons";
import { useState, type KeyboardEvent } from "react";
import type { ApprovalMode, ApprovalModeDescriptor } from "../../api/types";
import { displayApprovalMode } from "../../domain/displayNames";

type ApprovalModeMenuProps = {
  modes: ApprovalModeDescriptor[];
  selectedMode: ApprovalMode | null;
  disabled?: boolean;
  onSelect: (mode: ApprovalMode) => void;
};

export function ApprovalModeMenu({ modes, selectedMode, disabled = false, onSelect }: ApprovalModeMenuProps) {
  const [open, setOpen] = useState(false);
  const selected = modes.find((mode) => mode.mode === selectedMode) ?? modes[0] ?? null;
  const label = selected ? displayApprovalMode(selected.mode) : "Approval mode";
  const buttonLabel = selected ? `Approval: ${label}` : disabled ? "Loading approval settings" : "Approval settings unavailable";
  const closeOnEscape = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") setOpen(false);
  };

  return (
    <div className="relative" onKeyDown={closeOnEscape}>
      <button
        type="button"
        aria-label={buttonLabel}
        aria-expanded={open}
        aria-haspopup="menu"
        title={buttonLabel}
        className="inline-flex h-8 max-w-[190px] shrink-0 items-center gap-2 rounded-full border border-line px-2.5 text-xs font-medium text-muted hover:text-ink disabled:opacity-50"
        disabled={disabled || modes.length === 0}
        onClick={() => setOpen((current) => !current)}
      >
        <ShieldCheck size={14} className="shrink-0" />
        <span className="truncate">{label}</span>
        <ChevronDown size={13} className={`shrink-0 transition-transform ${open ? "rotate-180" : ""}`} />
      </button>
      {open ? (
        <div
          role="menu"
          aria-label="Approval mode"
          className="absolute bottom-10 left-0 z-30 w-64 rounded-desktop border border-line bg-elevated p-1 shadow-[0_18px_48px_rgba(15,23,42,0.18)]"
        >
          {modes.map((mode) => (
            <button
              key={mode.mode}
              type="button"
              role="menuitemradio"
              aria-checked={mode.mode === selectedMode}
              aria-label={displayApprovalMode(mode.mode)}
              className="grid w-full gap-0.5 rounded-[6px] px-3 py-2 text-left text-sm text-ink hover:bg-muted/10"
              onClick={() => {
                onSelect(mode.mode);
                setOpen(false);
              }}
            >
              <span className="font-medium">{displayApprovalMode(mode.mode)}</span>
              <span className="text-xs leading-5 text-muted">{mode.description}</span>
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}
