import { useLayoutEffect, useRef } from "react";
import type { BackendEventFrame } from "../../api/events";
import type {
  AgentSessionSnapshot,
  AgentWorkspaceSnapshot,
  ApprovalMode,
  ApprovalModesResponse,
  ApprovalResolveRequest,
  ApprovalSummary,
  ExecutionRouteOptionsResponse,
  ExternalAttachmentInput,
  SessionControlRequest,
} from "../../api/types";
import { Composer, sendDisabledReason } from "./AgentComposer";
import { ConversationTranscript } from "./AgentConversation";
import { RouteBlockedMessage } from "./RouteBlockedMessage";
import { EmptyWorkbenchActions } from "./AgentWorkbenchPanels";
import { ThreadApprovalPrompt } from "./ThreadApprovalPrompt";

export function AgentWorkspaceView({
  snapshot,
  selectedSession,
  forceEmptyThread,
  prompt,
  eventFrames,
  approvalModes,
  routeOptions,
  externalAttachments,
  pendingThreadApprovals,
  pendingEgressApproval,
  setPrompt,
  createPending,
  approvalModePending,
  routeSelectionPending,
  threadApprovalPending,
  threadApprovalError,
  contextAttachmentPending,
  onStart,
  onApproveEgress,
  onDenyEgress,
  onReturnLocalRoute,
  onApprovalModeChange,
  onRouteSelectionChange,
  onExternalAttachmentsChange,
  onResolveThreadApproval,
  onControl,
  onOpenSetup,
}: AgentWorkspaceViewProps) {
  const route = snapshot.route;
  const visibleSession = selectedSession ?? (forceEmptyThread ? null : snapshot.session);
  const sessionWorking = isAgentWorking(visibleSession);
  const working = createPending || sessionWorking || threadApprovalPending;
  const canStart = Boolean(route?.status === "selected" && route.backendId && prompt.trim().length > 0 && !working);
  const disabledReason = sendDisabledReason(route, prompt, working);
  const conversationRegionRef = useRef<HTMLDivElement>(null);
  const followConversationTailRef = useRef(true);
  const revision = conversationRevision(visibleSession, pendingThreadApprovals.length);
  useLayoutEffect(() => {
    const region = conversationRegionRef.current;
    if (region && followConversationTailRef.current) region.scrollTop = region.scrollHeight;
  }, [revision]);
  return (
    <section data-testid="agent-thread-surface" data-ui-route="agent" data-ui-state={agentUiState(visibleSession, pendingThreadApprovals.length)} className="relative mx-auto flex h-full min-h-0 w-full max-w-3xl flex-col">
      <h1 className="sr-only">Agent</h1>
      <h2 className="sr-only">Conversation</h2>
      <div
        ref={conversationRegionRef}
        data-testid="agent-conversation-scroll-region"
        className="min-h-0 flex-1 overflow-auto px-1 pb-8 pt-2"
        onScroll={(event) => {
          const region = event.currentTarget;
          followConversationTailRef.current = region.scrollHeight - region.scrollTop - region.clientHeight <= 80;
        }}
      >
        {route?.status === "blocked" ? <RouteBlockedMessage route={route} onOpenSetup={onOpenSetup} /> : null}
        {visibleSession ? <ConversationTranscript session={visibleSession} eventFrames={eventFrames} /> : <EmptyWorkbenchActions repositoryReady={Boolean(snapshot.context)} onChoose={setPrompt} />}
      </div>
      <ThreadApprovalPrompt approvals={pendingThreadApprovals} resolving={threadApprovalPending} failed={threadApprovalError} onResolve={onResolveThreadApproval} />
      {working ? (
        <div role="status" aria-live="polite" className="mb-2 flex items-center gap-2 px-2 text-xs font-medium text-muted">
          <span className="h-1.5 w-1.5 rounded-full bg-accent dl-running-dot" />
          <span>Agent is working</span>
        </div>
      ) : null}
      <Composer
        prompt={prompt}
        setPrompt={setPrompt}
        disabled={!canStart}
        disabledReason={disabledReason}
        working={working}
        route={route}
        session={visibleSession}
        approvalModes={approvalModes?.modes ?? []}
        approvalMode={approvalModes?.sessionMode ?? null}
        approvalModeDisabled={approvalModePending || !approvalModes}
        routeOptions={routeOptions}
        routeSelectionDisabled={routeSelectionPending || !routeOptions}
        externalAttachments={externalAttachments}
        pendingEgressApproval={pendingEgressApproval}
        contextAttachmentDisabled={contextAttachmentPending}
        onStart={onStart}
        onApproveEgress={onApproveEgress}
        onDenyEgress={onDenyEgress}
        onReturnLocalRoute={onReturnLocalRoute}
        onApprovalModeChange={onApprovalModeChange}
        onRouteSelectionChange={onRouteSelectionChange}
        onExternalAttachmentsChange={onExternalAttachmentsChange}
        onControl={onControl}
      />
    </section>
  );
}

