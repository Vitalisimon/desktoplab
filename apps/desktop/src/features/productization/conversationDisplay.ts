export function stripControlArtifacts(body: string): string {
  const stripped = body
    .replace(/(\b[\p{L}\p{N}_-]+)\s*\u001b\[\d+D\u001b\[K\1/gu, "$1")
    .replace(/\u001b\[[0-9;?]*[ -/]*[@-~]/g, "")
    .replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/g, "");
  return isRawDesktopLabToolCall(stripped) ? "" : stripped;
}

export function stripConversationMetadata(body: string): string {
  return body
    .replace(/(?:^|\s+)Git diff:\s+redacted=true\s+redaction_source=\S+.*$/s, "")
    .replace(/(?:^|\s+)clarification_required:[^\n]+$/i, "")
    .split(/\r?\n/)
    .filter((line) => !/^(?:state|source|tool|approval_mode|redacted|redaction_source)=[^\s]+(?:\s+\w[\w-]*=[^\s]+)*\s*$/i.test(line.trim()))
    .join("\n")
    .trim();
}

export function formatEventTime(createdAt: string): string {
  if (/^\d{10}$/.test(createdAt)) {
    return new Date(Number(createdAt) * 1000).toISOString().slice(11, 16);
  }
  const date = new Date(createdAt);
  if (Number.isNaN(date.getTime())) return createdAt;
  return date.toISOString().slice(11, 16);
}

export function displayEventMessage(kind: string, message: string): string {
  const terminalMessage = displayTerminalState(message);
  if (terminalMessage) return terminalMessage;
  if (kind === "tool_decision" && message.startsWith("provider_output_recovery:")) {
    if (message.includes("validation")) return "Re-running validation";
    if (message.includes("failed_action") || message.includes("conflict")) return "Replanning from current files";
    return "Refining the next action";
  }
  if (kind === "tool_decision") {
    const structuredMessage = displayStructuredToolDecision(message);
    if (structuredMessage) return structuredMessage;
    const directAction = toolAction(message);
    if (directAction) return directAction.label;
  }
  if (kind === "tool_decision" && message.startsWith("filesystem.read:")) {
    return `Read ${message.slice("filesystem.read:".length)}`;
  }
  if (kind === "tool_decision" && message.startsWith("filesystem.write:")) {
    return `Write ${message.slice("filesystem.write:".length)}`;
  }
  if (kind === "tool_decision" && message.startsWith("filesystem.patch:")) {
    return `Patch ${message.slice("filesystem.patch:".length)}`;
  }
  if (kind === "tool_decision" && message.startsWith("terminal:")) return "Ran a terminal command";
  if (kind === "blocked" && message === "local_inference_not_configured") return "Local inference is not configured.";
  if ((kind === "blocked" || kind === "failed") && message === "local_inference_failed") {
    return "Local inference failed before the agent could continue.";
  }
  return message;
}

function displayTerminalState(message: string): string | null {
  const normalized = message.trim().toLowerCase();
  if (normalized === "approval denied") return "Action was not approved.";
  if (normalized === "read_failed") return "The requested file could not be read.";
  if (normalized === "path_escape") return "DesktopLab blocked access outside this repository.";
  if (normalized === "waiting for approval") return "Waiting for approval.";
  if (normalized === "checkpoint blocked risky mutation") return "DesktopLab could not create a safety checkpoint for this action.";
  if (normalized === "agent_continuation_max_steps") return "The agent stopped after reaching its action limit.";
  if (normalized.startsWith("agent_no_progress")) return "The agent stopped because it could not make further progress.";
  if (normalized === "malformed structured file action") return "The model returned an invalid tool action.";
  if (/^state=checkpoint_ready(?:\s|$)/i.test(message.trim())) return "Safety checkpoint ready.";
  return null;
}

function displayStructuredToolDecision(message: string): string | null {
  const fields = parseFields(message);
  const tool = fields.get("tool");
  const state = fields.get("state");
  if (!tool || !state) return null;

  const action = toolAction(tool);
  if (!action) {
    if (state === "observed" && (tool.startsWith("clarify:") || fields.get("source") === "agent.clarify")) return "Waiting for clarification";
    return null;
  }

  if (state === "planned") return `Planned · ${action.label}`;
  if (state === "approval_required") return `Needs approval · ${action.label}`;
  if (state === "approved") return `Approved · ${action.label}`;
  if (state === "blocked") return `Blocked · ${action.label}`;
  if (state === "failed") return `Failed · ${action.label}`;
  if (state !== "executed") return action.label;

  if (action.kind === "filesystem.write") return `Changed ${action.target}`;
  if (action.kind === "filesystem.patch") return `Patched ${action.target}`;
  if (action.kind === "filesystem.read") return `Read ${action.target}`;
  if (action.kind === "filesystem.list") return "Listed files";
  if (action.kind === "workspace.search") return "Searched workspace";
  if (action.kind === "terminal") return "Ran terminal command";
  if (action.kind === "test.run") return "Validation completed";
  if (action.kind === "git.status") return "Inspected Git status";
  if (action.kind === "git.diff") return "Inspected Git diff";
  if (action.kind === "git.commit") return "Committed changes";
  if (action.kind === "git.push") return "Pushed changes";
  if (action.kind === "checkpoint.create") return "Safety checkpoint ready";
  if (action.kind === "runtime.install") return "Installed local runner";
  return action.label;
}

function parseFields(message: string): Map<string, string> {
  const fields = new Map<string, string>();
  for (const part of message.split(/\s+/)) {
    const separator = part.indexOf("=");
    if (separator <= 0) continue;
    fields.set(part.slice(0, separator), part.slice(separator + 1));
  }
  return fields;
}

function toolAction(tool: string): { kind: string; label: string; target: string } | null {
  const canonical = canonicalToolAction(tool);
  if (canonical) return canonical;
  if (tool === "filesystem.list" || tool.startsWith("filesystem.list:")) {
    return { kind: "filesystem.list", label: "List files", target: "" };
  }
  if (tool.startsWith("filesystem.read:")) {
    const target = tool.slice("filesystem.read:".length);
    return { kind: "filesystem.read", label: `Read ${target}`, target };
  }
  if (tool.startsWith("filesystem.write:")) {
    const target = tool.slice("filesystem.write:".length);
    return { kind: "filesystem.write", label: `Write ${target}`, target };
  }
  if (tool.startsWith("filesystem.patch:")) {
    const target = tool.slice("filesystem.patch:".length);
    return { kind: "filesystem.patch", label: `Patch ${target}`, target };
  }
  if (tool === "search.text" || tool.startsWith("search.text:")) {
    return { kind: "workspace.search", label: "Search workspace", target: "" };
  }
  if (tool.startsWith("terminal:")) return { kind: "terminal", label: "Run terminal command", target: "" };
  if (tool.startsWith("test.run:")) return { kind: "test.run", label: "Run validation", target: "" };
  if (tool === "git.status") return { kind: "git.status", label: "Inspect Git status", target: "" };
  if (tool === "git.diff" || tool.startsWith("git.diff:")) return { kind: "git.diff", label: "Inspect Git diff", target: "" };
  if (tool.startsWith("git.commit:")) return { kind: "git.commit", label: "Commit changes", target: "" };
  if (tool.startsWith("git.push:")) return { kind: "git.push", label: "Push changes", target: "" };
  if (tool === "checkpoint.create" || tool.startsWith("checkpoint.create:")) {
    return { kind: "checkpoint.create", label: "Create safety checkpoint", target: "" };
  }
  if (tool.startsWith("runtime.install:")) return { kind: "runtime.install", label: "Install local runner", target: "" };
  return null;
}

function canonicalToolAction(tool: string): { kind: string; label: string; target: string } | null {
  const actions: Record<string, { kind: string; label: string }> = {
    "desktoplab.list_files": { kind: "filesystem.list", label: "List files" },
    "desktoplab.read_file": { kind: "filesystem.read", label: "Read file" },
    "desktoplab.search_text": { kind: "workspace.search", label: "Search workspace" },
    "desktoplab.write_file": { kind: "filesystem.write", label: "Write file" },
    "desktoplab.patch_file": { kind: "filesystem.patch", label: "Patch file" },
    "desktoplab.create_directory": { kind: "filesystem.write", label: "Create directory" },
    "desktoplab.move_path": { kind: "filesystem.write", label: "Move path" },
    "desktoplab.delete_path": { kind: "filesystem.write", label: "Delete path" },
    "desktoplab.run_terminal": { kind: "terminal", label: "Run terminal command" },
    "desktoplab.run_tests": { kind: "test.run", label: "Run validation" },
    "desktoplab.start_process": { kind: "terminal", label: "Start process" },
    "desktoplab.poll_process": { kind: "terminal", label: "Check process" },
    "desktoplab.write_process_stdin": { kind: "terminal", label: "Send process input" },
    "desktoplab.kill_process": { kind: "terminal", label: "Stop process" },
    "desktoplab.git_status": { kind: "git.status", label: "Inspect Git status" },
    "desktoplab.git_diff": { kind: "git.diff", label: "Inspect Git diff" },
    "desktoplab.create_checkpoint": { kind: "checkpoint.create", label: "Create safety checkpoint" },
    "desktoplab.commit_changes": { kind: "git.commit", label: "Commit changes" },
    "desktoplab.push_changes": { kind: "git.push", label: "Push changes" },
    "desktoplab.update_plan": { kind: "agent.plan", label: "Update plan" },
    "desktoplab.complete": { kind: "agent.complete", label: "Complete task" },
    "desktoplab.clarify": { kind: "agent.clarify", label: "Ask for clarification" },
  };
  const action = actions[tool];
  return action ? { ...action, target: "" } : null;
}

function isRawDesktopLabToolCall(body: string): boolean {
  const trimmed = body.trim();
  if (!trimmed) return false;
  const values = parseConcatenatedJsonObjects(trimmed);
  return values.length > 0 && values.every(isToolCallValue);
}

function parseConcatenatedJsonObjects(body: string): unknown[] {
  const values: unknown[] = [];
  let index = 0;
  while (index < body.length) {
    while (/\s/.test(body[index] ?? "")) index += 1;
    if (index >= body.length) break;
    if (body[index] !== "{") return [];
    const end = jsonObjectEnd(body, index);
    if (end === -1) return [];
    try {
      values.push(JSON.parse(body.slice(index, end + 1)));
    } catch {
      return [];
    }
    index = end + 1;
  }
  return values;
}

function jsonObjectEnd(body: string, start: number): number {
  let depth = 0;
  let inString = false;
  let escaped = false;
  for (let index = start; index < body.length; index += 1) {
    const char = body[index];
    if (inString) {
      if (escaped) escaped = false;
      else if (char === "\\") escaped = true;
      else if (char === "\"") inString = false;
      continue;
    }
    if (char === "\"") inString = true;
    else if (char === "{") depth += 1;
    else if (char === "}") {
      depth -= 1;
      if (depth === 0) return index;
    }
  }
  return -1;
}

function isToolCallValue(value: unknown): boolean {
  if (!value || typeof value !== "object") return false;
  const record = value as { name?: unknown; arguments?: unknown };
  return typeof record.name === "string" && record.name.trim().length > 0 && record.arguments !== undefined;
}
