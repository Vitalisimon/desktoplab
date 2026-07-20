import type { DiagnosticsBundlePreview } from "../../api/types";
import { friendlyRuntimeId } from "../setup/recommendationLabels";

type DiagnosticsBundlePanelProps = {
  bundle: DiagnosticsBundlePreview;
};

export function DiagnosticsBundlePanel({ bundle }: DiagnosticsBundlePanelProps) {
  const hardware = bundle.hardware ?? [];
  const jobs = uniqueJobs(bundle.jobs ?? []);

  return (
    <section className="border-t border-line py-4" aria-labelledby="diagnostics-bundle-title">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 id="diagnostics-bundle-title" className="text-lg font-semibold">
            Setup diagnostics
          </h2>
          <p className="mt-1 text-sm leading-6 text-muted">Local setup evidence for troubleshooting. Sensitive values are redacted.</p>
          <p className="mt-1 text-sm leading-6 text-muted">Export bundle is local. Review it before sharing.</p>
        </div>
        <span className="rounded bg-elevated px-2 py-1 text-xs font-semibold text-muted">{bundle.redacted ? "Redacted" : "Local"}</span>
      </div>

      <div className="mt-4 grid gap-3 md:grid-cols-3">
        <DiagnosticTile label="Runtime" value={bundle.setup?.runtimeId ? friendlyRuntimeId(bundle.setup.runtimeId) : "Not selected"} />
        <DiagnosticTile label="Model" value={friendlyModelId(bundle.setup?.modelId)} />
        <DiagnosticTile label="Pipeline" value={friendlyPipeline(bundle.setup?.pipelineState)} />
      </div>

      {hardware.length > 0 ? (
        <div className="mt-4 grid gap-2 md:grid-cols-2">
          {hardware.map((fact) => (
            <DiagnosticTile key={`${fact.label}:${fact.value}`} label={fact.label} value={fact.value} detail={fact.confidence} />
          ))}
        </div>
      ) : null}

      {jobs.length > 0 ? (
        <ul className="mt-4 space-y-2">
          {jobs.map((job) => (
            <li key={`${job.kind}:${job.state}`} className="rounded-desktop bg-elevated px-3 py-2 text-sm text-ink">
              {friendlyJob(job.kind)}: {friendlyJobState(job.state)}
            </li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}

function friendlyModelId(modelId?: string | null): string {
  if (!modelId) return "Not selected";
  const normalized = modelId.toLowerCase();
  if (normalized.includes("qwen") && normalized.includes("coder")) return "Qwen Coder";
  return "Local model";
}

function uniqueJobs(jobs: NonNullable<DiagnosticsBundlePreview["jobs"]>) {
  return jobs.filter(
    (job, index) => jobs.findIndex((candidate) => candidate.kind === job.kind && candidate.state === job.state) === index,
  );
}

function DiagnosticTile({ label, value, detail }: { label: string; value: string; detail?: string }) {
  return (
    <div className="rounded-desktop bg-elevated px-3 py-2">
      <p className="text-xs font-medium text-muted">{label}</p>
      <p className="mt-1 truncate text-sm font-semibold text-ink">{value}</p>
      {detail ? <p className="mt-1 text-xs text-muted">{detail}</p> : null}
    </div>
  );
}

function friendlyPipeline(state?: string | null) {
  if (state === "runtime_installing") return "Installing runner";
  if (state === "model_downloading") return "Downloading model";
  if (state === "ready") return "Ready";
  if (state === "blocked") return "Needs attention";
  return "Not started";
}

function friendlyJob(kind: string) {
  if (kind.startsWith("runtime.install")) return "Runtime install";
  if (kind.startsWith("model.download")) return "Model download";
  return "Background work";
}

function friendlyJobState(state: string) {
  if (state === "blocked") return "Needs attention";
  if (state === "running") return "Running";
  if (state === "succeeded" || state === "completed") return "Completed";
  if (state === "failed") return "Failed";
  if (state === "cancelled") return "Cancelled";
  return state;
}
