import type { BackendEventFrame } from "./events";

export type AgentConversationEvent = {
  eventId: string;
  kind: string;
  message: string;
};

export function agentConversationEventsFromFrames(
  frames: BackendEventFrame[],
  sessionId?: string,
): AgentConversationEvent[] {
  const seen = new Set<string>();
  const events: AgentConversationEvent[] = [];
  for (const frame of frames) {
    if (frame.scope !== "session") continue;
    const payload = parsePayload(frame.payload);
    if (!payload || typeof payload.kind !== "string") continue;
    if (!payload.kind.startsWith("agent.")) continue;
    if (sessionId && payload.sessionId !== sessionId) continue;
    const eventId = typeof payload.eventId === "string" ? payload.eventId : `${frame.sequence}`;
    if (seen.has(eventId)) continue;
    seen.add(eventId);
    events.push({
      eventId,
      kind: payload.kind,
      message: typeof payload.message === "string" ? payload.message : agentEventMessage(payload.kind),
    });
  }
  return events;
}

function agentEventMessage(kind: string) {
  if (kind === "agent.prompt.accepted") return "Prompt accepted";
  if (kind === "agent.context.read") return "Repository context read";
  if (kind === "agent.step.blocked") return "Action blocked";
  if (kind === "agent.step.completed") return "Response complete";
  return kind;
}

function parsePayload(payload: string): Record<string, unknown> | null {
  try {
    return JSON.parse(payload) as Record<string, unknown>;
  } catch {
    return null;
  }
}
