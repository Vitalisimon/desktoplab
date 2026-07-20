import type { SetupPlanPreview } from "../../api/types";

export type HardwareWarningCopy = {
  title: string;
  impact: string;
  diagnosticCode: string;
};

const WARNING_COPY: Record<string, HardwareWarningCopy> = {
  driver_probe_deferred_to_v2: {
    title: "Driver check will run later",
    impact: "DesktopLab can start setup now and will verify drivers during runtime checks.",
    diagnosticCode: "driver_probe_deferred_to_v2",
  },
  gpu_probe_unavailable: {
    title: "Graphics check needs confirmation",
    impact: "DesktopLab can still continue, but model recommendations may be conservative.",
    diagnosticCode: "gpu_probe_unavailable",
  },
  limited_memory: {
    title: "Memory is limited",
    impact: "Smaller local models or cloud backends will be more reliable on this machine.",
    diagnosticCode: "limited_memory",
  },
  low_storage: {
    title: "Storage is tight",
    impact: "Choose smaller models first or free disk space before large downloads.",
    diagnosticCode: "low_storage",
  },
  unsupported_architecture: {
    title: "Processor architecture is not supported",
    impact: "DesktopLab cannot safely install local runtimes on this architecture yet.",
    diagnosticCode: "unsupported_architecture",
  },
  unsupported_operating_system: {
    title: "Operating system is not supported",
    impact: "DesktopLab needs macOS, Windows or Linux for automated setup.",
    diagnosticCode: "unsupported_operating_system",
  },
  vram_probe_unavailable: {
    title: "Accelerator memory was not confirmed",
    impact: "Recommendations will avoid assuming large GPU-only models until this is verified.",
    diagnosticCode: "vram_probe_unavailable",
  },
  workstation_class: {
    title: "Workstation-class local AI",
    impact: "DesktopLab can offer larger model families when storage and runtime support are ready.",
    diagnosticCode: "workstation_class",
  },
  dgx_workstation_class: {
    title: "DGX-class local AI",
    impact: "DesktopLab can offer workstation model families when the NVIDIA runtime path is available.",
    diagnosticCode: "dgx_workstation_class",
  },
};

const DIAGNOSTICS_ONLY_CODES = new Set(["driver_probe_deferred_to_v2"]);
const UNIFIED_MEMORY_DISCRETE_PROBE_CODES = new Set(["gpu_probe_unavailable", "vram_probe_unavailable"]);

export function getHardwareWarningCopy(code: string): HardwareWarningCopy {
  const normalized = normalizeWarningCode(code);
  return (
    WARNING_COPY[normalized] ?? {
      title: "Hardware check needs review",
      impact: "DesktopLab can continue with conservative recommendations until this check is understood.",
      diagnosticCode: normalized,
    }
  );
}

export function visibleHardwareWarningCopies(warnings: string[], hardware: SetupPlanPreview["hardware"]): HardwareWarningCopy[] {
  return warnings
    .map(normalizeWarningCode)
    .filter((code) => isBaseUserVisibleWarning(code, hardware))
    .map(getHardwareWarningCopy);
}

function isBaseUserVisibleWarning(code: string, hardware: SetupPlanPreview["hardware"]): boolean {
  if (DIAGNOSTICS_ONLY_CODES.has(code)) return false;
  if (hasUnifiedMemory(hardware) && UNIFIED_MEMORY_DISCRETE_PROBE_CODES.has(code)) return false;
  if (hardware.acceleratorKind?.value === "integrated" && code === "vram_probe_unavailable") return false;
  return true;
}

function hasUnifiedMemory(hardware: SetupPlanPreview["hardware"]): boolean {
  return typeof hardware.unifiedMemoryGb.value === "number" && hardware.unifiedMemoryGb.value > 0;
}

function normalizeWarningCode(code: string) {
  return code
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .replace(/([A-Z])([A-Z][a-z])/g, "$1_$2")
    .toLowerCase();
}
