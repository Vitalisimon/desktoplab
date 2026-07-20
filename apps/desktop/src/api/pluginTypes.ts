import type { TrustLevel } from "./coreTypes";

export type PluginTrust = "verified" | "unverified";

export type PluginDistributionKind = "local_loaded" | "registry_installable" | "marketplace_future";

export type PluginDistributionBoundary = {
  kind: PluginDistributionKind;
  label: string;
  installAvailable: boolean;
  reason: string;
};

export type PluginSummary = {
  pluginId: string;
  displayName: string;
  status: "available" | "disabled" | "blocked" | string;
  trust: PluginTrust;
  capabilities: string[];
  descriptorState?: "present" | "missing" | string;
  coldManifestState?: "present" | "missing" | string;
  runtimeRegistration?: "registered" | "not_registered" | string;
  installSource?: "bundled_descriptor" | "registry" | "workspace" | string;
  integrityStatus?: "verified" | "missing_signature" | "unpinned" | string;
  executionEligibility?: "enabled" | "disabled" | string;
  provenance?: {
    descriptorState: string;
    coldManifestState: string;
    runtimeRegistration: string;
    installSource: string;
    integrityStatus: string;
    executionEligibility: string;
    blockedReasons: string[];
  };
  executionBoundary?: {
    kind: string;
    reason: string;
  };
  blockedReasons: string[];
  trustActions: Array<{
    id: string;
    label: string;
    description: string;
  }>;
};

export type PluginsListResponse = {
  plugins: PluginSummary[];
};

export type PluginTrustRequest = {
  decision: "approve" | "deny";
};

export type PluginTrustResponse = {
  status: "approved" | "denied" | "blocked";
};

export type ExternalBackendRoute = {
  routeId: string;
  status: "ready" | "blocked" | "approval_required";
  reason: string;
  approvalRequired?: boolean;
  sessionOwnership?: string;
  blockedReasons?: string[];
};

export type ExternalBackendSummary = {
  backendId: string;
  displayName: string;
  kind: "codex" | "claude" | "acp" | "custom" | "external" | string;
  status?: "ready" | "degraded" | "blocked";
  state?: "ready" | "degraded" | "blocked";
  trust?: TrustLevel;
  capabilities: string[];
  routes?: ExternalBackendRoute[];
  pluginBacked?: boolean;
  pluginBoundary?: string;
};

export type ExternalBackendsResponse = {
  backends: ExternalBackendSummary[];
};

export type ExternalBackendRouteResolveRequest = {
  resolution: "approve" | "deny";
};

export type ExternalBackendRouteResolveResponse = {
  status: "approved" | "denied" | "blocked";
};
