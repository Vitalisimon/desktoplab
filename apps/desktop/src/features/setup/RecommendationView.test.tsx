// @vitest-environment jsdom
import { render, screen, within } from "@testing-library/react";
import { RecommendationView } from "./RecommendationView";
import type { SetupPlanPreview } from "../../api/types";

test("renders runtime and model recommendations from backend preview", () => {
  renderRecommendationView(preview());

  expect(screen.getByText("Recommended setup")).toBeInTheDocument();
  expect(screen.getByText("Ollama")).toBeInTheDocument();
  expect(screen.getByText("Qwen Coder")).toBeInTheDocument();
  expect(screen.getByText("7B")).toBeInTheDocument();
  expect(screen.getByText("Q4")).toBeInTheDocument();
  expect(screen.getByText("~4.7 GB")).toBeInTheDocument();
  expect(screen.getByText("License verified")).toBeInTheDocument();
  expect(screen.getByText("Fits this machine")).toBeInTheDocument();
  expect(screen.getByText("Local runner")).toBeInTheDocument();
  expect(screen.getByText("Coding model")).toBeInTheDocument();
  expect(screen.getByText("Recommended for this computer")).toBeInTheDocument();
  expect(screen.getAllByText("Works offline").length).toBeGreaterThanOrEqual(1);
  expect(screen.getByText("Needs about 4.7 GB disk")).toBeInTheDocument();
  expect(screen.queryByText("runtime.ollama")).not.toBeInTheDocument();
  expect(screen.queryByText("model.qwen-coder")).not.toBeInTheDocument();
});

test("keeps compatible model alternatives expandable instead of making setup a catalog", () => {
  renderRecommendationView(
    preview([], [
        { manifestId: "model.qwen-coder", displayName: "Qwen Coder", channel: "stable" },
        { manifestId: "model.deepseek-coder", displayName: "DeepSeek Coder", channel: "stable" },
        { manifestId: "model.glm-coder", displayName: "GLM Coder", channel: "beta" },
      ]),
  );

  expect(screen.getByText("Qwen Coder")).toBeInTheDocument();
  expect(screen.getByText("Other compatible models")).toBeInTheDocument();
  expect(screen.getByText("Other compatible models").closest("details")).not.toHaveAttribute("open");
  expect(screen.getByText("DeepSeek Coder")).toBeInTheDocument();
  expect(screen.getByText("GLM Coder")).toBeInTheDocument();
  expect(screen.queryByText("model.deepseek-coder")).not.toBeInTheDocument();
});

test("shows local model families with readable size and memory details", () => {
  renderRecommendationView(
    preview([], [
      model("model.qwen-coder-7b-q4", "Qwen Coder 7B", "Qwen", "small", 7, "Q4", 12, 4_700, "stable"),
      model("model.deepseek-coder-7b-q4", "DeepSeek Coder 7B", "DeepSeek", "small", 7, "Q4", 12, 3_800, "beta"),
      model("model.glm4-9b", "GLM 4 9B", "GLM", "small", 9, "Q4", 16, 5_500, "beta"),
      model("model.nemotron-70b-q4", "Nemotron 70B", "NVIDIA Nemotron", "workstation", 70, "Q4", 96, 43_000, "experimental"),
    ]),
  );

  expect(screen.getByText("Qwen")).toBeInTheDocument();
  expect(screen.getByText("DeepSeek")).toBeInTheDocument();
  expect(screen.getByText("GLM")).toBeInTheDocument();
  expect(screen.getByText("NVIDIA Nemotron")).toBeInTheDocument();
  expect(screen.getByText("More local model families are available as this catalog grows.")).toBeInTheDocument();
  expect(screen.getAllByText("12 GB memory class")).toHaveLength(2);
  expect(screen.getByText("16 GB memory class")).toBeInTheDocument();
  expect(screen.getByText("96 GB memory class")).toBeInTheDocument();
  expect(screen.getByText("~43.0 GB on disk")).toBeInTheDocument();
  expect(screen.getByText("Workstation")).toBeInTheDocument();
  expect(screen.getAllByText("Small").length).toBeGreaterThanOrEqual(2);
  expect(screen.queryByText("model.nemotron-70b-q4")).not.toBeInTheDocument();
});

test("shows cloud model families without pretending they are local downloads", () => {
  renderRecommendationView(
    preview([], [
      model("model.qwen-coder-7b-q4", "Qwen Coder 7B", "Qwen", "small", 7, "Q4", 12, 4_700, "stable"),
      {
        ...model("model.glm-5.2-cloud", "GLM 5.2", "GLM", "cloud", 0, "cloud", 0, 0, "experimental"),
        runtimeId: "runtime.ollama-cloud",
        compatibilityReason: "cloud model available after provider connection",
      },
    ]),
  );

  const cloudOption = screen.getByText("GLM 5.2").closest("li");
  expect(cloudOption).not.toBeNull();
  const cloud = within(cloudOption!);
  expect(cloud.getByText("Cloud")).toBeInTheDocument();
  expect(cloud.getByText("Connect provider to use")).toBeInTheDocument();
  expect(cloud.getByText("Runner Ollama Cloud")).toBeInTheDocument();
  expect(cloud.queryByText("Works offline")).not.toBeInTheDocument();
  expect(cloud.queryByText("Needs about 0.0 GB disk")).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Select GLM 5.2" })).not.toBeInTheDocument();
});

