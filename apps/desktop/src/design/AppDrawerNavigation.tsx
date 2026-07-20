import type { ReactNode } from "react";
import { Activity, Settings2, Stethoscope } from "./icons";
import type { AppRoute } from "../app/routes";
import type { DrawerPinnedItem } from "../api/appState";
import { Pin, PinOff } from "./icons";

export function PinToggleButton({
  pinned,
  item,
  labelPrefix,
  onTogglePin,
}: {
  pinned: boolean;
  item: DrawerPinnedItem;
  labelPrefix?: string;
  onTogglePin: (item: DrawerPinnedItem) => void;
}) {
  const label = `${labelPrefix ?? (pinned ? "Unpin" : "Pin")} ${item.type} ${item.label}`;
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className="grid h-7 w-7 shrink-0 place-items-center rounded-desktop text-muted transition-colors duration-150 hover:bg-elevated hover:text-ink"
      onClick={(event) => {
        event.stopPropagation();
        onTogglePin(item);
      }}
    >
      {pinned ? <PinOff size={14} /> : <Pin size={14} />}
    </button>
  );
}

export function ControlCenter({ activeSection, onNavigate }: { activeSection: AppRoute; onNavigate?: (section: AppRoute) => void }) {
  return (
    <nav aria-label="Control center" className="mt-1 space-y-1">
      <NavItem compact={false} icon={<Activity size={16} />} label="Setup" active={activeSection === "setup"} onClick={() => onNavigate?.("setup")} />
      <NavItem compact={false} icon={<Stethoscope size={16} />} label="Diagnostics" active={activeSection === "diagnostics"} onClick={() => onNavigate?.("diagnostics")} />
      <NavItem compact={false} icon={<Settings2 size={16} />} label="Settings" active={activeSection === "settings"} onClick={() => onNavigate?.("settings")} />
    </nav>
  );
}

export function DrawerSection({ label, open, children }: { label: string; open: boolean; children: ReactNode }) {
  return (
    <section aria-label={label} className="mt-6">
      {open ? (
        <div className="mb-2 flex items-center gap-2 px-2">
          <p className="text-[11px] font-semibold uppercase tracking-[0.08em] text-muted">{label}</p>
          <span className="h-px flex-1 bg-line/70" />
        </div>
      ) : null}
      <div className="space-y-1">{children}</div>
    </section>
  );
}

export function NavItem({
  icon,
  label,
  active = false,
  muted = false,
  compact,
  onClick,
  trailingAction = null,
  status = null,
}: {
  icon: ReactNode;
  label: string;
  active?: boolean;
  muted?: boolean;
  compact: boolean;
  onClick?: () => void;
  trailingAction?: ReactNode;
  status?: { label: string; className?: string; active?: boolean; icon?: ReactNode } | null;
}) {
  const disabled = muted && !onClick;
  const button = (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      aria-label={label}
      title={label}
      className={`group relative flex h-9 ${trailingAction ? "min-w-0 flex-1" : "w-full"} items-center gap-3 rounded-desktop px-2 text-sm transition-colors duration-150 ${active ? "bg-elevated font-medium text-ink shadow-sm" : muted ? "text-muted/60" : "text-muted hover:bg-elevated/65 hover:text-ink"} ${compact ? "justify-center" : ""} ${disabled ? "cursor-not-allowed" : ""}`}
    >
      {active ? <span aria-hidden="true" className="absolute left-0 top-1.5 h-6 w-[3px] rounded-full bg-accent shadow-[var(--dl-accent-glow)]" /> : null}
      {icon}
      {compact ? null : <span className="truncate">{label}</span>}
      {compact || !status ? null : status.icon ? (
        <span aria-hidden="true" className="ml-auto grid h-4 w-4 shrink-0 place-items-center" title={status.label}>{status.icon}</span>
      ) : (
        <span aria-hidden="true" className={`ml-auto h-1.5 w-1.5 shrink-0 rounded-full ${status.className ?? ""} ${status.active ? "dl-running-dot" : ""}`} title={status.label} />
      )}
    </button>
  );
  if (!trailingAction) return button;
  return (
    <div className="group/nav flex items-center gap-1">
      {button}
      <div className="opacity-0 transition-opacity group-hover/nav:opacity-100 group-focus-within/nav:opacity-100">{trailingAction}</div>
    </div>
  );
}

export function isSupportRoute(route: AppRoute) {
  return ["sessions", "providers", "models", "context", "diagnostics", "settings", "changes"].includes(route);
}
