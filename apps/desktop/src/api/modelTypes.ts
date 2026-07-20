import type { JobRetryClass } from "./jobTypes";

export type ModelInventoryItem = {
  modelId: string;
  displayName: string;
  runtimeId: string;
  channel: "stable" | "beta" | "experimental";
  familyId?: string;
  familyName?: string;
  pullRef?: string;
  parameterClass?: "small" | "medium" | "large" | "workstation";
  parametersBillion?: number;
  quantization?: string;
  requiredMemoryGb?: number;
  installState: "installed" | "downloadable" | "downloading" | "blocked";
  compatibility: "ready" | "compatible" | "blocked" | "unknown";
  sizeGb: number;
  recommended: boolean;
  agentQualification?: "runtime_validation_required";
  verification?: string;
  provenance?: {
    catalogSource: string;
    runtimeId: string;
    pullRef: string;
    verificationState: string;
    localVerification: string;
  };
  blockedReason?: string;
};

export type ModelsListResponse = {
  models: ModelInventoryItem[];
};

export type ModelDownloadRequest = {
  modelId: string;
  runtimeId: string;
  setupChoice?: "use_existing" | "install" | "replace";
};

export type ModelDownloadResponse = {
  jobId: string;
  modelId: string;
  familyId?: string;
  variantId?: string;
  runtimeId: string;
  state: "downloadable" | "running" | "downloading" | "verifying" | "ready" | "failed" | "blocked";
  progressPercent?: number;
  retryClass: JobRetryClass;
  blockedReason?: string;
  failureReason?: string | null;
  setupChoice?: "use_existing" | "install" | "replace";
  executionEvidence?: string;
};
