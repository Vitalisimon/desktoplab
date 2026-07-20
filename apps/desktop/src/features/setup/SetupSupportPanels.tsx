import type { CatalogRefreshStatusResponse } from "../../api/types";

type CatalogRefreshPanelProps = {
  status?: CatalogRefreshStatusResponse;
  queuedJobId: string | null;
  pending: boolean;
  onRefresh: () => void;
};

export function RuntimeOfflineNotice() {
  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <h2 className="text-lg font-semibold">Runtime setup is waiting for a verified offline installer.</h2>
      <p className="mt-1 text-sm text-muted">Reconnect or add a verified cached installer, then start setup again.</p>
    </section>
  );
}

export function CatalogRefreshPanel({ status, queuedJobId, pending, onRefresh }: CatalogRefreshPanelProps) {
  if (!status || status.state === "ready") return null;
  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <h2 className="text-lg font-semibold">Compatibility catalog</h2>
      <p className="mt-1 text-sm text-muted">Compatibility data is stale. You can keep using safe cached recommendations or refresh it now.</p>
      {status.degradedReasons.length > 0 ? (
        <ul className="mt-3 space-y-2">
          {status.degradedReasons.map((reason) => (
            <li key={reason} className="rounded-desktop bg-elevated px-3 py-2 text-sm text-ink">
              {reason}
            </li>
          ))}
        </ul>
      ) : null}
      {queuedJobId ? <CatalogQueuedNotice /> : <CatalogRefreshAction status={status} pending={pending} onRefresh={onRefresh} />}
    </section>
  );
}

export function SetupPanel({ title, body }: { title: string; body: string }) {
  return (
    <section className="rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <h1 className="text-xl font-semibold">{title}</h1>
      <p className="mt-2 text-sm text-muted">{body}</p>
    </section>
  );
}

function CatalogQueuedNotice() {
  return (
    <div className="mt-3 rounded-desktop bg-elevated px-3 py-2">
      <p className="text-sm font-semibold text-ink">Catalog refresh queued</p>
      <p className="mt-1 text-sm text-muted">Track progress in Background.</p>
    </div>
  );
}

function CatalogRefreshAction({ status, pending, onRefresh }: { status: CatalogRefreshStatusResponse; pending: boolean; onRefresh: () => void }) {
  if (!status.manualRefresh.available) {
    return <p className="mt-3 rounded-desktop bg-elevated px-3 py-2 text-sm text-muted">{status.manualRefresh.blockedReason ?? "Compatibility refresh is unavailable."}</p>;
  }
  return (
    <button type="button" className="mt-3 rounded-desktop bg-ink px-3 py-2 text-sm font-semibold text-canvas disabled:opacity-45" disabled={pending} onClick={onRefresh}>
      Refresh compatibility catalog
    </button>
  );
}
