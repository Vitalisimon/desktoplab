import type { TerminalCommandEvent, TerminalCommandResponse } from "./types";
import type { BackendEventFrame } from "./events";

export function terminalEventsFromFrames(
  frames: BackendEventFrame[],
  terminalId: string,
): TerminalCommandEvent[] {
  const seen = new Set<string>();
  const events: TerminalCommandEvent[] = [];
  for (const frame of frames) {
    if (frame.scope !== "terminal") continue;
    const event = parseTerminalEvent(frame.payload, terminalId);
    if (!event || seen.has(event.eventId)) continue;
    seen.add(event.eventId);
    events.push(event);
  }
  return events;
}

export function terminalResponseFromFrames(
  frames: BackendEventFrame[],
): TerminalCommandResponse | null {
  const payloads = frames
    .filter((frame) => frame.scope === "terminal")
    .map((frame) => parsePayload(frame.payload))
    .filter((payload): payload is Record<string, unknown> => Boolean(payload));
  const started = payloads.find(
    (payload) => payload.kind === "terminal.started" || payload.kind === "terminal.approval_required",
  );
  if (!started || typeof started.terminalId !== "string" || typeof started.command !== "string") return null;

  const terminalId = started.terminalId;
  const denied = payloads.find((payload) => payload.terminalId === terminalId && payload.kind === "terminal.denied");
  const state = denied ? "denied" : started.approvalState === "pending" ? "approval_required" : "completed";
  const approvalState = denied ? "denied" : started.approvalState === "pending" ? "pending" : "approved";
  return {
    terminalId,
    workspaceId: typeof started.workspaceId === "string" ? started.workspaceId : "",
    state,
    command: started.command,
    cwd: typeof started.cwd === "string" ? started.cwd : ".",
    approval: {
      approvalId: typeof started.approvalId === "string" && started.approvalId.length > 0 ? started.approvalId : `${terminalId}.approval`,
      state: approvalState,
      copy: denied && typeof denied.copy === "string" ? denied.copy : approvalCopy(started.command, started.cwd),
    },
    events: terminalEventsFromFrames(frames, terminalId),
  };
}

function parseTerminalEvent(payload: string, terminalId: string): TerminalCommandEvent | null {
  const value = parsePayload(payload);
  if (!value || value.terminalId !== terminalId) return null;
  if (typeof value.eventId !== "string") return null;
  if (value.kind !== "terminal.output" && value.kind !== "terminal.completed") return null;
  if (value.kind === "terminal.completed" && !value.stdout && !value.stderr) return null;
  return {
    eventId: value.eventId,
    kind: "output",
    stdout: typeof value.stdout === "string" ? value.stdout : "",
    stderr: typeof value.stderr === "string" ? value.stderr : "",
    status: terminalStatus(value.status),
    exitCode: typeof value.exitCode === "number" ? value.exitCode : null,
    stdoutTruncated: value.stdoutTruncated === true,
    redacted: value.redacted === true,
  };
}

function terminalStatus(value: unknown): TerminalCommandEvent["status"] {
  if (value === "timed_out" || value === "failed_to_spawn") return value;
  return "exited";
}

function parsePayload(payload: string): Record<string, unknown> | null {
  try {
    return JSON.parse(payload) as Record<string, unknown>;
  } catch {
    return null;
  }
}

function approvalCopy(command: string, cwd: unknown): string {
  return `Terminal command \`${command}\` in \`${typeof cwd === "string" ? cwd : "."}\` requires approval.`;
}
