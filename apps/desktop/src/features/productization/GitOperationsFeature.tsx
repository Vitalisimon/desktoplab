import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useApiClient } from "../../api/ApiProvider";
import type { GitOperationsSnapshot, SavePointSummary, WorktreeSummary } from "../../api/types";
import { EvidenceDisclosure, RouteExplanation, StatusRow } from "../../design/OperationalPrimitives";

export function GitOperationsFeature({
  workspaceId,
  workspaceRootPath,
}: {
  workspaceId: string;
  workspaceRootPath: string;
}) {
  const api = useApiClient();
  const queryClient = useQueryClient();
  const gitQueryKey = ["git-operations", workspaceId, workspaceRootPath];
  const query = useQuery({
    queryKey: gitQueryKey,
    queryFn: () => api.gitOperations(workspaceId),
    enabled: workspaceId.length > 0,
  });
  const rollback = useMutation({
    mutationFn: async (savePoint: SavePointSummary) => {
      const approval = await api.createApproval({
        sessionId: savePoint.sessionId,
        action: "git.rollback",
        operationId: savePoint.savePointId,
      });
      return api.rollbackSavePoint(savePoint.savePointId, { approvalId: approval.approvalId });
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: gitQueryKey }),
  });
  const commit = useMutation({
    mutationFn: async (snapshot: GitOperationsSnapshot) => {
      const approval = await api.createApproval({
        sessionId: snapshot.commit.sessionId,
        action: "git.commit",
        operationId: "git.commit",
        payload: {
          sessionId: snapshot.commit.sessionId,
          message: snapshot.commit.message,
          changeFingerprint: snapshot.commit.changeFingerprint,
          changedFiles: snapshot.changedFiles ?? [],
        },
      });
      return api.commitSession({
        workspaceId,
        sessionId: snapshot.commit.sessionId,
        message: snapshot.commit.message,
        changeFingerprint: snapshot.commit.changeFingerprint,
        changedFiles: snapshot.changedFiles ?? [],
        approvalId: approval.approvalId,
      });
    },
  });
  const push = useMutation({
    mutationFn: async (snapshot: GitOperationsSnapshot) => {
      const approval = await api.createApproval({
        sessionId: snapshot.commit.sessionId,
        action: "git.push",
        operationId: "git.push",
      });
      return api.pushBranch({ workspaceId, remote: snapshot.push.remote, branch: snapshot.push.branch, approvalId: approval.approvalId });
    },
  });
  const cleanup = useMutation({ mutationFn: (worktreeId: string) => api.cleanupWorktree(worktreeId) });

  if (query.isLoading) return <Panel title="Loading save points" body="DesktopLab is reading repository safety state." />;
  if (query.isError || !query.data) return <Panel title="Git unavailable" body="DesktopLab could not read repository safety state right now." />;

  return <GitOperationsView snapshot={query.data} rollback={(savePoint) => rollback.mutate(savePoint)} commit={() => commit.mutate(query.data)} push={() => push.mutate(query.data)} cleanup={(id) => cleanup.mutate(id)} />;
}

function GitOperationsView({
  snapshot,
  rollback,
  commit,
  push,
  cleanup,
}: {
  snapshot: GitOperationsSnapshot;
  rollback: (savePoint: SavePointSummary) => void;
  commit: () => void;
  push: () => void;
  cleanup: (worktreeId: string) => void;
}) {
  return (
    <div className="grid gap-4">
      <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
        <h2 className="text-lg font-semibold">Save points</h2>
        <p className="mt-1 text-sm text-muted">Review rollback readiness before applying agent changes.</p>
        <div className="mt-3 grid gap-3">
          {snapshot.warnings.map((warning) => (
            <StatusRow key={warning} label={warning} status={snapshot.workspaceState === "conflicted" ? "blocked" : "degraded"} detail="Review before rollback or commit." />
          ))}
          {snapshot.savePoints.map((savePoint) => (
            <SavePointRow key={savePoint.savePointId} savePoint={savePoint} onRollback={() => rollback(savePoint)} />
          ))}
        </div>
      </section>
      <section className="grid gap-4 lg:grid-cols-2">
        <ActionPanel title="Commit" body={snapshot.commit.preview} button="Commit approved work" disabled={!snapshot.commit.supported} onClick={commit} />
        <ActionPanel title="Push" body={snapshot.push.preview} button="Push with approval" disabled={!snapshot.push.supported} onClick={push} />
      </section>
      <PatchReviewPanel changedFiles={snapshot.changedFiles ?? []} diffPreview={snapshot.diffPreview} />
      <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
        <h2 className="text-lg font-semibold">Parallel work</h2>
        <div className="mt-3 grid gap-3">
          {snapshot.worktrees.map((worktree) => (
            <WorktreeRow key={worktree.worktreeId} worktree={worktree} onCleanup={() => cleanup(worktree.worktreeId)} />
          ))}
        </div>
      </section>
    </div>
  );
}

