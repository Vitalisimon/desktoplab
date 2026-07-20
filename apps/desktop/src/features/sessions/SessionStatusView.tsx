import type { AgentSessionSnapshot, AgentSessionState } from "../../api/types";
import { displayExecutionBackendName, displayWorkspaceName } from "../../domain/displayNames";

type SessionStatusViewProps = {
  session: AgentSessionSnapshot;
};

export function SessionStatusView({ session }: SessionStatusViewProps) {
  const state = stateUi(session.state);

  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm" aria-label={`Session ${session.sessionId}`}>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <p className="text-xs font-semibold uppercase text-muted">Session</p>
          <h2 className="mt-1 break-all text-lg font-semibold">{session.sessionId}</h2>
        </div>
        <span className={`rounded px-2 py-1 text-xs font-semibold ${state.className}`}>{state.label}</span>
      </div>

      <dl className="mt-4 grid gap-3 text-sm md:grid-cols-3">
        <div>
          <dt className="text-xs font-semibold uppercase text-muted">Owner</dt>
          <dd className="mt-1 break-all font-medium">{session.owner}</dd>
        </div>
        <div>
          <dt className="text-xs font-semibold uppercase text-muted">Agent runner</dt>
          <dd className="mt-1 break-all font-medium">{displayExecutionBackendName(session.executionBackendId)}</dd>
        </div>
        <div>
          <dt className="text-xs font-semibold uppercase text-muted">Repository</dt>
          <dd className="mt-1 break-all font-medium">{displayWorkspaceName(session.workspaceId)}</dd>
        </div>
      </dl>
    </section>
  );
}

function stateUi(state: AgentSessionState) {
  switch (state) {
    case "created":
      return { label: "Created", className: "bg-line text-muted" };
    case "planning":
      return { label: "Planning", className: "bg-accent/10 text-accent" };
    case "running":
      return { label: "Running", className: "bg-accent/10 text-accent" };
    case "paused":
      return { label: "Paused", className: "bg-warning/10 text-warning" };
    case "blocked":
      return { label: "Blocked", className: "bg-warning/10 text-warning" };
    case "failed":
      return { label: "Failed", className: "bg-danger/10 text-danger" };
    case "cancelled":
      return { label: "Cancelled", className: "bg-line text-muted" };
    case "completed":
      return { label: "Completed", className: "bg-success/10 text-success" };
  }
}
