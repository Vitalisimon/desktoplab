// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import type { JobSummary } from "../../api/types";
import { JobCenterView } from "./JobCenterView";

test("renders job center with job state and progress", () => {
  render(<JobCenterView jobs={[job("job.1", "running", 42), job("job.2", "completed", 100)]} onRetry={vi.fn()} />);

  expect(screen.getByText("Background work")).toBeInTheDocument();
  expect(screen.getByText("Download coding model")).toBeInTheDocument();
  expect(screen.getByText("Install local runner")).toBeInTheDocument();
  expect(screen.getByText("Running")).toBeInTheDocument();
  expect(screen.getByText("Completed")).toBeInTheDocument();
  expect(screen.getByLabelText("Download coding model progress")).toBeInTheDocument();
});

test("renders empty state without fake jobs", () => {
  render(<JobCenterView jobs={[]} onRetry={vi.fn()} />);

  expect(screen.getByText("No jobs have run yet.")).toBeInTheDocument();
});

test("shows retry only for retryable failed jobs", () => {
  render(
    <JobCenterView
      jobs={[
        { ...job("job.1", "failed", 20), retryClass: "retryable", failureReason: "network interrupted" },
        { ...job("job.2", "failed", 10), retryClass: "non_retryable", failureReason: "signature verification failed" },
      ]}
      onRetry={vi.fn()}
    />,
  );

  expect(screen.getByText("network interrupted")).toBeInTheDocument();
  expect(screen.getByText("signature verification failed")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /retry job.1/i })).toBeEnabled();
  expect(screen.queryByRole("button", { name: /retry job.2/i })).not.toBeInTheDocument();
  expect(screen.getByText("Retry unavailable")).toBeInTheDocument();
});

function job(jobId: string, state: JobSummary["state"], progressPercent: number): JobSummary {
  return {
    jobId,
    kind: jobId === "job.1" ? "model.download" : "runtime.install",
    state,
    progressPercent,
    retryClass: "unknown",
    updatedAt: "2026-06-25T19:55:00Z",
  };
}
