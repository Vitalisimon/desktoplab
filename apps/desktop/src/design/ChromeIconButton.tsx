import type { ReactNode } from "react";

export function ChromeIconButton({ label, children, pressed, onClick }: { label: string; children: ReactNode; pressed: boolean; onClick: () => void }) {
  return (
    <button
      type="button"
      aria-label={label}
      aria-pressed={pressed}
      title={label}
      data-chrome-control="true"
      className={`grid h-7 w-7 place-items-center rounded-[6px] border border-transparent bg-transparent transition-colors duration-150 hover:bg-elevated/70 hover:text-ink ${pressed ? "text-ink shadow-[var(--dl-accent-glow)]" : "text-muted"}`}
      onClick={onClick}
    >
      {children}
    </button>
  );
}
