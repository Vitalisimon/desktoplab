// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { HardwareSummary } from "./HardwareSummary";
import type { SetupPlanPreview } from "../../api/types";

test("renders hardware facts and warnings in plain language", () => {
  render(<HardwareSummary hardware={hardware({ unifiedMemoryGb: { label: "Unified memory", value: null, confidence: "unknown" } })} warnings={["GpuProbeUnavailable"]} />);

  expect(screen.getByText("Your computer")).toBeInTheDocument();
  expect(screen.getByText("Apple M4 Pro")).toBeInTheDocument();
  expect(screen.getAllByText("48 GB")).toHaveLength(1);
  expect(screen.getAllByText("Unknown")).toHaveLength(2);
  expect(screen.getAllByText("Needs check")).toHaveLength(2);
  expect(screen.getAllByText("Checked")).toHaveLength(3);
  expect(screen.queryByText("Unified memory")).not.toBeInTheDocument();
  expect(screen.queryByText("unknown")).not.toBeInTheDocument();
  expect(screen.queryByText("confirmed")).not.toBeInTheDocument();
  expect(screen.getByText("Graphics check needs confirmation")).toBeInTheDocument();
  expect(screen.getByText(/model recommendations may be conservative/i)).toBeInTheDocument();
  expect(screen.queryByText("GpuProbeUnavailable")).not.toBeInTheDocument();
});

test("omits unified memory when discrete RAM and GPU memory are available", () => {
  render(
    <HardwareSummary
      hardware={hardware({
        cpu: { label: "CPU", value: "AMD Ryzen 9", confidence: "confirmed" },
        ramGb: { label: "RAM", value: 16, confidence: "confirmed" },
        gpu: { label: "GPU", value: "NVIDIA GeForce RTX 5070 Laptop GPU", confidence: "confirmed" },
        vramGb: { label: "VRAM", value: 8, confidence: "confirmed" },
        unifiedMemoryGb: { label: "Unified memory", value: null, confidence: "unknown" },
        operatingSystem: { label: "OS", value: "Windows", confidence: "confirmed" },
      })}
      warnings={[]}
    />,
  );

  expect(screen.getByText("16 GB")).toBeInTheDocument();
  expect(screen.getByText("NVIDIA GeForce RTX 5070 Laptop GPU")).toBeInTheDocument();
  expect(screen.getByText("8 GB")).toBeInTheDocument();
  expect(screen.queryByText("Unified memory")).not.toBeInTheDocument();
  expect(screen.queryByText("Needs check")).not.toBeInTheDocument();
});

test("omits a dedicated VRAM check for integrated graphics", () => {
  render(
    <HardwareSummary
      hardware={hardware({
        cpu: { label: "CPU", value: "Intel N95", confidence: "confirmed" },
        ramGb: { label: "RAM", value: 8, confidence: "confirmed" },
        gpu: { label: "GPU", value: "Intel Corporation Alder Lake-N [UHD Graphics]", confidence: "confirmed" },
        acceleratorKind: { label: "Accelerator type", value: "integrated", confidence: "confirmed" },
        unifiedMemoryGb: { label: "Unified memory", value: null, confidence: "unknown" },
      })}
      warnings={["limited_memory"]}
    />,
  );

  expect(screen.getByText("Intel Corporation Alder Lake-N [UHD Graphics]")).toBeInTheDocument();
  expect(screen.queryByText("VRAM")).not.toBeInTheDocument();
  expect(screen.queryByText("Accelerator memory was not confirmed")).not.toBeInTheDocument();
});

test("treats Apple Silicon unified memory as the relevant accelerator memory", () => {
  render(
    <HardwareSummary
      hardware={hardware()}
      warnings={["driver_probe_deferred_to_v2", "gpu_probe_unavailable", "vram_probe_unavailable"]}
    />,
  );

  expect(screen.getByText("Unified memory")).toBeInTheDocument();
  expect(screen.getByText("48 GB shared memory")).toBeInTheDocument();
  expect(screen.queryByText("VRAM")).not.toBeInTheDocument();
  expect(screen.queryByText("GPU")).not.toBeInTheDocument();
  expect(screen.queryByText("Driver check will run later")).not.toBeInTheDocument();
  expect(screen.queryByText("Accelerator memory was not confirmed")).not.toBeInTheDocument();
});

function hardware(overrides: Partial<SetupPlanPreview["hardware"]> = {}): SetupPlanPreview["hardware"] {
  const base: SetupPlanPreview["hardware"] = {
    cpu: { label: "CPU", value: "Apple M4 Pro", confidence: "confirmed" },
    ramGb: { label: "RAM", value: 48, confidence: "confirmed" },
    gpu: { label: "GPU", value: null, confidence: "unknown" },
    vramGb: { label: "VRAM", value: null, confidence: "unknown" },
    unifiedMemoryGb: { label: "Unified memory", value: 48, confidence: "confirmed" },
    operatingSystem: { label: "OS", value: "macOS", confidence: "confirmed" },
    architecture: { label: "Architecture", value: "arm64", confidence: "confirmed" },
    storageAvailableGb: { label: "Storage", value: 900, confidence: "confirmed" },
  };
  return { ...base, ...overrides };
}
