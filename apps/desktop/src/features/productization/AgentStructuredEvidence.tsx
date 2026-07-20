import type { AgentSessionSnapshot } from "../../api/types";
import { Copy } from "../../design/icons";

export function AgentStructuredEvidence({ session }: { session: AgentSessionSnapshot }) {
  const diffs = session.details?.diffs ?? [];
  const validations = session.details?.validations ?? [];
  const latestValidationFailed = validations.length > 0 && validationFailed(validations.at(-1)!.message);
  if (diffs.length === 0 && validations.length === 0) return null;
  return (
    <section data-evidence-state={evidenceState(validations)} className="grid max-w-3xl gap-2" aria-label="Agent diff and validation evidence">
      {diffs.map((entry, index) => <DiffEvidence key={`${entry.message}-${index}`} message={entry.message} />)}
      {validations.map((entry, index) => (
        <ValidationEvidence
          key={`${entry.message}-${index}`}
          message={entry.message}
          expanded={latestValidationFailed && index === validations.length - 1}
        />
      ))}
    </section>
  );
}

function evidenceState(validations: Array<{ message: string }>) {
  const latest = validations.at(-1);
  if (!latest) return "diff";
  return validationFailed(latest.message) ? "validation-failed" : "validation-passed";
}

function DiffEvidence({ message }: { message: string }) {
  const diff = diffBody(message);
  const files = changedFiles(diff);
  return (
    <details className="min-w-0 rounded-desktop border border-line bg-panel px-3 py-2">
      <summary className="cursor-pointer text-sm font-semibold text-ink">
        {files.length === 1 ? `Changed ${files[0]}` : `${files.length} changed files`}
      </summary>
      {files.length > 0 ? <p className="mt-2 break-words text-xs text-muted">{files.join(" · ")}</p> : null}
      <EvidenceBody body={diff} label="Copy diff" />
    </details>
  );
}

function ValidationEvidence({ message, expanded }: { message: string; expanded: boolean }) {
  const failed = validationFailed(message);
  return (
    <details className="min-w-0 rounded-desktop border border-line bg-panel px-3 py-2" open={expanded}>
      <summary className={`cursor-pointer text-sm font-semibold ${failed ? "text-danger" : "text-success"}`}>
        {failed ? "Validation failed" : "Validation passed"}
      </summary>
      <EvidenceBody body={message} label="Copy validation output" />
    </details>
  );
}

function validationFailed(message: string) {
  return /failed|exit(?:ed)?(?:[:= ]+[1-9]\d*|\([1-9]\d*\))|timeout/i.test(message);
}

function EvidenceBody({ body, label }: { body: string; label: string }) {
  return (
    <div className="mt-2 min-w-0">
      <button type="button" aria-label={label} title={label} className="mb-2 inline-flex h-7 w-7 items-center justify-center rounded-desktop border border-line text-muted hover:text-ink" onClick={() => void navigator.clipboard?.writeText(body)}>
        <Copy size={14} />
      </button>
      <pre tabIndex={0} className="max-h-80 overflow-auto whitespace-pre font-mono text-xs leading-5 text-ink">{body}</pre>
    </div>
  );
}

function diffBody(message: string): string {
  const marker = message.indexOf("diff --git");
  return marker >= 0 ? message.slice(marker) : message.replace(/^Git diff:\s*/i, "");
}

function changedFiles(diff: string): string[] {
  return [...new Set([...diff.matchAll(/(?:^|\s)diff --git a\/(\S+) b\/(\S+)/g)].map((match) => match[2]))];
}
