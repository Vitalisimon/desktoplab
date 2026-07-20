import type { ReadinessResponse } from "./coreTypes";
import type { SetupPipelineSnapshot } from "./setupTypes";
import type { ApprovalMode, ApprovalModesResponse } from "./approvalTypes";

export type WorkspaceApiState = "clean" | "dirty";

export type WorkspaceCheckpointStatus = "ready";

export type WorkspaceSnapshot = {
  workspaceId: string;
  displayName: string;
  rootPath: string;
  rootExists?: boolean;
  stale?: boolean;
  readOnly?: boolean;
  blockedReason?: "workspace_root_missing" | string | null;
  gitDirPath: string;
  apiState: WorkspaceApiState;
  statusEntries: string[];
  diffText: string;
  checkpointStatus: WorkspaceCheckpointStatus;
  canCheckpointRiskyExecution: boolean;
};

export type AppStateResponse = {
  readiness: ReadinessResponse;
  setup?: {
    state: "not_started" | "in_progress" | "ready" | "blocked";
    runtimeId?: string | null;
    modelId?: string | null;
    blockedReason?: string | null;
    lastVerifiedAt?: string | null;
  };
  setupPipeline?: SetupPipelineSnapshot;
  currentWorkspace: WorkspaceSnapshot | null;
  workspaces?: WorkspaceSnapshot[];
  approvalModes?: ApprovalModesResponse;
  routeInput: {
    readiness: ReadinessResponse["state"];
    setupState?: "not_started" | "in_progress" | "ready" | "blocked";
    hasWorkspace: boolean;
    activeApprovalCount: number;
    activeSessionCount: number;
    approvalMode?: ApprovalMode;
  };
};

export type WorkspaceOpenRequest = {
  path: string;
  initializeGit?: boolean;
};

export type WorkspaceRelinkRequest = {
  path: string;
};

export type WorkspaceArchiveResponse = {
  archived: boolean;
  workspaceId: string;
};

export type WorkspaceFileTreeEntryKind = "directory" | "file" | "hidden_file" | "symlink";

export type WorkspaceFileProtection = "readable" | "protected";

export type WorkspaceFileTreeEntry = {
  path: string;
  kind: WorkspaceFileTreeEntryKind;
  protection: WorkspaceFileProtection;
};

export type WorkspaceFileTreeResponse = {
  workspaceId: string;
  entries: WorkspaceFileTreeEntry[];
  degraded: boolean;
  degradedReasons: string[];
  limits: {
    maxEntries: number;
    maxDepth: number;
  };
};

export type ContextAttachment = {
  path: string;
  label: string;
  state: "available" | "unavailable";
  disabledReason?: string | null;
};

export type ContextAttachmentsResponse = {
  workspaceId: string;
  attachments: ContextAttachment[];
};

export type WorkspaceFilePreviewBase = {
  workspaceId: string;
  path: string;
  originalBytes: number;
  originalLines: number;
  returnedLines: number;
  truncated: boolean;
  openAction?: {
    label: string;
  };
};

export type WorkspaceTextFilePreview = WorkspaceFilePreviewBase & {
  state: "text";
  text: string;
  deniedReason: null;
};

export type WorkspaceBinaryFilePreview = WorkspaceFilePreviewBase & {
  state: "binary";
  text: null;
  deniedReason: null;
};

export type WorkspaceDeniedFilePreview = WorkspaceFilePreviewBase & {
  state: "denied";
  text: null;
  deniedReason: "local_only_path" | string;
};

export type WorkspaceFilePreviewResponse =
  | WorkspaceTextFilePreview
  | WorkspaceBinaryFilePreview
  | WorkspaceDeniedFilePreview;

export type WorkspaceRuntimeHealth = {
  state: "ready" | "degraded" | "blocked";
  label: string;
};

export type RecentSessionSummary = {
  sessionId: string;
  backendId: string;
  state: "running" | "blocked" | "failed" | "completed" | "cancelled";
  updatedAt: string;
};

export type WorkspaceHomeSnapshot = {
  workspace: WorkspaceSnapshot;
  setupHealth: ReadinessResponse;
  runtimeHealth: WorkspaceRuntimeHealth;
  recentSessions: RecentSessionSummary[];
};

export type WorkspaceFact = {
  label: string;
  value: string;
  confidence: "confirmed" | "probable" | "unknown";
};

export type WorkspaceIntelligenceSnapshot = {
  workspaceId: string;
  projectType: string;
  stale: boolean;
  refreshSupported: boolean;
  facts: WorkspaceFact[];
  testCommands: Array<{ command: string; confidence: "confirmed" | "probable" | "unknown" }>;
  protectedSummary: string[];
  diagnosticsLink?: string;
};

export type WorkspaceScanRefreshResponse = {
  status: "accepted" | "blocked";
  reason?: string;
  source?: string;
};

export type WorkspaceMemoryItem = {
  memoryId: string;
  workspaceId?: string;
  kind: string;
  title: string;
  summary: string;
  decisions: string[];
  source: string;
  createdAt: string;
  redactionStatus: "local_only" | "provider_allowed" | string;
};

export type WorkspaceMemoryResponse = {
  memories: WorkspaceMemoryItem[];
};

export type DeleteMemoryResponse = {
  workspaceId: string;
  deletedMemoryId: string;
  status: "deleted" | "blocked";
  reason?: string;
};

export type SessionContextPreview = {
  summary: string;
  sizeBudget: string;
  provenance: string[];
  cloudEgressWarning?: string;
  excludedProtectedContent: string[];
};