function agentUiState(session: AgentSessionSnapshot | null, pendingApprovalCount: number) {
  if (pendingApprovalCount > 0) return "approval";
  if (!session) return "idle";
  if (session.state === "failed" || session.state === "cancelled") return "failure";
  if (session.state === "created" || session.state === "planning" || session.state === "running" || session.state === "paused") return "running";
  if (session.state === "blocked") return "blocked";
  return "completion";
}

function conversationRevision(session: AgentSessionSnapshot | null, pendingApprovalCount: number) {
  if (!session) return `empty:${pendingApprovalCount}`;
  const latestTranscript = session.transcript?.at(-1);
  const latestTimeline = session.timeline.at(-1);
  return [
    session.sessionId,
    session.state,
    session.summary,
    session.transcript?.length ?? 0,
    latestTranscript?.sequence ?? 0,
    latestTranscript?.content ?? "",
    session.timeline.length,
    latestTimeline?.sequence ?? 0,
    latestTimeline?.message ?? "",
    pendingApprovalCount,
  ].join(":");
}

export function AgentWorkspacePanel({ title, body }: { title: string; body: string }) {
  return (
    <section className="mx-auto max-w-3xl rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <h1 className="text-xl font-semibold">{title}</h1>
      <p className="mt-2 text-sm text-muted">{body}</p>
    </section>
  );
}

type AgentWorkspaceViewProps = {
  snapshot: AgentWorkspaceSnapshot;
  selectedSession: AgentSessionSnapshot | null;
  forceEmptyThread: boolean;
  prompt: string;
  eventFrames: BackendEventFrame[];
  approvalModes: ApprovalModesResponse | null;
  routeOptions: ExecutionRouteOptionsResponse | null;
  externalAttachments: ExternalAttachmentInput[];
  pendingThreadApprovals: ApprovalSummary[];
  pendingEgressApproval: boolean;
  setPrompt: (value: string) => void;
  createPending: boolean;
  approvalModePending: boolean;
  routeSelectionPending: boolean;
  threadApprovalPending: boolean;
  threadApprovalError: boolean;
  contextAttachmentPending: boolean;
  onStart: (prompt: string) => void;
  onApproveEgress: () => void;
  onDenyEgress: () => void;
  onReturnLocalRoute: () => void;
  onApprovalModeChange: (mode: ApprovalMode) => void;
  onRouteSelectionChange: (routeId: string) => void;
  onExternalAttachmentsChange: (attachments: ExternalAttachmentInput[]) => void;
  onResolveThreadApproval: (approval: ApprovalSummary, resolution: ApprovalResolveRequest["resolution"]) => void;
  onControl: (sessionId: string, action: SessionControlRequest["action"]) => void;
  onOpenSetup: () => void;
};

function isAgentWorking(session: AgentSessionSnapshot | null): boolean {
  return Boolean(session && (session.state === "created" || session.state === "planning" || session.state === "running"));
}
