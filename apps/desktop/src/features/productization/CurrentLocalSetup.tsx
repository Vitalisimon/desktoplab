import type { ModelInventoryItem, RuntimeInventoryItem } from "../../api/types";

export function CurrentLocalSetup({ runtimes, models }: { runtimes: RuntimeInventoryItem[]; models: ModelInventoryItem[] }) {
  const activeRuntime = runtimes.find((runtime) => runtime.status === "running" || runtime.status === "ready");
  const readyModel = models.find((model) => model.installState === "installed" && model.compatibility === "ready");
  const downloadable = models.find((model) => model.installState === "downloadable" && model.compatibility !== "blocked");

  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <div>
        <h2 className="text-lg font-semibold">Current local setup</h2>
        <p className="mt-1 text-sm leading-6 text-muted">
          Downloaded models can replace the active local model for new agent sessions.
        </p>
      </div>
      <div className="mt-3 grid gap-3 sm:grid-cols-2">
        <PlainLocalState
          label="Runner"
          value={activeRuntime ? `${activeRuntime.displayName} is ${activeRuntime.status}` : "No local runner is active yet"}
        />
        <PlainLocalState
          label="Model"
          value={
            readyModel
              ? `${readyModel.displayName} is ready now`
              : downloadable
                ? `${downloadable.displayName} can be downloaded`
                : "No compatible model is available yet"
          }
        />
      </div>
    </section>
  );
}

function PlainLocalState({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-desktop border border-line bg-elevated px-3 py-2">
      <p className="text-xs font-semibold uppercase text-muted">{label}</p>
      <p className="mt-1 text-sm font-semibold text-ink">{value}</p>
    </div>
  );
}
