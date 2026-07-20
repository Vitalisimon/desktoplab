import type { AgentSessionSnapshot } from "../../api/types";
import { type BackendEventFrame, agentConversationEventsFromFrames } from "../../api/events";

export function AgentStatusPill({ session }: { session: AgentSessionSnapshot }) {
  const status = agentStatusLabel(session);
  return (
    <div aria-label="Agent status" aria-live="polite" className="inline-flex items-center gap-2 rounded-full border border-line/80 bg-panel/70 px-2.5 py-1 text-xs font-medium text-muted">
      <span className={`h-1.5 w-1.5 rounded-full ${status.tone} ${status.active ? "dl-running-dot" : ""}`} />
      <span>{status.label}</span>
    </div>
  );
}

export function AgentEventStream({ events }: { events: ReturnType<typeof agentConversationEventsFromFrames> }) {
  const current = events.at(-1);
  if (!current) return null;
  return (
    <div aria-label="Agent progress" className="max-w-3xl text-sm text-muted">
      <div className="flex min-h-8 items-center gap-2">
        <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-accent" />
        <span className="min-w-0 truncate">{current.message}</span>
      </div>
      {events.length > 1 ? (
        <details className="mt-1">
          <summary className="cursor-pointer text-xs font-medium text-muted">Earlier activity ({events.length - 1})</summary>
          <ol className="mt-2 space-y-1 border-l border-line pl-3">
            {events.slice(0, -1).map((event) => <li key={event.eventId}>{event.message}</li>)}
          </ol>
        </details>
      ) : null}
    </div>
  );
}

export function streamedAgentEvents(session: AgentSessionSnapshot, eventFrames: BackendEventFrame[]) {
  return shouldShowProgressEvents(session) ? agentConversationEventsFromFrames(eventFrames, session.sessionId) : [];
}

export function shouldShowProgressEvents(session: AgentSessionSnapshot): boolean {
  if (session.state === "created" || session.state === "planning" || session.state === "running" || session.state === "paused") return true;
  if (session.state !== "blocked") return false;
  return (session.pendingApprovals?.length ?? 0) > 0 || session.timeline.some(isClarificationEvent);
}

function agentStatusLabel(session: AgentSessionSnapshot): { label: string; tone: string; active?: boolean } {
  if (session.state === "completed") return { label: "Complete", tone: "bg-success" };
  if (session.job?.state === "interrupted") return { label: "Interrupted", tone: "bg-warning" };
  if (session.state === "blocked" && (session.pendingApprovals?.length ?? 0) > 0) {
    return { label: "Waiting for approval", tone: "bg-warning" };
  }
  if (session.state === "blocked" && session.timeline.some(isClarificationEvent)) {
    return { label: "Needs input", tone: "bg-accent" };
  }
  if (session.state === "failed") return { label: "Failed", tone: "bg-warning" };
  if (session.state === "blocked") return { label: "Blocked", tone: "bg-warning" };
  if (session.state === "created" || session.state === "planning" || session.state === "running") return { label: "Working", tone: "bg-accent", active: true };
  if (session.state === "paused") return { label: "Paused", tone: "bg-muted" };
  return { label: "Complete", tone: "bg-success" };
}

function isClarificationEvent(event: AgentSessionSnapshot["timeline"][number]): boolean {
  return event.kind.toLowerCase().includes("clarif") || event.message.trim().toLowerCase().startsWith("clarification_required:");
}
