import type { AccountMode, ApprovalMode, ExternalBackendSummary, ModelDownloadResponse, ModelInventoryItem, PluginSummary, ProviderAccount, RecentSessionSummary, RuntimeInstallResponse } from "../api/types";

export function displayApprovalMode(mode: ApprovalMode): string {
  const labels: Record<ApprovalMode, string> = {
    require_approval: "Ask for approval",
    approve_for_me: "Approve routine actions",
    approve_workspace_writes_for_session: "Allow workspace writes",
    full_access: "Full local access",
  };
  return labels[mode];
}

export function displayExecutionBackendName(backendId: string): string {
  switch (backendId) {
    case "backend.ollama":
      return "Ollama local";
    case "backend.codex":
      return "Codex cloud";
    case "backend.claude":
      return "Claude cloud";
    default:
      return humanizeIdentifier(backendId.replace(/^backend\./, ""));
  }
}

export function displayExecutionBackendKind(kind: "local" | "cloud" | "external" | "custom" | null | undefined): string {
  switch (kind) {
    case "local":
      return "Local";
    case "cloud":
      return "Cloud";
    case "external":
      return "External";
    case "custom":
      return "Custom";
    default:
      return "Route not ready";
  }
}

export function displayApprovalPolicy(needsFallbackApproval: boolean): string {
  return needsFallbackApproval ? "Approvals: Required for fallback" : "Approvals: Ask before writes";
}

export function displayWorkspaceName(workspaceId: string): string {
  return humanizeIdentifier(workspaceId.replace(/^workspace\./, ""));
}

export function displayJobKind(kind: string): string {
  switch (kind) {
    case "model.download":
      return "Download coding model";
    case "model.verify":
      return "Check coding model";
    case "runtime.install":
      return "Install local runner";
    case "runtime.verify":
      return "Check local runner";
    case "registry.refresh":
      return "Refresh recommendations";
    default:
      return humanizeIdentifier(kind);
  }
}

export function displaySetupJobId(jobId: string): string {
  const [kind] = jobId.split(":");
  return displayJobKind(kind);
}

export type ProviderAccountDisplayState = {
  label: "Ready" | "Not connected" | "Needs approval" | "Disabled by policy" | "Unavailable";
  className: string;
};

export function displayProviderAccountState(provider: ProviderAccount): ProviderAccountDisplayState {
  if (provider.status === "connected") return { label: "Ready", className: "bg-success/10 text-success" };
  if (provider.status === "missing_credential") return { label: "Not connected", className: "bg-elevated text-muted" };
  if (provider.status === "degraded") return { label: "Needs approval", className: "bg-warning/10 text-warning" };
  if (provider.status === "blocked") {
    const message = provider.diagnostic.message.toLowerCase();
    if (message.includes("unavailable")) return { label: "Unavailable", className: "bg-elevated text-muted" };
    return { label: "Disabled by policy", className: "bg-danger/10 text-danger" };
  }
  return { label: "Unavailable", className: "bg-elevated text-muted" };
}

export function displayProviderAccountMode(mode: AccountMode): string {
  const labels: Record<AccountMode, string> = {
    api_key_billing: "API key",
    subscription_account: "Subscription login",
    oauth_device: "Browser sign-in",
    local_app_session: "Local app session",
    custom_endpoint: "Custom endpoint",
  };
  return labels[mode];
}

export function displayProviderAccountModeOption(mode: AccountMode): string {
  return `Use ${displayProviderAccountMode(mode).toLowerCase()}`;
}

export function displayProviderAuthMode(mode: string): string {
  return isAccountMode(mode) ? displayProviderAccountMode(mode) : humanizeIdentifier(mode);
}

export function displayProviderCredentialReference(kind: string): string {
  if (kind === "vault_ref") return "Stored in local vault";
  if (kind === "none") return "No credential stored";
  return humanizeIdentifier(kind);
}

export function displayProviderFallbackApproval(value: string): string {
  if (value === "explicit_user_approval_required") return "Approval required before fallback";
  if (value === "not_required") return "No additional approval required";
  return humanizeIdentifier(value);
}

