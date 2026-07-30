import type { ModelInventoryItem, RuntimeInventoryItem, SetupPlanPreview } from "../../api/types";
import { displayLocalModelName } from "../../domain/displayNames";

export function LocalConfigurationPanel({
  preview,
  models,
  runtimes,
}: {
  preview: SetupPlanPreview;
  models: ModelInventoryItem[];
  runtimes: RuntimeInventoryItem[];
}) {
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
