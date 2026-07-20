// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { JobRetryResponse, JobsListResponse } from "../../api/types";
import { JobsFeature } from "./JobsFeature";

test("loads jobs and retries retryable failure", async () => {
  const retryJob = vi.fn<(jobId: string) => Promise<JobRetryResponse>>().mockResolvedValue({ accepted: true });
  const apiClient = {
    listJobs: vi.fn<() => Promise<JobsListResponse>>().mockResolvedValue({
      jobs: [
        {
          jobId: "job.1",
          kind: "registry.refresh",
          state: "failed",
          progressPercent: 20,
          retryClass: "retryable",
          failureReason: "registry unavailable",
          updatedAt: "2026-06-25T19:55:00Z",
        },
      ],
    }),
    retryJob,
  } as unknown as DesktopLabApiClient;

  render(
    <AppProviders apiClient={apiClient}>
      <JobsFeature />
    </AppProviders>,
  );

  expect(await screen.findByText("Refresh recommendations")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /retry/i }));

  await waitFor(() => expect(retryJob).toHaveBeenCalledWith("job.1"));
});

test("renders local api failure as background work unavailable", async () => {
  const apiClient = {
    listJobs: vi.fn<() => Promise<JobsListResponse>>().mockRejectedValue(new Error("offline")),
  } as unknown as DesktopLabApiClient;

  render(
    <AppProviders apiClient={apiClient}>
      <JobsFeature />
    </AppProviders>,
  );

  expect(await screen.findByText("Background work unavailable")).toBeInTheDocument();
});
