import type { HighEndLocalSetupPreview } from "./frontierTypes";

export type HardwareFact<T = string | number> = {
  label: string;
  value: T | null;
  confidence: "confirmed" | "probable" | "unknown" | "conflicting" | "unsupported";
};

export type SetupRecommendation = {
  manifestId: string;
  displayName: string;
  channel: "stable" | "beta" | "experimental";
  role?: "recommended" | "alternative";
  installMode?: "automatic" | "external_guided" | "python_environment";
  familyId?: string;
  familyName?: string;
  parameterClass?: "small" | "medium" | "large" | "workstation" | "cloud";
  parametersBillion?: number;
  quantization?: string;
  contextWindowTokens?: number;
  agentContextWindowTokens?: number;
  agentRequestTimeoutSeconds?: number;
  requiredMemoryGb?: number;
  expectedDiskMb?: number;
  runtimeId?: string;
  compatibilityReason?: string;
  licenseState?: "known" | "unknown" | "restricted";
  trustLabel?: string;
  agentQualification?: "runtime_validation_required";
  hostInstallState?: "installed" | "missing";
  installedVersion?: string;
  installedPath?: string;
  endpoint?: string;
  defaultSetupChoice?: "use_existing" | "install" | "replace";
  setupChoiceRequired?: boolean;
};

export type SetupChoice = "use_existing" | "install" | "replace";

export type SetupPlanPreview = {
  registryState: "ready" | "degraded" | "blocked";
  hardware: {
    cpu: HardwareFact;
    ramGb: HardwareFact<number>;
    gpu: HardwareFact;
    acceleratorKind?: HardwareFact<"integrated" | "discrete" | "unified_memory">;
    vramGb: HardwareFact<number>;
    unifiedMemoryGb: HardwareFact<number>;
    operatingSystem: HardwareFact;
    architecture: HardwareFact;
    storageAvailableGb: HardwareFact<number>;
  };
  highEndLocal?: HighEndLocalSetupPreview;
  runtimeRecommendations: SetupRecommendation[];
  modelRecommendations: SetupRecommendation[];
  warnings: string[];
  expectedLimitations: string[];
  hiddenReasons: string[];
};

export type SetupAcceptanceResponse = {
  startedJobIds: string[];
  pipeline?: SetupPipelineSnapshot;
  jobs?: Array<{
    jobId: string;
    kind: string;
    state: SetupJobStatus;
    blockedReason?: string;
    dependsOn?: string;
  }>;
};

export type SetupAcceptanceRequest = {
  runtimeId: string;
  modelId?: string;
};

export type SetupPipelineState =
  | "not_started"
  | "selected"
  | "runtime_detecting"
  | "runtime_installing"
  | "runtime_verifying"
  | "model_downloading"
  | "model_verifying"
  | "ready"
  | "blocked";

export type SetupPipelineSnapshot = {
  state: SetupPipelineState;
  runtimeId?: string | null;
  modelId?: string | null;
  blockedReason?: string | null;
};

export type CatalogRefreshState = "ready" | "degraded" | "blocked";

export type CatalogRefreshManualControl = {
  available: boolean;
  jobId?: string;
  blockedReason?: string;
};

export type CatalogRefreshStatusResponse = {
  state: CatalogRefreshState;
  lastKnownGoodAvailable: boolean;
  degradedReasons: string[];
  manualRefresh: CatalogRefreshManualControl;
};

export type CatalogRefreshRequestResponse = {
  jobId?: string;
  blockedReason?: string;
};

export type SetupJobStatus = "queued" | "running" | "blocked" | "failed" | "completed" | "cancelled";

export type SetupJobProgressItem = {
  id: string;
  label: string;
  phaseLabel?: string;
  status: SetupJobStatus;
  progressPercent: number;
  nextAction?: string;
  retryAvailable?: boolean;
  cancelAvailable?: boolean;
};

export type SetupJobProgressSnapshot = {
  sequence: number;
  jobs: SetupJobProgressItem[];
};
