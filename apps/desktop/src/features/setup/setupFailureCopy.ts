const setupFailureMessages: Record<string, string> = {
  runtime_not_ready: "Install the local runner first, then download the model.",
  runtime_and_model_not_verified: "Verify the local runner and model before opening a repository.",
  runtime_not_verified: "Verify the local runner before downloading this model.",
  runtime_install_failed: "The local runner install did not finish. Start setup again.",
  backend_readiness_not_verified: "Verify the local runner and model before opening a repository.",
  requires_admin_action: "Finish the guided installer approval, then continue setup.",
  offline: "Reconnect to the internet, then try setup again.",
  network_unavailable: "Reconnect to the internet, then try setup again.",
  "network unavailable": "Reconnect to the internet, then try setup again.",
  insufficient_disk: "Free up disk space, then try setup again.",
  "insufficient disk": "Free up disk space, then try setup again.",
  unsupported_runtime: "Choose a compatible local runner for this model.",
  "unsupported runtime": "Choose a compatible local runner for this model.",
  unsafe_model_reference: "This model reference is not safe to run.",
  "unsafe model reference": "This model reference is not safe to run.",
  model_not_reported_by_runtime: "The local runner does not report this model yet.",
  resume_unsupported: "Start this model download again from the beginning.",
  "resume unsupported": "Start this model download again from the beginning.",
  non_retryable: "DesktopLab cannot continue this step automatically.",
  user_action: "DesktopLab needs an action from you before continuing.",
  setup_plan_not_accepted: "Confirm the setup plan before installing local tools.",
  "setup plan not accepted": "Confirm the setup plan before installing local tools.",
  missing_verification_metadata: "DesktopLab needs verified installer metadata before continuing.",
  "missing verification metadata": "DesktopLab needs verified installer metadata before continuing.",
};

export function setupFailureCopy(reason: string | null | undefined): string | undefined {
  const value = reason?.trim();
  if (!value) return undefined;
  const normalized = value.toLowerCase();
  if (setupFailureMessages[normalized]) return setupFailureMessages[normalized];
  if (looksLikeInternalCode(value)) return "DesktopLab needs one more verified setup step before continuing.";
  return value;
}

function looksLikeInternalCode(value: string): boolean {
  return /^[a-z0-9_.:-]+$/.test(value) && (value.includes("_") || value.includes("."));
}
