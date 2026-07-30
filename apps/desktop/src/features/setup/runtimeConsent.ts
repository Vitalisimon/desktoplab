import type { RuntimeInstallRequest, SetupChoice } from "../../api/types";

export type RuntimeConsentRequirement = {
  label: string;
  detail: string;
};

export function runtimeConsentRequirement(
  runtimeId?: string,
  setupChoice?: SetupChoice,
): RuntimeConsentRequirement | null {
  if (runtimeId === "runtime.lm-studio" && setupChoice !== "use_existing") {
    return {
      label: "I accept the LM Studio vendor terms for this managed Preview setup.",
      detail: "DesktopLab downloads the pinned official llmster artifact and keeps it in an isolated local runtime directory.",
    };
  }
  if (runtimeId === "runtime.mlx-lm") {
    return {
      label: "I accept the Apache-2.0 model license for this managed Preview setup.",
      detail: "DesktopLab installs the pinned MLX-LM environment and exact SmolLM3 model revision on Apple Silicon.",
    };
  }
  return null;
}

export function runtimeConsentSatisfied(
  runtimeId: string | undefined,
  accepted: boolean,
  setupChoice?: SetupChoice,
): boolean {
  return runtimeConsentRequirement(runtimeId, setupChoice) === null || accepted;
}

export function runtimeConsentRequest(
  runtimeId: string,
  accepted: boolean,
  setupChoice?: SetupChoice,
): Pick<RuntimeInstallRequest, "vendorTermsAccepted" | "modelLicenseAccepted"> {
  if (runtimeId === "runtime.lm-studio" && setupChoice !== "use_existing") {
    return { vendorTermsAccepted: accepted };
  }
  if (runtimeId === "runtime.mlx-lm") return { modelLicenseAccepted: accepted };
  return {};
}
