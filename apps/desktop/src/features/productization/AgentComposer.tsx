import { Send, Square } from "../../design/icons";
import type { KeyboardEvent } from "react";
import type { AgentRouteDecision, AgentSessionSnapshot, ApprovalMode, ApprovalModeDescriptor, ExecutionRouteOptionsResponse, ExternalAttachmentInput, SessionControlRequest } from "../../api/types";
import { ApprovalModeMenu } from "./ApprovalModeMenu";
import { ExternalAttachmentButton } from "./AttachContextMenu";
import { RouteModelMenu } from "./RouteModelMenu";

type AgentComposerProps = {
  prompt: string;
  setPrompt: (value: string) => void;
  disabled: boolean;
  disabledReason: string | null;
  working: boolean;
  route: AgentRouteDecision | null;
  session: AgentSessionSnapshot | null;
  approvalModes: ApprovalModeDescriptor[];
  approvalMode: ApprovalMode | null;
  approvalModeDisabled: boolean;
  routeOptions: ExecutionRouteOptionsResponse | null;
  routeSelectionDisabled: boolean;
  externalAttachments: ExternalAttachmentInput[];
  pendingEgressApproval: boolean;
  contextAttachmentDisabled: boolean;
  onStart: (prompt: string) => void;
  onApproveEgress: () => void;
  onDenyEgress: () => void;
  onReturnLocalRoute: () => void;
  onApprovalModeChange: (mode: ApprovalMode) => void;
  onRouteSelectionChange: (routeId: string) => void;
  onExternalAttachmentsChange: (attachments: ExternalAttachmentInput[]) => void;
  onControl: (sessionId: string, action: SessionControlRequest["action"]) => void;
};

export function Composer(props: AgentComposerProps) {
  const submitPrompt = (value: string) => {
    if (!props.working && !props.disabled) props.onStart(value);
  };
  const submitOnEnter = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.defaultPrevented) return;
    if (isSubmitKey(event) && !event.shiftKey) {
      event.preventDefault();
      submitPrompt(event.currentTarget.value);
    }
  };
  const routeLabel = selectedRouteLabel(props.route);
  const stoppableSession = props.session && isStoppableSession(props.session) ? props.session : null;
  const activeSessionCanStop = Boolean(stoppableSession && stoppableSession.controls?.cancel !== false);
  const buttonLabel = props.working ? (stoppableSession ? "Stop agent" : "Working") : "Send prompt";
  const sendTitle = props.working ? (stoppableSession ? "Stop agent" : "DesktopLab is starting this run.") : props.disabledReason ?? "Send prompt";
  const sendDisabled = props.working ? !activeSessionCanStop : props.disabled;
  return (
    <form
      data-testid="agent-composer"
      className="z-10 shrink-0 rounded-[22px] border border-line bg-panel/95 p-3 shadow-[0_18px_48px_rgba(15,23,42,0.14)] backdrop-blur transition-[border-color,box-shadow] duration-150 focus-within:border-accent/45 focus-within:shadow-[0_18px_48px_rgba(15,23,42,0.16)]"
      onSubmit={(event) => {
        event.preventDefault();
        submitPrompt(props.prompt);
      }}
    >
      <label className="grid gap-1 text-sm font-medium text-ink">
        <span className="sr-only">Prompt</span>
        <textarea
          className="min-h-24 resize-none rounded-[18px] border-0 bg-transparent px-2 py-2 text-sm leading-6 text-ink caret-ink outline-none ring-accent/20 placeholder:text-muted/70 focus:ring-0"
          placeholder="Ask DesktopLab to work on this repository"
          value={props.prompt}
          onChange={(event) => props.setPrompt(event.target.value)}
          onKeyDown={submitOnEnter}
        />
      </label>
      {props.pendingEgressApproval ? (
        <div role="group" aria-label="External route approval" className="mb-2 rounded-[14px] border border-line bg-elevated px-3 py-2 text-xs text-muted">
          <p className="font-medium text-ink">Send attached context to the external route?</p>
          <p className="mt-1 leading-5">Approve only if this file can leave this computer for the selected bridge.</p>
          <div className="mt-2 flex flex-wrap gap-2">
            <button type="button" className="rounded-full border border-line px-3 py-1.5 font-medium text-ink" onClick={props.onReturnLocalRoute}>
              Use local
            </button>
            <button type="button" className="rounded-full border border-line px-3 py-1.5 font-medium text-ink" onClick={props.onDenyEgress}>
              Deny
            </button>
            <button type="button" className="rounded-full bg-ink px-3 py-1.5 font-medium text-canvas" onClick={props.onApproveEgress}>
              Approve
            </button>
          </div>
        </div>
      ) : null}
      <div className="flex flex-wrap items-center justify-between gap-2 border-t border-line/70 px-1 pt-3">
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <ExternalAttachmentButton attachments={props.externalAttachments} disabled={props.contextAttachmentDisabled} onChange={props.onExternalAttachmentsChange} />
          <ApprovalModeMenu modes={props.approvalModes} selectedMode={props.approvalMode} disabled={props.approvalModeDisabled} onSelect={props.onApprovalModeChange} />
          <RouteModelMenu label={routeLabel} options={props.routeOptions} disabled={props.routeSelectionDisabled} onSelect={props.onRouteSelectionChange} />
        </div>
        {props.disabledReason && !props.working ? <p id="composer-send-disabled-reason" className="sr-only">{props.disabledReason}</p> : null}
        <button
          type={props.working ? "button" : "submit"}
          aria-label={buttonLabel}
          aria-describedby={props.disabledReason && !props.working ? "composer-send-disabled-reason" : undefined}
          title={sendTitle}
          className="inline-flex h-8 w-8 items-center justify-center rounded-full bg-ink text-xs font-medium text-canvas transition-[transform,background-color,opacity] duration-150 hover:bg-accent active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel disabled:opacity-60 disabled:hover:bg-ink disabled:active:scale-100"
          disabled={sendDisabled}
          onClick={() => {
            if (props.working) {
              if (stoppableSession) props.onControl(stoppableSession.sessionId, "cancel");
            }
          }}
        >
          <span className="sr-only">{buttonLabel}</span>
          {props.working ? <Square size={12} fill="currentColor" /> : <Send size={14} />}
        </button>
      </div>
    </form>
  );
}

export function sendDisabledReason(route: AgentRouteDecision | null, prompt: string, working: boolean): string | null {
  if (working) return "DesktopLab is already working.";
  if (route?.status === "blocked") return "Finish setup before sending a prompt.";
  if (!route || route.status !== "selected" || !route.backendId) return "Choose a model before sending a prompt.";
  if (prompt.trim().length === 0) return "Enter a prompt to send.";
  return null;
}

function isSubmitKey(event: KeyboardEvent<HTMLTextAreaElement>) {
  return event.key === "Enter" || event.key === "Return" || event.code === "Enter" || event.code === "NumpadEnter" || event.keyCode === 13;
}

function isStoppableSession(session: AgentSessionSnapshot): boolean {
  return session.state === "created" || session.state === "planning" || session.state === "running";
}

function selectedRouteLabel(route: AgentRouteDecision | null): string {
  if (!route) return "Choose a model";
  if (route.status === "blocked") return route.nextActionLabel ?? "Finish setup";
  return route.modelDisplayName ?? "No model selected";
}
