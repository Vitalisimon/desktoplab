import type { AgentSessionSnapshot } from "../../api/types";
import type { BackendEventFrame } from "../../api/events";
import { ConversationEvent, MessageBlock, UserMessageBlock } from "./ConversationEvent";
import { displayEventMessage, stripControlArtifacts, stripConversationMetadata } from "./conversationDisplay";
import { AgentEventStream, AgentStatusPill, shouldShowProgressEvents, streamedAgentEvents } from "./AgentProgress";
import { AgentTerminalExecutions } from "./AgentTerminalExecutions";
import { AgentEvidenceTimeline } from "./AgentEvidenceTimeline";
import { AgentStructuredEvidence } from "./AgentStructuredEvidence";
import { AgentFailureNotice } from "./AgentFailureNotice";

export function ConversationTranscript({ session, eventFrames = [] }: { session: AgentSessionSnapshot; eventFrames?: BackendEventFrame[] }) {
  const plan = session.plan ? visibleMessageBody(session.plan) : null;
  const transcript = session.transcript ?? [];
  const summary = visibleSummary(session.summary, transcript);
  const streamedEvents = activeProgressEvents(session, eventFrames);
  if (transcript.length > 0) {
    const outcomeEvents = terminalOutcomeEvents(session);
    return (
      <div className="space-y-7">
        <AgentStatusPill session={session} />
        {streamedEvents.length > 0 && shouldShowProgressEvents(session) ? <AgentEventStream events={streamedEvents} /> : null}
        {transcript.map((turn) => (
          <TranscriptTurn key={turn.sequence} turn={turn} />
        ))}
        {outcomeEvents.map((event) => (
          <ConversationEvent key={`outcome-${event.sequence}`} event={event} />
        ))}
        <AgentFailureNotice session={session} />
        <AgentEvidenceTimeline session={session} />
        <AgentStructuredEvidence session={session} />
        <AgentTerminalExecutions session={session} />
        {summary ? <MessageBlock body={summary} /> : null}
      </div>
    );
  }
  const hasPlanningEvent = session.timeline.some((event) => event.kind === "planning");
  const showProgressEvents = shouldShowProgressEvents(session);
  const events = [...session.timeline]
    .sort((left, right) => left.sequence - right.sequence)
    .filter((event) => !(event.kind === "planning" && !hasPlanningEvent && visibleMessageBody(event.message) === plan))
    .filter((event) => !(event.kind === "failed" && session.failureClassification))
    .filter((event) => !isProgressEvent(event));
  const streamedTimelineEvents = showProgressEvents ? activeProgressEvents(session, eventFrames) : [];
  return (
    <div className="space-y-7">
      <AgentStatusPill session={session} />
      {plan && !hasPlanningEvent ? <MessageBlock body={plan} /> : null}
      {events.length === 0 ? <p className="text-sm text-muted">No session events yet.</p> : null}
      {streamedTimelineEvents.length > 0 ? <AgentEventStream events={streamedTimelineEvents} /> : null}
      {events.map((event) => (
        <ConversationEvent key={event.sequence} event={event} />
      ))}
      <AgentFailureNotice session={session} />
      <AgentEvidenceTimeline session={session} />
      <AgentStructuredEvidence session={session} />
      <AgentTerminalExecutions session={session} />
      {summary ? <MessageBlock body={summary} /> : null}
    </div>
  );
}

function terminalOutcomeEvents(session: AgentSessionSnapshot) {
  if (session.state === "failed" && session.failureClassification) return [];
  const terminalKind = session.state === "blocked" || session.state === "failed" || session.state === "cancelled"
    ? session.state
    : null;
  if (!terminalKind) return [];
  return session.timeline.filter((event) => event.kind === terminalKind).slice(-1);
}

function activeProgressEvents(session: AgentSessionSnapshot, eventFrames: BackendEventFrame[]) {
  const streamed = streamedAgentEvents(session, eventFrames);
  if (streamed.length > 0) return streamed;
  return session.timeline
    .filter(isProgressEvent)
    .map((event) => ({
      eventId: `timeline-${event.sequence}`,
      kind: event.kind,
      message: progressMessage(event.kind, stripControlArtifacts(event.message.split("\n\nRepository context:")[0]).trim()),
    }));
}

function progressMessage(kind: string, message: string): string {
  if (kind === "tool" && message.includes("status=exited:0")) return "Command completed";
  if (kind === "tool" && message.includes("status=")) return "Command failed";
  if (kind === "test") return message || "Validation completed";
  return displayEventMessage(kind, message);
}

function TranscriptTurn({ turn }: { turn: NonNullable<AgentSessionSnapshot["transcript"]>[number] }) {
  const content = visibleMessageBody(turn.content);
  if (!content) return null;
  if (turn.role === "user") return <UserMessageBlock body={content} />;
  if (turn.role === "assistant") return <MessageBlock body={content} />;
  return null;
}

function isProgressEvent(event: AgentSessionSnapshot["timeline"][number]): boolean {
  if (event.kind === "assistant" && (event.message.startsWith("Git status:") || event.message.startsWith("Git diff:"))) return false;
  return event.kind === "tool_decision" || event.kind === "tool" || event.kind === "test" || event.kind === "checkpoint";
}

export function EmptyConversation() {
  return (
    <div className="flex min-h-full items-end pb-8">
      <p className="max-w-2xl text-[15px] leading-7 text-muted">Ask DesktopLab what to change, inspect, or verify in this repository.</p>
    </div>
  );
}

function visibleMessageBody(body: string): string {
  return stripConversationMetadata(stripControlArtifacts(body.split("\n\nRepository context:")[0]));
}

function visibleSummary(summary: string | null, transcript: NonNullable<AgentSessionSnapshot["transcript"]>): string | null {
  if (!summary || summary === "agent loop completed") return null;
  const visible = stripConversationMetadata(stripControlArtifacts(summary));
  const existingAssistantMessages = transcript
    .filter((turn) => turn.role === "assistant")
    .map((turn) => visibleMessageBody(turn.content));
  return existingAssistantMessages.includes(visible) ? null : visible;
}
