import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useApiClient } from "../../api/ApiProvider";
import type { ModelDownloadResponse, ModelInventoryItem, RuntimeInstallResponse } from "../../api/types";
import { CurrentLocalSetup } from "./CurrentLocalSetup";
import { ModelRow } from "./ModelRow";
import { RuntimeInspectPanel } from "./RuntimeInspectPanel";
import { RuntimeRow } from "./RuntimeRow";

export function RuntimeModelFeature() {
  const api = useApiClient();
  const queryClient = useQueryClient();
  const [runtimeInstallState, setRuntimeInstallState] = useState<Record<string, RuntimeInstallResponse>>({});
  const [modelDownloadState, setModelDownloadState] = useState<Record<string, ModelDownloadResponse>>({});
  const runtimes = useQuery({ queryKey: ["runtimes"], queryFn: () => api.listRuntimes() });
  const models = useQuery({ queryKey: ["models"], queryFn: () => api.listModels() });
  const inspect = useQuery({ queryKey: ["runtime-inspect"], queryFn: () => api.runtimeInspect(), retry: 2, retryDelay: 250 });
  const install = useMutation({
    mutationFn: (runtimeId: string) => api.startRuntimeInstall({ runtimeId }),
    onSuccess: (response) => {
      setRuntimeInstallState((state) => ({ ...state, [response.runtimeId]: response }));
      void queryClient.invalidateQueries({ queryKey: ["runtimes"] });
    },
  });
  const download = useMutation({
    mutationFn: (model: ModelInventoryItem) => api.startModelDownload({ modelId: model.modelId, runtimeId: model.runtimeId }),
    onSuccess: (response) => {
      setModelDownloadState((state) => ({ ...state, [response.modelId]: response }));
      void queryClient.invalidateQueries({ queryKey: ["models"] });
    },
  });

  if (runtimes.isLoading || models.isLoading) return <Panel title="Loading models" body="DesktopLab is reading local runner and model readiness." />;
  if (runtimes.isError || models.isError || !runtimes.data || !models.data) {
    return <Panel title="Models unavailable" body="DesktopLab could not read local model readiness right now." />;
  }

  return (
    <div className="mx-auto grid w-full max-w-6xl gap-4">
      <div>
        <h1 className="text-2xl font-semibold">Models</h1>
        <p className="mt-2 max-w-2xl text-sm leading-6 text-muted">
          Choose the local coding model DesktopLab can use. Downloads and verification run as background work.
        </p>
      </div>

      <CurrentLocalSetup runtimes={runtimes.data.runtimes} models={models.data.models} />

      <RuntimeInspectPanel inspect={inspect.data} />

      <div className="grid gap-4 xl:grid-cols-[0.9fr_1.1fr]">
        <section className="grid content-start gap-3" aria-labelledby="runners-title">
          <h2 id="runners-title" className="text-lg font-semibold">
            Local runners
          </h2>
          {runtimes.data.runtimes.map((runtime) => (
            <RuntimeRow
              key={runtime.runtimeId}
              runtime={runtime}
              installState={runtimeInstallState[runtime.runtimeId]}
              onInstall={() => install.mutate(runtime.runtimeId)}
              installing={install.isPending}
            />
          ))}
        </section>
        <section className="grid content-start gap-3" aria-labelledby="models-title">
          <h2 id="models-title" className="text-lg font-semibold">
            Coding models
          </h2>
          {models.data.models.map((model) => (
            <ModelRow key={model.modelId} model={model} downloadState={modelDownloadState[model.modelId]} onDownload={() => download.mutate(model)} downloading={download.isPending} />
          ))}
        </section>
      </div>
    </div>
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
