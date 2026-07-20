import { RotateCcw } from "../../design/icons";
import type { JobState, JobSummary } from "../../api/types";
import { displayJobKind } from "../../domain/displayNames";

type JobCenterViewProps = {
  jobs: JobSummary[];
  onRetry: (jobId: string) => void;
  retryingJobId?: string | null;
};

export function JobCenterView({ jobs, onRetry, retryingJobId = null }: JobCenterViewProps) {
  return (
    <div className="mx-auto grid w-full max-w-6xl gap-4">
      <section className="rounded-desktop border border-line bg-panel p-5 shadow-sm">
        <h1 className="text-2xl font-semibold">Background work</h1>
        <p className="mt-2 max-w-2xl text-sm leading-6 text-muted">
          Follow setup, downloads and checks while DesktopLab prepares the local environment.
        </p>
      </section>

      <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
        {jobs.length === 0 ? (
          <p className="rounded-desktop bg-elevated px-3 py-2 text-sm text-muted">No jobs have run yet.</p>
        ) : (
          <div className="divide-y divide-line">
            {jobs.map((job) => (
              <JobRow key={job.jobId} job={job} onRetry={onRetry} retrying={retryingJobId === job.jobId} />
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

function JobRow({ job, onRetry, retrying }: { job: JobSummary; onRetry: (jobId: string) => void; retrying: boolean }) {
  const state = stateUi(job.state);
  const canRetry = job.state === "failed" && job.retryClass === "retryable";
  const jobLabel = displayJobKind(job.kind);

  return (
    <div className="grid gap-3 py-3 first:pt-0 last:pb-0 md:grid-cols-[1fr_160px_120px] md:items-center">
      <div className="min-w-0">
        <div className="flex items-center gap-2">
          <p className="truncate text-sm font-semibold">{jobLabel}</p>
          <span className={`rounded px-2 py-1 text-xs font-semibold ${state.className}`}>{state.label}</span>
        </div>
        <p className="mt-1 truncate text-xs text-muted">Updated {formatUpdatedAt(job.updatedAt)}</p>
        {job.failureReason ? <p className="mt-2 text-sm leading-6 text-danger">{job.failureReason}</p> : null}
      </div>

      <div>
        <div className="h-1.5 overflow-hidden rounded-full bg-line" aria-label={`${jobLabel} progress`}>
          <div className={`h-full rounded-full ${state.barClassName}`} style={{ width: `${clampPercent(job.progressPercent)}%` }} />
        </div>
        <p className="mt-1 text-xs text-muted">{clampPercent(job.progressPercent)}%</p>
      </div>

      <div className="flex justify-start md:justify-end">
        {canRetry ? (
          <button
            type="button"
            aria-label={`Retry ${job.jobId}`}
            onClick={() => onRetry(job.jobId)}
            disabled={retrying}
            className="inline-flex h-9 items-center gap-2 rounded-desktop border border-line bg-panel px-3 text-sm font-semibold text-ink hover:bg-elevated disabled:cursor-not-allowed disabled:text-muted"
          >
            <RotateCcw size={14} />
            Retry
          </button>
        ) : job.state === "failed" && job.retryClass === "non_retryable" ? (
          <span className="text-xs font-semibold text-muted">Retry unavailable</span>
        ) : (
          <span className="whitespace-nowrap text-xs text-muted" title={job.updatedAt}>
            {formatUpdatedAt(job.updatedAt)}
          </span>
        )}
      </div>
    </div>
  );
}

function stateUi(state: JobState) {
  switch (state) {
    case "completed":
      return { label: "Completed", className: "bg-success/10 text-success", barClassName: "bg-success" };
    case "running":
      return { label: "Running", className: "bg-accent/10 text-accent", barClassName: "bg-accent" };
    case "blocked":
      return { label: "Blocked", className: "bg-warning/10 text-warning", barClassName: "bg-warning" };
    case "failed":
      return { label: "Failed", className: "bg-danger/10 text-danger", barClassName: "bg-danger" };
    case "cancelled":
      return { label: "Cancelled", className: "bg-line text-muted", barClassName: "bg-muted" };
    case "queued":
      return { label: "Queued", className: "bg-line text-muted", barClassName: "bg-muted" };
  }
}

function clampPercent(progressPercent: number): number {
  if (!Number.isFinite(progressPercent)) return 0;
  return Math.min(100, Math.max(0, progressPercent));
}

function formatUpdatedAt(updatedAt: string): string {
  const date = new Date(updatedAt);
  if (Number.isNaN(date.getTime())) return updatedAt;
  return date.toISOString().slice(11, 16);
}
