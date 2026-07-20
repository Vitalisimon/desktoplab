import type { AgentSessionSnapshot } from "../../api/types";
import { EvidenceDisclosure } from "../../design/OperationalPrimitives";

type SessionTimelineViewProps = {
  session: AgentSessionSnapshot;
  title?: string;
  showSummary?: boolean;
};

export function SessionTimelineView({ session, title = "Timeline", showSummary = true }: SessionTimelineViewProps) {
  const events = [...session.timeline].sort((left, right) => left.sequence - right.sequence);

  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm" aria-labelledby="session-timeline-title">
      <h2 id="session-timeline-title" className="text-lg font-semibold">
        {title}
      </h2>

      {session.plan ? (
        <div className="mt-3 rounded-desktop bg-elevated px-3 py-2">
          <p className="text-xs font-semibold uppercase text-muted">Plan</p>
          <p className="mt-1 text-sm leading-6 text-ink">{session.plan}</p>
        </div>
      ) : null}

      {events.length === 0 ? (
        <p className="mt-3 rounded-desktop bg-elevated px-3 py-2 text-sm text-muted">No session events yet.</p>
      ) : (
        <ol className="mt-3 divide-y divide-line">
          {events.map((event) => (
            <li key={event.sequence} className="grid gap-2 py-3 first:pt-0 last:pb-0 md:grid-cols-[80px_1fr_120px] md:items-center">
              <span className="text-xs font-semibold text-muted">#{event.sequence}</span>
              <div className="min-w-0">
                <p className="text-sm font-semibold text-ink">{event.message}</p>
                <p className="mt-1 truncate text-xs text-muted">{event.kind}</p>
                <EventEvidence event={event} />
              </div>
              <span className="text-xs text-muted md:text-right" title={event.createdAt}>
                {formatEventTime(event.createdAt)}
              </span>
            </li>
          ))}
        </ol>
      )}

      {showSummary && session.summary ? <p className="mt-3 rounded-desktop bg-elevated px-3 py-2 text-sm font-medium text-ink">{session.summary}</p> : null}
    </section>
  );
}

function EventEvidence({ event }: { event: AgentSessionSnapshot["timeline"][number] }) {
  if (event.evidence) return <EvidenceDisclosure title={event.evidence.title} body={event.evidence.body} />;
  if (event.test) return <EvidenceDisclosure title={event.message} body={`${event.test.command}\n${event.test.output}`} />;
  return null;
}

function formatEventTime(createdAt: string): string {
  if (/^\d{10}$/.test(createdAt)) {
    return new Date(Number(createdAt) * 1000).toISOString().slice(11, 16);
  }
  const date = new Date(createdAt);
  if (Number.isNaN(date.getTime())) return createdAt;
  return date.toISOString().slice(11, 16);
}
