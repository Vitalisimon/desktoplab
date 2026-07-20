export function InspectorEmptyState({
  title,
  description,
  actionLabel,
  onAction,
}: {
  title: string;
  description: string;
  actionLabel?: string;
  onAction?: () => void;
}) {
  return (
    <section className="rounded-desktop border border-line bg-canvas/60 p-3">
      <h3 className="text-sm font-semibold text-ink">{title}</h3>
      <p className="mt-2 text-sm leading-5 text-muted">{description}</p>
      {actionLabel ? (
        <button type="button" className="mt-4 h-8 rounded-desktop border border-line px-3 text-xs font-semibold text-ink hover:bg-elevated" onClick={onAction}>
          {actionLabel}
        </button>
      ) : null}
    </section>
  );
}
