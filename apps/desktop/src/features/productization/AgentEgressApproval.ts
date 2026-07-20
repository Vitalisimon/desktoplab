import type { DesktopLabApiClient } from "../../api/client";
import type {
  AgentRouteDecision,
  ApprovalCreateResponse,
  ExecutionRouteOption,
  ExecutionRouteOptionsResponse,
  ExternalAttachmentInput,
} from "../../api/types";

export type PendingEgressApproval = {
  approval: ApprovalCreateResponse;
  initialPrompt: string;
  attachments: ExternalAttachmentInput[];
};

export function needsExternalEgressApproval(
  route: AgentRouteDecision | null,
  routeOptions: ExecutionRouteOptionsResponse | null,
  attachments: ExternalAttachmentInput[],
): boolean {
  if (attachments.length === 0) return false;
  if (route?.status !== "selected" || route.backendKind !== "external" || !route.backendId) return false;
  if (route.repositoryContextEgress === "approval_required" || route.egressPolicy === "requires_approval") {
    return true;
  }
  const selected = routeOptions?.options.find((option) => option.backendId === route.backendId && option.backendKind === "external");
  return selected?.repositoryContextEgress === "approval_required" || selected?.egressPolicy === "requires_approval";
}

export function firstAvailableLocalRoute(routeOptions: ExecutionRouteOptionsResponse | null): ExecutionRouteOption | null {
  return routeOptions?.options.find((option) => option.backendKind === "local" && option.status === "available") ?? null;
}

export function requestProviderEgressApproval(
  api: DesktopLabApiClient,
  workspaceId: string,
  route: AgentRouteDecision | null,
  initialPrompt: string,
  attachments: ExternalAttachmentInput[],
): Promise<ApprovalCreateResponse> {
  const backendId = route?.backendId ?? "";
  return api.createApproval({
    sessionId: "session.pending",
    action: "provider.egress",
    operationId: `provider.openai:route.external.codex:${workspaceId}`,
    payload: {
      providerId: "provider.openai",
      routeId: "route.external.codex",
      backendId,
      workspaceId,
      initialPrompt,
      contextPaths: [],
      externalAttachments: attachmentMetadata(attachments),
    },
  });
}

function attachmentMetadata(attachments: ExternalAttachmentInput[]) {
  return attachments.map((attachment) => ({
    name: attachment.name,
    size: attachment.size,
    mediaType: attachment.mediaType,
    contentAttached: attachment.contentText !== undefined,
    contentSha256: attachment.contentSha256,
    truncated: Boolean(attachment.truncated),
  }));
}
