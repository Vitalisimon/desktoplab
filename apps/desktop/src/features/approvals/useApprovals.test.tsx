// @vitest-environment jsdom
import { renderHook, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { ApprovalResolveResponse, ApprovalSummary, ApprovalsListResponse } from "../../api/types";
import { useApprovals } from "./useApprovals";

test("loads approvals through the api client", async () => {
  const apiClient = clientFor({
    listApprovals: vi.fn<() => Promise<ApprovalsListResponse>>().mockResolvedValue({
      approvals: [approval("approval.1", "pending")],
    }),
  });

  const { result } = renderHook(() => useApprovals(), { wrapper: wrapper(apiClient) });

  await waitFor(() => expect(result.current.approvals).toHaveLength(1));
  expect(result.current.pendingApprovals).toHaveLength(1);
  expect(apiClient.listApprovals).toHaveBeenCalled();
});

test("resolves an approval through the api client", async () => {
  const resolveApproval = vi
    .fn<(approvalId: string, request: { resolution: "approve" | "deny" }) => Promise<ApprovalResolveResponse>>()
    .mockResolvedValue({ approvalId: "approval.1", state: "approved" });
  const apiClient = clientFor({
    listApprovals: vi.fn<() => Promise<ApprovalsListResponse>>().mockResolvedValue({ approvals: [] }),
    resolveApproval,
  });

  const { result } = renderHook(() => useApprovals(), { wrapper: wrapper(apiClient) });
  result.current.resolve.mutate({ approvalId: "approval.1", resolution: "approve" });

  await waitFor(() => expect(resolveApproval).toHaveBeenCalledWith("approval.1", { resolution: "approve" }));
});

function wrapper(apiClient: DesktopLabApiClient) {
  return ({ children }: { children: React.ReactNode }) => <AppProviders apiClient={apiClient}>{children}</AppProviders>;
}

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
