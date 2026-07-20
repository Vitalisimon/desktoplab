import type { CatalogRefreshStatusResponse, DoctorLintSnapshot, LocalAuditTransparencySnapshot, SecurityAuditSnapshot } from "../../api/types";
import { EvidenceDisclosure, StatusRow } from "../../design/OperationalPrimitives";

export function CatalogRefreshPanel({
  status,
  queuedJobId,
  pending,
  onRefresh,
}: {
  status?: CatalogRefreshStatusResponse;
  queuedJobId: string | null;
  pending: boolean;
  onRefresh: () => void;
}) {
  if (!status || status.state === "ready") return null;
  return (
    <section className="rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <h2 className="text-lg font-semibold">Compatibility catalog</h2>
      <p className="mt-1 text-sm text-muted">Refresh compatibility catalog when setup or support evidence says local recommendations are stale.</p>
      {status.degradedReasons.length > 0 ? (
        <ul className="mt-3 space-y-2">
          {status.degradedReasons.map((reason) => (
            <li key={reason} className="rounded-desktop bg-elevated px-3 py-2 text-sm text-ink">
              {reason}
            </li>
          ))}
        </ul>
      ) : null}
      {queuedJobId ? (
        <div className="mt-3 rounded-desktop bg-elevated px-3 py-2">
          <p className="text-sm font-semibold text-ink">Catalog refresh queued</p>
          <p className="mt-1 text-sm text-muted">Track progress in Background.</p>
        </div>
      ) : status.manualRefresh.available ? (
        <button type="button" className="mt-3 rounded-desktop bg-ink px-3 py-2 text-sm font-semibold text-canvas disabled:opacity-45" disabled={pending} onClick={onRefresh}>
          Refresh compatibility catalog
        </button>
      ) : (
        <p className="mt-3 rounded-desktop bg-elevated px-3 py-2 text-sm text-muted">{status.manualRefresh.blockedReason ?? "Compatibility refresh is unavailable."}</p>
      )}
    </section>
  );
}

export function DoctorLintPanel({ lint }: { lint?: DoctorLintSnapshot }) {
  if (!lint?.checks.length) return null;
  return (
    <section className="rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <PanelHeader title="Doctor lint" body="Read-only checks from the local control plane." state={lint.summary.state} />
      <div className="mt-4 grid gap-3">
        {lint.checks.map((check) => (
          <StatusRow key={check.checkId} label={check.label} status={check.severity} detail={check.fixHint} />
        ))}
      </div>
    </section>
  );
}

export function SecurityAuditPanel({ audit }: { audit?: SecurityAuditSnapshot }) {
  if (!audit?.findings.length) return null;
  const current = audit.findings.filter((finding) => !finding.suppressed);
  const unavailable = audit.findings.filter((finding) => finding.suppressed);
  return (
    <section className="rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <PanelHeader title="Security audit" body="Read-only security posture checks from the local control plane." state={audit.summary.state} />
      <div className="mt-4 grid gap-3">
        {current.map((finding) => (
          <StatusRow key={finding.checkId} label={finding.label} status={finding.severity} detail={finding.fixHint} />
        ))}
      </div>
      {unavailable.length > 0 ? (
        <div className="mt-5 border-t border-line pt-4">
          <h3 className="text-sm font-semibold text-ink">Unavailable capabilities</h3>
          {unavailable.map((finding) => <p key={finding.checkId} className="mt-2 text-sm text-muted"><span className="font-medium text-ink">{finding.label}:</span> {finding.fixHint}</p>)}
        </div>
      ) : null}
    </section>
  );
}

export function LocalAuditPanel({ audit, unavailable }: { audit?: LocalAuditTransparencySnapshot; unavailable: boolean }) {
  return (
    <section className="rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <h2 className="text-lg font-semibold">Recent local decisions</h2>
      <p className="mt-1 text-sm text-muted">Local approvals and route decisions are shown here with secrets removed.</p>
      {unavailable ? <p className="mt-3 text-sm text-muted">Local decisions are unavailable right now.</p> : null}
      {audit?.records.length ? (
        <ol className="mt-4 divide-y divide-line rounded-desktop border border-line">
          {audit.records.map((record) => (
            <li key={record.sequence} className="grid gap-1 px-3 py-3">
              <p className="text-sm font-semibold text-ink">{decisionTitle(record.action, record.outcome)}</p>
              <p className="text-sm leading-6 text-muted">{record.redactedDetails}</p>
            </li>
          ))}
        </ol>
      ) : null}
      {audit?.redactedExport ? (
        <div className="mt-3">
          <EvidenceDisclosure title="Redacted audit export" body={audit.redactedExport} />
        </div>
      ) : null}
    </section>
  );
}

function PanelHeader({ title, body, state }: { title: string; body: string; state: "ready" | "degraded" | "blocked" }) {
  const color = state === "ready" ? "bg-success/10 text-success" : state === "blocked" ? "bg-danger/10 text-danger" : "bg-warning/10 text-warning";
  return (
    <div className="flex items-start justify-between gap-4">
      <div>
        <h2 className="text-lg font-semibold">{title}</h2>
        <p className="mt-1 text-sm text-muted">{body}</p>
      </div>
      <span className={`rounded-full px-2 py-1 text-xs font-semibold ${color}`}>{state === "ready" ? "Ready" : state === "degraded" ? "Limited" : "Blocked"}</span>
    </div>
  );
}

function decisionTitle(action: string, outcome: string): string {
  return `${actionLabel(action)} ${outcome}`;
}

function actionLabel(action: string): string {
  const words = action.split("_").filter(Boolean);
  if (words.length === 0) return "Decision";
  return [words[0].charAt(0).toUpperCase() + words[0].slice(1), ...words.slice(1)].join(" ");
}
