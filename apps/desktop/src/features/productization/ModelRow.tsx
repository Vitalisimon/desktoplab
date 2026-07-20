import type { ModelDownloadResponse, ModelInventoryItem } from "../../api/types";
import { displayModelDownloadState, displayModelInstallState } from "../../domain/displayNames";
import { setupFailureCopy } from "../setup/setupFailureCopy";

export function ModelRow({
  model,
  downloadState,
  onDownload,
  downloading,
}: {
  model: ModelInventoryItem;
  downloadState?: ModelDownloadResponse;
  onDownload: () => void;
  downloading: boolean;
}) {
  const blocked = model.installState === "blocked" || model.compatibility === "blocked";
  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h3 className="text-base font-semibold">{model.displayName}</h3>
          <p className="mt-1 text-sm text-muted">{model.recommended ? "Recommended for this machine" : model.channel}</p>
        </div>
        <span className="rounded-full border border-line bg-elevated px-2 py-1 text-xs font-semibold text-muted">{displayModelInstallState(model.installState)}</span>
      </div>
      {model.agentQualification ? (
        <p className="mt-2 text-xs font-semibold text-muted">{agentQualificationLabel(model.agentQualification)}</p>
      ) : null}
      <p className="mt-3 text-sm text-muted">{setupFailureCopy(model.blockedReason) ?? model.verification ?? `${model.sizeGb} GB download`}</p>
      <dl className="mt-3 grid grid-cols-2 gap-2 text-xs text-muted sm:grid-cols-4">
        <ModelMetric label="Params" value={formatParams(model.parametersBillion)} />
        <ModelMetric label="Quant" value={model.quantization ?? "Unknown"} />
        <ModelMetric label="Memory" value={formatMemory(model.requiredMemoryGb)} />
        <ModelMetric label="Disk" value={`${model.sizeGb} GB`} />
      </dl>
      {downloadState ? <ModelDownloadStatePanel model={model} download={downloadState} /> : null}
      <button
        type="button"
        className="mt-3 rounded-desktop bg-ink px-3 py-2 text-sm font-medium text-canvas disabled:opacity-45"
        disabled={blocked || downloading || model.installState !== "downloadable"}
        onClick={onDownload}
      >
        Download {model.displayName}
      </button>
    </section>
  );
}

function ModelMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-desktop border border-line bg-elevated px-2 py-1">
      <dt className="text-[10px] font-semibold uppercase tracking-normal text-muted">{label}</dt>
      <dd className="mt-0.5 font-semibold text-ink">{value}</dd>
    </div>
  );
}

function ModelDownloadStatePanel({ model, download }: { model: ModelInventoryItem; download: ModelDownloadResponse }) {
  const body =
    download.state === "failed"
      ? `${model.displayName} can be retried from Background.`
      : download.state === "blocked"
        ? (setupFailureCopy(download.blockedReason) ?? `${model.displayName} needs a compatible local runner before download.`)
        : `${model.displayName}: ${displayModelDownloadState(download.state)}.`;
  return (
    <div className="mt-3 rounded-desktop bg-elevated px-3 py-2">
      <p className="text-sm font-semibold text-ink">Model download {displayModelDownloadState(download.state)}</p>
      <p className="mt-1 text-sm text-muted">{body}</p>
      {download.executionEvidence ? <p className="mt-1 font-mono text-xs text-muted">{download.executionEvidence}</p> : null}
    </div>
  );
}

function formatParams(parametersBillion: number | undefined) {
  return typeof parametersBillion === "number" ? `${parametersBillion}B` : "Unknown";
}

function formatMemory(requiredMemoryGb: number | undefined) {
  return typeof requiredMemoryGb === "number" ? `${requiredMemoryGb} GB` : "Unknown";
}

function agentQualificationLabel(qualification: ModelInventoryItem["agentQualification"]) {
  return qualification === "runtime_validation_required"
    ? "Agent protocol is validated after download"
    : "Agent validation required";
}
