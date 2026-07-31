import { ChevronRight } from "../../design/icons";

export function SettingsNavigationRow({ title, description, onOpen }: { title: string; description: string; onOpen: () => void }) {
  return (
    <button
      type="button"
      aria-label={title}
      className="group flex w-full items-center gap-4 border-t border-line px-1 py-4 text-left outline-none focus-visible:ring-2 focus-visible:ring-accent"
      onClick={onOpen}
    >
      <span className="min-w-0 flex-1">
        <span className="block text-sm font-semibold text-ink">{title}</span>
        <span className="mt-1 block text-xs font-normal leading-5 text-muted">{description}</span>
      </span>
      <ChevronRight aria-hidden="true" className="size-4 shrink-0 text-muted transition-transform group-hover:translate-x-0.5" />
    </button>
  );
}
