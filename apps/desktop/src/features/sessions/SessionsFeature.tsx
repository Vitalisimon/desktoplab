import { useEffect, useMemo, useState } from "react";
import type { AgentSessionSnapshot, SessionCreateRequest } from "../../api/types";
import { SessionStartForm } from "./SessionStartForm";
import { SessionStatusView } from "./SessionStatusView";
import { SessionSummaryView } from "./SessionSummaryView";
import { SessionTimelineView } from "./SessionTimelineView";
import { useSessions } from "./useSessions";

type SessionsFeatureProps = {
  workspaceId: string;
  executionBackends: string[];
};

export function SessionsFeature({ workspaceId, executionBackends }: SessionsFeatureProps) {
  const sessionsState = useSessions(workspaceId);
  const [selectedSession, setSelectedSession] = useState<AgentSessionSnapshot | null>(null);
  const sessions = sessionsState.sessions;

  useEffect(() => {
    if (!selectedSession && sessions.length > 0) {
      setSelectedSession(sessions[0]);
    }
  }, [selectedSession, sessions]);

  const activeSession = useMemo(
    () => sessions.find((session) => session.sessionId === selectedSession?.sessionId) ?? selectedSession ?? sessions[0] ?? null,
    [selectedSession, sessions],
  );

  async function createSession(request: SessionCreateRequest) {
    const created = await sessionsState.create.mutateAsync(request);
    setSelectedSession(created);
  }

  if (sessionsState.isLoading) {
    return <SessionsPanel title="Loading sessions" body="DesktopLab is reading local agent session history." />;
  }

  if (sessionsState.isError) {
    return <SessionsPanel title="Sessions unavailable" body="DesktopLab could not read session history right now." />;
  }

  return (
    <div className="mx-auto grid w-full max-w-6xl gap-4">
      <section className="rounded-desktop border border-line bg-panel p-5 shadow-sm">
        <h1 className="text-2xl font-semibold">Sessions</h1>
        <p className="mt-2 max-w-2xl text-sm leading-6 text-muted">
          Ask DesktopLab to work on this repository, then review progress, summaries and saved states.
        </p>
      </section>

      <SessionStartForm
        workspaceId={workspaceId}
        backends={executionBackends}
        isCreating={sessionsState.create.isPending}
        onCreate={(request) => {
          void createSession(request);
        }}
      />

      {activeSession ? (
        <div className="grid gap-4 lg:grid-cols-[360px_1fr]">
          <div className="grid content-start gap-4">
            <SessionStatusView session={activeSession} />
            <SessionSummaryView session={activeSession} />
          </div>
          <SessionTimelineView session={activeSession} showSummary={false} />
        </div>
      ) : (
        <p className="rounded-desktop border border-line bg-panel px-4 py-3 text-sm text-muted shadow-sm">No agent sessions for this workspace yet.</p>
      )}
    </div>
  );
}

function SessionsPanel({ title, body }: { title: string; body: string }) {
  return (
    <section className="mx-auto max-w-3xl rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <h1 className="text-2xl font-semibold">{title}</h1>
      <p className="mt-2 text-sm leading-6 text-muted">{body}</p>
    </section>
  );
}
