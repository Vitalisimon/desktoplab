import { Check, Copy, Plus, TerminalSquare, X } from "../../design/icons";
import { useRef, useState, type KeyboardEvent } from "react";
import { terminalEventsFromFrames, type BackendEventFrame } from "../../api/events";
import type { TerminalCommandRequest, TerminalCommandResponse } from "../../api/types";

type TerminalDrawerProps = {
  open: boolean;
  height: number;
  response?: TerminalCommandResponse | null;
  eventFrames?: BackendEventFrame[];
  workspacePath?: string;
  sessionLabel?: string | null;
  onApprove?: (approvalId: string) => void;
  onDeny?: (approvalId: string) => void;
  onRunCommand?: (request: TerminalCommandRequest) => Promise<TerminalCommandResponse>;
  onClose: () => void;
};

type TerminalTab = {
  id: string;
  title: string;
  command: string;
  response: TerminalCommandResponse | null;
  error: string | null;
  running: boolean;
};

export function TerminalDrawer({
  open,
  height,
  response = null,
  eventFrames = [],
  workspacePath = "Workspace shell",
  sessionLabel = null,
  onApprove,
  onDeny,
  onRunCommand,
  onClose,
}: TerminalDrawerProps) {
  const nextTabNumber = useRef(1);
  const [tabs, setTabs] = useState<TerminalTab[]>(() => [createTerminalTab(1, workspacePath)]);
  const [activeTabId, setActiveTabId] = useState("terminal.local.1");
  if (!open) return null;
  const activeTab = tabs.find((tab) => tab.id === activeTabId) ?? tabs[0];
  const visibleResponse = activeTab.response ?? response;
  const effectiveHeight = visibleResponse ? Math.max(height, 260) : height;
  const replayedResponse = activeTab.response
    ? activeTab.response
    : visibleResponse
      ? withReplayEvents(visibleResponse, eventFrames)
      : null;
  const outputText = replayedResponse ? terminalOutputText(replayedResponse) : "";
  const activeCwd = replayedResponse ? terminalDisplayCwd(replayedResponse.cwd, workspacePath) : workspacePath;
  const activeTabTitle = replayedResponse ? terminalTitle(activeCwd) : activeTab.title;
  const updateActiveTab = (patch: Partial<TerminalTab>) => {
    setTabs((current) => current.map((tab) => (tab.id === activeTab.id ? { ...tab, ...patch } : tab)));
  };
  const openWorkspaceTerminal = () => {
    const nextNumber = nextTabNumber.current + 1;
    nextTabNumber.current = nextNumber;
    const nextTab = createTerminalTab(nextNumber, workspacePath);
    setTabs((current) => [...current, nextTab]);
    setActiveTabId(nextTab.id);
  };
  const runCommand = async () => {
    if (!onRunCommand || !activeTab.command.trim()) return;
    const nextCommand = activeTab.command.trim();
    updateActiveTab({ running: true, error: null });
    try {
      const nextResponse = await onRunCommand({
        command: nextCommand,
        cwd: ".",
      });
      updateActiveTab({ response: nextResponse, command: "" });
    } catch {
      updateActiveTab({ error: "Terminal command could not be created." });
    } finally {
      updateActiveTab({ running: false });
    }
  };
  const onCommandKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key !== "Enter" || event.shiftKey) return;
    event.preventDefault();
    void runCommand();
  };

  return (
    <aside
      aria-label="Terminal"
      className="flex shrink-0 flex-col border-t border-line bg-canvas text-ink"
      role="complementary"
      style={{ height: effectiveHeight }}
    >
      <div className="flex h-10 shrink-0 items-center justify-between border-b border-line bg-panel/86 px-3">
        <div className="flex min-w-0 items-center gap-1" role="tablist" aria-label="Workspace terminals">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              type="button"
              role="tab"
              aria-selected={tab.id === activeTab.id}
              className={`inline-flex h-7 min-w-0 max-w-44 items-center gap-2 rounded-desktop px-3 text-sm font-medium ${tab.id === activeTab.id ? "bg-elevated text-ink" : "text-muted hover:bg-elevated/70 hover:text-ink"}`}
              onClick={() => setActiveTabId(tab.id)}
            >
              <TerminalSquare size={14} />
              <span className="truncate">{tab.id === activeTab.id ? activeTabTitle : tab.title}</span>
            </button>
          ))}
          <button
            type="button"
            aria-label="Open workspace terminal"
            className="grid h-7 w-7 place-items-center rounded-desktop text-muted hover:bg-elevated/70 hover:text-ink"
            onClick={openWorkspaceTerminal}
          >
            <Plus size={14} />
          </button>
        </div>
        <div className="flex items-center gap-2">
          {outputText ? (
            <button
              type="button"
              aria-label="Copy terminal output"
              className="grid h-8 w-8 place-items-center rounded-desktop border border-line text-muted hover:text-ink"
              onClick={() => void navigator.clipboard?.writeText(outputText)}
            >
              <Copy size={15} />
            </button>
          ) : null}
          <button
            type="button"
            aria-label="Close terminal drawer"
            className="grid h-8 w-8 place-items-center rounded-desktop border border-line text-muted hover:text-ink"
            onClick={onClose}
          >
            <X size={15} />
          </button>
        </div>
      </div>
      <div data-testid="terminal-scroll-region" className="min-h-0 flex-1 overflow-auto px-4 py-3 font-mono text-[13px]">
        <div data-testid="terminal-command-line" className="mb-3 flex items-center gap-2 leading-6">
          <span className="shrink-0 text-muted">{promptLabel(activeCwd)}</span>
          <label className="sr-only" htmlFor="terminal-command">
            Terminal input
          </label>
          <input
            id="terminal-command"
            aria-label="Terminal input"
            className="min-w-0 flex-1 bg-transparent font-mono text-[13px] text-ink caret-ink outline-none focus:outline-none focus:ring-0 focus-visible:outline-none"
            disabled={!onRunCommand || activeTab.running}
            placeholder=""
            value={activeTab.command}
            onChange={(event) => updateActiveTab({ command: event.target.value })}
            onKeyDown={onCommandKeyDown}
          />
        </div>
        {activeTab.error ? <p className="mt-1 text-xs text-danger">{activeTab.error}</p> : null}
        {replayedResponse ? (
          <TerminalCommandView
            response={replayedResponse}
            workspacePath={workspacePath}
            sessionLabel={sessionLabel}
            onApprove={onApprove}
            onDeny={onDeny}
          />
        ) : (
          <EmptyTerminal />
        )}
      </div>
    </aside>
  );
}

