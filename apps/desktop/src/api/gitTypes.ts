export type GitApproval = string;

export type SavePointSummary = {
  savePointId: string;
  title: string;
  sessionId: string;
  createdAt: string;
  rollbackSupported: boolean;
  rollbackPreview: string;
  protectedUntrackedFiles?: string[];
};

export type CommitPreview = {
  supported: boolean;
  sessionId: string;
  message: string;
  preview: string;
  changeFingerprint: string;
  requiresApproval: boolean;
};

export type PushPreview = {
  supported: boolean;
  remote: string;
  branch: string;
  preview: string;
  requiresApproval: boolean;
  normalizedReason?: string;
};

export type WorktreeSummary = {
  worktreeId: string;
  label: string;
  path: string;
  sessionId: string | null;
  cleanupSupported: boolean;
  userOwned: boolean;
  isolationReason: string;
  mergeRequiresApproval?: boolean;
};

export type GitOperationsSnapshot = {
  workspaceId: string;
  workspaceState: "clean" | "dirty" | "conflicted";
  warnings: string[];
  changedFiles?: string[];
  statusEntries?: string[];
  diffPreview?: string;
  savePoints: SavePointSummary[];
  commit: CommitPreview;
  push: PushPreview;
  worktrees: WorktreeSummary[];
};

export type RollbackSavePointRequest = {
  approvalId: GitApproval;
};

export type RollbackSavePointResponse = {
  status: "restored" | "denied" | "blocked";
  reason?: string;
};

export type CommitSessionRequest = {
  workspaceId: string;
  sessionId: string;
  message: string;
  changeFingerprint: string;
  changedFiles: string[];
  approvalId: GitApproval;
};

export type CommitSessionResponse = {
  status: "committed" | "denied" | "blocked";
  commitHash?: string;
  reason?: string;
};

export type PushBranchRequest = {
  workspaceId: string;
  remote: string;
  branch: string;
  approvalId: GitApproval;
};

export type PushBranchResponse = {
  status: "pushed" | "denied" | "blocked" | "failed";
  reason?: string;
};

export type WorktreeCleanupResponse = {
  status: "blocked" | "cleaned";
  reason?: string;
};
