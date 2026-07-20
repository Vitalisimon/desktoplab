import { RefreshCw } from "../../design/icons";
import type { UpdateStatusSnapshot } from "../../api/types";

type UpdateStatusPanelProps = {
  updateStatus: UpdateStatusSnapshot;
};

const channelLabel = {
  dev: "Dev",
  beta: "Beta",
  stable: "Stable",
} as const;

const stateLabel = {
  not_checked: "Not checked",
  checking: "Checking",
  up_to_date: "Ready",
  available: "Available",
  failed: "Needs attention",
  disabled: "Paused",
} as const;

export function UpdateStatusPanel({ updateStatus }: UpdateStatusPanelProps) {
  const muted = updateStatus.state === "failed" ? "text-warning" : "text-muted";

  return (
    <section className="py-4" aria-labelledby="update-status-title">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 id="update-status-title" className="text-lg font-semibold">
            Updates
          </h2>
          <p className="mt-1 text-sm leading-6 text-muted">DesktopLab app channel and update readiness.</p>
        </div>
        <span className={`rounded bg-elevated px-2 py-1 text-xs font-semibold ${muted}`}>{stateLabel[updateStatus.state]}</span>
      </div>

      <div className="mt-5 grid gap-3 sm:grid-cols-2">
        <Metric label="Channel" value={channelLabel[updateStatus.channel]} />
        <Metric label="Version" value={`DesktopLab ${updateStatus.currentVersion}`} />
      </div>

      <div className="mt-4 flex items-start gap-3 rounded-desktop border border-line bg-elevated px-4 py-3 text-sm">
        <RefreshCw className="mt-0.5 shrink-0 text-muted" size={16} />
        <p className="leading-6 text-muted">{updateStatus.message}</p>
      </div>
    </section>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-desktop bg-elevated px-4 py-3">
      <p className="text-xs font-semibold uppercase text-muted">{label}</p>
      <p className="mt-2 text-sm font-semibold text-ink">{value}</p>
    </div>
  );
}
