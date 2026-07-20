// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import { ProvidersFeature } from "./ProvidersFeature";
import type {
  ProviderBridgePairingPollResponse,
  ProviderBridgePairingStartResponse,
  ProviderConnectionResponse,
  ProvidersListResponse,
  RoutePreference,
} from "../../api/types";

test("renders provider accounts from backend data without making missing credentials look connected", async () => {
  renderProviders();

  expect(await screen.findByRole("heading", { name: "Accounts" })).toBeInTheDocument();
  expect(screen.getAllByText("OpenAI").length).toBeGreaterThan(0);
  expect(screen.getAllByText("Anthropic").length).toBeGreaterThan(0);
  expect(screen.getByText("Not connected")).toBeInTheDocument();
  expect(screen.getByText("Cloud access is locked by policy.")).toBeInTheDocument();
  expect(screen.getByText("Provider access needs approval before repository data leaves this machine.")).toBeInTheDocument();
  expect(screen.getByText("Connection health")).toBeInTheDocument();
  expect(screen.getByText("Credential: Stored in local vault")).toBeInTheDocument();
  expect(screen.getByText("Account mode: API key")).toBeInTheDocument();
  expect(screen.getByText("Fallback: Approval required before fallback")).toBeInTheDocument();
  expect(screen.getAllByText("Verified integration").length).toBeGreaterThan(0);
});

test("uses product account states and explains byo account modes", async () => {
  renderProviders({
    listProviders: vi.fn<() => Promise<ProvidersListResponse>>().mockResolvedValue({
      providers: [
        provider({ providerId: "provider.openai", displayName: "OpenAI", status: "connected", diagnostic: { state: "ready", message: "Connected", redactedEvidence: "ready" } }),
        provider({ providerId: "provider.anthropic", displayName: "Anthropic", status: "missing_credential", diagnostic: { state: "missing_credential", message: "Missing", redactedEvidence: "missing" } }),
        provider({ providerId: "provider.codex", displayName: "Codex", status: "degraded", diagnostic: { state: "degraded", message: "Approval needed", redactedEvidence: "approval" } }),
        provider({ providerId: "provider.custom", displayName: "Custom endpoint", status: "blocked", diagnostic: { state: "blocked", message: "Disabled by policy", redactedEvidence: "policy" } }),
      ],
    }),
  });

  expect(await screen.findByText("Ready")).toBeInTheDocument();
  expect(screen.getByText("Not connected")).toBeInTheDocument();
  expect(screen.getByText("Needs approval")).toBeInTheDocument();
  expect(screen.getByText("Disabled by policy")).toBeInTheDocument();
  expect(screen.getByText("Use your subscription login, browser sign-in, API key, local app session or custom endpoint when a provider supports it.")).toBeInTheDocument();
  expect(screen.getByText("Cloud accounts are optional. Local models stay the default route until you choose otherwise.")).toBeInTheDocument();
  expect(screen.getByText("Cloud models run only after you connect an account and approve the policy route.")).toBeInTheDocument();
});

test("translates provider capability identifiers into product labels", async () => {
  renderProviders({
    listProviders: vi.fn<() => Promise<ProvidersListResponse>>().mockResolvedValue({
      providers: [provider({ capabilities: ["llm.chat", "tool.filesystem.write"] })],
    }),
  });

  expect(await screen.findByText("Chat")).toBeInTheDocument();
  expect(screen.getByText("Write files")).toBeInTheDocument();
  expect(screen.queryByText("llm.chat")).not.toBeInTheDocument();
  expect(screen.queryByText("tool.filesystem.write")).not.toBeInTheDocument();
});

