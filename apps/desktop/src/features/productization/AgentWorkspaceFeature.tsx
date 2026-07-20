import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useApiClient } from "../../api/ApiProvider";
import type { AgentSessionSnapshot, ApprovalMode, ApprovalResolveRequest, ApprovalSummary, ExternalAttachmentInput, SessionControlRequest } from "../../api/types";
import {
  firstAvailableLocalRoute,
  needsExternalEgressApproval,
  requestProviderEgressApproval,
  type PendingEgressApproval,
} from "./AgentEgressApproval";
import { AgentWorkspacePanel, AgentWorkspaceView } from "./AgentWorkspaceView";
import { cacheDrawerSession } from "./drawerSessionCache";
import { latestSessionSnapshot, shouldRefreshSession } from "./sessionFreshness";

type AgentWorkspaceFeatureProps = {
  workspaceId: string;
  workspaceName: string;
  selectedSession?: AgentSessionSnapshot | null;
  forceEmptyThread?: boolean;
  onSessionStarted?: (session: AgentSessionSnapshot) => void;
  onOpenChanges: () => void;
  onOpenApprovals: () => void;
  onOpenSetup: () => void;
};

export function AgentWorkspaceFeature({ workspaceId, selectedSession = null, forceEmptyThread = false, onSessionStarted, onOpenSetup }: AgentWorkspaceFeatureProps) {
  const api = useApiClient();
  const queryClient = useQueryClient();
  const workspace = useQuery({
    queryKey: ["agent-workspace", workspaceId],
    queryFn: () => api.agentWorkspace(workspaceId),
    enabled: workspaceId.length > 0,
    retry: 3,
    retryDelay: 250,
    staleTime: 2_000,
    refetchInterval: (query) => shouldRefreshSession(query.state.data?.session ?? null) ? 500 : false,
  });
  const approvalModes = useQuery({ queryKey: ["approval-modes"], queryFn: () => api.approvalModes(), retry: 3, retryDelay: 250, staleTime: 5_000 });
  const approvals = useQuery({ queryKey: ["approvals", workspaceId], queryFn: () => api.listApprovals(), enabled: workspaceId.length > 0, retry: 1, staleTime: 1_000 });
  const routeOptions = useQuery({ queryKey: ["route-options", workspaceId], queryFn: () => api.routeOptions(), retry: 3, retryDelay: 250, staleTime: 5_000 });
  const events = useQuery({
    queryKey: ["agent-events", workspaceId],
    queryFn: () => api.replayEvents(),
    enabled: workspaceId.length > 0,
    refetchInterval: (query) => shouldRefreshSession(workspace.data?.session ?? null) ? 500 : false,
  });
  const [prompt, setPrompt] = useState("");
  const [externalAttachments, setExternalAttachments] = useState<ExternalAttachmentInput[]>([]);
  const [pendingEgress, setPendingEgress] = useState<PendingEgressApproval | null>(null);
  const [resolvedThreadApprovalIds, setResolvedThreadApprovalIds] = useState<Set<string>>(() => new Set());
  const visibleSession = forceEmptyThread && !selectedSession ? null : latestSessionSnapshot(selectedSession, workspace.data?.session ?? null);
  const send = useMutation({
    mutationFn: ({ initialPrompt, attachments, approvalId }: { initialPrompt: string; attachments: ExternalAttachmentInput[]; approvalId?: string }) => {
      const executionBackendId = workspace.data?.route?.backendId ?? "";
      if (visibleSession) {
        return api.continueSession(visibleSession.sessionId, {
          workspaceId,
          executionBackendId,
          prompt: initialPrompt,
          ...(attachments.length > 0 ? { externalAttachments: attachments } : {}),
          ...(approvalId ? { approvalId } : {}),
        });
      }
      return api.createSession({
        workspaceId,
        executionBackendId,
        initialPrompt,
        ...(forceEmptyThread && !selectedSession ? { newChat: true } : {}),
        ...(attachments.length > 0 ? { externalAttachments: attachments } : {}),
        ...(approvalId ? { approvalId } : {}),
      });
    },
    onSuccess: (session) => {
      cacheDrawerSession(queryClient, workspaceId, session);
      onSessionStarted?.(session);
      void queryClient.invalidateQueries({ queryKey: ["agent-workspace", workspaceId] });
      void queryClient.refetchQueries({ queryKey: ["approvals", workspaceId], type: "active" });
      void queryClient.invalidateQueries({ queryKey: ["agent-events", workspaceId] });
      void queryClient.invalidateQueries({ queryKey: ["drawer-project-threads", workspaceId] });
    },
  });
  const startAgent = (initialPrompt: string) => {
    const trimmed = initialPrompt.trim();
    if (!trimmed || send.isPending) return;
    const attachments = externalAttachments;
    if (needsExternalEgressApproval(workspace.data?.route ?? null, routeOptions.data ?? null, attachments)) {
      void requestProviderEgressApproval(api, workspaceId, workspace.data?.route ?? null, trimmed, attachments)
        .then((approval) => {
          setPendingEgress({ approval, initialPrompt: trimmed, attachments });
          setPrompt(trimmed);
          setExternalAttachments(attachments);
        })
        .catch(() => {
          setPrompt(trimmed);
          setExternalAttachments(attachments);
        });
      return;
    }
    setPrompt("");
    setExternalAttachments([]);
    send.mutate({ initialPrompt: trimmed, attachments }, {
      onError: () => {
        setPrompt(trimmed);
        setExternalAttachments(attachments);
      },
    });
  };
  const approvePendingEgress = () => {
    if (!pendingEgress) return;
    const pending = pendingEgress;
    void api.resolveApproval(pending.approval.approvalId, { resolution: "approve" }).then(() => {
      setPendingEgress(null);
      setPrompt("");
      setExternalAttachments([]);
      send.mutate({ initialPrompt: pending.initialPrompt, attachments: pending.attachments, approvalId: pending.approval.approvalId }, {
        onError: () => {
          setPrompt(pending.initialPrompt);
          setExternalAttachments(pending.attachments);
        },
      });
    });
  };
  const denyPendingEgress = () => {
    if (!pendingEgress) return;
    const pending = pendingEgress;
    void api.resolveApproval(pending.approval.approvalId, { resolution: "deny" });
    setPendingEgress(null);
    setPrompt(pending.initialPrompt);
    setExternalAttachments(pending.attachments);
  };
  const returnLocalRoute = () => {
    const localRoute = firstAvailableLocalRoute(routeOptions.data ?? null);
    if (localRoute) updateRouteSelection.mutate(localRoute.routeId);
    if (pendingEgress) {
      setPrompt(pendingEgress.initialPrompt);
      setExternalAttachments(pendingEgress.attachments);
    }
    setPendingEgress(null);
  };
  const control = useMutation({
    mutationFn: ({ sessionId, action }: { sessionId: string; action: SessionControlRequest["action"] }) => api.sessionControl(sessionId, { action }),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["agent-workspace", workspaceId] }),
  });
  const updateApprovalMode = useMutation({
    mutationFn: (mode: ApprovalMode) => api.updateSessionApprovalMode({ mode }),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["approval-modes"] }),
  });
  const updateRouteSelection = useMutation({
    mutationFn: (routeId: string) => api.updateRouteSelection({ routeId }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["route-options", workspaceId] });
      void queryClient.invalidateQueries({ queryKey: ["agent-workspace", workspaceId] });
    },
  });
  const resolveThreadApproval = useMutation({
    mutationFn: async ({ approval, resolution }: { approval: ApprovalSummary; resolution: ApprovalResolveRequest["resolution"] }) => {
      return api.resolveApproval(approval.approvalId, { resolution });
    },
    onSuccess: (_resolved, { approval }) => {
      setResolvedThreadApprovalIds((current) => new Set(current).add(approval.approvalId));
      void queryClient.invalidateQueries({ queryKey: ["approvals"] });
      void queryClient.invalidateQueries({ queryKey: ["agent-workspace", workspaceId] });
      void queryClient.invalidateQueries({ queryKey: ["agent-events", workspaceId] });
      void queryClient.invalidateQueries({ queryKey: ["drawer-project-threads", workspaceId] });
    },
    onError: () => {
      void queryClient.invalidateQueries({ queryKey: ["approvals"] });
      void queryClient.invalidateQueries({ queryKey: ["agent-workspace", workspaceId] });
    },
  });

  if (workspace.isLoading) return <AgentWorkspacePanel title="Loading agent" body="DesktopLab is reading route and workspace context." />;
  if (workspace.isError || !workspace.data) return <AgentWorkspacePanel title="Agent unavailable" body="DesktopLab could not read the agent workspace right now." />;

  return (
    <AgentWorkspaceView
      snapshot={workspace.data}
      selectedSession={visibleSession}
      forceEmptyThread={forceEmptyThread}
      prompt={prompt}
      eventFrames={events.data ?? []}
      approvalModes={approvalModes.data ?? null}
      routeOptions={routeOptions.data ?? null}
      externalAttachments={externalAttachments}
      pendingThreadApprovals={pendingThreadApprovals(
        [...(visibleSession?.pendingApprovals ?? []), ...(approvals.data?.approvals ?? [])],
        visibleSession,
        resolvedThreadApprovalIds,
      )}
      pendingEgressApproval={Boolean(pendingEgress)}
      setPrompt={setPrompt}
      createPending={send.isPending}
      approvalModePending={updateApprovalMode.isPending}
      routeSelectionPending={updateRouteSelection.isPending}
      threadApprovalPending={resolveThreadApproval.isPending}
      threadApprovalError={resolveThreadApproval.isError}
      contextAttachmentPending={false}
      onStart={startAgent}
      onApproveEgress={approvePendingEgress}
      onDenyEgress={denyPendingEgress}
      onReturnLocalRoute={returnLocalRoute}
      onApprovalModeChange={(mode) => updateApprovalMode.mutate(mode)}
      onRouteSelectionChange={(routeId) => updateRouteSelection.mutate(routeId)}
      onExternalAttachmentsChange={setExternalAttachments}
      onResolveThreadApproval={(approval, resolution) => {
        resolveThreadApproval.reset();
        resolveThreadApproval.mutate({ approval, resolution });
      }}
      onControl={(sessionId, action) => control.mutate({ sessionId, action })}
      onOpenSetup={onOpenSetup}
    />
  );
}

function pendingThreadApprovals(approvals: ApprovalSummary[], session: AgentSessionSnapshot | null, hiddenApprovalIds: Set<string>): ApprovalSummary[] {
  if (!session) return [];
  const byId = new Map<string, ApprovalSummary>();
  for (const approval of approvals) {
    if (approval.state !== "pending" || approval.sessionId !== session.sessionId || hiddenApprovalIds.has(approval.approvalId)) continue;
    byId.set(approval.approvalId, approval);
  }
  return [...byId.values()];
}
