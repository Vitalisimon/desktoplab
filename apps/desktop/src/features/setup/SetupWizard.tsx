import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { useApiClient } from "../../api/ApiProvider";
import { useControlPlaneStatus } from "../../app/useControlPlaneStatus";
import type { ModelInventoryItem, RuntimeInventoryItem, SetupChoice, SetupJobProgressItem, SetupPlanPreview } from "../../api/types";
import { SetupCatalogPanel } from "./SetupCatalogPanel";
import { HardwareSummary } from "./HardwareSummary";
import { HighEndLocalSetupPanel } from "./HighEndLocalSetupPanel";
import { displayLocalModelName } from "../../domain/displayNames";
import { InstallPlanPreview } from "./InstallPlanPreview";
import { RecommendationView } from "./RecommendationView";
import { isSelectableForLocalSetup, selectedRecommendation } from "./RecommendationDetails";
import { SetupJobProgress } from "./SetupJobProgress";
import { SetupReadinessResult } from "./SetupReadinessResult";
import { SetupStepper } from "./SetupStepper";
import { CatalogRefreshPanel, RuntimeOfflineNotice, SetupPanel } from "./SetupSupportPanels";
import { useSetupPreview } from "./useSetupPreview";
import { setupFailureCopy } from "./setupFailureCopy";
import {
  jobFromModelDownload,
  jobFromRuntimeInstall,
  jobFromStartedId,
  jobsFromAcceptance,
  jobsFromPipeline,
  mergeSetupJobs,
  selectedId,
  setupChoiceFor,
  uniqueJobIds,
} from "./setupJobMapping";

type SetupWizardProps = {
  onOpenRepository: () => void;
  hasActiveWorkspace?: boolean;
};

