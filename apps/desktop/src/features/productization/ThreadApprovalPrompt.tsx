import type { ApprovalResolveRequest, ApprovalSummary } from "../../api/types";

type ThreadApprovalPromptProps = {
  approvals: ApprovalSummary[];
  resolving: boolean;
  failed: boolean;
  onResolve: (approval: ApprovalSummary, resolution: ApprovalResolveRequest["resolution"]) => void;
};

export function ThreadApprovalPrompt({ approvals, resolving, failed, onResolve }: ThreadApprovalPromptProps) {
  if (approvals.length === 0) return null;

  return (
    <div className="mb-3 space-y-2" role="group" aria-label="Thread approval required">
      {approvals.map((approval) => {
        const terminal = terminalApprovalDetails(approval);
        return (
          <div key={approval.approvalId} className="rounded-desktop border border-warning/30 bg-warning/10 px-3 py-2 text-sm shadow-sm">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="font-medium text-ink">{approval.title || fallbackTitle(approval)}</p>
                <p className="mt-1 text-xs leading-5 text-muted">{approval.message || "DesktopLab needs your decision before continuing."}</p>
                {terminal ? <TerminalApprovalMetadata details={terminal} /> : null}
              </div>
              <div className="flex shrink-0 items-center gap-2">
                <button
                  type="button"
                  className="h-8 rounded-full border border-line bg-panel px-3 text-xs font-medium text-muted hover:bg-muted/10 disabled:opacity-50"
                  disabled={resolving}
                  onClick={() => onResolve(approval, "deny")}
                >
                  Deny
                </button>
                <button
                  type="button"
                  className="h-8 rounded-full bg-ink px-3 text-xs font-medium text-canvas hover:bg-accent disabled:opacity-50"
                  disabled={resolving}
                  onClick={() => onResolve(approval, "approve")}
                >
                  Approve
                </button>
              </div>
            </div>
          </div>
        );
      })}
      {failed ? <p role="alert" className="px-1 text-xs font-medium text-danger">Approval was not saved. Try again.</p> : null}
    </div>
  );
}

function TerminalApprovalMetadata({ details }: { details: TerminalApprovalDetails }) {
  return (
    <dl className="mt-2 grid gap-1 text-xs leading-5 text-muted sm:grid-cols-2">
      <div>
        <dt className="font-semibold text-ink">Command</dt>
        <dd className="break-all font-mono">{details.command}</dd>
      </div>
      <div>
        <dt className="font-semibold text-ink">Cwd</dt>
        <dd>{details.cwd}</dd>
      </div>
      <div>
        <dt className="font-semibold text-ink">Risk</dt>
        <dd>{details.risk}</dd>
      </div>
      <div>
        <dt className="font-semibold text-ink">Reason</dt>
        <dd>{details.reason}</dd>
      </div>
    </dl>
  );
}

type TerminalApprovalDetails = {
  command: string;
  cwd: string;
  risk: string;
  reason: string;
};

function terminalApprovalDetails(approval: ApprovalSummary): TerminalApprovalDetails | null {
  if (approval.action !== "terminal.command" && approval.action !== "test.run") return null;
  const command = approval.operationId?.split(":").slice(1).join(":").trim();
  if (!command) return null;
  return {
    command,
    cwd: "workspace",
    risk: approval.risk,
    reason: approval.message || approval.title || "approval required",
  };
}

function fallbackTitle(approval: ApprovalSummary): string {
  if (approval.operationId?.startsWith("filesystem.patch:")) return "Patch file";
  if (approval.action === "filesystem.write") return "Write file";
  if (approval.action === "terminal.command") return "Run command";
  if (approval.action === "test.run") return "Run validation";
  return "Approval required";
}
