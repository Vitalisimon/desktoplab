import type { RuntimeInventoryItem, RuntimeSetupCapability, SetupRecommendation } from "../../api/types";

export function runtimeCapabilityLabel(capability?: RuntimeSetupCapability): string {
  if (!capability) return "Not available";
  if (capability.availability === "certified") return "Ready and supported";
  if (capability.availability === "experimental") return "Preview · not certified";
  if (capability.availability === "planned") return "Planned runtime";
  if (capability.availability === "blocked") return "Needs attention";
  return "Not available on this computer";
}

export function runtimeCapabilityDetail(capability?: RuntimeSetupCapability): string {
  if (!capability) return "DesktopLab has no executable setup contract for this runtime.";
  if (capability.availability === "certified" && capability.setupMode === "managed") {
    return "DesktopLab can install and verify this runtime.";
  }
  if (capability.availability === "certified" && capability.setupMode === "connect_existing") {
    return "DesktopLab can verify and use an existing installation.";
  }
  if (capability.availability === "experimental" && capability.setupMode === "managed") {
    return "DesktopLab can run this managed Preview setup. Exact installed-app certification is still pending.";
  }
  if (capability.availability === "experimental") {
    return "This Preview route is visible but cannot be configured on this computer.";
  }
  if (capability.availability === "planned") {
    return "This runtime is planned and cannot be configured in the current beta.";
  }
  return "This runtime cannot be configured on this computer.";
}

export function runtimeRecommendationSelectable(recommendation?: SetupRecommendation): boolean {
  const capability = recommendation?.runtimeCapability;
  return Boolean(
    (capability?.availability === "certified" || capability?.availability === "experimental")
      && (capability.setupMode === "managed" || capability.setupMode === "connect_existing"),
  );
}

export function runtimeInventoryConfigurable(runtime: RuntimeInventoryItem): boolean {
  const capability = runtime.runtimeCapability;
  return Boolean(
    runtime.install.supported
      && (capability?.availability === "certified" || capability?.availability === "experimental")
      && (capability.setupMode === "managed" || capability.setupMode === "connect_existing"),
  );
}

export function runtimeSetupAction(runtime: RuntimeInventoryItem): string {
  if (runtime.runtimeCapability?.setupMode === "connect_existing") {
    return `Connect existing ${runtime.displayName}`;
  }
  if (runtime.runtimeCapability?.availability === "experimental") {
    return `Set up ${runtime.displayName} Preview`;
  }
  return `Install and verify ${runtime.displayName}`;
}
