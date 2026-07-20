import type { AccountMode } from "./providerTypes";
import type { ApprovalSummary } from "./approvalTypes";
import type { AgentFailureClassification } from "./agentFailureTypes";

export type AgentSessionState =
  | "created"
  | "planning"
  | "running"
  | "paused"
  | "blocked"
  | "failed"
  | "cancelled"
  | "completed";

export type AgentSessionTimelineEvent = {
  sequence: number;
  kind: string;
  message: string;
  createdAt: string;
  evidence?: {
    title: string;
    body: string;
    redacted: boolean;
  };
  test?: {
    state: "passed" | "failed" | "timeout";
    command: string;
    output: string;
  };
};

export type AgentSessionSnapshot = {
  sessionId: string;
  workspaceId: string;
  executionBackendId: string;
  owner: "desktoplab";
  state: AgentSessionState;
  plan: string | null;
  checkpoints: string[];
  summary: string | null;
  failureClassification?: AgentFailureClassification | null;
  timeline: AgentSessionTimelineEvent[];
  job?: AgentSessionJob | null;
  transcript?: AgentTranscriptTurn[];
  details?: AgentSessionDetails;
  pendingApprovals?: ApprovalSummary[];
  controls?: {
    pause: boolean;
    resume: boolean;
    cancel: boolean;
  };
};

export type AgentSessionJob = {
  jobId: string;
  state: "running" | "interrupted" | "cancelled" | "completed" | string;
  startedAt: string;
  lastHeartbeatAt?: string | null;
  lastObservation?: string | null;
  cancellable: boolean;
  recoveryGuidance?: string | null;
};

export type AgentTranscriptTurn = {
  sequence: number;
  role: "user" | "assistant" | "tool" | "status";
  content: string;
};

export type AgentToolDetail = {
  state: string;
  source: string;
  tool: string;
  approvalMode: string;
};

export type AgentSessionDetails = {
  plan: string | null;
  toolCalls: AgentToolDetail[];
  approvals: AgentToolDetail[];
  observations: Array<{ kind: string; message: string }>;
  diffs: Array<{ message: string }>;
  validations: Array<{ message: string }>;
};

export type AgentSessionSelection = {
  sessionId: string;
  workspaceId: string;
};

export type SessionsListResponse = {
  sessions: AgentSessionSnapshot[];
};

export type SessionCreateRequest = {
  workspaceId: string;
  executionBackendId: string;
  initialPrompt: string;
  newChat?: boolean;
  contextPaths?: string[];
  externalAttachments?: ExternalAttachmentInput[];
  approvalId?: string;
  toolPath?: string;
};

export type SessionContinueRequest = {
  workspaceId: string;
  executionBackendId: string;
  prompt: string;
  contextPaths?: string[];
  externalAttachments?: ExternalAttachmentInput[];
  approvalId?: string;
  toolPath?: string;
};

export type SessionArchiveResponse = {
  archived: boolean;
  sessionId: string;
};

export type ExternalAttachmentInput = {
  name: string;
  size: number;
  mediaType: string;
  contentText?: string;
  contentSha256?: string;
  truncated?: boolean;
};

export type AgentRouteDecision = {
  status: "selected" | "blocked";
  backendId?: string | null;
  backendDisplayName?: string | null;
  backendKind?: "local" | "cloud" | "external" | "custom" | null;
  modelDisplayName?: string | null;
  runtimeDisplayName?: string | null;
  modelAgentCapability?: {
    class: "chat_capable" | "limited_agent_capable" | "full_coding_agent_capable" | string;
    routeLabel: string;
    claim: string;
  } | null;
  accountMode?: AccountMode;
  egressPolicy?: "local_only" | "requires_approval" | "allowed";
  repositoryContextEgress?: "local_only" | "approval_required" | "allowed";
  summary: string;
  reasons: string[];
  blockedReasons?: string[];
  nextAction?: string;
  nextActionLabel?: string;
  requiredCapabilities: string[];
  needsFallbackApproval: boolean;
};

export type WorkspaceContextSummary = {
  workspaceId: string;
  languages: string[];
  frameworks: string[];
  testCommands: Array<{ command: string; confidence: "confirmed" | "probable" | "unknown" }>;
  protectedSummary: string[];
  stale: boolean;
  refreshSupported: boolean;
};

export type AgentWorkspaceSnapshot = {
  route: AgentRouteDecision | null;
  context: WorkspaceContextSummary | null;
  session: AgentSessionSnapshot | null;
};

export type SessionControlRequest = {
  action: "pause" | "resume" | "cancel";
};

export type SessionControlResponse = {
  accepted?: boolean;
  status?: "blocked" | "accepted";
  reason?: string;
};
