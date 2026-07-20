import { Check, X } from "../../design/icons";
import type { ApprovalResolveRequest, ApprovalSummary } from "../../api/types";

type ApprovalCardProps = {
  approval: ApprovalSummary;
  isResolving?: boolean;
  onResolve: (approvalId: string, resolution: ApprovalResolveRequest["resolution"]) => void;
};

export function ApprovalCard({ approval, isResolving = false, onResolve }: ApprovalCardProps) {
  const state = stateUi(approval.state);
  const risk = riskUi(approval.risk);

  return (
    <article className="rounded-desktop border border-line bg-panel p-4 shadow-sm" aria-label={approval.title}>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <span className="rounded bg-elevated px-2 py-1 text-xs font-semibold text-muted">{actionLabel(approval.action)}</span>
            <span className={`rounded px-2 py-1 text-xs font-semibold ${risk.className}`}>{risk.label}</span>
          </div>
          <h2 className="mt-3 text-lg font-semibold">{approval.title}</h2>
          <p className="mt-2 max-w-2xl text-sm leading-6 text-muted">{approval.message}</p>
        </div>
        <span className={`rounded px-2 py-1 text-xs font-semibold ${state.className}`}>{state.label}</span>
      </div>

      <dl className="mt-4 grid gap-3 text-sm md:grid-cols-2">
        <div>
          <dt className="text-xs font-semibold uppercase text-muted">Session</dt>
          <dd className="mt-1 break-all font-medium">{approval.sessionId}</dd>
        </div>
        <div>
          <dt className="text-xs font-semibold uppercase text-muted">Requested</dt>
          <dd className="mt-1 font-medium" title={approval.requestedAt}>
            {formatTime(approval.requestedAt)}
          </dd>
        </div>
      </dl>

      {approval.policyReason ? (
        <p className="mt-4 rounded-desktop bg-elevated px-3 py-2 text-sm leading-6 text-muted">{approval.policyReason}</p>
      ) : null}

      {approval.state === "pending" ? (
        <div className="mt-4 flex flex-wrap gap-2">
          <button
            type="button"
            disabled={isResolving}
            onClick={() => onResolve(approval.approvalId, "approve")}
            className="inline-flex h-10 items-center gap-2 rounded-desktop bg-ink px-4 text-sm font-semibold text-canvas disabled:cursor-not-allowed disabled:bg-muted"
          >
            <Check size={15} />
            Allow
          </button>
          <button
            type="button"
            disabled={isResolving}
            onClick={() => onResolve(approval.approvalId, "deny")}
            className="inline-flex h-10 items-center gap-2 rounded-desktop border border-line bg-panel px-4 text-sm font-semibold text-ink hover:bg-elevated disabled:cursor-not-allowed disabled:text-muted"
          >
            <X size={15} />
            Deny
          </button>
        </div>
      ) : null}
    </article>
  );
}

function actionLabel(action: ApprovalSummary["action"]): string {
  switch (action) {
    case "filesystem.write":
      return "File change";
    case "terminal.command":
      return "Run command";
    case "git.commit":
      return "Git commit";
    case "git.push":
      return "Git push";
    case "provider.egress":
      return "External provider";
    case "fallback.route":
      return "Fallback";
    default:
      return "Approval";
  }
}

function riskUi(risk: ApprovalSummary["risk"]) {
  if (risk === "high") return { label: "High attention", className: "bg-danger/10 text-danger" };
  if (risk === "medium") return { label: "Medium attention", className: "bg-warning/10 text-warning" };
  return { label: "Low attention", className: "bg-success/10 text-success" };
}

function stateUi(state: ApprovalSummary["state"]) {
  switch (state) {
    case "pending":
      return { label: "Pending", className: "bg-warning/10 text-warning" };
    case "approved":
      return { label: "Approved", className: "bg-success/10 text-success" };
    case "denied":
      return { label: "Denied", className: "bg-danger/10 text-danger" };
    case "expired":
      return { label: "Expired", className: "bg-line text-muted" };
  }
}

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toISOString().slice(11, 16);
}
