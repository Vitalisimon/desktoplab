// @vitest-environment jsdom
import { fireEvent, render, screen } from "@testing-library/react";
import { InstallPlanPreview } from "./InstallPlanPreview";
import type { SetupPlanPreview } from "../../api/types";

test("shows friendly local setup jobs without provider jargon", () => {
  const onAccept = vi.fn();
  render(<InstallPlanPreview preview={preview()} onAccept={onAccept} />);

  expect(screen.getByText("Install local runner")).toBeInTheDocument();
  expect(screen.getByText("Download coding model")).toBeInTheDocument();
  expect(screen.getByText(/DesktopLab keeps this setup on your computer/i)).toBeInTheDocument();
  expect(screen.queryByText(/Provider egress/i)).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /start setup/i }));
  expect(onAccept).toHaveBeenCalledTimes(1);
});

test("blocks plan acceptance when registry state is blocked", () => {
  render(<InstallPlanPreview preview={{ ...preview(), registryState: "blocked" }} onAccept={vi.fn()} />);

  expect(screen.getByRole("button", { name: /start setup/i })).toBeDisabled();
});

test("describes installed selections as reuse instead of installation", () => {
  const setup = preview();
  setup.runtimeRecommendations[0] = {
    ...setup.runtimeRecommendations[0],
    setupChoiceRequired: true,
    defaultSetupChoice: "use_existing",
  };
  setup.modelRecommendations[0] = {
    ...setup.modelRecommendations[0],
    setupChoiceRequired: true,
    defaultSetupChoice: "use_existing",
  };

  render(<InstallPlanPreview preview={setup} onAccept={vi.fn()} />);

  expect(screen.getByText("Use installed local runner")).toBeInTheDocument();
  expect(screen.getByText("Use installed coding model")).toBeInTheDocument();
  expect(screen.queryByText("Install local runner")).not.toBeInTheDocument();
  expect(screen.queryByText("Download coding model")).not.toBeInTheDocument();
});

test("tracks replacement choices and selected alternatives", () => {
  const setup = preview();
  setup.runtimeRecommendations.push({
    manifestId: "runtime.alternative",
    displayName: "Alternative runner",
    channel: "stable",
    role: "alternative",
    setupChoiceRequired: true,
  });
  setup.modelRecommendations.push({
    manifestId: "model.alternative",
    displayName: "Alternative model",
    channel: "stable",
    role: "alternative",
    setupChoiceRequired: true,
  });

  render(
    <InstallPlanPreview
      preview={setup}
      selectedRuntimeId="runtime.alternative"
      selectedModelId="model.alternative"
      runtimeSetupChoice="replace"
      modelSetupChoice="replace"
      onAccept={vi.fn()}
    />,
  );

  expect(screen.getByText("Alternative runner")).toBeInTheDocument();
  expect(screen.getByText("Alternative model")).toBeInTheDocument();
  expect(screen.getByText("Replace local runner")).toBeInTheDocument();
  expect(screen.getByText("Replace coding model")).toBeInTheDocument();
  expect(screen.queryByText("Ollama")).not.toBeInTheDocument();
  expect(screen.queryByText("Qwen Coder")).not.toBeInTheDocument();
});

function preview(): SetupPlanPreview {
  return {
    registryState: "ready",
    hardware: {
      cpu: { label: "CPU", value: "Apple M4 Pro", confidence: "confirmed" },
      ramGb: { label: "RAM", value: 48, confidence: "confirmed" },
      gpu: { label: "GPU", value: null, confidence: "unknown" },
      vramGb: { label: "VRAM", value: null, confidence: "unknown" },
      unifiedMemoryGb: { label: "Unified memory", value: 48, confidence: "confirmed" },
      operatingSystem: { label: "OS", value: "macOS", confidence: "confirmed" },
      architecture: { label: "Architecture", value: "arm64", confidence: "confirmed" },
      storageAvailableGb: { label: "Storage", value: 900, confidence: "confirmed" },
    },
    runtimeRecommendations: [{ manifestId: "runtime.ollama", displayName: "Ollama", channel: "stable" }],
    modelRecommendations: [{ manifestId: "model.qwen-coder", displayName: "Qwen Coder", channel: "stable" }],
    warnings: [],
    expectedLimitations: [],
    hiddenReasons: [],
  };
}
