import type { RuntimeInspectSnapshot } from "../../api/types";

export function RuntimeInspectPanel({ inspect }: { inspect?: RuntimeInspectSnapshot }) {
  if (!inspect) return null;
  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold">Active runtime route</h2>
          <p className="mt-1 text-sm leading-6 text-muted">{inspect.active.selectedRouteId}</p>
        </div>
        <span className="rounded bg-elevated px-2 py-1 text-xs font-semibold text-muted">{inspect.inspectState}</span>
      </div>
      <div className="mt-3 grid gap-3 sm:grid-cols-2">
        <InspectTile title="Configured" lines={[inspect.active.backendId, inspect.active.runtimeId ?? "No runtime", inspect.active.modelId ?? "No model"]} />
        <InspectTile title="Live evidence" lines={[inspect.evidence.liveRuntime.state, inspect.evidence.liveRuntime.evidence ?? inspect.active.degradedReason ?? "No live evidence"]} />
      </div>
    </section>
  );
}

function InspectTile({ title, lines }: { title: string; lines: string[] }) {
  return (
    <div className="rounded-desktop bg-elevated px-3 py-2">
      <p className="text-xs font-semibold uppercase text-muted">{title}</p>
      <div className="mt-1 grid gap-1">
        {lines.map((line) => (
          <p key={line} className="truncate text-sm font-medium text-ink">
            {line}
          </p>
        ))}
      </div>
    </div>
  );
}
