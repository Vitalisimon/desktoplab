import { ChevronDown } from "../../design/icons";
import { useState } from "react";
import type { ModelInventoryItem, RuntimeInventoryItem, SetupPlanPreview } from "../../api/types";
import { visibleExpectedLimitations } from "./expectedLimitationCopy";
import { setupFailureCopy } from "./setupFailureCopy";
import { displayLocalModelName } from "../../domain/displayNames";

type SetupCatalogPanelProps = {
  preview: SetupPlanPreview;
  models: ModelInventoryItem[];
  runtimes: RuntimeInventoryItem[];
  downloading: boolean;
  installingRunner: boolean;
  onDownloadModel: (model: ModelInventoryItem) => Promise<void>;
  onInstallRuntime: (runtime: RuntimeInventoryItem) => Promise<void>;
};

export function SetupCatalogPanel({ preview, models, runtimes, downloading, installingRunner, onDownloadModel, onInstallRuntime }: SetupCatalogPanelProps) {
  const activeModel = models.find((model) => model.installState === "installed" && model.compatibility === "ready");
  const downloadableModels = models.filter((model) => model.installState === "downloadable" || model.installState === "blocked");
  const activeRuntime = runtimes.find((runtime) => runtime.status === "running" || runtime.status === "ready");
  const configurableRuntimes = runtimes.filter((runtime) => runtime.status !== "running" && runtime.status !== "ready");
  const defaultModelId = downloadableModels[0]?.modelId ?? "";
  const defaultRuntimeId = configurableRuntimes[0]?.runtimeId ?? "";
  const [selectedModelId, setSelectedModelId] = useState(defaultModelId);
  const [selectedRuntimeId, setSelectedRuntimeId] = useState(defaultRuntimeId);
  const [startedModelName, setStartedModelName] = useState<string | null>(null);
  const [startedRuntimeName, setStartedRuntimeName] = useState<string | null>(null);
  const selectedModel = downloadableModels.find((model) => model.modelId === (selectedModelId || defaultModelId));
  const selectedRuntime = configurableRuntimes.find((runtime) => runtime.runtimeId === (selectedRuntimeId || defaultRuntimeId));
  const visibleLimitations = visibleExpectedLimitations(preview.expectedLimitations);
  const blocked = !selectedModel || selectedModel.installState === "blocked" || selectedModel.compatibility === "blocked";
  const runtimeBlocked = !selectedRuntime || !selectedRuntime.install.supported;

  const downloadSelected = async () => {
    if (!selectedModel || blocked) return;
    await onDownloadModel(selectedModel);
    setStartedModelName(displayLocalModelName(selectedModel));
  };
  const installSelectedRuntime = async () => {
    if (!selectedRuntime || runtimeBlocked) return;
    await onInstallRuntime(selectedRuntime);
    setStartedRuntimeName(selectedRuntime.displayName);
  };

  return (
    <section className="border-t border-line py-5" aria-labelledby="catalog-title">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 id="catalog-title" className="text-lg font-semibold">
            Local models
          </h2>
          <p className="mt-1 text-sm leading-6 text-muted">See the active model and download another compatible one when you want to switch.</p>
        </div>
        <RegistryState state={preview.registryState} />
      </div>

      <div className="mt-5 grid gap-4 border-y border-line py-4">
        <h3 className="text-sm font-semibold text-ink">Local runners</h3>
        <StatusLine label="Active runner" value={activeRuntime ? `${activeRuntime.displayName} is active` : "No local runner is active yet"} />
        {activeRuntime ? <p className="text-sm text-muted">{runnerOwnershipLabel(activeRuntime)}</p> : null}
        <label className="grid gap-2 text-sm font-medium text-ink">
          Runner to configure
          <span className="relative">
            <select
              aria-label="Choose a runner to configure"
              className="h-10 w-full appearance-none rounded-desktop border border-line bg-panel px-3 pr-9 text-sm text-ink transition-colors duration-150 focus:border-accent"
              disabled={configurableRuntimes.length === 0 || installingRunner}
              value={selectedRuntime?.runtimeId ?? ""}
              onChange={(event) => setSelectedRuntimeId(event.target.value)}
            >
              {configurableRuntimes.length === 0 ? (
                <option value="">No additional local runner</option>
              ) : (
                configurableRuntimes.map((runtime) => (
                  <option key={runtime.runtimeId} value={runtime.runtimeId}>
                    {runtime.displayName} · {runnerSetupLabel(runtime)}
                  </option>
                ))
              )}
            </select>
            <ChevronDown size={15} className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-muted" />
          </span>
        </label>
        {selectedRuntime ? <p className="text-sm text-muted">{runnerSetupLabel(selectedRuntime)}</p> : null}
        {startedRuntimeName ? <p className="text-sm font-medium text-success">Runner setup started for {startedRuntimeName}.</p> : null}
        <button
          type="button"
          className="h-10 rounded-desktop bg-ink px-4 text-sm font-semibold text-canvas transition-colors duration-150 hover:bg-accent disabled:opacity-45"
          disabled={runtimeBlocked || installingRunner}
          onClick={() => void installSelectedRuntime()}
        >
          Configure selected runner
        </button>
      </div>

      <div className="mt-5 grid gap-4 border-b border-line pb-4">
        <StatusLine label="Active model" value={activeModel ? `${displayLocalModelName(activeModel)} is ready now` : "No local model is ready yet"} />

        <label className="grid gap-2 text-sm font-medium text-ink">
          Model to download
          <span className="relative">
            <select
              aria-label="Choose a model to download"
              className="h-10 w-full appearance-none rounded-desktop border border-line bg-panel px-3 pr-9 text-sm text-ink transition-colors duration-150 focus:border-accent"
              disabled={downloadableModels.length === 0 || downloading}
              value={selectedModel?.modelId ?? ""}
              onChange={(event) => setSelectedModelId(event.target.value)}
            >
              {downloadableModels.length === 0 ? (
                <option value="">No additional compatible model</option>
              ) : (
                downloadableModels.map((model) => (
                  <option key={model.modelId} value={model.modelId}>
                    {displayLocalModelName(model)} · {modelSummary(model)}
                  </option>
                ))
              )}
            </select>
            <ChevronDown size={15} className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-muted" />
          </span>
        </label>

        {selectedModel ? <p className="text-sm text-muted">{setupFailureCopy(selectedModel.blockedReason) ?? modelDetails(selectedModel)}</p> : null}
        {selectedModel?.provenance ? (
          <p className="text-xs text-muted">
            {catalogSourceLabel(selectedModel.provenance.catalogSource)} · {selectedModel.provenance.pullRef} · {verificationStateLabel(selectedModel.provenance.verificationState)}
          </p>
        ) : null}
        {startedModelName ? <p className="text-sm font-medium text-success">Download started for {startedModelName}.</p> : null}

        <button
          type="button"
          className="h-10 rounded-desktop bg-ink px-4 text-sm font-semibold text-canvas transition-colors duration-150 hover:bg-accent disabled:opacity-45"
          disabled={blocked || downloading}
          onClick={() => void downloadSelected()}
        >
          Download selected model
        </button>
      </div>

      {visibleLimitations.length > 0 ? (
        <div className="mt-4 rounded-desktop border border-warning/30 bg-warning/10 px-4 py-3 text-sm text-ink">
          {visibleLimitations.join(", ")}
        </div>
      ) : null}
    </section>
  );
}

