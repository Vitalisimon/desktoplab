import { useQuery } from "@tanstack/react-query";
import { useApiClient } from "../../api/ApiProvider";
import { displayLocalModelName } from "../../domain/displayNames";

export function ProductizationSettingsSummary() {
  const api = useApiClient();
  const runtimes = useQuery({ queryKey: ["settings", "runtimes"], queryFn: () => api.listRuntimes() });
  const models = useQuery({ queryKey: ["settings", "models"], queryFn: () => api.listModels() });

  if (runtimes.isLoading || models.isLoading) {
    return <SummaryPanel title="Current setup" rows={[["Configuration", "Reading local settings."]]} />;
  }

  if (!runtimes.data || !models.data) {
    return <SummaryPanel title="Current setup" rows={[["Configuration", "Summary unavailable."]]} />;
  }

  const notInstalled = runtimes.data.runtimes.filter((runtime) => runtime.status === "not_installed").length;
  const activeRuntime = runtimes.data.runtimes.find((runtime) => runtime.status === "running" || runtime.status === "ready");
  const activeModel = models.data.models.find((model) => model.installState === "installed" && model.compatibility === "ready");
  const downloadableModels = models.data.models.filter((model) => model.installState === "downloadable").length;

  return (
    <section className="border-b border-line py-4">
      <h2 className="text-lg font-semibold">Current setup</h2>
      <div className="mt-3 grid border-y border-line lg:grid-cols-2">
        <PlainStatusCard
          title="Active local setup"
          rows={[
            activeRuntime ? `${activeRuntime.displayName} is ${activeRuntime.status}` : "No local runner is active yet",
            activeModel ? `${displayLocalModelName(activeModel)} is ready now` : "No local model is ready yet",
          ]}
        />
        <PlainStatusCard
          title="Available changes"
          rows={[
            downloadableModels > 0
              ? `${downloadableModels} compatible ${pluralize(downloadableModels, "model", "models")} can be downloaded`
              : "No additional compatible model is available now",
            notInstalled > 0
              ? `${notInstalled} optional local ${pluralize(notInstalled, "runner", "runners")} can be configured`
              : "The local runner is configured",
          ]}
        />
      </div>
    </section>
  );
}

function pluralize(count: number, singular: string, plural: string) {
  return count === 1 ? singular : plural;
}

function SummaryPanel({ title, rows }: { title: string; rows: Array<[string, string]> }) {
  return (
    <section className="border-b border-line py-4">
      <h2 className="text-lg font-semibold">{title}</h2>
      <SummaryRows rows={rows} />
    </section>
  );
}

function PlainStatusCard({ title, rows }: { title: string; rows: string[] }) {
  return (
    <div className="px-1 py-3 lg:first:border-r lg:first:pr-5 lg:last:pl-5">
      <h3 className="text-sm font-semibold text-ink">{title}</h3>
      <div className="mt-2 grid gap-1">
        {rows.map((row) => (
          <p key={row} className="text-sm text-muted">{row}</p>
        ))}
      </div>
    </div>
  );
}

function SummaryRows({ rows }: { rows: Array<[string, string]> }) {
  return (
    <div className="mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
      {rows.map(([label, value]) => (
        <div key={label} className="rounded-desktop border border-line bg-panel px-4 py-3">
          <p className="text-xs font-semibold uppercase text-muted">{label}</p>
          <p className="mt-2 text-sm font-semibold text-ink">{value}</p>
        </div>
      ))}
    </div>
  );
}
