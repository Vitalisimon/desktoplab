export type ApprovalState = "pending" | "approved" | "denied" | "expired";

export type ApprovalRisk = "low" | "medium" | "high";

export type ApprovalSummary = {
  approvalId: string;
  sessionId: string;
  action: "filesystem.write" | "terminal.command" | "git.commit" | "git.push" | "provider.egress" | "fallback.route" | string;
  operationId?: string;
  state: ApprovalState;
  risk: ApprovalRisk;
  title: string;
  message: string;
  requestedAt: string;
  policyReason?: string;
};

export type ApprovalsListResponse = {
  approvals: ApprovalSummary[];
};

export type ApprovalResolveRequest = {
  resolution: "approve" | "deny";
};

export type ApprovalCreateRequest = {
  sessionId: string;
  action: ApprovalSummary["action"];
  operationId: string;
  payload?: unknown;
};

export type ApprovalCreateResponse = {
  approvalId: string;
  sessionId: string;
  action: ApprovalSummary["action"];
  operationId: string;
  state: "pending";
};

export type ApprovalResolveResponse = {
  approvalId: string;
  state: "approved" | "denied";
};

export type ApprovalMode =
  | "require_approval"
  | "approve_for_me"
  | "approve_workspace_writes_for_session"
  | "full_access";

export type ApprovalModeDescriptor = {
  mode: ApprovalMode;
  label: string;
  description: string;
};

export type ApprovalModesResponse = {
  modes: ApprovalModeDescriptor[];
  defaultMode: ApprovalMode;
  sessionMode: ApprovalMode;
};

export type ApprovalModeUpdateRequest = {
  mode: ApprovalMode;
};
