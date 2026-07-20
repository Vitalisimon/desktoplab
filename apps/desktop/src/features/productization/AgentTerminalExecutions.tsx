import type { AgentSessionSnapshot } from "../../api/types";
import { Copy } from "../../design/icons";

export function AgentTerminalExecutions({ session }: { session: AgentSessionSnapshot }) {
  const executions = terminalExecutions(session);
  if (executions.length === 0) return null;
  return (
    <div className="grid max-w-3xl gap-2" aria-label="Agent terminal executions">
      {executions.map((execution) => (
        <details key={`${execution.command}-${execution.sequence}`} className="min-w-0 rounded-desktop border border-line bg-panel px-3 py-2" open={execution.failed}>
          <summary className={`cursor-pointer text-sm font-semibold ${execution.failed ? "text-danger" : "text-ink"}`}>
            {execution.failed ? "Command failed" : "Command completed"} · {execution.command}
          </summary>
          <dl className="mt-2 grid gap-2 text-xs text-muted sm:grid-cols-4">
            <ExecutionField label="Cwd" value={execution.cwd} />
            <ExecutionField label="Status" value={execution.status} />
            <ExecutionField label="Duration" value={execution.duration} />
            <ExecutionField label="Action" value={sourceLabel(execution.source)} />
          </dl>
          <button type="button" aria-label="Copy command output" title="Copy command output" className="mt-2 inline-flex h-7 w-7 items-center justify-center rounded-desktop border border-line text-muted hover:text-ink" onClick={() => void navigator.clipboard?.writeText(execution.output)}>
            <Copy size={14} />
          </button>
          <pre tabIndex={0} className="mt-2 max-h-72 overflow-auto whitespace-pre-wrap text-[13px] leading-6 text-ink">{execution.output}</pre>
        </details>
      ))}
    </div>
  );
}

function sourceLabel(source: string): string {
  if (source === "test.run") return "Test";
  if (source === "terminal.command") return "Terminal";
  return "Agent";
}

function ExecutionField({ label, value }: { label: string; value: string }) {
  return <div className="min-w-0"><dt className="font-semibold text-ink">{label}</dt><dd className="break-words">{value}</dd></div>;
}

type AgentTerminalExecution = {
  sequence: number;
  command: string;
  cwd: string;
  status: string;
  duration: string;
  source: string;
  output: string;
  failed: boolean;
};

function terminalExecutions(session: AgentSessionSnapshot): AgentTerminalExecution[] {
  const executions: AgentTerminalExecution[] = [];
  let command = { command: "agent command", source: "agent" };
  for (const event of [...session.timeline].sort((left, right) => left.sequence - right.sequence)) {
    if (event.kind === "tool_decision") command = commandFromDecision(event.message) ?? command;
    if (event.kind !== "tool") continue;
    const execution = terminalEvidenceFromMessage(event.sequence, event.message, command);
    if (execution) executions.push(execution);
  }
  return executions;
}

function commandFromDecision(message: string): { command: string; source: string } | null {
  if (message.includes("terminal:")) return { command: commandTail(message, "terminal:"), source: "terminal.command" };
  if (message.includes("test.run:")) return { command: commandTail(message, "test.run:"), source: "test.run" };
  return null;
}

function commandTail(message: string, marker: string): string {
  const tail = message.split(marker)[1]?.trim();
  if (!tail) return "agent command";
  return tail.split(" approval_mode=")[0].split(" source=")[0].split(" state=")[0].trim();
}

function terminalEvidenceFromMessage(
  sequence: number,
  message: string,
  command: { command: string; source: string },
): AgentTerminalExecution | null {
  const status = processStatus(message);
  if (status === null || !message.includes("stdout:") || !message.includes("stderr:")) return null;
  const durationMs = message.match(/duration_ms=(\d+)/)?.[1]
    ?? message.match(/finished with status [^.]+ in (\d+)ms\./)?.[1];
  const cwd = message.match(/cwd=([^\n ]+)/)?.[1] ?? "workspace";
  const stdout = message.split("stdout:\n")[1]?.split("\nstderr:\n")[0] ?? "";
  const stderr = message.split("\nstderr:\n")[1] ?? "";
  const output = [`stdout:\n${stdout}`, `stderr:\n${stderr}`].join("\n").trim();
  return {
    sequence,
    command: command.command,
    cwd,
    status,
    duration: durationMs ? `${durationMs} ms` : "Not recorded",
    source: command.source,
    output,
    failed: status !== "exited:0",
  };
}

function processStatus(message: string): string | null {
  const terminalStatus = message.match(/(?:^|\n)status=([^\n ]+)/)?.[1];
  if (terminalStatus) return terminalStatus;

  const testExitCode = message.match(/finished with status Exited\((-?\d+)\)/)?.[1];
  if (testExitCode) return `exited:${testExitCode}`;
  if (message.includes("finished with status TimedOut")) return "timed_out";
  if (message.includes("finished with status FailedToSpawn")) return "failed_to_spawn";
  return null;
}
