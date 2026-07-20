// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import type { ApprovalSummary } from "../../api/types";
import { ThreadApprovalPrompt } from "./ThreadApprovalPrompt";

test("labels a localized patch truthfully when backend copy is unavailable", () => {
  const approval = {
    approvalId: "approval.patch.1",
    sessionId: "session.1",
    action: "filesystem.write",
    operationId: "filesystem.patch:calculator.js",
    state: "pending",
    risk: "medium",
    title: "",
    message: "",
    requestedAt: "2026-07-15T00:00:00Z",
  } satisfies ApprovalSummary;

  render(
    <ThreadApprovalPrompt
      approvals={[approval]}
      resolving={false}
      failed={false}
      onResolve={vi.fn()}
    />,
  );

  expect(screen.getByText("Patch file")).toBeInTheDocument();
});