test("connects a provider through the vault flow and clears the raw key from the UI", async () => {
  const connectProvider = vi.fn().mockResolvedValue({
    providerId: "provider.openai",
    status: "connected",
    vaultRef: "vault://providers/openai/default",
    message: "Credential stored",
  } satisfies ProviderConnectionResponse);
  renderProviders({ connectProvider });

  await screen.findByRole("heading", { name: "Accounts" });
  fireEvent.change(screen.getByLabelText("Provider"), { target: { value: "provider.openai" } });
  fireEvent.change(screen.getByLabelText("Account mode"), { target: { value: "api_key_billing" } });
  fireEvent.change(screen.getByLabelText("API key"), { target: { value: "sk-test-secret" } });
  fireEvent.click(screen.getByRole("button", { name: "Connect account" }));

  await waitFor(() =>
    expect(connectProvider).toHaveBeenCalledWith({
      providerId: "provider.openai",
      accountMode: "api_key_billing",
      apiKey: "sk-test-secret",
    }),
  );
  expect(screen.queryByDisplayValue("sk-test-secret")).not.toBeInTheDocument();
  expect(await screen.findByText("Credential reference connected")).toBeInTheDocument();
});

test("blocked vault responses stay explicitly not stored", async () => {
  const connectProvider = vi.fn().mockResolvedValue({
    providerId: "provider.openai",
    status: "blocked",
    vaultRef: null,
    message: "Vault unavailable on this machine.",
  } satisfies ProviderConnectionResponse);
  renderProviders({ connectProvider });

  await screen.findByRole("heading", { name: "Accounts" });
  fireEvent.change(screen.getByLabelText("API key"), { target: { value: "sk-test-secret" } });
  fireEvent.click(screen.getByRole("button", { name: "Connect account" }));

  await waitFor(() => expect(connectProvider).toHaveBeenCalled());
  expect(await screen.findByText("Vault unavailable on this machine.")).toBeInTheDocument();
  expect(screen.getByText("Credential not stored")).toBeInTheDocument();
  expect(screen.queryByText("Credential reference connected")).not.toBeInTheDocument();
  expect(screen.queryByDisplayValue("sk-test-secret")).not.toBeInTheDocument();
});

test("checks native storage and removes provider credentials through backend routes", async () => {
  const connectProvider = vi.fn().mockResolvedValue({
    providerId: "provider.openai",
    status: "connected",
    vaultRef: "vault://providers/openai/default",
    message: "Credential reference stored.",
  } satisfies ProviderConnectionResponse);
  const testProviderCredential = vi.fn().mockResolvedValue({
    state: "degraded",
    message: "The credential is readable from the operating-system vault. Remote authentication was not attempted.",
    redactedEvidence: "credential=[REDACTED]; vault_read=verified; remote_call=not_run",
  });
  const removeProviderCredential = vi.fn().mockResolvedValue({
    providerId: "provider.openai",
    status: "removed",
    vaultRef: "vault://providers/openai/default",
    message: "Credential reference removed.",
  });
  renderProviders({ connectProvider, testProviderCredential, removeProviderCredential });

  await screen.findByRole("heading", { name: "Accounts" });
  fireEvent.change(screen.getByLabelText("API key"), { target: { value: "sk-test-secret" } });
  fireEvent.click(screen.getByRole("button", { name: "Connect account" }));
  fireEvent.click(await screen.findByRole("button", { name: "Check stored credential" }));
  expect(await screen.findByText("The credential is readable from the operating-system vault. Remote authentication was not attempted.")).toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: "Remove credential" }));
  expect(await screen.findByText("Credential reference removed.")).toBeInTheDocument();
  expect(testProviderCredential).toHaveBeenCalledWith({ providerId: "provider.openai", accountMode: "api_key_billing" });
  expect(removeProviderCredential).toHaveBeenCalledWith({ providerId: "provider.openai", accountMode: "api_key_billing" });
  expect(screen.queryByDisplayValue("sk-test-secret")).not.toBeInTheDocument();
});

