import { CheckCircle2, Download, Play, RefreshCw, ShieldCheck } from "../../design/icons";
import type { SetupChoice, SetupPlanPreview, SetupRecommendation } from "../../api/types";
import { displayJobKind } from "../../domain/displayNames";

type InstallPlanPreviewProps = {
  preview: SetupPlanPreview;
  onAccept: () => void;
  disabled?: boolean;
  selectedRuntimeId?: string;
  selectedModelId?: string;
  runtimeSetupChoice?: SetupChoice;
  modelSetupChoice?: SetupChoice;
};

export function InstallPlanPreview({
  preview,
  onAccept,
  disabled = false,
  selectedRuntimeId,
  selectedModelId,
  runtimeSetupChoice,
  modelSetupChoice,
}: InstallPlanPreviewProps) {
  const runtime = selected(preview.runtimeRecommendations, selectedRuntimeId);
  const model = selected(preview.modelRecommendations, selectedModelId);
  const runtimeJobs = runtime ? [runtime].map((runtime) => ({
    id: `runtime.install:${runtime.manifestId}`,
    label: runtime.displayName,
    action: setupPlanAction("runtime", runtime, runtimeSetupChoice),
  })) : [];
  const modelJobs = model ? [model].map((model) => ({
    id: `model.download:${model.manifestId}`,
    label: model.displayName,
    action: setupPlanAction("model", model, modelSetupChoice),
  })) : [];
  const jobs = [...runtimeJobs, ...modelJobs];

  return (
    <section aria-labelledby="install-plan-title" className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 id="install-plan-title" className="text-lg font-semibold">
            Setup plan
          </h2>
          <p className="mt-1 text-sm leading-6 text-muted">
            DesktopLab keeps this setup on your computer and starts only after confirmation.
          </p>
        </div>
        <ShieldCheck className="text-success" size={20} />
      </div>

      <div className="mt-4 space-y-2">
        {jobs.map((job) => (
          <div key={job.id} className="flex min-h-11 items-center gap-3 rounded-desktop bg-elevated px-3 py-2 text-sm">
            {job.action.kind === "use_existing" ? <CheckCircle2 size={15} className="text-success" /> : null}
            {job.action.kind === "install" ? <Download size={15} className="text-muted" /> : null}
            {job.action.kind === "replace" ? <RefreshCw size={15} className="text-muted" /> : null}
            <span className="min-w-0">
              <span className="block truncate font-semibold text-ink">{job.label}</span>
              <span className="block text-xs text-muted">{job.action.label}</span>
            </span>
          </div>
        ))}
      </div>

      <button
        type="button"
        onClick={onAccept}
        disabled={disabled || jobs.length === 0 || preview.registryState === "blocked"}
        className="mt-4 inline-flex h-10 items-center gap-2 rounded-desktop bg-ink px-4 text-sm font-semibold text-canvas outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed disabled:bg-muted"
      >
        <Play size={15} />
        Start setup
      </button>
    </section>
  );
}

function selected<T extends { manifestId: string; role?: "recommended" | "alternative" }>(items: T[], manifestId?: string): T | undefined {
  return items.find((item) => item.manifestId === manifestId) ?? items.find((item) => item.role === "recommended") ?? items[0];
}

function setupPlanAction(
  target: "runtime" | "model",
  recommendation: SetupRecommendation,
  selectedChoice?: SetupChoice,
): { kind: SetupChoice; label: string } {
  const kind = recommendation.setupChoiceRequired
    ? selectedChoice ?? recommendation.defaultSetupChoice ?? "use_existing"
    : "install";
  if (kind === "use_existing") {
    return { kind, label: target === "runtime" ? "Use installed local runner" : "Use installed coding model" };
  }
  if (kind === "replace") {
    return { kind, label: target === "runtime" ? "Replace local runner" : "Replace coding model" };
  }
  return { kind, label: displayJobKind(target === "runtime" ? "runtime.install" : "model.download") };
}
