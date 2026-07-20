import type { BackendEventFrame } from "../../api/events";
import type { JobRetryClass, JobState, JobSummary } from "../../api/types";

type JobEventPayload = {
  jobId?: unknown;
  kind?: unknown;
  state?: unknown;
  progressPercent?: unknown;
  retryClass?: unknown;
  updatedAt?: unknown;
  failureReason?: unknown;
};

const jobStates: JobState[] = ["queued", "running", "blocked", "failed", "cancelled", "completed"];
const retryClasses: JobRetryClass[] = ["retryable", "non_retryable", "unknown"];

export function reduceJobEvents(jobs: JobSummary[], frames: BackendEventFrame[]): JobSummary[] {
  const jobsById = new Map(jobs.map((job) => [job.jobId, { ...job }]));

  for (const frame of frames) {
    if (frame.scope !== "job") continue;
    const payload = parsePayload(frame.payload);
    if (!payload || typeof payload.jobId !== "string") continue;

    const current = jobsById.get(payload.jobId);
    if (!current) continue;

    jobsById.set(payload.jobId, {
      ...current,
      kind: typeof payload.kind === "string" ? payload.kind : current.kind,
      state: isJobState(payload.state) ? payload.state : current.state,
      progressPercent: typeof payload.progressPercent === "number" ? clampPercent(payload.progressPercent) : current.progressPercent,
      retryClass: isRetryClass(payload.retryClass) ? payload.retryClass : current.retryClass,
      updatedAt: typeof payload.updatedAt === "string" ? payload.updatedAt : current.updatedAt,
      failureReason: typeof payload.failureReason === "string" ? payload.failureReason : current.failureReason,
    });
  }

  return jobs.map((job) => jobsById.get(job.jobId) ?? job);
}

function parsePayload(payload: string): JobEventPayload | null {
  try {
    const parsed = JSON.parse(payload);
    return typeof parsed === "object" && parsed !== null ? (parsed as JobEventPayload) : null;
  } catch {
    return null;
  }
}

function isJobState(value: unknown): value is JobState {
  return typeof value === "string" && jobStates.includes(value as JobState);
}

function isRetryClass(value: unknown): value is JobRetryClass {
  return typeof value === "string" && retryClasses.includes(value as JobRetryClass);
}

function clampPercent(progressPercent: number): number {
  if (!Number.isFinite(progressPercent)) return 0;
  return Math.min(100, Math.max(0, progressPercent));
}
