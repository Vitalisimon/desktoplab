import type { AgentSessionSnapshot } from "../../api/types";

type SessionSummaryViewProps = {
  session: AgentSessionSnapshot;
};

export function SessionSummaryView({ session }: SessionSummaryViewProps) {
  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm" aria-labelledby="session-summary-title">
      <h2 id="session-summary-title" className="text-lg font-semibold">
        Summary
      </h2>

      {session.summary ? (
        <p className="mt-3 rounded-desktop bg-elevated px-3 py-2 text-sm leading-6 text-ink">{session.summary}</p>
      ) : (
        <p className="mt-3 rounded-desktop bg-elevated px-3 py-2 text-sm text-muted">No summary yet.</p>
      )}

      <h3 className="mt-4 text-sm font-semibold">Save points</h3>
      {session.checkpoints.length === 0 ? (
        <p className="mt-2 rounded-desktop bg-elevated px-3 py-2 text-sm text-muted">No save points yet.</p>
      ) : (
        <ul className="mt-2 space-y-2">
          {session.checkpoints.map((checkpoint) => (
            <li key={checkpoint} className="break-all rounded-desktop bg-elevated px-3 py-2 text-sm font-medium">
              {checkpoint}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
