import { ExternalLink, FileWarning, X } from "../../design/icons";
import type { WorkspaceFilePreviewResponse } from "../../api/types";

type FilePreviewPanelProps = {
  path: string | null;
  preview?: WorkspaceFilePreviewResponse;
  loading?: boolean;
  onClose?: () => void;
};

export function FilePreviewPanel({ path, preview, loading = false, onClose }: FilePreviewPanelProps) {
  if (!path) {
    return (
      <section className="min-h-28 rounded-desktop border border-line bg-canvas/60 p-3 text-sm text-muted">
        <FileWarning size={15} className="mb-2" />
        Select a file to preview it.
      </section>
    );
  }

  return (
    <section className="flex min-h-0 flex-1 flex-col rounded-desktop border border-line bg-canvas/60">
      <header className="flex min-h-12 items-center justify-between gap-2 border-b border-line px-3">
        <div className="min-w-0">
          <p className="truncate text-xs font-semibold text-ink">{path}</p>
          <div className="mt-1 flex flex-wrap items-center gap-1.5">{previewBadges(preview)}</div>
        </div>
        {onClose ? (
          <div className="flex shrink-0 items-center gap-1">
            {preview?.openAction ? (
              <button type="button" className="grid h-7 w-7 place-items-center rounded-md text-muted hover:bg-elevated hover:text-ink" aria-label={preview.openAction.label}>
                <ExternalLink size={14} />
              </button>
            ) : null}
            <button type="button" aria-label="Close preview" className="grid h-7 w-7 place-items-center rounded-md text-muted hover:bg-elevated hover:text-ink" onClick={onClose}>
              <X size={14} />
            </button>
          </div>
        ) : preview?.openAction ? (
          <button type="button" className="grid h-7 w-7 shrink-0 place-items-center rounded-md text-muted hover:bg-elevated hover:text-ink" aria-label={preview.openAction.label}>
            <ExternalLink size={14} />
          </button>
        ) : null}
      </header>
      <div className="min-h-0 flex-1 overflow-auto p-3">{previewContent(preview, loading)}</div>
    </section>
  );
}

function previewBadges(preview: WorkspaceFilePreviewResponse | undefined) {
  if (!preview) return <PreviewBadge label="Loading" />;
  return (
    <>
      <PreviewBadge label={previewKindLabel(preview)} />
      {preview.state === "text" ? <PreviewBadge label={`${preview.returnedLines} of ${preview.originalLines} lines`} /> : null}
      {preview.truncated ? <PreviewBadge label="Preview limited" /> : null}
      {preview.state === "text" && preview.text.includes("[REDACTED_SECRET]") ? <PreviewBadge label="Redacted" /> : null}
    </>
  );
}

function previewKindLabel(preview: WorkspaceFilePreviewResponse) {
  if (preview.state === "text") return "Text preview";
  if (preview.state === "binary") return "Binary metadata";
  return "Protected";
}

function PreviewBadge({ label }: { label: string }) {
  return <span className="rounded-full border border-line bg-elevated px-1.5 py-0.5 text-[10px] font-medium text-muted">{label}</span>;
}

function previewContent(preview: WorkspaceFilePreviewResponse | undefined, loading: boolean) {
  if (loading || !preview) return <p className="text-sm text-muted">Loading preview...</p>;
  if (preview.state === "denied") return <p className="text-sm text-muted">Protected local file.</p>;
  if (preview.state === "binary") return <p className="text-sm text-muted">Binary file, {preview.originalBytes} bytes.</p>;
  return (
    <div className="grid gap-2">
      <div className="font-mono text-xs leading-5 text-ink">
        {preview.text.split("\n").map((line, index) => (
          <div key={`${preview.path}-${index}`}>{line || " "}</div>
        ))}
      </div>
    </div>
  );
}
