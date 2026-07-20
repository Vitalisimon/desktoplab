import type { RepairAction } from "./coreTypes";
import type { JobRetryClass } from "./jobTypes";

export type RuntimeInstallMetadata = {
  supported: boolean;
  blockedReason?: string;
  diskRequiredGb?: number;
};

export type RuntimeLifecycleState = "supported" | "blocked" | "packaging_managed";

export type RuntimeLifecycleControl = {
  state: RuntimeLifecycleState;
  label: string;
  reason: string;
};

export type RuntimeLifecycleBoundary = {
  update: RuntimeLifecycleControl;
  uninstall: RuntimeLifecycleControl;
};

export type RuntimeProvenance = {
  runtimeId: string;
  version?: string | null;
  installSource: string;
  verificationMethod: string;
  integrity: {
    state: "unavailable";
    reason: string;
  };
};

export type RuntimeInventoryItem = {
  runtimeId: string;
  displayName: string;
  ownership: "desktoplab_managed" | "user_owned" | "externally_managed";
  status: "ready" | "running" | "stopped" | "installed" | "not_installed" | "blocked" | "degraded" | "unknown";
  detectionSource?: "host_probe" | "registry" | "unknown";
  version?: string;
  capabilities: string[];
  install: RuntimeInstallMetadata;
  provenance?: RuntimeProvenance;
  lifecycle?: RuntimeLifecycleBoundary;
  repairActions: RepairAction[];
  logExcerpt?: string;
};

export type RuntimesListResponse = {
  runtimes: RuntimeInventoryItem[];
};

export type RuntimeInstallRequest = {
  runtimeId: string;
  setupChoice?: "use_existing" | "install" | "replace";
};

export type RuntimeInstallResponse = {
  jobId: string;
  runtimeId: string;
  state: "planning" | "downloading" | "verifying" | "installing" | "completed" | "blocked" | "external_guided" | "failed";
  verificationState: "pending" | "verified" | "blocked" | "requires_external_app" | "failed";
  retryClass: JobRetryClass;
  setupChoice?: "use_existing" | "install" | "replace";
  executionEvidence?: string;
  blockedReason?: string | null;
  remediation?: string;
};
