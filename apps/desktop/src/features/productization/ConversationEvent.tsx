import type { AgentSessionSnapshot } from "../../api/types";
import { EvidenceDisclosure } from "../../design/OperationalPrimitives";
import { displayEventMessage, formatEventTime, stripControlArtifacts, stripConversationMetadata } from "./conversationDisplay";

export function ConversationEvent({ event }: { event: AgentSessionSnapshot["timeline"][number] }) {
  const message = visibleEventMessage(event.message);
  if (!message) return null;
  if (event.kind === "planning") return <UserMessageBlock body={message} />;
  if (event.kind === "assistant") return <MessageBlock body={message} />;
  if (event.kind === "completed" || message === "agent loop completed") {
    return <p className="text-xs font-medium text-muted">Response complete</p>;
  }
  return (
    <article className="grid gap-2">
      <div className="flex items-center gap-2 text-sm text-muted">
        <span className="h-1.5 w-1.5 rounded-full bg-muted/50" />
        <span>{displayEventMessage(event.kind, message)}</span>
        <span className="text-xs" title={event.createdAt}>
          {formatEventTime(event.createdAt)}
        </span>
      </div>
      <EventEvidence event={event} />
    </article>
  );
}

function visibleEventMessage(body: string): string {
  if (body.trim().toLowerCase().startsWith("clarification_required:")) {
    return `Clarification needed: ${body.trim().slice("clarification_required:".length).trim()}`;
  }
  return stripConversationMetadata(stripControlArtifacts(body.split("\n\nRepository context:")[0]));
}

export function MessageBlock({ body }: { body: string }) {
  return <p className="max-w-2xl whitespace-pre-wrap break-words text-[15px] leading-7 text-ink">{body}</p>;
}

export function UserMessageBlock({ body }: { body: string }) {
  return <p className="ml-auto max-w-2xl whitespace-pre-wrap break-words rounded-desktop bg-elevated px-4 py-2 text-[15px] leading-7 text-ink">{body}</p>;
}

function EventEvidence({ event }: { event: AgentSessionSnapshot["timeline"][number] }) {
  if (event.evidence) return <EvidenceDisclosure title={event.evidence.title} body={event.evidence.body} />;
  if (event.test) return <EvidenceDisclosure title={event.message} body={`${event.test.command}\n${event.test.output}`} />;
  return null;
}
