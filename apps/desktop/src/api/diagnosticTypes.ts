import type { LocalAuditTransparencySnapshot } from "./auditTypes";

export type DiagnosticFamily = "runtime" | "model" | "provider" | "plugin" | "workspace_scan" | "registry" | "storage" | "job";

export type DiagnosticServiceSummary = {
  family: DiagnosticFamily;
  label: string;
  state: "ready" | "degraded" | "blocked";
  message: string;
};

export type DiagnosticRepairAction = {
  repairId: string;
  family: DiagnosticFamily;
  label: string;
  reason: string;
  mode: "executable" | "guidance_only";
  repairKind?: "guidance_only" | "local_config" | "stale_state_cleanup" | "external_manual";
};

export type DoctorLintCheck = {
  checkId: string;
  label: string;
  severity: "ready" | "degraded" | "blocked";
  source: DiagnosticFamily;
  message: string;
  fixHint: string;
  repairId?: string;
};

export type DoctorLintSnapshot = {
  source: "service_backed";
  mode: "lint";
  repairable: false;
  summary: {
    state: "ready" | "degraded" | "blocked";
    blocked: number;
    degraded: number;
    ready: number;
  };
  checks: DoctorLintCheck[];
};

export type SecurityAuditFinding = {
  checkId: string;
  label: string;
  severity: "ready" | "degraded" | "blocked";
  source: string;
  message: string;
  fixHint: string;
  repairId?: string;
  suppressed: boolean;
};

export type SecurityAuditSnapshot = {
  source: "service_backed";
  kind: "security_audit";
  redacted: boolean;
  exportSafe: boolean;
  summary: {
    state: "ready" | "degraded" | "blocked";
    blocked: number;
    degraded: number;
    ready: number;
  };
  findings: SecurityAuditFinding[];
  remediationPolicy: string;
};

export type DiagnosticsBundlePreview = {
  summary: string;
  setup?: {
    runtimeId?: string | null;
    modelId?: string | null;
    pipelineState?: string | null;
  };
  hardware?: Array<{
    label: string;
    value: string;
    confidence: "confirmed" | "probable" | "unknown";
  }>;
  jobs?: Array<{
    kind: string;
    state: string;
  }>;
  redactedErrors?: Array<{
    kind: string;
    message: string;
    redacted: boolean;
  }>;
  sizeBytes: number;
  maxBytes: number;
  redacted: boolean;
};

export type DiagnosticsExportBundle = {
  manifest: {
    kind: "desktoplab.diagnostics.export";
    schemaVersion: number;
    redactionProfile?: string;
  };
  summary?: {
    state?: "ready" | "degraded" | "blocked";
    redacted: boolean;
    sizeBytes: number;
    maxBytes: number;
  };
  reviewBeforeSharing?: boolean;
};

export type RuntimeInspectSnapshot = {
  source: "service_backed";
  inspectState: "ready" | "degraded" | "blocked";
  active: {
    selectedRouteId: string;
    backendId: string;
    runtimeId?: string | null;
    modelId?: string | null;
    accountMode: string;
    egress: string;
    toolCapability: string;
    degradedReason?: string | null;
  };
  evidence: {
    coldManifest: {
      source: string;
      runtimeId?: string | null;
      modelId?: string | null;
    };
    liveRuntime: {
      state: "verified" | "not_verified" | "unknown";
      evidence?: string | null;
    };
  };
};

export type UpdateStatusSnapshot = {
  channel: "dev" | "beta" | "stable";
  currentVersion: string;
  state: "not_checked" | "checking" | "up_to_date" | "available" | "failed" | "disabled";
  message: string;
  canInstall: boolean;
};

export type StabilitySnapshot = {
  kind: "desktoplab.stability.snapshot";
  schemaVersion: number;
  redacted: boolean;
  payloadFree: boolean;
  startupPhase: "setup_pending" | "workspace_pending" | "ready";
  uptimeMs: number;
  localApiHealth: {
    state: "responding" | "degraded" | "blocked";
    scope: string;
    payloadFree: boolean;
  };
  routeDecisionRecency: {
    state: "current" | "stale" | "unknown";
    selectedRouteId: string;
    lastChangedAgoMs: number;
  };
  queueBackpressure: {
    state: "idle" | "busy" | "attention_required";
    queued: number;
    running: number;
    awaitingApproval: number;
    blocked: number;
    failed: number;
    active: number;
    payloadFree: boolean;
  };
  budgets: {
    memory: { budgetMb: number; sampleState: "not_sampled" | "sampled" };
    disk: { minimumFreeMb: number; sampleState: "not_sampled" | "sampled" };
  };
  degradedReasons: string[];
  jobStates: Array<{ kind: string; state: string }>;
};

export type DiagnosticsSnapshot = {
  state: "ready" | "degraded" | "blocked";
  services: DiagnosticServiceSummary[];
  repairActions: DiagnosticRepairAction[];
  bundlePreview: DiagnosticsBundlePreview;
  updateStatus: UpdateStatusSnapshot;
  doctorLint?: DoctorLintSnapshot;
  securityAudit?: SecurityAuditSnapshot;
  stability?: StabilitySnapshot;
  localAudit?: LocalAuditTransparencySnapshot;
};

export type DiagnosticRepairRunResponse = {
  status: "accepted" | "blocked";
  repairId?: string;
  repairKind?: "guidance_only" | "local_config" | "stale_state_cleanup" | "external_manual" | "unsupported";
  jobId?: string;
  reason?: string;
  source?: string;
  requiresApproval?: boolean;
  sideEffects?: string[];
};