export function displayCapability(capability: string): string {
  const labels: Record<string, string> = {
    "llm.chat": "Chat",
    "tool.filesystem.write": "Write files",
    "agent.events.stream": "Stream agent events",
    "agent.external": "External agent",
  };
  return labels[capability] ?? (capability.includes(".") ? humanizeIdentifier(capability) : capability);
}

export const providerByoAccountModesCopy =
  "Use your subscription login, browser sign-in, API key, local app session or custom endpoint when a provider supports it.";

export function displayPluginCompatibility(plugin: PluginSummary): string {
  if (plugin.executionBoundary?.reason) return plugin.executionBoundary.reason;
  if (plugin.blockedReasons.length > 0) return plugin.blockedReasons[0];
  if (plugin.status === "blocked") return "Needs review before use";
  if (plugin.status === "available") return "Compatible";
  return "Compatibility not verified yet";
}

export function displayExternalBackendBoundary(backend: ExternalBackendSummary): string {
  if (backend.pluginBoundary) return backend.pluginBoundary;
  if (backend.state === "ready" || backend.status === "ready") return "Ready for approved routes";
  if (backend.state === "degraded" || backend.status === "degraded") return "Needs attention before routing";
  if (backend.state === "blocked" || backend.status === "blocked") return "Disabled until approved";
  return "Route availability not verified yet";
}

export function displayModelInstallState(state: ModelInventoryItem["installState"]): string {
  const labels: Record<ModelInventoryItem["installState"], string> = {
    installed: "Installed",
    downloadable: "Available",
    downloading: "Downloading",
    blocked: "Needs local runner",
  };
  return labels[state];
}

export function displayLocalModelName(model: {
  displayName: string;
  familyName?: string;
  parametersBillion?: number;
  quantization?: string;
}): string {
  const precise = [
    model.familyName,
    typeof model.parametersBillion === "number" ? `${model.parametersBillion}B` : null,
    model.quantization,
  ].filter(Boolean);
  return precise.length >= 2 ? precise.join(" ") : model.displayName;
}

export function displayModelDownloadState(state: ModelDownloadResponse["state"]): string {
  const labels: Record<ModelDownloadResponse["state"], string> = {
    downloadable: "Ready to download",
    running: "Downloading",
    downloading: "Downloading",
    verifying: "Checking model",
    ready: "Ready",
    failed: "Failed",
    blocked: "Waiting for local runner",
  };
  return labels[state];
}

export function displayRuntimeInstallState(state: RuntimeInstallResponse["state"]): string {
  const labels: Record<RuntimeInstallResponse["state"], string> = {
    planning: "Planning install",
    downloading: "Downloading runner",
    verifying: "Checking runner",
    installing: "Installing runner",
    completed: "Installed",
    blocked: "Needs attention",
    external_guided: "Open external app",
    failed: "Failed",
  };
  return labels[state];
}

export function displayRuntimeVerificationState(state: RuntimeInstallResponse["verificationState"]): string {
  const labels: Record<RuntimeInstallResponse["verificationState"], string> = {
    pending: "Check pending",
    verified: "Verified",
    blocked: "Needs attention",
    requires_external_app: "Open external app",
    failed: "Failed",
  };
  return labels[state];
}

export function displayRecentSessionState(state: RecentSessionSummary["state"]): string {
  const labels: Record<RecentSessionSummary["state"], string> = {
    running: "Working",
    blocked: "Waiting for approval",
    failed: "Failed",
    completed: "Complete",
    cancelled: "Cancelled",
  };
  return labels[state];
}

function humanizeIdentifier(value: string): string {
  return value
    .split(/[._:-]+/)
    .filter(Boolean)
    .map((part) => part[0]?.toUpperCase() + part.slice(1))
    .join(" ");
}

function isAccountMode(value: string): value is AccountMode {
  return ["api_key_billing", "subscription_account", "oauth_device", "local_app_session", "custom_endpoint"].includes(value);
}
