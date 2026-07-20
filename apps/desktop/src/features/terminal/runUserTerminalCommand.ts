import { invoke } from "@tauri-apps/api/core";
import type { TerminalCommandRequest, TerminalCommandResponse } from "../../api/types";

export type UserTerminalCommandInput = {
  workspaceId: string;
  workspacePath: string;
  request: TerminalCommandRequest;
};

export function runUserTerminalCommand(input: UserTerminalCommandInput): Promise<TerminalCommandResponse> {
  return invoke<TerminalCommandResponse>("run_user_terminal_command", {
    workspaceId: input.workspaceId,
    workspacePath: input.workspacePath,
    command: input.request.command,
    cwd: input.request.cwd ?? ".",
  });
}
