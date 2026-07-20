import {
  type AgentWorkspaceSnapshot, type AppStateResponse,
  type ApprovalCreateRequest, type ApprovalCreateResponse,
  type ApprovalModeUpdateRequest, type ApprovalModesResponse,
  type ApprovalResolveRequest, type ApprovalResolveResponse,
  type ApprovalsListResponse,
  type CatalogRefreshRequestResponse, type CatalogRefreshStatusResponse,
  type CommitSessionRequest, type CommitSessionResponse,
  type ContextAttachmentsResponse, type DeleteMemoryResponse,
  type DiagnosticRepairRunResponse, type DiagnosticsExportBundle, type DiagnosticsSnapshot,
  type ExternalBackendRouteResolveRequest, type ExternalBackendRouteResolveResponse,
  type ExternalBackendsResponse, type ExecutionRouteOptionsResponse,
  type ExecutionRouteSelectionRequest, type GitOperationsSnapshot,
  type HealthResponse, type JobRetryResponse, type JobsListResponse,
  type HighEndRuntimeAttachRequest, type HighEndRuntimeDiscoveryRequest,
  type HighEndRuntimeDiscoveryResponse, type HighEndRuntimeHealthResponse,
  type LocalAuditTransparencySnapshot,
  type ModelDownloadRequest, type ModelDownloadResponse, type ModelsListResponse,
  type PluginTrustRequest, type PluginTrustResponse, type PluginsListResponse,
  type PushBranchRequest, type PushBranchResponse,
  type ProviderBridgePairingCompleteRequest, type ProviderBridgePairingCompleteResponse,
  type ProviderBridgePairingPollRequest, type ProviderBridgePairingPollResponse,
  type ProviderBridgePairingStartRequest, type ProviderBridgePairingStartResponse,
  type ProviderConnectionRequest, type ProviderConnectionResponse,
  type ProviderCredentialRemovalRequest, type ProviderCredentialRemovalResponse,
  type ProviderCredentialTestRequest, type ProviderDiagnostic, type ProvidersListResponse,
  type ReadinessResponse, type RoutePreference, type RoutePreferenceUpdateRequest,
  type RuntimeInspectSnapshot, type RuntimeInstallRequest, type RuntimeInstallResponse, type SecurityAuditSnapshot,
  type RuntimesListResponse, type RollbackSavePointRequest, type RollbackSavePointResponse,
  type AgentSessionSnapshot,
  type SessionArchiveResponse,
  type SessionCreateRequest,
  type SessionContinueRequest,
  type SessionContextPreview,
  type SessionControlRequest,
  type SessionControlResponse,
  type SessionsListResponse,
  type SetupAcceptanceResponse,
  type SetupAcceptanceRequest,
  type SetupPlanPreview,
  type TerminalCommandRequest,
  type TerminalCommandResponse,
  type VersionResponse,
  type WorkspaceFilePreviewResponse,
  type WorkspaceFileTreeResponse,
  type WorktreeCleanupResponse,
  type WorkspaceIntelligenceSnapshot,
  type WorkspaceMemoryResponse,
  type WorkspaceOpenRequest,
  type WorkspaceRelinkRequest,
  type WorkspaceArchiveResponse,
  type WorkspaceScanRefreshResponse,
  type WorkspaceSnapshot,
} from "./types";
import { BackendEventClient, type BackendEventFrame } from "./events";
import { ApiRequester } from "./requester";
import type { ApiTransport } from "./transport";
import { DesktopLabAgentClient } from "@desktoplab/client-sdk";

export type DesktopLabApiClientOptions = {
  authToken: string;
  transport: ApiTransport;
};

export class DesktopLabApiClient {
  private readonly agentClient: DesktopLabAgentClient;
  private readonly eventClient: BackendEventClient;
  private readonly requester: ApiRequester;

  constructor(options: DesktopLabApiClientOptions) {
    this.agentClient = new DesktopLabAgentClient(options);
    this.eventClient = new BackendEventClient(options.transport, options.authToken);
    this.requester = new ApiRequester(options.transport, options.authToken);
  }

  health(): Promise<HealthResponse> { return this.get("/health"); }

  readiness(): Promise<ReadinessResponse> { return this.get("/v1/readiness"); }

  version(): Promise<VersionResponse> { return this.get("/v1/version"); }

  appState(): Promise<AppStateResponse> { return this.get("/v1/app/state"); }