function createTerminalTab(number: number, workspacePath: string): TerminalTab {
  const title = number === 1 ? terminalTitle(workspacePath) : `${terminalTitle(workspacePath)} ${number}`;
  return {
    id: `terminal.local.${number}`,
    title,
    command: "",
    response: null,
    error: null,
    running: false,
  };
}

function withReplayEvents(response: TerminalCommandResponse, eventFrames: BackendEventFrame[]): TerminalCommandResponse {
  const eventsById = new Map(response.events.map((event) => [event.eventId, event]));
  for (const event of terminalEventsFromFrames(eventFrames, response.terminalId)) {
    eventsById.set(event.eventId, event);
  }
  return { ...response, events: [...eventsById.values()] };
}

function EmptyTerminal() {
  return null;
}

function promptLabel(workspacePath: string) {
  return `${terminalTitle(workspacePath)} %`;
}

function terminalTitle(workspacePath: string) {
  const parts = workspacePath.split("/").filter(Boolean);
  return parts.at(-1) ?? "terminal";
}

function terminalDisplayCwd(cwd: string, workspacePath: string) {
  return cwd === "." || cwd === "" ? workspacePath : cwd;
}

function TerminalCommandView({
  response,
  workspacePath,
  sessionLabel,
  onApprove,
  onDeny,
}: {
  response: TerminalCommandResponse;
  workspacePath: string;
  sessionLabel?: string | null;
  onApprove?: (approvalId: string) => void;
  onDeny?: (approvalId: string) => void;
}) {
  const showApprovalCopy = response.approval.state === "pending" || response.approval.state === "denied";
  const displayCwd = terminalDisplayCwd(response.cwd, workspacePath);
  return (
    <div className="grid gap-2">
      {sessionLabel ? <p className="text-xs font-semibold text-muted">{sessionLabel} command</p> : null}
      <pre className="whitespace-pre-wrap text-[13px] leading-6 text-ink">{`${promptLabel(displayCwd)} ${response.command}`}</pre>
      {showApprovalCopy ? (
        <p className="max-w-3xl text-[13px] leading-6 text-muted">{response.approval.copy}</p>
      ) : null}
      {response.approval.state === "pending" ? (
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            aria-label="Approve command"
            className="inline-flex h-9 items-center gap-2 rounded-desktop bg-ink px-3 text-sm font-semibold text-canvas"
            onClick={() => onApprove?.(response.approval.approvalId)}
          >
            <Check size={15} />
            Approve
          </button>
          <button
            type="button"
            aria-label="Deny command"
            className="inline-flex h-9 items-center gap-2 rounded-desktop border border-line bg-panel px-3 text-sm font-semibold text-ink"
            onClick={() => onDeny?.(response.approval.approvalId)}
          >
            <X size={15} />
            Deny
          </button>
        </div>
      ) : null}
      {response.state !== "approval_required" ? <span className="text-xs text-muted">{stateLabel(response.state)}</span> : null}
      <div className="grid gap-2">
        {response.events.map((event) => (
          <TerminalEventLine key={event.eventId} event={event} />
        ))}
      </div>
    </div>
  );
}

function TerminalEventLine({ event }: { event: TerminalCommandResponse["events"][number] }) {
  const output = [event.stdout, event.stderr].filter(Boolean).join("\n");
  return (
    <div>
      <div className="mb-1 flex flex-wrap items-center gap-2 text-xs text-muted">
        <span>{eventStatusLabel(event.status)}</span>
        {event.exitCode !== null ? <span>exit {event.exitCode}</span> : null}
        {event.stdoutTruncated ? <span>Truncated</span> : null}
        {event.redacted ? <span>Redacted</span> : null}
      </div>
      {output ? <pre className="whitespace-pre-wrap text-[13px] leading-6 text-ink">{output}</pre> : null}
    </div>
  );
}

function terminalOutputText(response: TerminalCommandResponse) {
  return response.events
    .flatMap((event) => [event.stdout, event.stderr])
    .filter(Boolean)
    .join("\n");
}

function stateLabel(state: TerminalCommandResponse["state"]) {
  if (state === "approval_required") return "Approval required";
  if (state === "denied") return "Denied";
  return "Completed";
}

function eventStatusLabel(status: TerminalCommandResponse["events"][number]["status"]) {
  if (status === "failed_to_spawn") return "Failed to start";
  if (status === "timed_out") return "Timed out";
  return "Completed";
}
