// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { ModelsListResponse, RuntimeInspectSnapshot, RuntimesListResponse } from "../../api/types";
import { RuntimeModelFeature } from "./RuntimeModelFeature";

test("renders local runners and coding models from backend inventory", async () => {
  renderRuntimeModel();

  expect(await screen.findByRole("heading", { name: "Models" })).toBeInTheDocument();
  expect(screen.getByText("Active runtime route")).toBeInTheDocument();
  expect(screen.getByText("route.local.qwen-coder-7b")).toBeInTheDocument();
  expect(screen.getByText("Configured")).toBeInTheDocument();
  expect(screen.getByText("Live evidence")).toBeInTheDocument();
  expect(screen.getByText("verified")).toBeInTheDocument();
  expect(screen.getByText("Current local setup")).toBeInTheDocument();
  expect(screen.getByText("No local runner is active yet")).toBeInTheDocument();
  expect(screen.getByText("Qwen Coder can be downloaded")).toBeInTheDocument();
  expect(screen.getByText("Downloaded models can replace the active local model for new agent sessions.")).toBeInTheDocument();
  expect(screen.getByText("Ollama")).toBeInTheDocument();
  expect(screen.getByText("LM Studio")).toBeInTheDocument();
  expect(screen.getAllByText("Guided setup").length).toBeGreaterThan(0);
  expect(screen.getAllByText(/DesktopLab app updates are separate from local runner installs/).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/never request administrator access silently/).length).toBeGreaterThan(0);
  expect(screen.getByText("Updates are handled by the DesktopLab installer.")).toBeInTheDocument();
  expect(screen.getByText("Remove LM Studio from its own app.")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Update Ollama" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Uninstall Ollama" })).not.toBeInTheDocument();
  expect(screen.getByText("Qwen Coder")).toBeInTheDocument();
  expect(screen.getByText("DeepSeek Coder")).toBeInTheDocument();
  expect(screen.getByText("Requires local runner repair")).toBeInTheDocument();
});

test("starts supported runtime installs and model downloads through backend commands", async () => {
  const startRuntimeInstall = vi.fn().mockResolvedValue({
    jobId: "job.runtime.install",
    runtimeId: "runtime.ollama",
    state: "verifying",
    verificationState: "pending",
    retryClass: "retryable",
  });
  const startModelDownload = vi.fn().mockResolvedValue({
    jobId: "job.model.download",
    modelId: "model.qwen-coder",
    familyId: "family.qwen",
    variantId: "model.qwen-coder",
    runtimeId: "runtime.ollama",
    state: "downloading",
    retryClass: "retryable",
  });
  renderRuntimeModel({ startRuntimeInstall, startModelDownload });

  await screen.findByRole("heading", { name: "Models" });
  fireEvent.click(screen.getByRole("button", { name: "Install Ollama" }));
  fireEvent.click(screen.getByRole("button", { name: "Download Qwen Coder" }));

  await waitFor(() => expect(startRuntimeInstall).toHaveBeenCalledWith({ runtimeId: "runtime.ollama" }));
  expect(await screen.findByText(withTextContent("Runtime install Checking runner"))).toBeInTheDocument();
  expect(screen.getByText(withTextContent("Verification Check pending"))).toBeInTheDocument();
  expect(startModelDownload).toHaveBeenCalledWith({ modelId: "model.qwen-coder", runtimeId: "runtime.ollama" });
  expect(await screen.findByText(withTextContent("Model download Downloading"))).toBeInTheDocument();
  expect(screen.getByText("Qwen Coder: Downloading.")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Install LM Studio" })).not.toBeInTheDocument();
});

test("shows runtime verification failure as a diagnostics handoff", async () => {
  const startRuntimeInstall = vi.fn().mockResolvedValue({
    jobId: "job.runtime.install",
    runtimeId: "runtime.ollama",
    state: "failed",
    verificationState: "failed",
    retryClass: "non_retryable",
  });
  renderRuntimeModel({ startRuntimeInstall });

  await screen.findByRole("heading", { name: "Models" });
  fireEvent.click(screen.getByRole("button", { name: "Install Ollama" }));

  expect(await screen.findByText(withTextContent("Runtime install Failed"))).toBeInTheDocument();
  expect(screen.getByText(withTextContent("Verification Failed"))).toBeInTheDocument();
  expect(screen.getByText("Open Diagnostics")).toBeInTheDocument();
});

test("shows installed, downloadable and hardware-blocked catalog models honestly", async () => {
  const startModelDownload = vi.fn().mockResolvedValue({
    jobId: "job.model.download",
    modelId: "model.llama-8b",
    familyId: "family.llama",
    variantId: "model.llama-8b",
    runtimeId: "runtime.ollama",
    state: "running",
    retryClass: "retryable",
  });
  renderRuntimeModel({
    startModelDownload,
    listModels: vi.fn<() => Promise<ModelsListResponse>>().mockResolvedValue({
      models: [
        model({
          modelId: "model.qwen-coder-7b-q4",
          displayName: "Qwen Coder small",
          installState: "installed",
          compatibility: "ready",
          verification: "Found in Ollama",
          parametersBillion: 7,
          quantization: "Q4_K_M",
          requiredMemoryGb: 12,
          sizeGb: 5,
        }),
        model({
          modelId: "model.llama-8b",
          displayName: "Llama 3.1 small",
          installState: "downloadable",
          compatibility: "compatible",
          verification: "Ready to download through Ollama",
          parametersBillion: 8,
          quantization: "Q4_K_M",
          requiredMemoryGb: 12,
          sizeGb: 6,
        }),
        model({
          modelId: "model.nemotron-49b",
          displayName: "Nemotron workstation",
          installState: "blocked",
          compatibility: "blocked",
          blockedReason: "Requires 96 GB memory class; this computer reports 16 GB.",
          parametersBillion: 49,
          quantization: "Q4_K_M",
          requiredMemoryGb: 96,
          sizeGb: 31,
        }),
      ],
    }),
  });

  expect(await screen.findByText("Found in Ollama")).toBeInTheDocument();
  expect(screen.getByText("7B")).toBeInTheDocument();
  expect(screen.getAllByText("Q4_K_M")).toHaveLength(3);
  expect(screen.getByText("Requires 96 GB memory class; this computer reports 16 GB.")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Download Qwen Coder small" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Download Nemotron workstation" })).toBeDisabled();

  fireEvent.click(screen.getByRole("button", { name: "Download Llama 3.1 small" }));

  await waitFor(() => expect(startModelDownload).toHaveBeenCalledWith({ modelId: "model.llama-8b", runtimeId: "runtime.ollama" }));
});

