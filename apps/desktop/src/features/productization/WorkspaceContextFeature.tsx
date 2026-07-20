import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useApiClient } from "../../api/ApiProvider";
import type { SessionContextPreview, WorkspaceIntelligenceSnapshot, WorkspaceMemoryItem } from "../../api/types";
import { CapabilityList, EvidenceDisclosure, StatusRow } from "../../design/OperationalPrimitives";

export function WorkspaceContextFeature({ workspaceId }: { workspaceId: string }) {
  const api = useApiClient();
  const queryClient = useQueryClient();
  const intelligence = useQuery({ queryKey: ["workspace-intelligence", workspaceId], queryFn: () => api.workspaceIntelligence(workspaceId), enabled: workspaceId.length > 0 });
  const memory = useQuery({ queryKey: ["workspace-memory", workspaceId], queryFn: () => api.listWorkspaceMemory(workspaceId), enabled: workspaceId.length > 0 });
  const preview = useQuery({ queryKey: ["context-preview", workspaceId], queryFn: () => api.sessionContextPreview(workspaceId), enabled: workspaceId.length > 0 });
  const refresh = useMutation({
    mutationFn: () => api.refreshWorkspaceScan(workspaceId),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["workspace-intelligence", workspaceId] }),
  });
  const remove = useMutation({
    mutationFn: (memoryId: string) => api.deleteMemory(memoryId),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["workspace-memory", workspaceId] }),
  });

  if (intelligence.isLoading || memory.isLoading || preview.isLoading) return <Panel title="Loading context" body="DesktopLab is reading repository knowledge." />;
  if (intelligence.isError || memory.isError || preview.isError || !intelligence.data || !memory.data || !preview.data) {
    return <Panel title="Context unavailable" body="DesktopLab could not read repository knowledge right now." />;
  }

  return <WorkspaceContextView intelligence={intelligence.data} memories={memory.data.memories} preview={preview.data} onRefresh={() => refresh.mutate()} onDelete={(id) => remove.mutate(id)} />;
}

function WorkspaceContextView({
  intelligence,
  memories,
  preview,
  onRefresh,
  onDelete,
}: {
  intelligence: WorkspaceIntelligenceSnapshot;
  memories: WorkspaceMemoryItem[];
  preview: SessionContextPreview;
  onRefresh: () => void;
  onDelete: (memoryId: string) => void;
}) {
  return (
    <div className="mx-auto grid w-full max-w-6xl gap-4">
      <div>
        <h1 className="text-2xl font-semibold">Context</h1>
        <p className="mt-2 max-w-2xl text-sm leading-6 text-muted">Review what DesktopLab knows before an agent uses repository context.</p>
      </div>
      <section className="grid gap-4 lg:grid-cols-[1fr_0.9fr]">
        <div className="grid content-start gap-4">
          <IntelligencePanel intelligence={intelligence} onRefresh={onRefresh} />
          <ContextPreviewPanel preview={preview} />
        </div>
        <MemoryPanel memories={memories} onDelete={onDelete} />
      </section>
    </div>
  );
}

function IntelligencePanel({ intelligence, onRefresh }: { intelligence: WorkspaceIntelligenceSnapshot; onRefresh: () => void }) {
  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold">{intelligence.projectType}</h2>
          <p className="mt-1 text-sm text-muted">{intelligence.stale ? "Scan may be stale" : "Scan is current"}</p>
        </div>
        <button type="button" className="rounded-desktop border border-line px-3 py-2 text-sm font-medium disabled:opacity-45" disabled={!intelligence.refreshSupported} onClick={onRefresh}>
          Refresh scan
        </button>
      </div>
      <div className="mt-3 grid gap-2">
        {intelligence.facts.map((fact) => (
          <StatusRow key={`${fact.label}:${fact.value}`} label={fact.value} status={fact.confidence === "confirmed" ? "ready" : "degraded"} detail={fact.confidence} />
        ))}
        <CapabilityList capabilities={intelligence.testCommands.map((command) => command.command)} />
      </div>
      <ul className="mt-3 space-y-1">
        {intelligence.protectedSummary.map((item) => (
          <li key={item} className="text-sm text-muted">{item}</li>
        ))}
      </ul>
    </section>
  );
}

function ContextPreviewPanel({ preview }: { preview: SessionContextPreview }) {
  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <h2 className="text-lg font-semibold">Session context</h2>
      <p className="mt-2 text-sm text-muted">{preview.summary}</p>
      <EvidenceDisclosure title="Size and provenance" body={`${preview.sizeBudget}\n${preview.provenance.join("\n")}`} />
      {preview.cloudEgressWarning ? <p className="mt-3 text-sm text-warning">{preview.cloudEgressWarning}</p> : null}
    </section>
  );
}

function MemoryPanel({ memories, onDelete }: { memories: WorkspaceMemoryItem[]; onDelete: (memoryId: string) => void }) {
  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <h2 className="text-lg font-semibold">Memory</h2>
      <div className="mt-3 grid gap-3">
        {memories.map((memory) => (
          <div key={memory.memoryId} className="rounded-desktop border border-line p-3">
            <p className="text-sm font-semibold text-ink">{memory.title}</p>
            <p className="mt-1 text-sm text-muted">{memory.kind}</p>
            <p className="mt-1 text-sm text-muted">{memory.summary}</p>
            {memory.decisions.length > 0 ? (
              <ul className="mt-2 space-y-1">
                {memory.decisions.map((decision) => (
                  <li key={decision} className="text-xs font-medium text-muted">{decision}</li>
                ))}
              </ul>
            ) : null}
            <p className="mt-2 text-xs font-semibold text-muted">Source: {memory.source}</p>
            <p className="mt-1 text-xs font-semibold text-muted">{memory.redactionStatus === "local_only" ? "Local-only memory; export is not wired yet" : "Provider allowed"}</p>
            <div className="mt-3 flex gap-2">
              <button type="button" className="rounded-desktop border border-line px-3 py-2 text-sm font-medium" onClick={() => onDelete(memory.memoryId)}>
                Delete memory
              </button>
            </div>
          </div>
        ))}
      </div>
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
