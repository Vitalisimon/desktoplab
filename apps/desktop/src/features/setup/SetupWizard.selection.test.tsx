// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ApiProvider } from "../../api/ApiProvider";
import type { DesktopLabApiClient } from "../../api/client";
import type { SetupAcceptanceResponse, SetupPlanPreview } from "../../api/types";
import { SetupWizard } from "./SetupWizard";

test("accepts only the selected setup plan and keeps alternatives out of progress", async () => {
  const api = client({
    preview: preview(),
    acceptance: {
      startedJobIds: ["job.1", "job.2"],
      jobs: [
        { jobId: "job.1", kind: "runtime.install", state: "running" },
        { jobId: "job.2", kind: "model.download", state: "blocked", blockedReason: "runtime_not_ready" },
      ],
    },
  });
  renderSetup(api);

  expect((await screen.findAllByText("Ollama")).length).toBeGreaterThan(0);
  expect(screen.getAllByText("Qwen Coder").length).toBeGreaterThan(0);
  expect(screen.getByText("DeepSeek Coder")).toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: /select deepseek coder/i }));

  fireEvent.click(screen.getByRole("button", { name: /start setup/i }));

  await waitFor(() =>
    expect(api.acceptSetupPlan).toHaveBeenCalledWith({
      runtimeId: "runtime.ollama",
      modelId: "model.deepseek-coder-7b-q4",
    }),
  );
  expect(await screen.findByText("Setup progress")).toBeInTheDocument();
  expect(screen.getAllByText("Install local runner").length).toBeGreaterThan(0);
  expect(screen.getAllByText("Download coding model").length).toBeGreaterThan(0);
  expect(screen.getByText("Verify the local runner before downloading this model.")).toBeInTheDocument();
  expect(screen.queryByText("runtime_not_ready")).not.toBeInTheDocument();
  expect(screen.queryByText("runtime_not_verified")).not.toBeInTheDocument();
  expect(screen.queryByText("model.deepseek-coder-7b-q4")).not.toBeInTheDocument();
});

test("lets users keep existing local installs and sends that choice to setup routes", async () => {
  const api = client({
    preview: previewWithExistingLocalInventory(),
    acceptance: { startedJobIds: [], jobs: [] },
  });
  renderSetup(api);

  expect((await screen.findAllByText("Ollama")).length).toBeGreaterThan(0);
  expect(screen.getAllByText("Already installed").length).toBeGreaterThanOrEqual(2);
  const keepChoices = screen.getAllByRole("radio", { name: /keep my current local setup/i });
  expect(keepChoices).toHaveLength(2);
  expect(keepChoices[0]).toBeChecked();
  expect(keepChoices[1]).toBeChecked();

  fireEvent.click(screen.getByRole("button", { name: /start setup/i }));

  await waitFor(() =>
    expect(api.startRuntimeInstall).toHaveBeenCalledWith({
      runtimeId: "runtime.ollama",
      setupChoice: "use_existing",
    }),
  );
  expect(api.startModelDownload).toHaveBeenCalledWith({
    modelId: "model.qwen-coder-7b-q4",
    runtimeId: "runtime.ollama",
    setupChoice: "use_existing",
  });
});

test("shows existing local runtime and model evidence before setup starts", async () => {
  const api = client({
    preview: previewWithExistingLocalInventory(),
    acceptance: { startedJobIds: [], jobs: [] },
  });
  renderSetup(api);

  expect((await screen.findAllByText("Ollama")).length).toBeGreaterThan(0);
  expect(screen.getByText("Version 0.9.1")).toBeInTheDocument();
  expect(screen.getByText("/Applications/Ollama.app")).toBeInTheDocument();
  expect(screen.getByText("http://127.0.0.1:11434")).toBeInTheDocument();
  expect(screen.getAllByText("Already installed").length).toBeGreaterThanOrEqual(2);
  expect(screen.getAllByText("Qwen Coder 7B Q4").length).toBeGreaterThan(0);
});

test("lets users replace existing local installs and sends replace to setup routes", async () => {
  const api = client({
    preview: previewWithExistingLocalInventory(),
    acceptance: { startedJobIds: [], jobs: [] },
  });
  renderSetup(api);

  expect((await screen.findAllByText("Ollama")).length).toBeGreaterThan(0);
  const freshChoices = screen.getAllByRole("radio", { name: /install a fresh local setup/i });
  expect(freshChoices).toHaveLength(2);
  fireEvent.click(freshChoices[0]);
  fireEvent.click(freshChoices[1]);
  fireEvent.click(screen.getByRole("button", { name: /start setup/i }));

  await waitFor(() =>
    expect(api.startRuntimeInstall).toHaveBeenCalledWith({
      runtimeId: "runtime.ollama",
      setupChoice: "replace",
    }),
  );
  expect(api.startModelDownload).toHaveBeenCalledWith({
    modelId: "model.qwen-coder-7b-q4",
    runtimeId: "runtime.ollama",
    setupChoice: "replace",
  });
});

