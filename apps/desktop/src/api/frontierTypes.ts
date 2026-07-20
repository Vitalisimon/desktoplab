export type HighEndRuntimeChoice = {
  runtimeId: string;
  displayName: string;
  defaultEndpoint: string;
  recommended: boolean;
};

export type HighEndLocalSetupPreview = {
  status: "candidate" | "standard";
  source: "hardware_probe" | "dev_test_control";
  profile: string;
  profileLabel: string;
  hardwareSummary: string;
  recommendedRuntimeId: string;
  runtimeChoices: HighEndRuntimeChoice[];
  storageTarget: { path: string; displayPath?: string; freeGb?: number | null };
  expectedCapability: string;
  claimState: "certification_required";
  blockingReasons: string[];
  details: {
    gpuModels: string[];
    driver?: string | null;
    cuda?: string | null;
    nvlink: "detected" | "not_detected" | "unknown";
    nvswitch: "detected" | "not_detected" | "unknown";
    mig: "detected" | "not_detected" | "unknown";
  };
};

export type HighEndRuntimeDiscoveryRequest = { runtimeId: string; endpoint: string };
export type HighEndRuntimeDiscoveryResponse = {
  source: "runtime_probe";
  runtimeId: string;
  endpoint: string;
  models: string[];
};
export type HighEndRuntimeAttachRequest = HighEndRuntimeDiscoveryRequest & { modelId: string };
export type HighEndRuntimeHealthResponse = {
  source: "runtime_probe";
  state: "unconfigured" | "reachable" | "model_loading" | "model_ready" | "degraded" | "busy" | "failed";
  routeEligibility: "eligible" | "blocked";
  runtimeId?: string;
  endpoint?: string;
  modelId?: string;
  evidence?: { reason?: string | null };
};
