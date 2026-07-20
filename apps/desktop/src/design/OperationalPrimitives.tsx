import type { ReactNode } from "react";
import { CheckCircle2, CircleAlert, CircleDashed, CircleDot, ShieldCheck, ShieldQuestion } from "./icons";
import type { TrustLevel } from "../api/types";

type UiStatus = "ready" | "degraded" | "blocked" | "running" | "stopped" | "installed" | "not_installed" | "completed" | "unknown";

export function StatusRow({ label, status, detail }: { label: string; status: UiStatus; detail: string }) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div className="min-w-0">
        <p className="text-sm font-semibold text-ink">{label}</p>
        <p className="mt-1 text-sm leading-5 text-muted">{detail}</p>
      </div>
      <span className={`shrink-0 rounded-full px-2 py-1 text-xs font-semibold ${statusClass(status)}`}>{statusLabel(status)}</span>
    </div>
  );
}

export function CapabilityList({ capabilities, format = (capability) => capability }: { capabilities: string[]; format?: (capability: string) => string }) {
  return (
    <ul className="flex flex-wrap gap-2" aria-label="Capabilities">
      {capabilities.map((capability) => (
        <li key={capability} className="rounded-full border border-line bg-elevated px-2 py-1 text-xs font-medium text-muted">
          {format(capability)}
        </li>
      ))}
    </ul>
  );
}

export function RepairActionRow({
  label,
  description,
  disabled,
  onClick,
}: {
  label: string;
  description: string;
  disabled: boolean;
  onClick?: () => void;
}) {
  return (
    <div className="flex items-center justify-between gap-4 rounded-desktop bg-elevated px-3 py-3">
      <p className="text-sm text-muted">{description}</p>
      <button type="button" className="rounded-desktop bg-ink px-3 py-2 text-sm font-medium text-canvas disabled:opacity-45" disabled={disabled} onClick={onClick}>
        {label}
      </button>
    </div>
  );
}

export function ProgressTimeline({ items }: { items: Array<{ id: string; label: string; status: UiStatus }> }) {
  return (
    <ol className="divide-y divide-line rounded-desktop border border-line bg-panel">
      {items.map((item) => (
        <li key={item.id} className="flex items-center gap-3 px-3 py-2">
          {iconFor(item.status)}
          <span className="text-sm font-medium text-ink">{item.label}</span>
          <span className="ml-auto text-xs text-muted">{statusLabel(item.status)}</span>
        </li>
      ))}
    </ol>
  );
}

export function EvidenceDisclosure({ title, body, children }: { title: string; body: string; children?: ReactNode }) {
  return (
    <details className="rounded-desktop border border-line bg-panel px-3 py-2">
      <summary className="cursor-pointer text-sm font-semibold text-ink">{title}</summary>
      <p className="mt-2 whitespace-pre-wrap text-sm leading-6 text-muted">{body}</p>
      {children}
    </details>
  );
}

export function TrustBadge({ trust, label }: { trust: TrustLevel; label?: string }) {
  const verified = trust === "verified" || trust === "local";
  return (
    <span className="inline-flex items-center gap-1 rounded-full border border-line bg-elevated px-2 py-1 text-xs font-semibold text-muted">
      {verified ? <ShieldCheck size={13} /> : <ShieldQuestion size={13} />}
      {label ?? (trust === "unverified" ? "Unverified" : trust === "local" ? "Local" : "Verified")}
    </span>
  );
}

export function RouteExplanation({ kind, summary, reasons }: { kind: "local" | "cloud" | "external"; summary: string; reasons: string[] }) {
  return (
    <div className="rounded-desktop border border-line bg-panel p-4">
      <p className="text-xs font-semibold uppercase text-muted">{kind === "local" ? "Local route" : kind === "cloud" ? "Cloud route" : "External route"}</p>
      <p className="mt-1 text-sm font-semibold text-ink">{summary}</p>
      <ul className="mt-2 space-y-1">
        {reasons.map((reason) => (
          <li key={reason} className="text-sm text-muted">
            {reason}
          </li>
        ))}
      </ul>
    </div>
  );
}

function statusClass(status: UiStatus) {
  if (status === "blocked") return "bg-danger/10 text-danger";
  if (status === "degraded" || status === "not_installed") return "bg-warning/10 text-warning";
  return "bg-success/10 text-success";
}

function statusLabel(status: UiStatus) {
  const labels: Record<UiStatus, string> = {
    ready: "Ready",
    degraded: "Limited",
    blocked: "Blocked",
    running: "Running",
    stopped: "Stopped",
    installed: "Installed",
    not_installed: "Not installed",
    completed: "Completed",
    unknown: "Unknown",
  };
  return labels[status];
}

function iconFor(status: UiStatus) {
  if (status === "completed" || status === "ready") return <CheckCircle2 size={15} className="text-success" />;
  if (status === "blocked") return <CircleAlert size={15} className="text-danger" />;
  if (status === "running") return <CircleDot size={15} className="text-accent" />;
  return <CircleDashed size={15} className="text-muted" />;
}
