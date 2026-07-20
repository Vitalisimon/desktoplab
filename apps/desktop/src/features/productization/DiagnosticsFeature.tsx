import { useMutation, useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { useApiClient } from "../../api/ApiProvider";
import type { DiagnosticRepairAction, DiagnosticRepairRunResponse, DiagnosticServiceSummary } from "../../api/types";
import { RepairActionRow, StatusRow } from "../../design/OperationalPrimitives";
import { CatalogRefreshPanel, DoctorLintPanel, LocalAuditPanel, SecurityAuditPanel } from "./DiagnosticsPanels";

export function DiagnosticsFeature() {
  const api = useApiClient();
  const query = useQuery({ queryKey: ["diagnostics"], queryFn: () => api.diagnostics(), retry: 3, retryDelay: 250 });
  const audit = useQuery({ queryKey: ["diagnostics", "local-audit"], queryFn: () => api.localAuditTransparency(), retry: 2, retryDelay: 250 });
  const securityAudit = useQuery({ queryKey: ["diagnostics", "security-audit"], queryFn: () => api.securityAudit(), retry: 2, retryDelay: 250 });
  const catalogRefresh = useQuery({ queryKey: ["diagnostics", "catalog-refresh"], queryFn: () => api.catalogRefreshStatus(), retry: 2, retryDelay: 250 });
  const [queuedJob, setQueuedJob] = useState<string | null>(null);
  const [repairBlockedReason, setRepairBlockedReason] = useState<string | null>(null);
  const [repairBlockedKind, setRepairBlockedKind] = useState<string | null>(null);
  const [catalogRefreshJobId, setCatalogRefreshJobId] = useState<string | null>(null);
  const repair = useMutation({
    mutationFn: (repairId: string) => api.runDiagnosticRepair(repairId),
    onSuccess: (response) => {
      if (response.status === "blocked") {
        setQueuedJob(null);
        setRepairBlockedReason(displayRepairBlockedReason(response.reason));
        setRepairBlockedKind(response.repairKind ? displayRepairKind(response.repairKind) : null);
        return;
      }
      setRepairBlockedReason(null);
      setRepairBlockedKind(null);
      setQueuedJob(response.jobId ?? "queued");
    },
  });
  const refreshCatalog = useMutation({
    mutationFn: () => api.startCatalogRefresh(),
    onSuccess: (response) => setCatalogRefreshJobId(response.jobId ?? "blocked"),
  });

  if (query.isLoading) return <Panel title="Checking diagnostics" body="DesktopLab is reading repair readiness." />;
  if (query.isError || !query.data) return <Panel title="Diagnostics unavailable" body="DesktopLab could not read repair readiness right now." />;

  return (
    <div data-ui-route="diagnostics" data-ui-state="ready" className="mx-auto grid w-full max-w-6xl gap-4 pb-16">
      <section data-testid="control-surface-header" className="border-b border-line pb-4">
        <div className="flex items-start justify-between gap-4">
          <div>
            <h1 className="text-2xl font-semibold">Diagnostics</h1>
            <p className="mt-2 max-w-2xl text-sm leading-6 text-muted">Review repairable issues without exposing secrets or repository content.</p>
          </div>
          <span className={`rounded-full px-2 py-1 text-xs font-semibold ${query.data.state === "ready" ? "bg-success/10 text-success" : "bg-warning/10 text-warning"}`}>
            {diagnosticsStateLabel(query.data.state)}
          </span>
        </div>
      </section>
      {queuedJob ? (
        <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
          <h2 className="text-sm font-semibold">Repair queued</h2>
          <p className="mt-1 text-sm text-muted">Track progress in Background.</p>
        </section>
      ) : null}
      {repairBlockedReason ? (
        <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
          <h2 className="text-sm font-semibold">Repair blocked</h2>
          <p className="mt-1 text-sm text-muted">{repairBlockedReason}</p>
          {repairBlockedKind ? <p className="mt-1 text-xs font-semibold uppercase text-muted">{repairBlockedKind}</p> : null}
        </section>
      ) : null}
      <section className="grid items-start gap-4 lg:grid-cols-[0.95fr_1.05fr]">
        <ServicePanel services={query.data.services} />
        <RepairPanel actions={query.data.repairActions} onRun={(id) => repair.mutate(id)} />
      </section>
      <DoctorLintPanel lint={query.data.doctorLint} />
      <SecurityAuditPanel audit={securityAudit.data ?? query.data.securityAudit} />
      <CatalogRefreshPanel status={catalogRefresh.data} queuedJobId={catalogRefreshJobId} pending={refreshCatalog.isPending} onRefresh={() => refreshCatalog.mutate()} />
      <LocalAuditPanel audit={audit.data ?? query.data.localAudit} unavailable={audit.isError && !query.data.localAudit} />
    </div>
  );
}

function ServicePanel({ services }: { services: DiagnosticServiceSummary[] }) {
  return (
    <section className="rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <h2 className="text-lg font-semibold">System checks</h2>
      <div className="mt-4 grid gap-4">
        {services.map((service) => (
          <StatusRow key={service.family} label={service.label} status={service.state} detail={service.message} />
        ))}
      </div>
    </section>
  );
}

function RepairPanel({ actions, onRun }: { actions: DiagnosticRepairAction[]; onRun: (repairId: string) => void }) {
  return (
    <section className="rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <h2 className="text-lg font-semibold">Repairs</h2>
      <div className="mt-4 grid gap-3">
        {actions.length > 0 ? (
          actions.map((action) => action.mode === "executable" ? (
            <RepairActionRow key={action.repairId} label={action.label} description={action.reason} disabled={false} onClick={() => onRun(action.repairId)} />
          ) : (
            <div key={action.repairId} className="flex min-h-12 items-center justify-between gap-4 rounded-desktop bg-elevated px-3 py-2">
              <div className="min-w-0">
                <p className="text-sm font-medium text-ink">{action.label}</p>
                <p className="mt-0.5 text-xs leading-5 text-muted">No automatic repair is available.</p>
              </div>
              <span className="shrink-0 text-xs font-medium text-muted">Guidance only</span>
            </div>
          ))
        ) : (
          <p className="text-sm text-muted">No repairs are needed.</p>
        )}
      </div>
    </section>
  );
}

function diagnosticsStateLabel(state: "ready" | "degraded" | "blocked") {
  if (state === "ready") return "Ready";
  if (state === "degraded") return "Limited";
  return "Blocked";
}

function displayRepairBlockedReason(reason?: string): string {
  if (reason === "external_manual_repair_required") return "Complete this repair outside DesktopLab.";
  if (reason === "diagnostic_repair_not_connected") return "This repair is unavailable in the current setup.";
  if (!reason) return "This repair is unavailable right now.";
  if (reason.includes(" ")) return reason;
  const words = reason.replaceAll("_", " ");
  return `${words[0]?.toUpperCase() ?? ""}${words.slice(1)}.`;
}

function displayRepairKind(kind: NonNullable<DiagnosticRepairRunResponse["repairKind"]>): string {
  const labels: Record<NonNullable<DiagnosticRepairRunResponse["repairKind"]>, string> = {
    guidance_only: "Guidance only",
    local_config: "Local configuration",
    stale_state_cleanup: "Local cleanup",
    external_manual: "Manual repair",
    unsupported: "Unsupported repair",
  };
  return labels[kind];
}

function Panel({ title, body }: { title: string; body: string }) {
  return (
    <section className="mx-auto max-w-3xl rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <h1 className="text-xl font-semibold">{title}</h1>
      <p className="mt-2 text-sm text-muted">{body}</p>
    </section>
  );
}
