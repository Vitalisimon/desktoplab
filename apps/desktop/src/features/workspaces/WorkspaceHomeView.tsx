import type { ReactNode } from "react";
import { CheckCircle2, GitBranch, History, MessageSquarePlus, ShieldAlert } from "../../design/icons";
import type { RecentSessionSummary, WorkspaceHomeSnapshot } from "../../api/types";
import { displayExecutionBackendName, displayRecentSessionState } from "../../domain/displayNames";

type WorkspaceHomeViewProps = {
  home: WorkspaceHomeSnapshot;
};

export function WorkspaceHomeView({ home }: WorkspaceHomeViewProps) {
  const workspace = home.workspace;
  const checkpointLabel = workspace.canCheckpointRiskyExecution ? "Save point ready" : "Save point blocked";

  return (
    <div className="mx-auto grid w-full max-w-6xl gap-4">
      <section className="rounded-desktop border border-line p-5 dl-panel">
        <div className="flex items-start justify-between gap-4">
          <div>
            <h1 className="text-2xl font-semibold">{workspace.displayName}</h1>
            <p className="mt-2 text-sm text-muted">{workspace.rootPath}</p>
          </div>
          <span className={`rounded px-2 py-1 text-xs font-semibold ${workspace.apiState === "clean" ? "bg-success/10 text-success" : "bg-warning/10 text-warning"}`}>
            {workspace.apiState === "clean" ? "Clean" : "Dirty"}
          </span>
        </div>

        <div className="mt-5 grid gap-3 md:grid-cols-3">
          <SummaryTile label="Repository" value={workspace.rootPath} icon={<GitBranch size={16} />} />
          <SummaryTile label="Save point" value={checkpointLabel} icon={<ShieldAlert size={16} />} />
          <SummaryTile label="Local setup" value={home.runtimeHealth.label} icon={<CheckCircle2 size={16} />} />
        </div>
      </section>

      <section className="rounded-desktop border border-line p-5 dl-panel">
        <h2 className="text-lg font-semibold">Repository changes</h2>
        {workspace.statusEntries.length === 0 ? (
          <EmptyState icon={<CheckCircle2 size={28} />} title="Working tree is clean" description="No local changes reported." />
        ) : (
          <ul className="mt-3 space-y-2">
            {workspace.statusEntries.map((entry) => (
              <li key={entry} className="rounded-desktop bg-elevated px-3 py-2 text-sm font-medium">
                {entry}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="rounded-desktop border border-line p-5 dl-panel">
        <div className="flex items-center gap-2">
          <History size={17} className="text-muted" />
          <h2 className="text-lg font-semibold">Recent sessions</h2>
        </div>
        {home.recentSessions.length === 0 ? (
          <EmptyState icon={<MessageSquarePlus size={30} />} title="Ready when you are" description="Start a new chat to create the first agent session for this repository." />
        ) : (
          <div className="mt-3 divide-y divide-line rounded-desktop border border-line">
            {home.recentSessions.map((session) => (
              <SessionRow key={session.sessionId} session={session} />
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

function SummaryTile({ label, value, icon }: { label: string; value: string; icon: ReactNode }) {
  return (
    <div className="min-w-0 rounded-desktop p-3 dl-elevated">
      <div className="flex items-center gap-2 text-xs font-semibold uppercase text-muted">
        {icon}
        {label}
      </div>
      <p className="mt-2 break-words text-sm font-semibold leading-5">{value}</p>
    </div>
  );
}

function EmptyState({ icon, title, description }: { icon: ReactNode; title: string; description: string }) {
  return (
    <div className="mt-3 flex items-center gap-3 rounded-desktop border border-line px-4 py-4 dl-elevated">
      <div className="grid h-12 w-12 shrink-0 place-items-center rounded-desktop bg-accent/10 text-accent">{icon}</div>
      <div>
        <p className="text-sm font-semibold text-ink">{title}</p>
        <p className="mt-1 text-sm leading-5 text-muted">{description}</p>
      </div>
    </div>
  );
}

function SessionRow({ session }: { session: RecentSessionSummary }) {
  return (
    <div className="grid grid-cols-[1fr_auto] gap-3 px-3 py-2 text-sm">
      <div className="min-w-0">
        <p className="truncate font-semibold">{displayExecutionBackendName(session.backendId)}</p>
        <p className="truncate text-xs text-muted">{session.sessionId}</p>
      </div>
      <div className="text-right">
        <p className="font-semibold">{displayRecentSessionState(session.state)}</p>
        <p className="text-xs text-muted">{session.updatedAt}</p>
      </div>
    </div>
  );
}
