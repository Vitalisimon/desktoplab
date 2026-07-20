import type { SetupPlanPreview, SetupRecommendation } from "../../api/types";

export function isHardwareHiddenReason(reason: string): boolean {
  return reason.includes("hidden_hardware") || reason.includes("workstation") || reason.includes("hardware");
}

export function friendlyParameterClass(parameterClass: NonNullable<SetupRecommendation["parameterClass"]>): string {
  if (parameterClass === "small") return "Small";
  if (parameterClass === "medium") return "Medium";
  if (parameterClass === "large") return "Large";
  if (parameterClass === "cloud") return "Cloud";
  return "Workstation";
}

export function friendlyRuntimeId(runtimeId: string): string {
  if (runtimeId === "runtime.ollama") return "Ollama";
  if (runtimeId === "runtime.ollama-cloud") return "Ollama Cloud";
  if (runtimeId === "runtime.lm-studio") return "LM Studio";
  return "Future runtime";
}

export function licenseLabel(licenseState: SetupRecommendation["licenseState"]) {
  if (licenseState === "known") return "License verified";
  if (licenseState === "restricted") return "Restricted terms";
  if (licenseState === "unknown") return "License needs review";
  return null;
}

export function registryLabel(state: SetupPlanPreview["registryState"]) {
  if (state === "ready") return "Ready";
  if (state === "degraded") return "Limited";
  return "Needs attention";
}

export function friendlyCompatibilityReason(reason: string) {
  if (reason === "fits this machine") return "Fits this machine";
  if (reason === "cloud model available after provider connection") return "Connect provider to use";
  if (reason === "not enough free storage") return "Not enough free storage";
  if (reason === "not recommended on this computer") return "Not recommended on this computer";
  return "Compatibility checked";
}

export function friendlyChannel(channel: SetupRecommendation["channel"]): string {
  if (channel === "stable") return "Stable";
  if (channel === "beta") return "Beta";
  return "Experimental";
}

export function friendlyInstallMode(installMode: SetupRecommendation["installMode"]): string {
  if (installMode === "external_guided") return "Guided external setup";
  if (installMode === "python_environment") return "Local Python setup";
  return "One-click setup";
}

export function friendlyHiddenReason(reason: string): string {
  if (reason.includes("hidden_channel:beta")) return "Beta catalog entries are hidden until you enable advanced choices.";
  if (reason.includes("hidden_channel:experimental")) return "Experimental catalog entries are hidden until you enable advanced choices.";
  const family = familyNameFromReason(reason);
  if (family) return `${family} is not recommended on this computer.`;
  if (isHardwareHiddenReason(reason)) return "A larger model is not recommended on this computer.";
  return "A catalog option is hidden until you enable advanced choices.";
}

function familyNameFromReason(reason: string): string | null {
  const lower = reason.toLowerCase();
  if (lower.includes("qwen")) return "Qwen";
  return null;
}