test("labels externally guided runtimes without presenting them as automatic installs", () => {
  renderRecommendationView(
    preview([], undefined, [
        { manifestId: "runtime.ollama", displayName: "Ollama", channel: "stable", role: "recommended", installMode: "automatic" },
        { manifestId: "runtime.lm-studio", displayName: "LM Studio", channel: "stable", role: "alternative", installMode: "external_guided" },
      ]),
  );

  expect(screen.getByText("Guided external setup")).toBeInTheDocument();
  expect(screen.queryByText("Install LM Studio automatically")).not.toBeInTheDocument();
});

test("labels python environment runtimes without presenting them as one click installs", () => {
  renderRecommendationView(
    preview([], undefined, [
      { manifestId: "runtime.ollama", displayName: "Ollama", channel: "stable", role: "recommended", installMode: "automatic" },
      { manifestId: "runtime.mlx-lm", displayName: "MLX-LM", channel: "beta", role: "alternative", installMode: "python_environment" },
    ]),
  );

  expect(screen.getByText("Local Python setup")).toBeInTheDocument();
  expect(screen.queryAllByText("One-click setup")).toHaveLength(1);
});

test("uses user-readable setup choice copy for existing local installs", () => {
  renderRecommendationView(
    preview(
      [],
      [
        {
          manifestId: "model.qwen-coder",
          displayName: "Qwen Coder",
          channel: "stable",
          setupChoiceRequired: true,
          defaultSetupChoice: "use_existing",
        },
      ],
      [
        {
          manifestId: "runtime.ollama",
          displayName: "Ollama",
          channel: "stable",
          setupChoiceRequired: true,
          defaultSetupChoice: "use_existing",
        },
      ],
    ),
  );

  expect(screen.getAllByText("Keep my current local setup")).toHaveLength(2);
  expect(screen.getAllByText("Install a fresh local setup")).toHaveLength(2);
  expect(screen.queryByText("Use existing local runner")).not.toBeInTheDocument();
  expect(screen.queryByText("Replace local runner")).not.toBeInTheDocument();
});

test("keeps model size parameters quantization and disk details secondary but readable", () => {
  renderRecommendationView(preview());

  expect(screen.getByRole("button", { name: "Model and runner details" })).toBeInTheDocument();
  expect(screen.getByText("7B")).toBeInTheDocument();
  expect(screen.getByText("Q4")).toBeInTheDocument();
  expect(screen.getByText("~4.7 GB")).toBeInTheDocument();
});

test("explains blocked large variants without exposing catalog jargon", () => {
  renderRecommendationView(preview(["model.large-local:hidden_hardware:workstation"]));

  expect(screen.getByText("Not recommended on this computer")).toBeInTheDocument();
  expect(screen.getByText("A larger model is not recommended on this computer.")).toBeInTheDocument();
  expect(screen.queryByText("model.large-local:hidden_hardware:workstation")).not.toBeInTheDocument();
});

test("keeps beta or experimental entries as human advanced detail", () => {
  renderRecommendationView(preview(["runtime.beta:hidden_channel:beta"]));

  expect(screen.getByText("Advanced catalog details")).toBeInTheDocument();
  expect(screen.getByText("Beta catalog entries are hidden until you enable advanced choices.")).toBeInTheDocument();
  expect(screen.queryByText("runtime.beta:hidden_channel:beta")).not.toBeInTheDocument();
});

test("keeps driver probe implementation limitations out of the user setup page", () => {
  renderRecommendationView({
    ...preview(),
    expectedLimitations: ["accelerator confidence requires v2 driver/runtime probing"],
  });

  expect(screen.queryByText("GPU acceleration will be checked in a later driver pass. Local setup can still continue with the safe options shown here.")).not.toBeInTheDocument();
  expect(screen.queryByText("accelerator confidence requires v2 driver/runtime probing")).not.toBeInTheDocument();
});

function renderRecommendationView(previewData: SetupPlanPreview) {
  render(
    <RecommendationView
      preview={previewData}
      runtimeSetupChoice="use_existing"
      modelSetupChoice="use_existing"
      onRuntimeSetupChoiceChange={vi.fn()}
      onModelSetupChoiceChange={vi.fn()}
    />,
  );
}

function preview(
  hiddenReasons: string[] = [],
  modelRecommendations: SetupPlanPreview["modelRecommendations"] = [
    {
      manifestId: "model.qwen-coder",
      displayName: "Qwen Coder",
      channel: "stable",
      parametersBillion: 7,
      quantization: "Q4",
      expectedDiskMb: 4_700,
      runtimeId: "runtime.ollama",
      compatibilityReason: "fits this machine",
      licenseState: "known",
      trustLabel: "License verified",
    },
  ],
  runtimeRecommendations: SetupPlanPreview["runtimeRecommendations"] = [{ manifestId: "runtime.ollama", displayName: "Ollama", channel: "stable" }],
): SetupPlanPreview {
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
    runtimeRecommendations,
    modelRecommendations,
    warnings: [],
    expectedLimitations: ["standard local coding assistance"],
    hiddenReasons,
  };
}

function model(
  manifestId: string,
  displayName: string,
  familyName: string,
  parameterClass: "small" | "medium" | "large" | "workstation" | "cloud",
  parametersBillion: number,
  quantization: string,
  requiredMemoryGb: number,
  expectedDiskMb: number,
  channel: "stable" | "beta" | "experimental",
): SetupPlanPreview["modelRecommendations"][number] {
  return {
    manifestId,
    displayName,
    familyName,
    parameterClass,
    channel,
    parametersBillion,
    quantization,
    requiredMemoryGb,
    expectedDiskMb,
    compatibilityReason: "fits this machine",
    licenseState: "known",
    trustLabel: "License verified",
  };
}
