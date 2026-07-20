import type { ApprovalResolveRequest } from "../../api/types";
import { ApprovalCard } from "./ApprovalCard";
import { useApprovals } from "./useApprovals";

export function ApprovalsFeature() {
  const approvals = useApprovals();

  if (approvals.isLoading) {
    return <ApprovalsPanel title="Loading approvals" body="DesktopLab is reading decisions that need your review." />;
  }

  if (approvals.isError) {
    return <ApprovalsPanel title="Approvals unavailable" body="DesktopLab could not read pending approvals right now." />;
  }

  return (
    <div className="mx-auto grid w-full max-w-5xl gap-4">
      <section className="rounded-desktop border border-line bg-panel p-5 shadow-sm">
        <h1 className="text-2xl font-semibold">Approvals</h1>
        <p className="mt-2 max-w-2xl text-sm leading-6 text-muted">
          Review actions before DesktopLab lets an agent change files, run commands, use Git or send data outside the local machine.
        </p>
      </section>

      {approvals.approvals.length === 0 ? (
        <p className="rounded-desktop border border-line bg-panel px-4 py-3 text-sm text-muted shadow-sm">No approvals waiting.</p>
      ) : (
        <div className="grid gap-3">
          {approvals.approvals.map((approval) => (
            <ApprovalCard
              key={approval.approvalId}
              approval={approval}
              isResolving={approvals.resolve.isPending && approvals.resolve.variables?.approvalId === approval.approvalId}
              onResolve={(approvalId, resolution) => resolveApproval(approvals.resolve.mutate, approvalId, resolution)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function resolveApproval(
  mutate: (input: ApprovalResolveRequest & { approvalId: string }) => void,
  approvalId: string,
  resolution: ApprovalResolveRequest["resolution"],
) {
  mutate({ approvalId, resolution });
}

function ApprovalsPanel({ title, body }: { title: string; body: string }) {
  return (
    <section className="mx-auto max-w-3xl rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <h1 className="text-2xl font-semibold">{title}</h1>
      <p className="mt-2 text-sm leading-6 text-muted">{body}</p>
    </section>
  );
}
