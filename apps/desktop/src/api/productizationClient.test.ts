import { DesktopLabApiClient } from "./client";
import type { ApiTransport, TransportRequest } from "./transport";

test("maps productization account runtime model and agent methods to local api paths", async () => {
  const requests: TransportRequest[] = [];
  const client = new DesktopLabApiClient({
    authToken: "local-test-token",
    transport: transportFor(requests),
  });

  await client.listProviders();
  await client.connectProvider({ providerId: "provider.openai", accountMode: "api_key_billing", apiKey: "sk-test-secret" });
  await client.startOpenAiCodexBridgePairing({ accountMode: "subscription_account" });
  await client.pollOpenAiCodexBridgePairing({ pairingId: "desktoplab_bridge_pair_001" });
  await client.completeOpenAiCodexBridgePairing({
    pairingId: "desktoplab_bridge_pair_001",
    pairingCode: "DL-ABC12345",
    bridgeInstanceId: "desktoplab-local",
    providerAccountLabel: "OpenAI Codex",
    localCredentialRef: "vault://desktoplab/external-backend/openai-codex/profile/simone",
    responderUrl: "http://127.0.0.1:43109",
  });
  await client.testProviderCredential({ providerId: "provider.openai", accountMode: "api_key_billing" });
  await client.removeProviderCredential({ providerId: "provider.openai", accountMode: "api_key_billing" });
  await client.providerDiagnostics("provider.openai");
  await client.routePreference();
  await client.updateRoutePreference({ mode: "cloud_optional" });
  await client.routeOptions();
  await client.updateRouteSelection({ routeId: "route.local.qwen-coder-7b" });
  await client.listRuntimes();
  await client.runtimeInspect();
  const runtimeInstall = await client.startRuntimeInstall({ runtimeId: "runtime.ollama" });
  await client.listModels();
  const modelDownload = await client.startModelDownload({ modelId: "model.qwen3-coder", runtimeId: "runtime.ollama" });
  await client.catalogRefreshStatus();
  await client.startCatalogRefresh();
  await client.agentWorkspace("workspace.desktoplab");
  await client.contextAttachments("workspace.desktoplab");
  await client.sessionControl("session.1", { action: "pause" });
  await client.localAuditTransparency();
  await client.securityAudit();

  expect(requests.map((request) => `${request.method} ${request.path}`)).toEqual([
    "GET /v1/providers",
    "POST /v1/providers/provider.openai/connect",
    "POST /v1/provider-bridges/openai-codex/pairing/start",
    "POST /v1/provider-bridges/openai-codex/pairing/poll",
    "POST /v1/provider-bridges/openai-codex/pairing/complete",
    "POST /v1/providers/provider.openai/test",
    "POST /v1/providers/provider.openai/disconnect",
    "GET /v1/providers/provider.openai/diagnostics",
    "GET /v1/routing/preference",
    "POST /v1/routing/preference",
    "GET /v1/routing/options",
    "POST /v1/routing/options/selection",
    "GET /v1/runtimes",
    "GET /v1/runtime/inspect",
    "POST /v1/runtimes/runtime.ollama/install",
    "GET /v1/models",
    "POST /v1/models/model.qwen3-coder/download",
    "GET /v1/setup/catalog-refresh",
    "POST /v1/setup/catalog-refresh",
    "GET /v1/agent/workspace?workspace_id=workspace.desktoplab",
    "GET /v1/workspaces/workspace.desktoplab/context-attachments",
    "POST /v1/sessions/session.1/control",
    "GET /v1/audit/local",
    "GET /v1/security/audit",
  ]);
  expect(requests[1].body).toEqual({ accountMode: "api_key_billing", apiKey: "sk-test-secret" });
  expect(requests[2].body).toEqual({ accountMode: "subscription_account" });
  expect(requests[3].body).toEqual({ pairingId: "desktoplab_bridge_pair_001" });
  expect(requests[4].body).toEqual({
    pairingId: "desktoplab_bridge_pair_001",
    pairingCode: "DL-ABC12345",
    bridgeInstanceId: "desktoplab-local",
    providerAccountLabel: "OpenAI Codex",
    localCredentialRef: "vault://desktoplab/external-backend/openai-codex/profile/simone",
    responderUrl: "http://127.0.0.1:43109",
  });
  expect(requests[5].body).toEqual({ accountMode: "api_key_billing" });
  expect(requests[6].body).toEqual({ accountMode: "api_key_billing" });
  expect(requests[9].body).toEqual({ mode: "cloud_optional" });
  expect(requests[11].body).toEqual({ routeId: "route.local.qwen-coder-7b" });
  expect(runtimeInstall).toEqual({
    jobId: "job.runtime.install",
    runtimeId: "runtime.ollama",
    state: "downloading",
    verificationState: "pending",
    retryClass: "retryable",
  });
  expect(requests[16].body).toEqual({ runtimeId: "runtime.ollama" });
  expect(modelDownload).toEqual({
    jobId: "job.model.download",
    modelId: "model.qwen3-coder",
    familyId: "family.qwen",
    variantId: "model.qwen3-coder",
    runtimeId: "runtime.ollama",
    state: "downloading",
    retryClass: "retryable",
  });
  expect(requests[21].body).toEqual({ action: "pause" });
});

