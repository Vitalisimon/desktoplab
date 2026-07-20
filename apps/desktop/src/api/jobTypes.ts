export type JobState = "queued" | "running" | "blocked" | "failed" | "cancelled" | "completed";

export type JobRetryClass = "retryable" | "non_retryable" | "unknown";

export type JobSummary = {
  jobId: string;
  kind: string;
  state: JobState;
  progressPercent: number;
  retryClass: JobRetryClass;
  updatedAt: string;
  failureReason?: string;
};

export type JobsListResponse = {
  jobs: JobSummary[];
};

export type JobRetryResponse = {
  accepted: boolean;
};