function StatusLine({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex min-w-0 items-center justify-between gap-4">
      <p className="text-xs font-semibold uppercase text-muted">{label}</p>
      <p className="truncate text-sm font-semibold text-ink">{value}</p>
    </div>
  );
}

function modelSummary(model: ModelInventoryItem): string {
  return [formatParams(model.parametersBillion), model.quantization, `${model.sizeGb} GB`].filter(Boolean).join(" · ");
}

function modelDetails(model: ModelInventoryItem): string {
  return [
    model.familyName,
    formatParams(model.parametersBillion),
    model.quantization,
    model.requiredMemoryGb ? `${model.requiredMemoryGb} GB memory class` : null,
    `${model.sizeGb} GB on disk`,
  ]
    .filter(Boolean)
    .join(" · ");
}

function formatParams(parametersBillion?: number) {
  return typeof parametersBillion === "number" ? `${parametersBillion}B` : null;
}

function runnerSetupLabel(runtime: RuntimeInventoryItem): string {
  if (!runtime.install.supported) return setupFailureCopy(runtime.install.blockedReason) ?? "Not available on this computer";
  if (runtime.runtimeId === "runtime.mlx-lm") return "Local Python setup";
  if (runtime.ownership === "externally_managed") return "Guided external setup";
  if (runtime.ownership === "user_owned") return "Already installed on this computer";
  return "DesktopLab-managed setup";
}

function runnerOwnershipLabel(runtime: RuntimeInventoryItem): string {
  if (runtime.ownership === "desktoplab_managed") return "Managed by DesktopLab";
  if (runtime.ownership === "user_owned") return "Already installed on this computer";
  return "Managed outside DesktopLab";
}

function catalogSourceLabel(source: string): string {
  if (source === "bundled_seed_catalog") return "Bundled catalog";
  if (source === "last_known_good_catalog") return "Saved catalog";
  if (source === "signed_remote_catalog") return "Signed catalog";
  return "Catalog";
}

function verificationStateLabel(state: string): string {
  if (state === "verified_local_inventory") return "Verified locally";
  if (state === "downloadable_not_installed") return "Downloadable";
  if (state === "runtime_verification_required") return "Runner check needed";
  return "Not verified locally";
}

function RegistryState({ state }: { state: SetupPlanPreview["registryState"] }) {
  const label = state === "ready" ? "Catalog ready" : state === "degraded" ? "Catalog limited" : "Catalog blocked";
  const className =
    state === "ready"
      ? "bg-success/10 text-success"
      : state === "degraded"
        ? "bg-warning/10 text-warning"
        : "bg-danger/10 text-danger";

  return <span className={`rounded px-2 py-1 text-xs font-semibold ${className}`}>{label}</span>;
}
