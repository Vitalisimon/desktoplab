import { describe, expect, test } from "vitest";
import { getHardwareWarningCopy, visibleHardwareWarningCopies } from "./hardwareWarningCopy";
import type { SetupPlanPreview } from "../../api/types";

describe("getHardwareWarningCopy", () => {
  test("turns probe codes into user-readable setup guidance", () => {
    expect(getHardwareWarningCopy("gpu_probe_unavailable")).toMatchObject({
      title: "Graphics check needs confirmation",
      impact: "DesktopLab can still continue, but model recommendations may be conservative.",
      diagnosticCode: "gpu_probe_unavailable",
    });
  });

  test("accepts legacy enum-style warning codes without exposing them to users", () => {
    expect(getHardwareWarningCopy("VramProbeUnavailable")).toMatchObject({
      title: "Accelerator memory was not confirmed",
      diagnosticCode: "vram_probe_unavailable",
    });
  });

  test("hides deferred probe and discrete vram warnings on unified-memory machines", () => {
    const copies = visibleHardwareWarningCopies(
      ["driver_probe_deferred_to_v2", "gpu_probe_unavailable", "vram_probe_unavailable"],
      hardware({ unifiedMemoryGb: { label: "Unified memory", value: 48, confidence: "confirmed" } }),
    );

    expect(copies).toHaveLength(0);
  });

  test("hides dedicated VRAM warnings for integrated graphics", () => {
    const copies = visibleHardwareWarningCopies(
      ["vram_probe_unavailable", "limited_memory"],
      hardware({ acceleratorKind: { label: "Accelerator type", value: "integrated", confidence: "confirmed" } }),
    );

    expect(copies.map((copy) => copy.diagnosticCode)).toEqual(["limited_memory"]);
  });

  test("uses explicit workstation language when the backend classifies the machine", () => {
    const copies = visibleHardwareWarningCopies(["workstation_class"], hardware({ unifiedMemoryGb: { label: "Unified memory", value: 128, confidence: "confirmed" } }));

    expect(copies[0]).toMatchObject({
      title: "Workstation-class local AI",
      impact: "DesktopLab can offer larger model families when storage and runtime support are ready.",
    });
  });
});

function hardware(overrides: Partial<SetupPlanPreview["hardware"]> = {}): SetupPlanPreview["hardware"] {
  return {
    cpu: { label: "CPU", value: "Apple M4 Ultra", confidence: "confirmed" },
    ramGb: { label: "RAM", value: 128, confidence: "confirmed" },
    gpu: { label: "GPU", value: null, confidence: "unknown" },
    vramGb: { label: "VRAM", value: null, confidence: "unknown" },
    unifiedMemoryGb: { label: "Unified memory", value: 128, confidence: "confirmed" },
    operatingSystem: { label: "OS", value: "macOS", confidence: "confirmed" },
    architecture: { label: "Architecture", value: "arm64", confidence: "confirmed" },
    storageAvailableGb: { label: "Storage", value: 900, confidence: "confirmed" },
    ...overrides,
  };
}
