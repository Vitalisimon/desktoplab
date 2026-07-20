import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useApiClient } from "../../api/ApiProvider";
import type {
  AccountMode,
  ProviderAccount,
  ProviderBridgePairingStartResponse,
  ProviderConnectionRequest,
} from "../../api/types";
import { openExternalUrl } from "../../design/externalUrlOpen";
import {
  displayProviderAccountModeOption,
  displayProviderAuthMode,
  displayProviderCredentialReference,
  displayProviderFallbackApproval,
  providerByoAccountModesCopy,
} from "../../domain/displayNames";

export function ProviderConnectPanel({ providers }: { providers: ProviderAccount[] }) {
  const api = useApiClient();
  const queryClient = useQueryClient();
  const [selectedProvider, setSelectedProvider] = useState("");
  const [selectedAccountMode, setSelectedAccountMode] = useState<AccountMode>("api_key_billing");
  const [apiKey, setApiKey] = useState("");
  const [endpointUrl, setEndpointUrl] = useState("");
  const [allowRemoteHttps, setAllowRemoteHttps] = useState(false);
  const [vaultRef, setVaultRef] = useState<string | null>(null);
  const [credentialStatus, setCredentialStatus] = useState<"connected" | "not_stored" | null>(null);
  const [diagnostic, setDiagnostic] = useState<string | null>(null);
  const [bridgePairing, setBridgePairing] = useState<ProviderBridgePairingStartResponse | null>(null);
  const connect = useMutation({
    mutationFn: (request: ProviderConnectionRequest) => api.connectProvider(request),
    onSuccess: (response) => {
      setApiKey("");
      setVaultRef(response.vaultRef ?? null);
      setCredentialStatus(response.vaultRef ? "connected" : "not_stored");
      setDiagnostic(response.message ?? "Credential reference stored.");
      void queryClient.invalidateQueries({ queryKey: ["providers"] });
    },
  });
  const pollCodexBridge = useMutation({
    mutationFn: (request: { pairingId: string }) => api.pollOpenAiCodexBridgePairing(request),
    onSuccess: (response) => {
      setDiagnostic(response.message ?? "Waiting for OpenAI consent.");
      if (response.status !== "connected") return;
      setVaultRef(response.vaultRef ?? null);
      setCredentialStatus(response.vaultRef ? "connected" : "not_stored");
      setBridgePairing(null);
      void queryClient.invalidateQueries({ queryKey: ["providers"] });
      void queryClient.invalidateQueries({ queryKey: ["routing-options"] });
      void queryClient.invalidateQueries({ queryKey: ["route-options"] });
    },
  });
  const startCodexBridge = useMutation({
    mutationFn: () => api.startOpenAiCodexBridgePairing({ accountMode: accountMode as "subscription_account" | "oauth_device" | "local_app_session" }),
    onSuccess: (response) => {
      setBridgePairing(response);
      openConsentUrl(response.authorizationUrl);
      setDiagnostic("OpenAI consent opened in your browser.");
      pollCodexBridge.mutate({ pairingId: response.pairingId });
    },
  });
  const testCredential = useMutation({
    mutationFn: () => api.testProviderCredential({ providerId: providerValue, accountMode }),
    onSuccess: (response) => setDiagnostic(response.message),
  });
  const removeCredential = useMutation({
    mutationFn: () => api.removeProviderCredential({ providerId: providerValue, accountMode }),
    onSuccess: (response) => {
      setVaultRef(null);
      setCredentialStatus("not_stored");
      setDiagnostic(response.message ?? "Credential reference removed.");
      void queryClient.invalidateQueries({ queryKey: ["providers"] });
    },
  });
  const firstProvider = providers[0]?.providerId ?? "";
  const providerValue = selectedProvider || firstProvider;
  const selectedProviderAccount = providers.find((provider) => provider.providerId === providerValue);
  const supportedAccountModes = accountModesFor(selectedProviderAccount);
  const accountMode = supportedAccountModes.includes(selectedAccountMode) ? selectedAccountMode : supportedAccountModes[0];
  const requiresApiKey = accountMode === "api_key_billing";
  const customEndpointMode = accountMode === "custom_endpoint";
  const codexBridgeMode = providerValue === "provider.openai" && isCodexBridgeAccountMode(accountMode);
  const executableMode = requiresApiKey || customEndpointMode || codexBridgeMode;
  const connectLabel = codexBridgeMode ? "Connect OpenAI Codex" : customEndpointMode ? "Check endpoint" : "Connect account";

  useEffect(() => {
    if (!bridgePairing) return;
    const intervalSeconds = Math.max(bridgePairing.deviceLogin?.intervalSeconds ?? 5, 1);
    const pollInterval = window.setInterval(() => {
      if (!pollCodexBridge.isPending) {
        pollCodexBridge.mutate({ pairingId: bridgePairing.pairingId });
      }
    }, intervalSeconds * 1000);
    return () => window.clearInterval(pollInterval);
  }, [bridgePairing, pollCodexBridge]);

  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <h2 className="text-lg font-semibold">Connect account</h2>
      <p className="mt-2 text-sm leading-6 text-muted">{providerByoAccountModesCopy}</p>
      <div className="mt-3 grid gap-3">
        <label className="grid gap-1 text-sm font-medium text-ink">
          Provider
          <select
            className="h-10 rounded-desktop border border-line bg-panel px-3 text-sm text-ink"
            value={providerValue}
            onChange={(event) => {
              const nextProvider = providers.find((provider) => provider.providerId === event.target.value);
              setSelectedProvider(event.target.value);
              setSelectedAccountMode(accountModesFor(nextProvider)[0]);
              setApiKey("");
              setEndpointUrl("");
              setAllowRemoteHttps(false);
              setCredentialStatus(null);
              resetBridgeState(setBridgePairing);
            }}
          >
            {providers.map((provider) => (
              <option key={provider.providerId} value={provider.providerId}>
                {provider.displayName}
              </option>
            ))}
          </select>
        </label>
        <label className="grid gap-1 text-sm font-medium text-ink">
          Account mode
          <select
            className="h-10 rounded-desktop border border-line bg-panel px-3 text-sm text-ink"
            value={accountMode}
            onChange={(event) => {
              setSelectedAccountMode(event.target.value as AccountMode);
              setApiKey("");
              setEndpointUrl("");
              setAllowRemoteHttps(false);
              setCredentialStatus(null);
              resetBridgeState(setBridgePairing);
            }}
          >
            {supportedAccountModes.map((mode) => (
              <option key={mode} value={mode}>
                {accountModeLabel(mode)}
              </option>
            ))}
          </select>
        </label>
        {requiresApiKey ? (
          <label className="grid gap-1 text-sm font-medium text-ink">
            API key
            <input
              className="h-10 rounded-desktop border border-line bg-panel px-3 text-sm text-ink"
              type="password"
              value={apiKey}
              onChange={(event) => setApiKey(event.target.value)}
            />
          </label>
        ) : null}
        {customEndpointMode ? (
          <div className="grid gap-2">
            <label className="grid gap-1 text-sm font-medium text-ink">
              Endpoint URL
              <input
                className="h-10 rounded-desktop border border-line bg-panel px-3 text-sm text-ink"
                placeholder="http://127.0.0.1:11434/v1"
                type="url"
                value={endpointUrl}
                onChange={(event) => setEndpointUrl(event.target.value)}
              />
            </label>
            <label className="flex items-center gap-2 text-sm text-muted">
              <input
                className="h-4 w-4 rounded border-line"
                type="checkbox"
                checked={allowRemoteHttps}
                onChange={(event) => setAllowRemoteHttps(event.target.checked)}
              />
              Allow remote HTTPS endpoint
            </label>
            <p className="text-sm text-muted">DesktopLab checks the endpoint shape only. Health and model listing stay blocked until a real probe is certified.</p>
          </div>
        ) : null}
        {codexBridgeMode ? (
          <div className="grid gap-2 rounded-desktop border border-line bg-elevated p-3">
            <p className="text-sm text-muted">Connect with your OpenAI Codex subscription through a local bridge. DesktopLab stores only the connection reference returned by the backend.</p>
            {bridgePairing ? (
              <div className="grid gap-2">
                <p className="text-sm font-medium text-ink">Pairing code: {bridgePairing.pairingCode}</p>
                {bridgePairing.deviceLogin?.userCode ? (
                  <p className="text-sm font-medium text-ink">OpenAI user code: {bridgePairing.deviceLogin.userCode}</p>
                ) : null}
                <p className="text-sm text-muted">Waiting for OpenAI consent. DesktopLab checks the local account connection automatically.</p>
                <button
                  type="button"
                  className="rounded-desktop border border-line px-3 py-2 text-sm font-medium text-ink"
                  onClick={() => openConsentUrl(bridgePairing.authorizationUrl)}
                >
                  Open OpenAI consent again
                </button>
              </div>
            ) : null}
          </div>
        ) : null}
        <button
          type="button"
          className="rounded-desktop bg-ink px-3 py-2 text-sm font-medium text-canvas disabled:opacity-45"
          disabled={
            connect.isPending ||
            startCodexBridge.isPending ||
            providerValue.length === 0 ||
            !executableMode ||
            (requiresApiKey && apiKey.trim().length === 0) ||
            (customEndpointMode && endpointUrl.trim().length === 0)
          }
          onClick={() => {
            if (codexBridgeMode) {
              startCodexBridge.mutate();
              return;
            }
            connect.mutate(connectionRequest(providerValue, accountMode, apiKey, endpointUrl, allowRemoteHttps));
          }}
        >
          {connectLabel}
        </button>
        {!executableMode ? (
          <p className="text-sm text-muted">{blockedAccountModeCopy(accountMode)}</p>
        ) : null}
        {credentialStatus === "not_stored" ? (
          <div className="rounded-desktop border border-line bg-elevated p-3">
            <p className="text-sm font-medium text-muted">Credential not stored</p>
            <p className="mt-1 text-sm text-muted">DesktopLab did not keep this credential because the backend did not return a verified vault reference.</p>
          </div>
        ) : null}
        {selectedProviderAccount?.authProfileHealth ? (
          <div className="grid gap-1 rounded-desktop border border-line bg-elevated p-3 text-sm">
            <p className="font-medium text-ink">Connection health</p>
            <p className="text-muted">Account mode: {displayProviderAuthMode(selectedProviderAccount.authProfileHealth.authMode)}</p>
            <p className="text-muted">Credential: {displayProviderCredentialReference(selectedProviderAccount.authProfileHealth.credentialReferenceKind)}</p>
            <p className="text-muted">Fallback: {displayProviderFallbackApproval(selectedProviderAccount.authProfileHealth.fallbackApproval)}</p>
          </div>
        ) : null}
        {vaultRef ? (
          <div className="grid gap-2 rounded-desktop border border-line bg-elevated p-3">
            <p className="text-sm font-medium text-success">Credential reference connected</p>
            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                className="rounded-desktop border border-line px-3 py-2 text-sm font-medium text-ink disabled:opacity-45"
                disabled={testCredential.isPending}
                onClick={() => testCredential.mutate()}
              >
                Check stored credential
              </button>
              <button
                type="button"
                className="rounded-desktop border border-line px-3 py-2 text-sm font-medium text-danger disabled:opacity-45"
                disabled={removeCredential.isPending}
                onClick={() => removeCredential.mutate()}
              >
                Remove credential
              </button>
            </div>
          </div>
        ) : null}
        {diagnostic ? <p className="text-sm text-muted">{diagnostic}</p> : null}
      </div>
    </section>
  );
}

