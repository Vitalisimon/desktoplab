import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import type { HighEndLocalSetupPreview, HighEndRuntimeHealthResponse } from "../../api/types";
import { useApiClient } from "../../api/ApiProvider";
import { CheckCircle2, ChevronDown, Cpu, HardDrive, PlugZap, RefreshCw } from "../../design/icons";

export function HighEndLocalSetupPanel({
  setup,
  embedded = false,
}: {
  setup: HighEndLocalSetupPreview;
  embedded?: boolean;
}) {
  const api = useApiClient();
  const queryClient = useQueryClient();
  const initialRuntime = setup.runtimeChoices.find((choice) => choice.runtimeId === setup.recommendedRuntimeId) ?? setup.runtimeChoices[0];
  const [runtimeId, setRuntimeId] = useState(initialRuntime?.runtimeId ?? "");
  const [endpoint, setEndpoint] = useState(initialRuntime?.defaultEndpoint ?? "");
  const [models, setModels] = useState<string[]>([]);
  const [modelId, setModelId] = useState("");
  const [route, setRoute] = useState<HighEndRuntimeHealthResponse | null>(null);
  const selectedRuntime = setup.runtimeChoices.find((choice) => choice.runtimeId === runtimeId);
  const discovery = useMutation({
    mutationFn: () => api.discoverHighEndRuntime({ runtimeId, endpoint }),
    onSuccess: (response) => {
      setModels(response.models);
      setModelId(response.models[0] ?? "");
      setRoute(null);
    },
  });
  const attach = useMutation({
    mutationFn: () => api.attachHighEndRuntime({ runtimeId, endpoint, modelId }),
    onSuccess: async (response) => {
      setRoute(response);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["app-state"] }),
        queryClient.invalidateQueries({ queryKey: ["route-options"] }),
        queryClient.invalidateQueries({ queryKey: ["settings", "setup-preview"] }),
      ]);
    },
  });

  return (
    <section
      aria-labelledby="high-end-local-title"
      className={embedded ? "grid gap-4" : "rounded-desktop border border-line p-4 dl-panel"}
    >
      <header className="flex items-start justify-between gap-4">
        <div>
          <h2 id="high-end-local-title" className="text-lg font-semibold">High-capacity local setup</h2>
          <p className="mt-1 max-w-2xl text-sm leading-6 text-muted">
            DesktopLab found hardware suited to a larger local model service.
          </p>
        </div>
        <span className="shrink-0 rounded bg-success/10 px-2 py-1 text-xs font-semibold text-success">Detected</span>
      </header>

      <div className="grid gap-x-6 gap-y-3 border-y border-line py-4 sm:grid-cols-2">
        <SetupFact icon={<Cpu size={16} />} label="Computer" value={setup.profileLabel} detail={setup.hardwareSummary} />
        <SetupFact icon={<PlugZap size={16} />} label="Recommended runner" value={selectedRuntime?.displayName ?? "Choose a runner"} detail="Local or private network only" />
        <SetupFact icon={<HardDrive size={16} />} label="Model storage" value={setup.storageTarget.displayPath ?? compactStoragePath(setup.storageTarget.path)} detail={setup.storageTarget.freeGb ? `${setup.storageTarget.freeGb} GB free` : "Capacity check required"} />
        <SetupFact icon={<CheckCircle2 size={16} />} label="Expected route" value="Large local coding model" detail="Live certification remains separate" />
      </div>

      <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
        <label className="grid gap-1.5 text-sm font-medium text-ink">
          Local runner
          <select
            value={runtimeId}
            onChange={(event) => {
              const choice = setup.runtimeChoices.find((candidate) => candidate.runtimeId === event.target.value);
              setRuntimeId(event.target.value);
              setEndpoint(choice?.defaultEndpoint ?? "");
              setModels([]);
              setModelId("");
              setRoute(null);
            }}
            className="h-10 w-full rounded-desktop border border-line bg-elevated px-3 text-sm text-ink outline-none focus:border-accent"
          >
            {setup.runtimeChoices.map((choice) => <option key={choice.runtimeId} value={choice.runtimeId}>{choice.displayName}</option>)}
          </select>
        </label>
        <button
          type="button"
          disabled={discovery.isPending || !runtimeId || !endpoint}
          onClick={() => discovery.mutate()}
          className="inline-flex h-10 items-center justify-center gap-2 rounded-desktop bg-accent px-4 text-sm font-semibold text-white disabled:cursor-not-allowed disabled:opacity-50"
        >
          <RefreshCw size={16} /> {discovery.isPending ? "Checking" : "Check local runner"}
        </button>
      </div>

      {discovery.isError ? (
        <p role="alert" className="border-l-2 border-warning pl-3 text-sm leading-6 text-muted">
          The runner did not answer yet. Start {selectedRuntime?.displayName ?? "the selected runner"}, then retry. Open connection details only for a private network service.
        </p>
      ) : null}

      {models.length > 0 ? (
        <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
          <label className="grid gap-1.5 text-sm font-medium text-ink">
            Available model
            <select value={modelId} onChange={(event) => setModelId(event.target.value)} className="h-10 w-full rounded-desktop border border-line bg-elevated px-3 text-sm text-ink outline-none focus:border-accent">
              {models.map((model) => <option key={model} value={model}>{model}</option>)}
            </select>
          </label>
          <button
            type="button"
            disabled={attach.isPending || !modelId}
            onClick={() => attach.mutate()}
            className="inline-flex h-10 items-center justify-center gap-2 rounded-desktop border border-accent bg-panel px-4 text-sm font-semibold text-accent disabled:cursor-not-allowed disabled:opacity-50"
          >
            <PlugZap size={16} /> {attach.isPending ? "Connecting" : "Use this local route"}
          </button>
        </div>
      ) : null}

      {route ? <RouteResult route={route} /> : null}
      {attach.isError ? <p role="alert" className="text-sm text-danger">The model service answered, but this model is not ready. Check the selected model and retry.</p> : null}

      <details className="border-t border-line pt-3 text-sm">
        <summary className="flex cursor-pointer list-none items-center justify-between font-medium text-muted">
          Connection details <ChevronDown size={15} />
        </summary>
        <label className="mt-3 grid gap-1.5 text-sm font-medium text-ink">
          Local or private network address
          <input value={endpoint} onChange={(event) => { setEndpoint(event.target.value); setModels([]); setModelId(""); setRoute(null); }} className="h-10 rounded-desktop border border-line bg-elevated px-3 font-mono text-xs text-ink outline-none focus:border-accent" />
        </label>
        <p className="mt-2 text-xs leading-5 text-muted">Public internet addresses are rejected. DesktopLab keeps session ownership and approvals.</p>
      </details>
    </section>
  );
}

function SetupFact({ icon, label, value, detail }: { icon: React.ReactNode; label: string; value: string; detail: string }) {
  return <div className="flex min-w-0 gap-3"><span className="mt-0.5 text-accent">{icon}</span><span className="min-w-0"><span className="block text-xs font-semibold uppercase text-muted">{label}</span><span className="mt-1 block break-words text-sm font-semibold text-ink">{value}</span><span className="mt-0.5 block text-xs text-muted">{detail}</span></span></div>;
}

function RouteResult({ route }: { route: HighEndRuntimeHealthResponse }) {
  const ready = route.routeEligibility === "eligible";
  return <div role="status" className={`border-l-2 pl-3 text-sm leading-6 ${ready ? "border-success text-success" : "border-warning text-muted"}`}>{ready ? "Local agent route ready" : "Runner connected; the selected model is still loading"}</div>;
}

function compactStoragePath(path: string) {
  const marker = "/.desktoplab/";
  const index = path.indexOf(marker);
  return index >= 0 ? `~${path.slice(index)}` : path;
}
