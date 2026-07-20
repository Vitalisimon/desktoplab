// @vitest-environment jsdom
import { fireEvent, render, screen } from "@testing-library/react";
import type { ApprovalSummary } from "../../api/types";
import { ApprovalCard } from "./ApprovalCard";

test("renders approval in user-facing language with policy reason", () => {
  render(<ApprovalCard approval={approval("filesystem.write")} onResolve={vi.fn()} />);

  expect(screen.getByText("Review file change")).toBeInTheDocument();
  expect(screen.getByText("The agent wants to edit files in the active repository.")).toBeInTheDocument();
  expect(screen.getByText("Medium attention")).toBeInTheDocument();
  expect(screen.getByText("Filesystem writes need confirmation.")).toBeInTheDocument();
  expect(screen.getByText("File change")).toBeInTheDocument();
});

test("resolves approve and deny decisions", () => {
  const onResolve = vi.fn();
  render(<ApprovalCard approval={approval("terminal.command")} onResolve={onResolve} />);

  fireEvent.click(screen.getByRole("button", { name: "Allow" }));
  fireEvent.click(screen.getByRole("button", { name: "Deny" }));

  expect(onResolve).toHaveBeenCalledWith("approval.1", "approve");
  expect(onResolve).toHaveBeenCalledWith("approval.1", "deny");
});

test("renders terminal command approvals in product language", () => {
  render(<ApprovalCard approval={approval("terminal.command")} onResolve={vi.fn()} />);

  expect(screen.getByText("Run command")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Allow" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Deny" })).toBeInTheDocument();
});

test("does not show decision controls for resolved approvals", () => {
  render(<ApprovalCard approval={{ ...approval("git.push"), state: "denied" }} onResolve={vi.fn()} />);

  expect(screen.getByText("Denied")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Allow" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Deny" })).not.toBeInTheDocument();
});

function approval(action: ApprovalSummary["action"]): ApprovalSummary {
  return {
    approvalId: "approval.1",
    sessionId: "session.1",
    action,
    state: "pending",
    risk: "medium",
    title: "Review file change",
    message: "The agent wants to edit files in the active repository.",
    requestedAt: "2026-06-25T20:30:00Z",
    policyReason: "Filesystem writes need confirmation.",
  };
}