export function accountModesFor(provider: ProviderAccount | undefined): AccountMode[] {
  if (provider?.supportedAccountModes?.length) return provider.supportedAccountModes;
  if (provider?.activeAccountMode && provider.activeAccountMode !== "api_key_billing") {
    return ["api_key_billing", provider.activeAccountMode];
  }
  return ["api_key_billing"];
}

export function accountModeLabel(mode: AccountMode): string {
  return displayProviderAccountModeOption(mode);
}

function connectionRequest(providerId: string, accountMode: AccountMode, apiKey: string, endpointUrl: string, allowRemoteHttps: boolean): ProviderConnectionRequest {
  const request: ProviderConnectionRequest = { providerId, accountMode };
  if (accountMode === "api_key_billing") request.apiKey = apiKey;
  if (accountMode === "custom_endpoint") {
    request.endpointUrl = endpointUrl.trim();
    request.allowRemoteHttps = allowRemoteHttps;
  }
  return request;
}

function blockedAccountModeCopy(mode: AccountMode): string {
  if (mode === "subscription_account") {
    return "Subscription login is not executable until DesktopLab has a certified account bridge.";
  }
  if (mode === "oauth_device") {
    return "Browser or device login is not executable until DesktopLab can verify account ownership through a certified bridge.";
  }
  if (mode === "local_app_session") {
    return "Local app sessions stay blocked until DesktopLab can verify the local app account and capability boundary.";
  }
  return "This connection mode is visible now, but setup is unavailable until DesktopLab can verify ownership and capabilities.";
}

function isCodexBridgeAccountMode(mode: AccountMode): mode is "subscription_account" | "oauth_device" | "local_app_session" {
  return mode === "subscription_account" || mode === "oauth_device" || mode === "local_app_session";
}

function resetBridgeState(setBridgePairing: (value: ProviderBridgePairingStartResponse | null) => void) {
  setBridgePairing(null);
}

function openConsentUrl(url: string) {
  void openExternalUrl(url);
}