function withTextContent(expected: string) {
  return (_content: string, element: Element | null) => element?.textContent === expected;
}

function renderRuntimeModel(overrides: Partial<DesktopLabApiClient> = {}) {
  const client = {
    listRuntimes: vi.fn<() => Promise<RuntimesListResponse>>().mockResolvedValue({
      runtimes: [
        {
          runtimeId: "runtime.ollama",
          displayName: "Ollama",
          ownership: "desktoplab_managed",
          status: "not_installed",
          capabilities: ["Local chat", "Model downloads"],
          install: { supported: true, diskRequiredGb: 3 },
          lifecycle: {
            update: { state: "packaging_managed", label: "Installer managed", reason: "Updates are handled by the DesktopLab installer." },
            uninstall: { state: "packaging_managed", label: "Installer managed", reason: "Runtime removal is handled by the DesktopLab installer." },
          },
          repairActions: [{ id: "install", label: "Install Ollama", description: "Download the local runner." }],
          logExcerpt: "authorization=[REDACTED]",
        },
        {
          runtimeId: "runtime.lm-studio",
          displayName: "LM Studio",
          ownership: "externally_managed",
          status: "blocked",
          capabilities: ["OpenAI-compatible local endpoint"],
          install: { supported: false, blockedReason: "Guided setup" },
          lifecycle: {
            update: { state: "blocked", label: "External app", reason: "Managed outside DesktopLab." },
            uninstall: { state: "blocked", label: "External app", reason: "Remove LM Studio from its own app." },
          },
          repairActions: [],
        },
      ],
    }),
    listModels: vi.fn<() => Promise<ModelsListResponse>>().mockResolvedValue({
      models: [
        {
          modelId: "model.qwen-coder",
          displayName: "Qwen Coder",
          runtimeId: "runtime.ollama",
          channel: "stable",
          installState: "downloadable",
          compatibility: "compatible",
          sizeGb: 8,
          recommended: true,
          verification: "Not downloaded",
        },
        {
          modelId: "model.deepseek-coder",
          displayName: "DeepSeek Coder",
          runtimeId: "runtime.lm-studio",
          channel: "beta",
          installState: "blocked",
          compatibility: "blocked",
          sizeGb: 14,
          recommended: false,
          blockedReason: "Requires local runner repair",
        },
      ],
    }),
    startRuntimeInstall: vi.fn(),
    startModelDownload: vi.fn(),
    runtimeInspect: vi.fn<() => Promise<RuntimeInspectSnapshot>>().mockResolvedValue(runtimeInspect()),
    ...overrides,
  } as unknown as DesktopLabApiClient;

  return render(
    <AppProviders apiClient={client}>
      <RuntimeModelFeature />
    </AppProviders>,
  );
}

function runtimeInspect(): RuntimeInspectSnapshot {
  return {
    source: "service_backed",
    inspectState: "ready",
    active: {
      selectedRouteId: "route.local.qwen-coder-7b",
      backendId: "backend.ollama",
      runtimeId: "runtime.ollama",
      modelId: "model.qwen-coder-7b-q4",
      accountMode: "local_runtime",
      egress: "local_or_approval_gated",
      toolCapability: "filesystem_write_requires_approval",
      degradedReason: null,
    },
    evidence: {
      coldManifest: { source: "route_selection", runtimeId: "runtime.ollama", modelId: "model.qwen-coder-7b-q4" },
      liveRuntime: { state: "verified", evidence: "ollama 0.5.0" },
    },
  };
}

function model(overrides: Partial<ModelsListResponse["models"][number]>): ModelsListResponse["models"][number] {
  return {
    modelId: "model.test",
    displayName: "Test Model",
    runtimeId: "runtime.ollama",
    channel: "stable",
    installState: "downloadable",
    compatibility: "compatible",
    sizeGb: 1,
    recommended: false,
    ...overrides,
  };
}
