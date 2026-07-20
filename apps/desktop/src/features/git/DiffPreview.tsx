type DiffPreviewProps = {
  diffText: string;
};

export function DiffPreview({ diffText }: DiffPreviewProps) {
  return (
    <section className="rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <h2 className="text-lg font-semibold">File preview</h2>
      {diffText.trim().length === 0 ? (
        <p className="mt-3 rounded-desktop bg-elevated px-3 py-2 text-sm text-muted">No file preview yet.</p>
      ) : (
        <pre
          aria-label="Change preview"
          className="mt-3 max-h-[420px] overflow-auto rounded-desktop bg-elevated p-3 text-xs leading-5 text-ink"
        >
          {diffText}
        </pre>
      )}
    </section>
  );
}