test("does not start a local model download for provider or cloud-only catalog entries", async () => {
  const api = client({
    preview: {
      ...preview(),
      modelRecommendations: [
        {
          manifestId: "model.glm-5.2-cloud",
          displayName: "GLM 5.2",
          channel: "experimental",
          role: "recommended",
          runtimeId: "runtime.ollama-cloud",
          parameterClass: "cloud",
          compatibilityReason: "cloud model available after provider connection",
        },
      ],
    },
    acceptance: { startedJobIds: ["job.runtime"], jobs: [] },
  });
  renderSetup(api);

  expect((await screen.findAllByText("GLM 5.2")).length).toBeGreaterThan(0);
  expect(screen.getByRole("button", { name: /start setup/i })).toBeDisabled();

  fireEvent.click(screen.getByRole("button", { name: /start setup/i }));
  expect(api.acceptSetupPlan).not.toHaveBeenCalled();
  expect(api.startModelDownload).not.toHaveBeenCalled();
});

function renderSetup(api: Partial<DesktopLabApiClient>) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={queryClient}>
      <ApiProvider client={api as DesktopLabApiClient}>
        <SetupWizard onOpenRepository={vi.fn()} />
      </ApiProvider>
    </QueryClientProvider>,
  );
}

function previewWithExistingLocalInventory(): SetupPlanPreview {
  const base = preview();
  return {
    ...base,
    runtimeRecommendations: base.runtimeRecommendations.map((runtime) =>
      runtime.manifestId === "runtime.ollama"
        ? {
            ...runtime,
            hostInstallState: "installed",
            defaultSetupChoice: "use_existing",
            setupChoiceRequired: true,
            installedVersion: "0.9.1",
            installedPath: "/Applications/Ollama.app",
            endpoint: "http://127.0.0.1:11434",
          }
        : runtime,
    ),
    modelRecommendations: base.modelRecommendations.map((model) =>
      model.manifestId === "model.qwen-coder-7b-q4"
        ? {
            ...model,
            displayName: "Qwen Coder 7B Q4",
            hostInstallState: "installed",
            defaultSetupChoice: "use_existing",
            setupChoiceRequired: true,
          }
        : model,
    ),
  };
}

function client(options: { preview: SetupPlanPreview; acceptance: SetupAcceptanceResponse }): Partial<DesktopLabApiClient> {
  return {
    setupPreview: vi.fn<() => Promise<SetupPlanPreview>>().mockResolvedValue(options.preview),
    acceptSetupPlan: vi.fn().mockResolvedValue(options.acceptance),
    readiness: vi.fn().mockResolvedValue({ state: "ready" }),
    catalogRefreshStatus: vi.fn().mockResolvedValue({ state: "ready", lastKnownGoodAvailable: true, degradedReasons: [], manualRefresh: { available: true } }),
    startRuntimeInstall: vi.fn().mockResolvedValue({ jobId: "job.3", runtimeId: "runtime.ollama", state: "blocked", verificationState: "blocked", retryClass: "user_action", remediation: "Ollama was not detected." }),
    startModelDownload: vi.fn().mockResolvedValue({ jobId: "job.4", modelId: "model.qwen-coder-7b-q4", runtimeId: "runtime.ollama", state: "blocked", retryClass: "non_retryable", blockedReason: "runtime_not_verified" }),
  };
}

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
    runtimeRecommendations: [
      { manifestId: "runtime.ollama", displayName: "Ollama", channel: "stable", role: "recommended" },
      { manifestId: "runtime.lm-studio", displayName: "LM Studio", channel: "stable", role: "alternative" },
    ],
    modelRecommendations: [
      { manifestId: "model.qwen-coder-7b-q4", displayName: "Qwen Coder", channel: "stable", role: "recommended" },
      { manifestId: "model.deepseek-coder-7b-q4", displayName: "DeepSeek Coder", channel: "stable", role: "alternative" },
    ],
    warnings: [],
    expectedLimitations: [],
    hiddenReasons: [],
  };
}