export function SetupWizard({ onOpenRepository, hasActiveWorkspace = false }: SetupWizardProps) {
  const api = useApiClient();
  const queryClient = useQueryClient();
  const setup = useSetupPreview();
  const controlPlane = useControlPlaneStatus();
  const appState = useQuery({
    queryKey: ["app-state"],
    queryFn: () => api.appState(),
    refetchInterval: (query) => (query.state.data?.setup?.state === "ready" ? false : 1_000),
  });
  const [startedJobIds, setStartedJobIds] = useState<string[]>([]);
  const [backendJobs, setBackendJobs] = useState<SetupJobProgressItem[]>([]);
  const [catalogRefreshJobId, setCatalogRefreshJobId] = useState<string | null>(null);
  const [runtimeInstallBlock, setRuntimeInstallBlock] = useState<"offline" | null>(null);
  const [selectedRuntimeId, setSelectedRuntimeId] = useState<string | undefined>();
  const [selectedModelId, setSelectedModelId] = useState<string | undefined>();
  const [runtimeSetupChoice, setRuntimeSetupChoice] = useState<SetupChoice>("use_existing");
  const [modelSetupChoice, setModelSetupChoice] = useState<SetupChoice>("use_existing");
  const models = useQuery({ queryKey: ["setup", "catalog-models"], queryFn: () => api.listModels() });
  const runtimes = useQuery({ queryKey: ["setup", "catalog-runtimes"], queryFn: () => api.listRuntimes() });
  const installRuntime = useMutation({
    mutationFn: (runtime: RuntimeInventoryItem) => api.startRuntimeInstall({ runtimeId: runtime.runtimeId, setupChoice: "install" }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["setup", "catalog-runtimes"] });
      void queryClient.invalidateQueries({ queryKey: ["app-state"] });
    },
  });
  const downloadModel = useMutation({
    mutationFn: (model: ModelInventoryItem) =>
      api.startModelDownload({ modelId: model.modelId, runtimeId: model.runtimeId, setupChoice: "install" }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["setup", "catalog-models"] });
      void queryClient.invalidateQueries({ queryKey: ["route-options"] });
      void queryClient.invalidateQueries({ queryKey: ["app-state"] });
    },
  });
  const catalogRefresh = useQuery({ queryKey: ["setup", "catalog-refresh"], queryFn: () => api.catalogRefreshStatus() });
  const startCatalogRefresh = useMutation({
    mutationFn: () => api.startCatalogRefresh(),
    onSuccess: (response) => setCatalogRefreshJobId(response.jobId ?? "blocked"),
  });

  const progress = useMemo(
    () => ({
      sequence: startedJobIds.length,
      jobs:
        backendJobs.length > 0
          ? backendJobs
          : jobsFromPipeline(appState.data?.setupPipeline).length > 0
            ? jobsFromPipeline(appState.data?.setupPipeline)
            : startedJobIds.map(jobFromStartedId),
    }),
    [appState.data?.setupPipeline, backendJobs, startedJobIds],
  );

  if (setup.preview.isLoading || controlPlane.isLoading || appState.isLoading) {
    return <SetupPanel title="Checking local setup" body="DesktopLab is reading hardware and compatibility data." />;
  }

  if (setup.preview.isError || !setup.preview.data) {
    return <SetupPanel title="Setup data unavailable" body="DesktopLab could not read setup recommendations right now." />;
  }

  const preview = setup.preview.data;
  const setupState = appState.data?.setup?.state ?? "not_started";
  const readiness =
    setupState === "ready"
      ? (appState.data?.readiness ?? controlPlane.readiness.data ?? { state: "ready" as const })
      : {
          state: "blocked" as const,
          degradedReasons: [setupFailureCopy(appState.data?.setup?.blockedReason) ?? "Setup is not verified yet."],
        };
  const setupReady = setupState === "ready";
  const selectedModelForPlan = selectedRecommendation(preview.modelRecommendations, selectedModelId);
  const selectedRuntimeIdForPlan = selectedId(preview.runtimeRecommendations, selectedRuntimeId);
  const canStartLocalSetup = Boolean(selectedRuntimeIdForPlan && modelDownloadAllowed(selectedModelForPlan, selectedRuntimeIdForPlan));

  return (
    <div data-ui-route="setup" data-ui-state={setupReady ? "ready" : "recommendation"} className="mx-auto grid w-full max-w-6xl gap-4 pb-16">
      <SetupStepper ready={setupReady} hasProgress={progress.jobs.length > 0} />

      <div data-testid="control-surface-header" className="border-b border-line pb-4">
        <h1 className="text-3xl font-semibold tracking-normal">{setupReady ? (hasActiveWorkspace ? "Local setup" : "Open a repository") : "Finish local setup"}</h1>
        <p className="mt-2 max-w-2xl text-sm leading-6 text-muted">
          {setupReady
            ? hasActiveWorkspace ? "Your local setup is verified for the active repository." : "Your local setup is verified. Open a repository to start working."
            : "Install the recommended local runner and coding model, or keep what is already installed."}
        </p>
        <p className="mt-2 inline-flex rounded-full border border-line bg-panel px-3 py-1 text-xs font-semibold text-muted shadow-[var(--dl-edge-light)]">
          Local by default
        </p>
      </div>

      {setupReady ? <SetupReadinessResult readiness={readiness} onOpenRepository={onOpenRepository} actionLabel={hasActiveWorkspace ? "Return to workspace" : "Open Repository"} /> : null}

      {preview.highEndLocal?.status === "candidate" ? <HighEndLocalSetupPanel setup={preview.highEndLocal} /> : null}

      {setupReady ? (
        <>
          <LocalConfigurationPanel preview={preview} models={models.data?.models ?? []} runtimes={runtimes.data?.runtimes ?? []} />
          {models.data && runtimes.data ? (
            <SetupCatalogPanel
              preview={preview}
              models={models.data.models}
              runtimes={runtimes.data.runtimes}
              downloading={downloadModel.isPending}
              installingRunner={installRuntime.isPending}
              onDownloadModel={(model) => downloadModel.mutateAsync(model).then(() => undefined)}
              onInstallRuntime={(runtime) => installRuntime.mutateAsync(runtime).then(() => undefined)}
            />
          ) : (
            <SetupPanel title="Local catalog unavailable" body="DesktopLab could not read local models and runners right now." />
          )}
        </>
      ) : (
        <>
          <InstallPlanPreview
            preview={preview}
            disabled={setup.accept.isPending || !canStartLocalSetup}
            selectedRuntimeId={selectedRuntimeId}
            selectedModelId={selectedModelId}
            runtimeSetupChoice={runtimeSetupChoice}
            modelSetupChoice={modelSetupChoice}
            onAccept={async () => {
              const runtimeId = selectedId(preview.runtimeRecommendations, selectedRuntimeId);
              const rawModelId = selectedId(preview.modelRecommendations, selectedModelId);
              const selectedRuntime = selectedRecommendation(preview.runtimeRecommendations, runtimeId);
              const selectedModel = selectedRecommendation(preview.modelRecommendations, rawModelId);
              const modelId = modelDownloadAllowed(selectedModel, runtimeId) ? rawModelId : undefined;
              const runtimeChoice = setupChoiceFor(selectedRuntime, runtimeSetupChoice);
              const modelChoice = setupChoiceFor(selectedModel, modelSetupChoice);
              const acceptance = await setup.accept.mutateAsync({ runtimeId: runtimeId ?? "", modelId });
              await queryClient.invalidateQueries({ queryKey: ["app-state"] });
              setBackendJobs(jobsFromAcceptance(acceptance.jobs, acceptance.pipeline));
              if (!runtimeId) {
                setStartedJobIds(acceptance.startedJobIds);
                return;
              }
              try {
                const runtimeInstall = await api.startRuntimeInstall({
                  runtimeId,
                  ...(runtimeChoice ? { setupChoice: runtimeChoice } : {}),
                });
                const modelDownload = modelId
                  ? await api.startModelDownload({
                      modelId,
                      runtimeId,
                      ...(modelChoice ? { setupChoice: modelChoice } : {}),
                    })
                  : null;
                setRuntimeInstallBlock(null);
                setStartedJobIds(uniqueJobIds([runtimeInstall.jobId, modelDownload?.jobId, ...acceptance.startedJobIds]));
                setBackendJobs((jobs) => mergeSetupJobs(jobs, [
                  jobFromRuntimeInstall(runtimeInstall),
                  ...(modelDownload ? [jobFromModelDownload(modelDownload)] : []),
                ]));
                await queryClient.invalidateQueries({ queryKey: ["app-state"] });
              } catch {
                setRuntimeInstallBlock("offline");
                setStartedJobIds(acceptance.startedJobIds);
              }
            }}
          />
          <div className="grid gap-4">
            <RecommendationView
              preview={preview}
              selectedRuntimeId={selectedRuntimeId}
              selectedModelId={selectedModelId}
              onSelectRuntime={setSelectedRuntimeId}
              onSelectModel={setSelectedModelId}
              runtimeSetupChoice={runtimeSetupChoice}
              modelSetupChoice={modelSetupChoice}
              onRuntimeSetupChoiceChange={setRuntimeSetupChoice}
              onModelSetupChoiceChange={setModelSetupChoice}
            />
          </div>

        </>
      )}

      {runtimeInstallBlock === "offline" ? <RuntimeOfflineNotice /> : null}

      <CatalogRefreshPanel
        status={catalogRefresh.data}
        queuedJobId={catalogRefreshJobId}
        pending={startCatalogRefresh.isPending}
        onRefresh={() => startCatalogRefresh.mutate()}
      />

      {!setupReady && progress.jobs.length > 0 ? <SetupJobProgress progress={progress} /> : null}

      <HardwareSummary hardware={preview.hardware} warnings={preview.warnings} />

      {!setupReady ? <SetupReadinessResult readiness={readiness} onOpenRepository={onOpenRepository} /> : null}
    </div>
  );
}

