import type { RepairAction, TrustLevel } from "./coreTypes";

export type AccountMode = "api_key_billing" | "subscription_account" | "oauth_device" | "local_app_session" | "custom_endpoint";

export type ProviderDiagnostic = {
  state: "ready" | "missing_credential" | "degraded" | "blocked";
  message: string;
  redactedEvidence: string;
  repairActions?: RepairAction[];
};

export type ProviderAccount = {
  providerId: string;
  displayName: string;
  status: "connected" | "missing_credential" | "degraded" | "blocked";
  trust: TrustLevel;
  egress: "local_only" | "requires_approval" | "allowed";
  capabilities: string[];
  supportedAccountModes?: AccountMode[];
  activeAccountMode?: AccountMode;
  diagnostic: ProviderDiagnostic;
  vaultRef?: string;
  authProfileHealth?: {
    authMode: AccountMode | string;
    credentialReferenceKind: "vault_ref" | "none" | string;
    credentialRef?: string | null;
    lastHealthState: string;
    cooldownState: string;
    fallbackOrder: string[];
    fallbackApproval: string;
    degradedReason?: string | null;
  };
};

export type ProvidersListResponse = {
  providers: ProviderAccount[];
};

export type ProviderConnectionRequest = {
  providerId: string;
  accountMode: AccountMode;
  apiKey?: string;
  endpointUrl?: string;
  allowRemoteHttps?: boolean;
};

export type ProviderConnectionResponse = {
  providerId: string;
  status: "connected" | "degraded" | "blocked";
  vaultRef: string | null;
  accountMode?: AccountMode;
  bridgeResponderUrl?: string | null;
  message?: string;
};

export type ProviderBridgePairingStartRequest = {
  accountMode: Extract<AccountMode, "subscription_account" | "oauth_device" | "local_app_session">;
};

export type ProviderBridgePairingStartResponse = {
  providerId: string;
  accountMode: AccountMode;
  status: "authorization_required";
  pairingId: string;
  pairingCode: string;
  authorizationUrl: string;
  redirectUri: string;
  tokenStorage: "vault_ref_only";
  completionPath: string;
  pollPath?: string;
  deviceLogin?: {
    deviceAuthId: string;
    userCode: string;
    verificationUrl: string;
    intervalSeconds: number;
  };
  message?: string;
};

export type ProviderBridgePairingPollRequest = {
  pairingId: string;
};

export type ProviderBridgePairingPollResponse =
  | {
      providerId: string;
      status: "authorization_pending";
      pairingId: string;
      message?: string;
    }
  | (ProviderConnectionResponse & {
      status: "connected";
      bridgeResponderUrl?: string | null;
    });

export type ProviderBridgePairingCompleteRequest = {
  pairingId: string;
  pairingCode: string;
  bridgeInstanceId: string;
  providerAccountLabel: string;
  localCredentialRef: string;
  responderUrl: string;
};

export type ProviderBridgePairingCompleteResponse = ProviderConnectionResponse & {
  status: "connected";
  bridgeResponderUrl: string;
};

export type ProviderCredentialTestRequest = {
  providerId: string;
  accountMode: AccountMode;
};

export type ProviderCredentialRemovalRequest = {
  providerId: string;
  accountMode: AccountMode;
};

export type ProviderCredentialRemovalResponse = {
  providerId: string;
  status: "removed";
  vaultRef: string;
  accountMode?: AccountMode;
  message?: string;
};

export type RoutePreferenceMode = "local_only" | "local_first" | "cloud_optional";

export type RoutePreference = {
  mode: RoutePreferenceMode;
  status?: "selected" | "blocked";
  backendId?: string | null;
  cloudAllowed?: boolean;
  lockedByPolicy?: boolean;
  explanation: string;
  requiredCapabilities?: string[];
  blockedReasons?: string[];
};

export type RoutePreferenceUpdateRequest = {
  mode: RoutePreferenceMode;
};

export type ExecutionRouteOption = {
  routeId: string;
  backendId: string;
  backendKind: "local" | "cloud" | "external" | "custom";
  label: string;
  modelId?: string;
  runtimeId?: string;
  executionBackendId?: string;
  modelDisplayName: string;
  runtimeDisplayName: string;
  status: "available" | "unavailable";
  disabledReason?: string;
  egressPolicy?: "requires_approval" | string;
  repositoryContextEgress?: "approval_required" | string;
};

export type ExecutionRouteOptionsResponse = {
  selectedRouteId: string;
  options: ExecutionRouteOption[];
};

export type ExecutionRouteSelectionRequest = {
  routeId: string;
};
