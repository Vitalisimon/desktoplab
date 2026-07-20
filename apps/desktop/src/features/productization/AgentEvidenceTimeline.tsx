import type { AgentSessionSnapshot } from "../../api/types";
import { displayEventMessage, stripControlArtifacts } from "./conversationDisplay";

export function AgentEvidenceTimeline({ session }: { session: AgentSessionSnapshot }) {
  const entries = evidenceEntries(session);
  if (entries.length === 0) return null;
  return (
    <details className="max-w-3xl border-t border-line pt-3" aria-label="Technical evidence timeline">
      <summary className="cursor-pointer text-xs font-semibold text-muted">Technical evidence ({entries.length})</summary>
      <ol className="mt-3 space-y-2 text-sm text-muted">
        {entries.map((entry, index) => <li key={`${entry}-${index}`}>{entry}</li>)}
      </ol>
    </details>
  );
}

function evidenceEntries(session: AgentSessionSnapshot): string[] {
  if (session.details) {
    return [
      ...session.details.toolCalls
        .filter((entry) => entry.state.trim() && entry.tool.trim())
        .map((entry) => displayEventMessage("tool_decision", `state=${entry.state} source=${entry.source} tool=${entry.tool}`)),
      ...session.details.observations.map((entry) => readableObservation(entry.message)),
    ];
  }
  return session.timeline
    .filter(isTechnicalEvent)
    .map((event) => displayEvidenceEntry(event.kind, visibleMessageBody(event.message)))
    .filter((entry): entry is string => Boolean(entry));
}

function readableObservation(message: string): string {
  if (/\b(?:state|source|redacted|redaction_source|approval_mode)=/i.test(message)) return "Result recorded with sensitive details removed";
  if (/^read\s/i.test(message)) return message;
  if (/^changed\s/i.test(message)) return message;
  if (/^command (?:completed|failed)/i.test(message)) return message;
  return `Observed · ${message}`;
}

function isTechnicalEvent(event: AgentSessionSnapshot["timeline"][number]): boolean {
  return event.kind === "tool_decision" || event.kind === "tool" || event.kind === "test" || event.kind === "checkpoint";
}

function displayEvidenceEntry(kind: string, message: string): string | null {
  if (!message || message.includes("status=") || message.includes("stdout:")) return null;
  if (/\b(?:state|source|redacted|redaction_source|approval_mode)=/i.test(message)) return "Result recorded with sensitive details removed";
  return `${kind.replaceAll("_", " ")}: ${displayEventMessage(kind, message)}`;
}

function visibleMessageBody(body: string): string {
  return stripControlArtifacts(body.split("\n\nRepository context:")[0]).trim();
}