function modelDownloadAllowed(model: ReturnType<typeof selectedRecommendation>, runtimeId?: string) {
  if (!isSelectableForLocalSetup(model)) return false;
  if (!model?.runtimeId || !runtimeId) return true;
  return model.runtimeId === runtimeId;
}

function LocalConfigurationPanel({ preview, models, runtimes }: { preview: SetupPlanPreview; models: ModelInventoryItem[]; runtimes: RuntimeInventoryItem[] }) {
  const activeRuntime = runtimes.find((runtime) => runtime.status === "running" || runtime.status === "ready");
  const activeModel = models.find((model) => model.installState === "installed" && model.compatibility === "ready");
  const runtime = activeRuntime?.displayName ?? preview.runtimeRecommendations[0]?.displayName ?? "No local runner verified";
  const recommendedModel = preview.modelRecommendations[0];
  const model = activeModel
    ? displayLocalModelName(activeModel)
    : recommendedModel
      ? displayLocalModelName(recommendedModel)
      : "No local model verified";
  return (
    <section aria-labelledby="local-configuration-title" className="rounded-desktop border border-line p-4 dl-panel">
      <h2 id="local-configuration-title" className="text-lg font-semibold">
        Local configuration
      </h2>
      <p className="mt-1 text-sm leading-6 text-muted">Local tools are configured. Use this page to review them or add another model.</p>
      <div className="mt-4 grid gap-3 sm:grid-cols-2">
        <ConfigurationRow label="Active local runner" value={runtime} />
        <ConfigurationRow label="Active coding model" value={model} />
      </div>
    </section>
  );
}

function ConfigurationRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-desktop border border-line px-4 py-3 dl-elevated">
      <p className="text-xs font-semibold uppercase text-muted">{label}</p>
      <p className="mt-1 text-sm font-semibold text-ink">{value}</p>
    </div>
  );
}