  setupPreview(): Promise<SetupPlanPreview> { return this.get("/v1/setup/preview"); }

  acceptSetupPlan(request?: SetupAcceptanceRequest): Promise<SetupAcceptanceResponse> {
    return this.request("POST", "/v1/setup/accept", request);
  }

  catalogRefreshStatus(): Promise<CatalogRefreshStatusResponse> { return this.get("/v1/setup/catalog-refresh"); }

  startCatalogRefresh(): Promise<CatalogRefreshRequestResponse> {
    return this.request("POST", "/v1/setup/catalog-refresh");
  }

  listProviders(): Promise<ProvidersListResponse> {
    return this.get("/v1/providers");
  }

  connectProvider(request: ProviderConnectionRequest): Promise<ProviderConnectionResponse> {
    const { providerId, apiKey, ...baseBody } = request;
    const body = apiKey ? { ...baseBody, apiKey } : baseBody;
    return this.request("POST", `/v1/providers/${encodeURIComponent(providerId)}/connect`, body);
  }

  startOpenAiCodexBridgePairing(request: ProviderBridgePairingStartRequest): Promise<ProviderBridgePairingStartResponse> { return this.request("POST", "/v1/provider-bridges/openai-codex/pairing/start", request); }

  pollOpenAiCodexBridgePairing(request: ProviderBridgePairingPollRequest): Promise<ProviderBridgePairingPollResponse> { return this.request("POST", "/v1/provider-bridges/openai-codex/pairing/poll", request); }

  completeOpenAiCodexBridgePairing(request: ProviderBridgePairingCompleteRequest): Promise<ProviderBridgePairingCompleteResponse> { return this.request("POST", "/v1/provider-bridges/openai-codex/pairing/complete", request); }

  testProviderCredential(request: ProviderCredentialTestRequest): Promise<ProviderDiagnostic> {
    const { providerId, ...body } = request;
    return this.request("POST", `/v1/providers/${encodeURIComponent(providerId)}/test`, body);
  }

  removeProviderCredential(request: ProviderCredentialRemovalRequest): Promise<ProviderCredentialRemovalResponse> {
    const { providerId, ...body } = request;
    return this.request("POST", `/v1/providers/${encodeURIComponent(providerId)}/disconnect`, body);
  }

  providerDiagnostics(providerId: string): Promise<ProviderDiagnostic> {
    return this.get(`/v1/providers/${encodeURIComponent(providerId)}/diagnostics`);
  }

  localAuditTransparency(): Promise<LocalAuditTransparencySnapshot> {
    return this.get("/v1/audit/local");
  }

  routePreference(): Promise<RoutePreference> {
    return this.get("/v1/routing/preference");
  }

  updateRoutePreference(request: RoutePreferenceUpdateRequest): Promise<RoutePreference> {
    return this.request("POST", "/v1/routing/preference", request);
  }

  routeOptions(): Promise<ExecutionRouteOptionsResponse> {
    return this.get("/v1/routing/options");
  }

  updateRouteSelection(request: ExecutionRouteSelectionRequest): Promise<ExecutionRouteOptionsResponse> {
    return this.request("POST", "/v1/routing/options/selection", request);
  }

  listRuntimes(): Promise<RuntimesListResponse> {
    return this.get("/v1/runtimes");
  }

  runtimeInspect(): Promise<RuntimeInspectSnapshot> {
    return this.get("/v1/runtime/inspect");
  }

  discoverHighEndRuntime(request: HighEndRuntimeDiscoveryRequest): Promise<HighEndRuntimeDiscoveryResponse> { return this.request("POST", "/v1/runtimes/high-end/discover", request); }

  attachHighEndRuntime(request: HighEndRuntimeAttachRequest): Promise<HighEndRuntimeHealthResponse> { return this.request("POST", "/v1/runtimes/high-end/attach", request); }

  inspectHighEndRuntime(): Promise<HighEndRuntimeHealthResponse> { return this.get("/v1/runtimes/high-end/inspect"); }

  startRuntimeInstall(request: RuntimeInstallRequest): Promise<RuntimeInstallResponse> {
    return this.request("POST", `/v1/runtimes/${encodeURIComponent(request.runtimeId)}/install`, {
      setupChoice: request.setupChoice,
    });
  }

  listModels(): Promise<ModelsListResponse> {
    return this.get("/v1/models");
  }

