import { useQuery } from "@tanstack/react-query";
import { FolderOpen } from "../../design/icons";
import type { WorkspaceSnapshot } from "../../api/types";
import { useApiClient } from "../../api/ApiProvider";
import { ChangeStatusList } from "./ChangeStatusList";
import { CommitPolicyPanel } from "./CommitPolicyPanel";
import { DiffPreview } from "./DiffPreview";
import { SavePointPanel } from "./SavePointPanel";
import { GitOperationsFeature } from "../productization/GitOperationsFeature";

type ChangesFeatureProps = {
  workspace: WorkspaceSnapshot | null;
};

export function ChangesFeature({ workspace }: ChangesFeatureProps) {
  const api = useApiClient();
  const git = useQuery({
    queryKey: ["git-operations", workspace?.workspaceId, workspace?.rootPath],
    queryFn: () => api.gitOperations(workspace!.workspaceId),
    enabled: Boolean(workspace?.workspaceId),
  });

  if (!workspace) {
    return (
      <section className="mx-auto max-w-3xl rounded-desktop border border-line bg-panel p-5 shadow-sm">
        <div className="flex items-start gap-3">
          <div className="grid h-9 w-9 place-items-center rounded-desktop bg-elevated text-muted">
            <FolderOpen size={18} />
          </div>
          <div>
            <h1 className="text-2xl font-semibold">Open a project folder</h1>
            <p className="mt-2 text-sm leading-6 text-muted">Open a code folder before reviewing changes and save points.</p>
          </div>
        </div>
      </section>
    );
  }

  const state = git.data?.workspaceState ?? workspace.apiState;
  const statusEntries = git.data?.statusEntries
    ?? git.data?.changedFiles?.map((file) => liveStatusEntry(file, workspace.statusEntries))
    ?? workspace.statusEntries;
  const diffText = git.data?.diffPreview ?? workspace.diffText;
  const hasChanges = state !== "clean" || statusEntries.length > 0 || diffText.trim().length > 0;

  return (
    <div className="mx-auto grid w-full max-w-6xl gap-4">
      <section className="rounded-desktop border border-line bg-panel p-5 shadow-sm">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h1 className="text-2xl font-semibold">Changes</h1>
            <p className="mt-2 max-w-2xl text-sm leading-6 text-muted">
              Review repository changes, file preview and save point readiness before an agent writes code.
            </p>
          </div>
          <span className={`rounded px-2 py-1 text-xs font-semibold ${hasChanges ? "bg-warning/10 text-warning" : "bg-success/10 text-success"}`}>
            {hasChanges ? "Changes found" : "No changes"}
          </span>
        </div>
      </section>

      <div className="grid gap-4 lg:grid-cols-[1fr_320px]">
        <div className="grid content-start gap-4">
          <ChangeStatusList entries={statusEntries} />
          <DiffPreview diffText={diffText} />
        </div>
        <div className="grid content-start gap-4">
          <SavePointPanel workspace={workspace} />
          <CommitPolicyPanel />
        </div>
      </div>

      <GitOperationsFeature
        workspaceId={workspace.workspaceId}
        workspaceRootPath={workspace.rootPath}
      />
    </div>
  );
}

function liveStatusEntry(file: string, snapshotEntries: string[]) {
  return snapshotEntries.find((entry) => entry.endsWith(file)) ?? file;
}