function transportFor(requests: TransportRequest[]): ApiTransport {
  return {
    async request(request) {
      requests.push(request);
      return { status: 200, body: responseFor(request.path) };
    },
  };
}

function responseFor(path: string) {
  if (path === "/v1/providers") return { providers: [] };
  if (path === "/v1/provider-bridges/openai-codex/pairing/start") return { providerId: "provider.openai", accountMode: "subscription_account", status: "authorization_required", pairingId: "desktoplab_bridge_pair_001", pairingCode: "DL-ABC12345", authorizationUrl: "https://auth.openai.com/oauth/authorize", redirectUri: "http://localhost:1455/auth/callback", tokenStorage: "vault_ref_only", completionPath: "/v1/provider-bridges/openai-codex/pairing/complete" };
  if (path === "/v1/provider-bridges/openai-codex/pairing/poll") return { providerId: "provider.openai", accountMode: "subscription_account", status: "connected", vaultRef: "vault://desktoplab/external-backend/openai-codex/desktoplab_bridge_pair_001", bridgeResponderUrl: null };
  if (path === "/v1/provider-bridges/openai-codex/pairing/complete") return { providerId: "provider.openai", accountMode: "subscription_account", status: "connected", vaultRef: "vault://desktoplab/external-backend/openai-codex/profile/simone", bridgeResponderUrl: "http://127.0.0.1:43109" };
  if (path.endsWith("/connect")) return { providerId: "provider.openai", status: "connected", vaultRef: "vault://provider/openai" };
  if (path.endsWith("/test")) return { providerId: "provider.openai", state: "blocked", redactedEvidence: "credential=[REDACTED]" };
  if (path.endsWith("/disconnect")) return { providerId: "provider.openai", status: "removed", vaultRef: "vault://provider/openai" };
  if (path.endsWith("/diagnostics")) return { providerId: "provider.openai", state: "ready", redactedEvidence: "Bearer [REDACTED]" };
  if (path === "/v1/security/audit") return { source: "service_backed", kind: "security_audit", redacted: true, exportSafe: true, summary: { state: "blocked", blocked: 1, degraded: 0, ready: 0 }, findings: [], remediationPolicy: "safe_remediation_routes_through_doctor_repair_contract" };
  if (path === "/v1/routing/preference") return { mode: "local_first", cloudAllowed: false, lockedByPolicy: true, explanation: "Local first" };
  if (path === "/v1/routing/options" || path === "/v1/routing/options/selection") return { selectedRouteId: "route.local.qwen-coder-7b", options: [] };
  if (path === "/v1/runtimes") return { runtimes: [] };
  if (path === "/v1/runtime/inspect") return { source: "service_backed", inspectState: "ready", active: { selectedRouteId: "route.local.qwen-coder-7b", backendId: "backend.ollama", accountMode: "local_runtime", egress: "local_or_approval_gated", toolCapability: "filesystem_write_requires_approval" }, evidence: { coldManifest: { source: "route_selection" }, liveRuntime: { state: "verified" } } };
  if (path.endsWith("/install")) return { jobId: "job.runtime.install", runtimeId: "runtime.ollama", state: "downloading", verificationState: "pending", retryClass: "retryable" };
  if (path === "/v1/models") return { models: [] };
  if (path.endsWith("/download")) {
    return {
      jobId: "job.model.download",
      modelId: "model.qwen3-coder",
      familyId: "family.qwen",
      variantId: "model.qwen3-coder",
      runtimeId: "runtime.ollama",
      state: "downloading",
      retryClass: "retryable",
    };
  }
  if (path === "/v1/setup/catalog-refresh") return { state: "ready", lastKnownGoodAvailable: false, degradedReasons: [], manualRefresh: { available: true, jobId: "registry.refresh.manual" } };
  if (path.startsWith("/v1/agent/workspace?")) return { route: null, context: null, session: null };
  if (path.endsWith("/context-attachments")) return { workspaceId: "workspace.desktoplab", attachments: [] };
  if (path.endsWith("/control")) return { accepted: true };
  if (path === "/v1/audit/local") return { scope: "local_single_user", records: [], redactedExport: "" };
  return {};
}
