// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { ApprovalResolveResponse, ApprovalSummary, ApprovalsListResponse } from "../../api/types";
import { ApprovalsFeature } from "./ApprovalsFeature";

test("renders pending approvals and resolves a decision", async () => {
  const resolveApproval = vi
    .fn<(approvalId: string, request: { resolution: "approve" | "deny" }) => Promise<ApprovalResolveResponse>>()
    .mockResolvedValue({ approvalId: "approval.1", state: "approved" });
  const apiClient = clientFor({
    listApprovals: vi.fn<() => Promise<ApprovalsListResponse>>().mockResolvedValue({
      approvals: [approval("approval.1", "pending")],
    }),
    resolveApproval,
  });

  render(
    <AppProviders apiClient={apiClient}>
      <ApprovalsFeature />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Approvals" })).toBeInTheDocument();
  expect(screen.getByText("Review file change")).toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: "Allow" }));

  await waitFor(() => expect(resolveApproval).toHaveBeenCalledWith("approval.1", { resolution: "approve" }));
});

test("renders empty approval queue without fake requests", async () => {
  const apiClient = clientFor({
    listApprovals: vi.fn<() => Promise<ApprovalsListResponse>>().mockResolvedValue({ approvals: [] }),
  });

  render(
    <AppProviders apiClient={apiClient}>
      <ApprovalsFeature />
    </AppProviders>,
  );

  expect(await screen.findByText("No approvals waiting.")).toBeInTheDocument();
  expect(screen.queryByText("Review file change")).not.toBeInTheDocument();
});

test("renders approval api failure as unavailable state", async () => {
  const apiClient = clientFor({
    listApprovals: vi.fn<() => Promise<ApprovalsListResponse>>().mockRejectedValue(new Error("offline")),
  });

  render(
    <AppProviders apiClient={apiClient}>
      <ApprovalsFeature />
    </AppProviders>,
  );

  expect(await screen.findByText("Approvals unavailable")).toBeInTheDocument();
});

function clientFor(methods: Partial<DesktopLabApiClient>): DesktopLabApiClient {
  return methods as DesktopLabApiClient;
}

function approval(approvalId: string, state: ApprovalSummary["state"]): ApprovalSummary {
  return {
    approvalId,
    sessionId: "session.1",
    action: "filesystem.write",
    state,
    risk: "medium",
    title: "Review file change",
    message: "The agent wants to edit files in the active repository.",
    requestedAt: "2026-06-25T20:30:00Z",
    policyReason: "Filesystem writes need confirmation.",
  };
}
