import { AlertTriangle, CheckCircle2, CircleDashed, RotateCcw, XCircle } from "../../design/icons";
import type { SetupJobProgressItem, SetupJobProgressSnapshot, SetupJobStatus } from "../../api/types";
import { setupFailureCopy } from "./setupFailureCopy";

type SetupJobProgressProps = {
  progress: SetupJobProgressSnapshot;
  onRetry?: (jobId: string) => void;
  onCancel?: (jobId: string) => void;
};

export function SetupJobProgress({ progress, onRetry, onCancel }: SetupJobProgressProps) {
  return (
    <section aria-labelledby="setup-progress-title" className="rounded-desktop border border-line p-4 dl-panel">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 id="setup-progress-title" className="text-lg font-semibold">
            Setup progress
          </h2>
          <p className="mt-1 text-sm text-muted">DesktopLab updates this as each local setup step moves forward.</p>
        </div>
        <span className="text-xs font-medium text-muted">#{progress.sequence}</span>
      </div>

      <div className="mt-4 space-y-3">
        {progress.jobs.map((job) => (
          <JobRow key={job.id} job={job} onRetry={onRetry} onCancel={onCancel} />
        ))}
      </div>
    </section>
  );
}

function JobRow({
  job,
  onRetry,
  onCancel,
}: {
  job: SetupJobProgressItem;
  onRetry?: (jobId: string) => void;
  onCancel?: (jobId: string) => void;
}) {
  const state = stateUi(job.status);

  return (
    <div className="rounded-desktop border border-line p-3 dl-elevated">
      <div className="flex min-h-8 items-center gap-3">
        <state.Icon className={`${state.iconClassName} ${job.status === "running" ? "dl-running-dot" : ""}`} size={16} />
        <div className="min-w-0">
          <p className="truncate text-sm font-semibold">{job.label}</p>
          {job.phaseLabel ? <p className="mt-0.5 truncate text-xs text-muted">{job.phaseLabel}</p> : null}
        </div>
        <span className={`ml-auto rounded px-2 py-1 text-xs font-semibold ${state.badgeClassName}`}>{state.label}</span>
      </div>

      <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-line" aria-label={`${job.label} progress`}>
        <div className={`h-full rounded-full transition-[width] duration-200 ease-out ${state.barClassName}`} style={{ width: `${clampPercent(job.progressPercent)}%` }} />
      </div>

      {job.nextAction ? <p className="mt-3 text-sm leading-6 text-ink">{setupFailureCopy(job.nextAction)}</p> : null}

      {job.status === "failed" && job.retryAvailable ? (
        <button
          type="button"
          onClick={() => onRetry?.(job.id)}
          className="mt-3 inline-flex h-9 items-center gap-2 rounded-desktop border border-line bg-panel px-3 text-sm font-semibold text-ink hover:bg-elevated"
        >
          <RotateCcw size={14} />
          Retry setup
        </button>
      ) : null}

      {job.status === "running" && job.cancelAvailable ? (
        <button
          type="button"
          onClick={() => onCancel?.(job.id)}
          className="mt-3 inline-flex h-9 items-center gap-2 rounded-desktop border border-line bg-panel px-3 text-sm font-semibold text-ink hover:bg-elevated"
        >
          <XCircle size={14} />
          Cancel download
        </button>
      ) : null}
    </div>
  );
}

function stateUi(status: SetupJobStatus) {
  switch (status) {
    case "completed":
      return {
        label: "Completed",
        Icon: CheckCircle2,
        iconClassName: "text-success",
        badgeClassName: "bg-success/10 text-success",
        barClassName: "bg-success",
      };
    case "blocked":
      return {
        label: "Blocked",
        Icon: AlertTriangle,
        iconClassName: "text-warning",
        badgeClassName: "bg-warning/10 text-warning",
        barClassName: "bg-warning",
      };
    case "failed":
      return {
        label: "Failed",
        Icon: XCircle,
        iconClassName: "text-danger",
        badgeClassName: "bg-danger/10 text-danger",
        barClassName: "bg-danger",
      };
    case "cancelled":
      return {
        label: "Cancelled",
        Icon: XCircle,
        iconClassName: "text-muted",
        badgeClassName: "bg-line text-muted",
        barClassName: "bg-muted",
      };
    case "running":
      return {
        label: "Running",
        Icon: CircleDashed,
        iconClassName: "text-accent",
        badgeClassName: "bg-accent/10 text-accent",
        barClassName: "bg-accent",
      };
    case "queued":
      return {
        label: "Queued",
        Icon: CircleDashed,
        iconClassName: "text-muted",
        badgeClassName: "bg-line text-muted",
        barClassName: "bg-muted",
      };
  }
}

function clampPercent(progressPercent: number): number {
  if (!Number.isFinite(progressPercent)) return 0;
  return Math.min(100, Math.max(0, progressPercent));
}
