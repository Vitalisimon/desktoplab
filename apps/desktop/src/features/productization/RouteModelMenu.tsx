import { Brain, ChevronDown } from "../../design/icons";
import { useState, type KeyboardEvent } from "react";
import type { ExecutionRouteOption, ExecutionRouteOptionsResponse } from "../../api/types";

type RouteModelMenuProps = {
  label: string;
  options: ExecutionRouteOptionsResponse | null;
  disabled?: boolean;
  onSelect: (routeId: string) => void;
};

export function RouteModelMenu({ label, options, disabled = false, onSelect }: RouteModelMenuProps) {
  const [open, setOpen] = useState(false);
  const installedModels = options?.options.filter((option) => option.backendKind === "local" && option.status === "available") ?? [];
  const readOnly = disabled || !options || installedModels.length === 0;
  const closeOnEscape = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") setOpen(false);
  };

  return (
    <div className="relative min-w-0" onKeyDown={closeOnEscape}>
      <button
        type="button"
        aria-label={`Selected model ${label}`}
        aria-expanded={readOnly ? undefined : open}
        aria-haspopup={readOnly ? undefined : "menu"}
        title={`Selected model ${label}`}
        className="inline-flex h-8 max-w-[280px] shrink min-w-0 items-center gap-2 rounded-full border border-line px-3 text-xs font-medium text-muted hover:text-ink disabled:opacity-50"
        disabled={readOnly}
        onClick={() => {
          if (!readOnly) setOpen((current) => !current);
        }}
      >
        <Brain size={14} className="shrink-0" />
        <span className="truncate">{label}</span>
        <ChevronDown size={13} className={`shrink-0 transition-transform ${open ? "rotate-180" : ""}`} />
      </button>

      {open && options && !readOnly ? (
        <div
          role="menu"
          aria-label="Execution route"
          className="absolute bottom-10 left-0 z-30 w-72 rounded-desktop border border-line bg-elevated p-1 shadow-[0_18px_48px_rgba(15,23,42,0.18)]"
        >
          {installedModels.map((option) => (
            <RouteOptionButton
              key={option.routeId}
              option={option}
              selected={option.routeId === options.selectedRouteId}
              onSelect={() => {
                onSelect(option.routeId);
                setOpen(false);
              }}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

function RouteOptionButton({ option, selected, onSelect }: { option: ExecutionRouteOption; selected: boolean; onSelect: () => void }) {
  const disabled = option.status !== "available";
  const copy = routeOptionCopy(option);
  return (
    <button
      type="button"
      role="menuitemradio"
      aria-checked={selected}
      aria-label={option.label}
      className="grid w-full gap-0.5 rounded-[6px] px-3 py-2 text-left text-sm text-ink hover:bg-muted/10 disabled:cursor-not-allowed disabled:opacity-55"
      disabled={disabled}
      onClick={onSelect}
    >
      <span className="font-medium">{option.label}</span>
      <span className="text-xs leading-5 text-muted">{disabled ? option.disabledReason : copy}</span>
    </button>
  );
}

function routeOptionCopy(option: ExecutionRouteOption) {
  return `Runs locally with ${option.runtimeDisplayName}.`;
}