  startModelDownload(request: ModelDownloadRequest): Promise<ModelDownloadResponse> {
    return this.request("POST", `/v1/models/${encodeURIComponent(request.modelId)}/download`, {
      runtimeId: request.runtimeId,
      setupChoice: request.setupChoice,
    });
  }

  agentWorkspace(workspaceId: string): Promise<AgentWorkspaceSnapshot> {
    return this.get(`/v1/agent/workspace?workspace_id=${encodeURIComponent(workspaceId)}`);
  }

  gitOperations(workspaceId: string): Promise<GitOperationsSnapshot> {
    return this.get(`/v1/git/operations?workspace_id=${encodeURIComponent(workspaceId)}`);
  }

  rollbackSavePoint(savePointId: string, request: RollbackSavePointRequest): Promise<RollbackSavePointResponse> {
    return this.request("POST", `/v1/git/savepoints/${encodeURIComponent(savePointId)}/rollback`, request);
  }

  commitSession(request: CommitSessionRequest): Promise<CommitSessionResponse> {
    return this.request("POST", "/v1/git/commit", request);
  }

  pushBranch(request: PushBranchRequest): Promise<PushBranchResponse> {
    return this.request("POST", "/v1/git/push", request);
  }

  cleanupWorktree(worktreeId: string): Promise<WorktreeCleanupResponse> {
    return this.request("POST", `/v1/git/worktrees/${encodeURIComponent(worktreeId)}/cleanup`);
  }

  workspaceIntelligence(workspaceId: string): Promise<WorkspaceIntelligenceSnapshot> {
    return this.get(`/v1/workspaces/${encodeURIComponent(workspaceId)}/intelligence`);
  }

  refreshWorkspaceScan(workspaceId: string): Promise<WorkspaceScanRefreshResponse> {
    return this.request("POST", `/v1/workspaces/${encodeURIComponent(workspaceId)}/intelligence/refresh`);
  }

  listWorkspaceMemory(workspaceId: string): Promise<WorkspaceMemoryResponse> {
    return this.get(`/v1/workspaces/${encodeURIComponent(workspaceId)}/memory`);
  }

  deleteMemory(memoryId: string): Promise<DeleteMemoryResponse> {
    return this.request("POST", `/v1/workspaces/memory/${encodeURIComponent(memoryId)}/delete`);
  }

  sessionContextPreview(workspaceId: string): Promise<SessionContextPreview> {
    return this.get(`/v1/workspaces/${encodeURIComponent(workspaceId)}/context-preview`);
  }

  contextAttachments(workspaceId: string): Promise<ContextAttachmentsResponse> {
    return this.get(`/v1/workspaces/${encodeURIComponent(workspaceId)}/context-attachments`);
  }

  listPlugins(): Promise<PluginsListResponse> {
    return this.get("/v1/plugins");
  }

  trustPlugin(pluginId: string, request: PluginTrustRequest): Promise<PluginTrustResponse> {
    return this.request("POST", `/v1/plugins/${encodeURIComponent(pluginId)}/trust`, request);
  }

  listExternalBackends(): Promise<ExternalBackendsResponse> {
    return this.get("/v1/external-backends");
  }

  approveExternalBackendRoute(
    routeId: string,
    request: ExternalBackendRouteResolveRequest,
  ): Promise<ExternalBackendRouteResolveResponse> {
    return this.request("POST", `/v1/external-backends/routes/${encodeURIComponent(routeId)}/resolve`, request);
  }

  diagnostics(): Promise<DiagnosticsSnapshot> {
    return this.get("/v1/diagnostics");
  }

  diagnosticsExport(): Promise<DiagnosticsExportBundle> {
    return this.get("/v1/diagnostics/export");
  }

  securityAudit(): Promise<SecurityAuditSnapshot> {
    return this.get("/v1/security/audit");
  }

  runDiagnosticRepair(repairId: string): Promise<DiagnosticRepairRunResponse> {
    return this.request("POST", `/v1/diagnostics/repairs/${encodeURIComponent(repairId)}/run`);
  }

  openWorkspace(request: WorkspaceOpenRequest): Promise<WorkspaceSnapshot> {
    return this.request("POST", "/v1/workspaces/open", request);
  }

  relinkWorkspace(workspaceId: string, request: WorkspaceRelinkRequest): Promise<WorkspaceSnapshot> {
    return this.request("POST", `/v1/workspaces/${encodeURIComponent(workspaceId)}/relink`, request);
  }