function PatchReviewPanel({ changedFiles, diffPreview }: { changedFiles: string[]; diffPreview?: string }) {
  if (!changedFiles.length && !diffPreview) return null;
  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <h2 className="text-lg font-semibold">Patch review</h2>
      {changedFiles.length ? <p className="mt-2 text-sm text-muted">Changed files: {changedFiles.join(", ")}</p> : null}
      {diffPreview ? <EvidenceDisclosure title="Diff preview" body={diffPreview} /> : null}
    </section>
  );
}

function SavePointRow({ savePoint, onRollback }: { savePoint: SavePointSummary; onRollback: () => void }) {
  const [preview, setPreview] = useState(false);
  return (
    <div className="rounded-desktop border border-line bg-panel p-3">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-sm font-semibold text-ink">{savePoint.title}</p>
          <p className="mt-1 text-sm text-muted">{savePoint.rollbackPreview}</p>
        </div>
        <button type="button" className="rounded-desktop border border-line px-3 py-2 text-sm font-medium disabled:opacity-45" disabled={!savePoint.rollbackSupported} onClick={() => setPreview(true)}>
          Rollback
        </button>
      </div>
      {preview ? (
        <div className="mt-3 rounded-desktop bg-elevated p-3">
          <p className="text-sm text-muted">{savePoint.rollbackPreview}</p>
          {savePoint.protectedUntrackedFiles?.length ? (
            <p className="mt-2 text-xs font-medium text-muted">Leaves local untracked files untouched: {savePoint.protectedUntrackedFiles.join(", ")}</p>
          ) : null}
          <button type="button" className="mt-2 rounded-desktop bg-ink px-3 py-2 text-sm font-medium text-canvas" onClick={onRollback}>
            Approve rollback
          </button>
        </div>
      ) : null}
    </div>
  );
}

function WorktreeRow({ worktree, onCleanup }: { worktree: WorktreeSummary; onCleanup: () => void }) {
  return (
    <div className="rounded-desktop border border-line bg-panel p-3">
      <RouteExplanation kind="local" summary={worktree.label} reasons={[worktree.isolationReason]} />
      <EvidenceDisclosure title="Worktree path" body={worktree.path} />
      {worktree.mergeRequiresApproval ? <p className="mt-2 text-xs font-medium text-muted">Merge requires explicit review and approval.</p> : null}
      <button type="button" className="mt-3 rounded-desktop border border-line px-3 py-2 text-sm font-medium disabled:opacity-45" disabled={!worktree.cleanupSupported || worktree.userOwned} onClick={onCleanup}>
        {worktree.userOwned ? "Clean up user worktree" : "Clean up worktree"}
      </button>
    </div>
  );
}

function ActionPanel({ title, body, button, disabled, onClick }: { title: string; body: string; button: string; disabled: boolean; onClick: () => void }) {
  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <h2 className="text-lg font-semibold">{title}</h2>
      <p className="mt-2 text-sm leading-6 text-muted">{body}</p>
      <button type="button" className="mt-3 rounded-desktop bg-ink px-3 py-2 text-sm font-medium text-canvas disabled:opacity-45" disabled={disabled} onClick={onClick}>
        {button}
      </button>
    </section>
  );
}

function Panel({ title, body }: { title: string; body: string }) {
  return (
    <section className="mx-auto max-w-3xl rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <h1 className="text-xl font-semibold">{title}</h1>
      <p className="mt-2 text-sm text-muted">{body}</p>
    </section>
  );
}
