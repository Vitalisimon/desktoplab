export type TerminalCommandApproval = {
  approvalId: string;
  state: "pending" | "approved" | "denied";
  copy: string;
};

export type TerminalCommandEvent = {
  eventId: string;
  kind: "output" | "status";
  stdout: string;
  stderr: string;
  status: "exited" | "timed_out" | "failed_to_spawn";
  exitCode: number | null;
  stdoutTruncated: boolean;
  redacted: boolean;
};

export type TerminalCommandRequest = {
  command: string;
  cwd?: string;
  approvalId?: string;
  approvalRequired?: boolean;
};

export type TerminalCommandResponse = {
  terminalId: string;
  workspaceId: string;
  state: "approval_required" | "completed" | "denied";
  command: string;
  cwd: string;
  approval: TerminalCommandApproval;
  events: TerminalCommandEvent[];
};
