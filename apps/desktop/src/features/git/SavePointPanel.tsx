import type { WorkspaceSnapshot } from "../../api/types";

type SavePointPanelProps = {
  workspace: WorkspaceSnapshot;
};

export function SavePointPanel({ workspace }: SavePointPanelProps) {
  const ready = workspace.canCheckpointRiskyExecution;

  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <p className={`text-sm font-semibold ${ready ? "text-success" : "text-warning"}`}>{ready ? "Save point ready" : "Save point blocked"}</p>
      <h2 className="mt-1 text-lg font-semibold">Risky work gate</h2>
      <p className="mt-2 text-sm leading-6 text-muted">
        {ready
          ? "DesktopLab can require a save point before risky agent work."
          : "Commit or discard local changes before risky agent work."}
      </p>
    </section>
  );
}
