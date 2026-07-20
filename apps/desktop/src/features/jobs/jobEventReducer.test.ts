import { reduceJobEvents } from "./jobEventReducer";
import type { BackendEventFrame } from "../../api/events";
import type { JobSummary } from "../../api/types";

test("applies ordered job event frames to job summaries", () => {
  const jobs: JobSummary[] = [
    {
      jobId: "job.1",
      kind: "model.download",
      state: "queued",
      progressPercent: 0,
      retryClass: "unknown",
      updatedAt: "2026-06-25T19:00:00Z",
    },
  ];

  const reduced = reduceJobEvents(jobs, [
    frame(1, { jobId: "job.1", state: "running", progressPercent: 36, updatedAt: "2026-06-25T19:01:00Z" }),
    frame(2, { jobId: "job.1", state: "completed", progressPercent: 100, updatedAt: "2026-06-25T19:02:00Z" }),
  ]);

  expect(reduced).toEqual([
    expect.objectContaining({
      jobId: "job.1",
      state: "completed",
      progressPercent: 100,
      updatedAt: "2026-06-25T19:02:00Z",
    }),
  ]);
});

test("ignores non-job frames and malformed job payloads", () => {
  const jobs: JobSummary[] = [
    {
      jobId: "job.1",
      kind: "runtime.install",
      state: "running",
      progressPercent: 12,
      retryClass: "unknown",
      updatedAt: "2026-06-25T19:00:00Z",
    },
  ];

  const reduced = reduceJobEvents(jobs, [
    { sequence: 1, scope: "setup", payload: "{}" },
    { sequence: 2, scope: "job", payload: "not-json" },
  ]);

  expect(reduced).toEqual(jobs);
});

test("applies runtime install progress and retry class from event frames", () => {
  const jobs: JobSummary[] = [
    {
      jobId: "job.runtime.install",
      kind: "runtime.install",
      state: "queued",
      progressPercent: 0,
      retryClass: "unknown",
      updatedAt: "2026-06-25T19:00:00Z",
    },
  ];

  const reduced = reduceJobEvents(jobs, [
    frame(3, {
      jobId: "job.runtime.install",
      kind: "runtime.install",
      state: "failed",
      progressPercent: 40,
      retryClass: "retryable",
      failureReason: "network unavailable",
      updatedAt: "2026-06-25T19:03:00Z",
    }),
  ]);

  expect(reduced[0]).toEqual(
    expect.objectContaining({
      kind: "runtime.install",
      state: "failed",
      progressPercent: 40,
      retryClass: "retryable",
      failureReason: "network unavailable",
    }),
  );
});

function frame(sequence: number, payload: Record<string, unknown>): BackendEventFrame {
  return { sequence, scope: "job", payload: JSON.stringify(payload) };
}
