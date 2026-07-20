import { Activity, CheckCircle2 } from "../../design/icons";
import type { HealthResponse, ReadinessResponse, StabilitySnapshot, VersionResponse } from "../../api/types";

type LocalDiagnosticsPanelProps = {
  health: HealthResponse;
  readiness: ReadinessResponse;
  version: VersionResponse;
  stability?: StabilitySnapshot;
};

const readinessCopy = {
  starting: "Local services starting",
  ready: "Local services ready",
  degraded: "Local services limited",
  blocked: "Local services blocked",
} as const;

const statusPillCopy = {
  starting: "Starting",
  ready: "Ready",
  degraded: "Limited",
  blocked: "Blocked",
} as const;

export function LocalDiagnosticsPanel({ health, readiness, version, stability }: LocalDiagnosticsPanelProps) {
  const issues = readiness.degradedReasons ?? [];

  return (
    <section className="py-4" aria-labelledby="local-diagnostics-title">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 id="local-diagnostics-title" className="text-lg font-semibold">
            Local status
          </h2>
          <p className="mt-1 text-sm leading-6 text-muted">DesktopLab engine and local API health.</p>
        </div>
        <StatusPill state={readiness.state} />
      </div>

      <div className="mt-5 grid gap-3 sm:grid-cols-2">
        <Metric icon={<CheckCircle2 size={16} />} label="App" value={`DesktopLab ${version.productVersion}`} />
        <Metric icon={<Activity size={16} />} label="API" value={`API ${version.apiVersion}`} />
      </div>

      {stability ? (
        <div className="mt-3 grid gap-3 sm:grid-cols-2">
          <Metric icon={<Activity size={16} />} label="Uptime" value={formatUptime(stability.uptimeMs)} />
          <Metric icon={<Activity size={16} />} label="Backpressure" value={backpressureCopy[stability.queueBackpressure.state]} />
        </div>
      ) : null}

      <div className="mt-4 rounded-desktop border border-line bg-elevated px-4 py-3 text-sm">
        <div className="font-medium">{readinessCopy[readiness.state]}</div>
        <div className="mt-1 text-muted">{health.status === "healthy" ? "Engine responding normally." : "Engine is draining work."}</div>
        {stability ? (
          <div className="mt-1 text-muted">
            Stability snapshot: {startupPhaseCopy[stability.startupPhase]}; route decision current.
          </div>
        ) : null}
      </div>

      {issues.length > 0 ? (
        <ul className="mt-3 space-y-2 text-sm text-muted">
          {issues.map((issue) => (
            <li key={issue}>{issue}</li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}

const startupPhaseCopy = {
  setup_pending: "setup pending",
  workspace_pending: "workspace pending",
  ready: "ready",
} as const;

const backpressureCopy = {
  idle: "Idle",
  busy: "Busy",
  attention_required: "Needs attention",
} as const;

function formatUptime(ms: number) {
  if (ms < 1000) return "<1s";
  return `${Math.floor(ms / 1000)}s`;
}

function Metric({ icon, label, value }: { icon: React.ReactNode; label: string; value: string }) {
  return (
    <div className="rounded-desktop bg-elevated px-4 py-3">
      <div className="flex items-center gap-2 text-xs font-medium uppercase text-muted">
        {icon}
        {label}
      </div>
      <div className="mt-2 text-sm font-semibold">{value}</div>
    </div>
  );
}

function StatusPill({ state }: { state: ReadinessResponse["state"] }) {
  const className =
    state === "ready"
      ? "bg-success/10 text-success"
      : state === "blocked"
        ? "bg-danger/10 text-danger"
        : "bg-warning/10 text-warning";

  return <span className={`rounded px-2 py-1 text-xs font-semibold ${className}`}>{statusPillCopy[state]}</span>;
}
