// @vitest-environment jsdom
import { renderHook, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { JobRetryResponse, JobsListResponse } from "../../api/types";
import { useJobs } from "./useJobs";

test("loads jobs through the api client", async () => {
  const apiClient = clientFor({
    listJobs: vi.fn<() => Promise<JobsListResponse>>().mockResolvedValue({
      jobs: [job("job.1", "running")],
    }),
  });

  const { result } = renderHook(() => useJobs(), { wrapper: wrapper(apiClient) });

  await waitFor(() => expect(result.current.jobs).toHaveLength(1));
  expect(result.current.jobs[0].jobId).toBe("job.1");
});

test("applies bounded replay events to fetched jobs without duplicating rows", async () => {
  const replayEvents = vi.fn().mockResolvedValue([
    event(7, "job.1", "running", 88, "download token=[REDACTED]"),
    event(7, "job.1", "running", 88, "download token=[REDACTED]"),
  ]);
  const apiClient = clientFor({
    listJobs: vi.fn<() => Promise<JobsListResponse>>().mockResolvedValue({
      jobs: [job("job.1", "queued")],
    }),
    replayEvents,
  });

  const { result } = renderHook(() => useJobs(), { wrapper: wrapper(apiClient) });

  await waitFor(() => expect(result.current.jobs[0]?.progressPercent).toBe(88));
  expect(result.current.jobs).toHaveLength(1);
  expect(result.current.jobs[0]).toEqual(
    expect.objectContaining({
      jobId: "job.1",
      state: "running",
      failureReason: "download token=[REDACTED]",
    }),
  );
  expect(replayEvents).toHaveBeenCalledTimes(1);
});

test("applies model download replay progress to existing background jobs", async () => {
  const replayEvents = vi.fn().mockResolvedValue([
    event(8, "model.download.model.qwen-coder", "running", 64, "pulling layers", {
      kind: "model.download",
      retryClass: "retryable",
    }),
  ]);
  const apiClient = clientFor({
    listJobs: vi.fn<() => Promise<JobsListResponse>>().mockResolvedValue({
      jobs: [job("model.download.model.qwen-coder", "queued")],
    }),
    replayEvents,
  });

  const { result } = renderHook(() => useJobs(), { wrapper: wrapper(apiClient) });

  await waitFor(() => expect(result.current.jobs[0]?.progressPercent).toBe(64));
  expect(result.current.jobs[0]).toEqual(
    expect.objectContaining({
      kind: "model.download",
      state: "running",
      retryClass: "retryable",
      failureReason: "pulling layers",
    }),
  );
});

test("falls back to ordinary job fetch when replay is unavailable", async () => {
  const apiClient = clientFor({
    listJobs: vi.fn<() => Promise<JobsListResponse>>().mockResolvedValue({
      jobs: [job("job.1", "running")],
    }),
    replayEvents: vi.fn().mockRejectedValue(new Error("replay unavailable")),
  });

  const { result } = renderHook(() => useJobs(), { wrapper: wrapper(apiClient) });

  await waitFor(() => expect(result.current.jobs).toHaveLength(1));
  expect(result.current.jobs[0].state).toBe("running");
  expect(result.current.isError).toBe(false);
});

test("retries a job through the api client", async () => {
  const retryJob = vi.fn<(jobId: string) => Promise<JobRetryResponse>>().mockResolvedValue({ accepted: true });
  const apiClient = clientFor({
    listJobs: vi.fn<() => Promise<JobsListResponse>>().mockResolvedValue({ jobs: [] }),
    retryJob,
  });

  const { result } = renderHook(() => useJobs(), { wrapper: wrapper(apiClient) });
  result.current.retry.mutate("job.2");

  await waitFor(() => expect(retryJob).toHaveBeenCalledWith("job.2"));
});

function wrapper(apiClient: DesktopLabApiClient) {
  return ({ children }: { children: React.ReactNode }) => <AppProviders apiClient={apiClient}>{children}</AppProviders>;
}

function clientFor(methods: Partial<DesktopLabApiClient>): DesktopLabApiClient {
  return methods as DesktopLabApiClient;
}

function job(jobId: string, state: JobsListResponse["jobs"][number]["state"]) {
  return {
    jobId,
    kind: "model.download",
    state,
    progressPercent: 42,
    retryClass: "unknown" as const,
    updatedAt: "2026-06-25T19:55:00Z",
  };
}

function event(
  sequence: number,
  jobId: string,
  state: JobsListResponse["jobs"][number]["state"],
  progressPercent: number,
  failureReason: string,
  overrides: Record<string, unknown> = {},
) {
  return {
    sequence,
    scope: "job" as const,
    payload: JSON.stringify({
      jobId,
      state,
      progressPercent,
      failureReason,
      updatedAt: "2026-06-25T19:58:00Z",
      ...overrides,
    }),
  };
}