test("openai subscription mode opens browser consent without exposing bridge internals", async () => {
  const startOpenAiCodexBridgePairing = vi.fn().mockResolvedValue({
    providerId: "provider.openai",
    accountMode: "subscription_account",
    status: "authorization_required",
    pairingId: "desktoplab_bridge_pair_001",
    pairingCode: "DL-ABC12345",
    authorizationUrl: "https://auth.openai.com/codex/device",
    redirectUri: "http://localhost:1455/auth/callback",
    tokenStorage: "vault_ref_only",
    completionPath: "/v1/provider-bridges/openai-codex/pairing/complete",
    deviceLogin: {
      deviceAuthId: "device_auth_test",
      userCode: "ABCD-EFGH",
      verificationUrl: "https://auth.openai.com/codex/device",
      intervalSeconds: 1,
    },
    message: "Sign in with OpenAI Codex.",
  } satisfies ProviderBridgePairingStartResponse);
  const pollOpenAiCodexBridgePairing = vi.fn().mockResolvedValue({
    providerId: "provider.openai",
    status: "authorization_pending",
    pairingId: "desktoplab_bridge_pair_001",
    message: "Waiting for OpenAI consent.",
  } satisfies ProviderBridgePairingPollResponse);
  const completeOpenAiCodexBridgePairing = vi.fn();
  const connectProvider = vi.fn();
  const openConsent = vi.spyOn(window, "open").mockImplementation(() => null);
  renderProviders({
    connectProvider,
    startOpenAiCodexBridgePairing,
    pollOpenAiCodexBridgePairing,
    completeOpenAiCodexBridgePairing,
  });

  await screen.findByRole("heading", { name: "Accounts" });
  fireEvent.change(screen.getByLabelText("Provider"), { target: { value: "provider.openai" } });
  fireEvent.change(screen.getByLabelText("Account mode"), { target: { value: "subscription_account" } });

  expect(screen.queryByLabelText("API key")).not.toBeInTheDocument();
  expect(screen.getByText("Connect with your OpenAI Codex subscription through a local bridge. DesktopLab stores only the connection reference returned by the backend.")).toBeInTheDocument();
  expect(screen.queryByLabelText("Local credential reference")).not.toBeInTheDocument();
  expect(screen.queryByLabelText("Bridge responder URL")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Connect OpenAI Codex" }));

  await waitFor(() =>
    expect(startOpenAiCodexBridgePairing).toHaveBeenCalledWith({
      accountMode: "subscription_account",
    }),
  );
  await waitFor(() =>
    expect(openConsent).toHaveBeenCalledWith(
      "https://auth.openai.com/codex/device",
      "_blank",
      "noopener,noreferrer",
    ),
  );
  expect(await screen.findByText("Pairing code: DL-ABC12345")).toBeInTheDocument();
  expect(screen.getByText("OpenAI user code: ABCD-EFGH")).toBeInTheDocument();
  await waitFor(() =>
    expect(pollOpenAiCodexBridgePairing).toHaveBeenCalledWith({
      pairingId: "desktoplab_bridge_pair_001",
    }),
  );
  expect(screen.getByText("Waiting for OpenAI consent. DesktopLab checks the local account connection automatically.")).toBeInTheDocument();
  expect(screen.queryByLabelText("Local credential reference")).not.toBeInTheDocument();
  expect(screen.queryByLabelText("Bridge responder URL")).not.toBeInTheDocument();
  expect(completeOpenAiCodexBridgePairing).not.toHaveBeenCalled();
  expect(connectProvider).not.toHaveBeenCalled();
  expect(screen.queryByDisplayValue("sk-test-secret")).not.toBeInTheDocument();
  openConsent.mockRestore();
});

test("custom endpoints validate syntax without claiming live provider readiness", async () => {
  const connectProvider = vi.fn().mockResolvedValue({
    providerId: "provider.openai",
    status: "blocked",
    vaultRef: null,
    accountMode: "custom_endpoint",
    message: "Endpoint syntax is valid, but DesktopLab has not certified health/model-listing execution for this endpoint yet.",
  } satisfies ProviderConnectionResponse);
  renderProviders({ connectProvider });

  await screen.findByRole("heading", { name: "Accounts" });
  fireEvent.change(screen.getByLabelText("Provider"), { target: { value: "provider.openai" } });
  fireEvent.change(screen.getByLabelText("Account mode"), { target: { value: "custom_endpoint" } });

  expect(screen.queryByLabelText("API key")).not.toBeInTheDocument();
  expect(screen.getByLabelText("Endpoint URL")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Check endpoint" })).toBeDisabled();

  fireEvent.change(screen.getByLabelText("Endpoint URL"), { target: { value: "http://127.0.0.1:11434/v1" } });
  fireEvent.click(screen.getByRole("button", { name: "Check endpoint" }));

  await waitFor(() =>
    expect(connectProvider).toHaveBeenCalledWith({
      providerId: "provider.openai",
      accountMode: "custom_endpoint",
      endpointUrl: "http://127.0.0.1:11434/v1",
      allowRemoteHttps: false,
    }),
  );
  expect(await screen.findByText("Endpoint syntax is valid, but DesktopLab has not certified health/model-listing execution for this endpoint yet.")).toBeInTheDocument();
});

function renderProviders(overrides: Partial<DesktopLabApiClient> = {}) {
  const client = {
    listProviders: vi.fn<() => Promise<ProvidersListResponse>>().mockResolvedValue({
      providers: [
        {
          providerId: "provider.openai",
          displayName: "OpenAI",
          status: "connected",
          trust: "verified",
          egress: "requires_approval",
          capabilities: ["Chat", "Code reasoning"],
          supportedAccountModes: ["api_key_billing", "subscription_account", "oauth_device", "local_app_session", "custom_endpoint"],
          activeAccountMode: "subscription_account",
          diagnostic: { state: "ready", message: "Connected", redactedEvidence: "Bearer [REDACTED]" },
          vaultRef: "vault://providers/openai/default",
          authProfileHealth: {
            authMode: "subscription_account",
            credentialReferenceKind: "vault_ref",
            credentialRef: "vault://providers/openai/default",
            lastHealthState: "probe_required",
            cooldownState: "not_probed",
            fallbackOrder: ["subscription_account", "api_key_billing"],
            fallbackApproval: "explicit_user_approval_required",
            degradedReason: "provider_probe_required",
          },
        },
        {
          providerId: "provider.anthropic",
          displayName: "Anthropic",
          status: "missing_credential",
          trust: "verified",
          egress: "requires_approval",
          capabilities: ["Chat"],
          supportedAccountModes: ["api_key_billing", "subscription_account"],
          diagnostic: { state: "missing_credential", message: "Credential missing", redactedEvidence: "Bearer [REDACTED]" },
        },
      ],
    }),
    routePreference: vi.fn<() => Promise<RoutePreference>>().mockResolvedValue({
      mode: "local_first",
      cloudAllowed: false,
      lockedByPolicy: true,
      explanation: "Provider access needs approval before repository data leaves this machine.",
      requiredCapabilities: ["Chat", "Tool use"],
      blockedReasons: ["Cloud access is locked by policy."],
    }),
    updateRoutePreference: vi.fn(),
    connectProvider: vi.fn(),
    ...overrides,
  } as unknown as DesktopLabApiClient;

  return render(
    <AppProviders apiClient={client}>
      <ProvidersFeature />
    </AppProviders>,
  );
}

function provider(overrides: Partial<ProvidersListResponse["providers"][number]>): ProvidersListResponse["providers"][number] {
  return {
    providerId: "provider.test",
    displayName: "Provider",
    status: "missing_credential",
    trust: "verified",
    egress: "requires_approval",
    capabilities: ["Chat"],
    supportedAccountModes: ["api_key_billing", "subscription_account", "oauth_device", "local_app_session", "custom_endpoint"],
    diagnostic: { state: "missing_credential", message: "Missing credential", redactedEvidence: "redacted" },
    authProfileHealth: {
      authMode: "api_key_billing",
      credentialReferenceKind: "none",
      lastHealthState: "missing_credential",
      cooldownState: "none",
      fallbackOrder: ["api_key_billing"],
      fallbackApproval: "explicit_user_approval_required",
      degradedReason: "credential_reference_missing",
    },
    ...overrides,
  };
}