  archiveWorkspace(workspaceId: string): Promise<WorkspaceArchiveResponse> {
    return this.request("POST", `/v1/workspaces/${encodeURIComponent(workspaceId)}/archive`);
  }

  listWorkspaceFiles(workspaceId: string): Promise<WorkspaceFileTreeResponse> {
    return this.get(`/v1/workspaces/${encodeURIComponent(workspaceId)}/files`);
  }

  previewWorkspaceFile(workspaceId: string, path: string): Promise<WorkspaceFilePreviewResponse> {
    return this.get(
      `/v1/workspaces/${encodeURIComponent(workspaceId)}/files/preview?path=${encodeURIComponent(path)}`,
    );
  }

  createTerminalCommand(
    workspaceId: string,
    request: TerminalCommandRequest,
  ): Promise<TerminalCommandResponse> {
    return this.request(
      "POST",
      `/v1/workspaces/${encodeURIComponent(workspaceId)}/terminal/commands`,
      request,
    );
  }

  listJobs(): Promise<JobsListResponse> {
    return this.get("/v1/jobs");
  }

  replayEvents(): Promise<BackendEventFrame[]> {
    return this.eventClient.replay();
  }

  retryJob(jobId: string): Promise<JobRetryResponse> {
    return this.request("POST", `/v1/jobs/${encodeURIComponent(jobId)}/retry`);
  }

  listSessions(workspaceId: string): Promise<SessionsListResponse> {
    return this.get(`/v1/sessions?workspace_id=${encodeURIComponent(workspaceId)}`);
  }

  createSession(request: SessionCreateRequest): Promise<AgentSessionSnapshot> {
    return this.agentClient.run({
      workspaceId: request.workspaceId,
      executionBackendId: request.executionBackendId,
      prompt: request.initialPrompt,
      contextPaths: request.contextPaths,
      externalAttachments: request.externalAttachments,
      approvalId: request.approvalId,
      newChat: request.newChat,
    }).then((result) => result.session as AgentSessionSnapshot);
  }

  continueSession(sessionId: string, request: SessionContinueRequest): Promise<AgentSessionSnapshot> {
    return this.agentClient.run({
      sessionId,
      workspaceId: request.workspaceId,
      executionBackendId: request.executionBackendId,
      prompt: request.prompt,
      contextPaths: request.contextPaths,
      externalAttachments: request.externalAttachments,
      approvalId: request.approvalId,
    }).then((result) => result.session as AgentSessionSnapshot);
  }

  archiveSession(sessionId: string): Promise<SessionArchiveResponse> {
    return this.request("POST", `/v1/sessions/${encodeURIComponent(sessionId)}/archive`);
  }

  sessionControl(sessionId: string, request: SessionControlRequest): Promise<SessionControlResponse> {
    if (request.action === "cancel") {
      return this.agentClient.cancel(sessionId).then((result) => result.session as SessionControlResponse);
    }
    return this.request("POST", `/v1/sessions/${encodeURIComponent(sessionId)}/control`, request);
  }

  listApprovals(): Promise<ApprovalsListResponse> {
    return this.get("/v1/approvals");
  }

  createApproval(request: ApprovalCreateRequest): Promise<ApprovalCreateResponse> {
    return this.request("POST", "/v1/approvals", request);
  }

  resolveApproval(approvalId: string, request: ApprovalResolveRequest): Promise<ApprovalResolveResponse> {
    return this.request("POST", `/v1/approvals/${encodeURIComponent(approvalId)}/resolve`, request);
  }

  approvalModes(): Promise<ApprovalModesResponse> {
    return this.get("/v1/approval-modes");
  }

  updateDefaultApprovalMode(request: ApprovalModeUpdateRequest): Promise<ApprovalModesResponse> {
    return this.request("POST", "/v1/approval-modes/default", request);
  }

  updateSessionApprovalMode(request: ApprovalModeUpdateRequest): Promise<ApprovalModesResponse> {
    return this.request("POST", "/v1/approval-modes/session", request);
  }

  private async get<T>(path: string): Promise<T> {
    return this.requester.get<T>(path);
  }

  private request<T>(method: "GET" | "POST", path: string, body?: unknown): Promise<T> {
    return this.requester.request<T>(method, path, body);
  }
}
