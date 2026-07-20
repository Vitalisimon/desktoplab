// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { CatalogRefreshRequestResponse, CatalogRefreshStatusResponse, HealthResponse, ModelDownloadResponse, ModelsListResponse, ReadinessResponse, RuntimeInstallResponse, RuntimesListResponse, SetupAcceptanceResponse, SetupPlanPreview } from "../../api/types";
import { SetupWizard } from "./SetupWizard";

test("renders setup preview and starts backend setup jobs", async () => {
  const startRuntimeInstall = vi.fn<() => Promise<RuntimeInstallResponse>>().mockResolvedValue({
    jobId: "runtime.install.runtime.ollama",
    runtimeId: "runtime.ollama",
    state: "downloading",
    verificationState: "pending",
    retryClass: "retryable",
  });
  const apiClient = clientFor({
    preview: preview("ready"),
    acceptance: { startedJobIds: ["runtime.install:runtime.ollama", "model.download:model.qwen-coder"] },
    readiness: { state: "ready" },
    setupState: "not_started",
    startRuntimeInstall,
    startModelDownload: vi.fn<() => Promise<ModelDownloadResponse>>().mockResolvedValue({
      jobId: "model.download.model.qwen-coder",
      modelId: "model.qwen-coder",
      familyId: "family.qwen",
      variantId: "model.qwen-coder",
      runtimeId: "runtime.ollama",
      state: "downloading",
      retryClass: "retryable",
      progressPercent: 64,
    }),
  });

  render(
    <AppProviders apiClient={apiClient}>
      <SetupWizard onOpenRepository={vi.fn()} />
    </AppProviders>,
  );

  expect(await screen.findByText("Apple M4 Pro")).toBeInTheDocument();
  expect(screen.getByText("Apple M4 Pro").closest("[data-ui-route='setup']")).toHaveClass("pb-16");
  expect(screen.getAllByText("Ollama").length).toBeGreaterThanOrEqual(1);
  expect(screen.queryByRole("button", { name: "Refresh compatibility catalog" })).not.toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: /start setup/i }));

  await waitFor(() => expect(startRuntimeInstall).toHaveBeenCalledWith({ runtimeId: "runtime.ollama" }));
  await waitFor(() => expect(apiClient.startModelDownload).toHaveBeenCalledWith({ modelId: "model.qwen-coder", runtimeId: "runtime.ollama" }));
  await waitFor(() => expect(screen.getByText("Setup progress")).toBeInTheDocument());
  expect(screen.getAllByText("Install local runner").length).toBeGreaterThan(0);
  expect(screen.getAllByText("Download coding model").length).toBeGreaterThan(0);
  expect(screen.getByLabelText("Download coding model progress").firstElementChild).toHaveStyle({ width: "64%" });
});

test("ready setup becomes local configuration without rerunning start setup", async () => {
  const apiClient = clientFor({
    preview: preview("ready"),
    acceptance: { startedJobIds: [] },
    readiness: { state: "ready" },
  });

  render(
    <AppProviders apiClient={apiClient}>
      <SetupWizard onOpenRepository={vi.fn()} />
    </AppProviders>,
  );

  const decision = await screen.findByRole("heading", { name: "Open a repository" });
  const configuration = screen.getByRole("heading", { name: "Local configuration" });
  const hardwareDisclosure = screen.getByRole("button", { name: /Your computer/i });
  const openRepository = screen.getByRole("button", { name: "Open Repository" });

  expect(screen.getByText("Local by default")).toBeInTheDocument();
  expect(screen.getByText("Your local setup is verified. Open a repository to start working.")).toBeInTheDocument();
  expect(screen.getByText("Active local runner")).toBeInTheDocument();
  expect(screen.getByText("Active coding model")).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Recommended setup" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: /start setup/i })).not.toBeInTheDocument();
  expect(openRepository.compareDocumentPosition(configuration) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(decision.compareDocumentPosition(configuration) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(configuration.compareDocumentPosition(hardwareDisclosure) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(screen.getByText("Apple M4 Pro")).not.toBeVisible();

  fireEvent.click(hardwareDisclosure);

  expect(screen.getByText("Apple M4 Pro")).toBeVisible();
});

test("ready setup owns local model and runner changes instead of settings", async () => {
  const startModelDownload = vi.fn().mockResolvedValue({
    jobId: "model.download.deepseek",
    modelId: "model.deepseek-coder",
    runtimeId: "runtime.ollama",
    state: "downloading",
    retryClass: "retryable",
  });
  const startRuntimeInstall = vi.fn().mockResolvedValue({
    jobId: "runtime.install.runtime.mlx-lm",
    runtimeId: "runtime.mlx-lm",
    state: "installing",
    verificationState: "pending",
    retryClass: "retryable",
  } satisfies RuntimeInstallResponse);
  const apiClient = clientFor({
    preview: {
      ...preview("ready"),
      expectedLimitations: ["accelerator confidence requires v2 driver/runtime probing"],
    },
    acceptance: { startedJobIds: [] },
    readiness: { state: "ready" },
    setupState: "ready",
    startModelDownload,
    startRuntimeInstall,
    listRuntimes: vi.fn<() => Promise<RuntimesListResponse>>().mockResolvedValue(runtimesWithMlx()),
  });

  render(
    <AppProviders apiClient={apiClient}>
      <SetupWizard onOpenRepository={vi.fn()} />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Local models" })).toBeInTheDocument();
  expect(screen.getByText("Qwen 7B Q4 is ready now")).toBeInTheDocument();
  expect(screen.getByRole("combobox", { name: "Choose a model to download" })).toBeInTheDocument();
  expect(screen.queryByText("GPU acceleration will be checked in a later driver pass. Local setup can still continue with the safe options shown here.")).not.toBeInTheDocument();
  expect(screen.queryByText("accelerator confidence requires v2 driver/runtime probing")).not.toBeInTheDocument();

  fireEvent.change(screen.getByRole("combobox", { name: "Choose a model to download" }), {
    target: { value: "model.deepseek-coder" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Download selected model" }));
  await waitFor(() =>
    expect(startModelDownload).toHaveBeenCalledWith({
      modelId: "model.deepseek-coder",
      runtimeId: "runtime.ollama",
      setupChoice: "install",
    }),
  );

  fireEvent.change(screen.getByRole("combobox", { name: "Choose a runner to configure" }), {
    target: { value: "runtime.mlx-lm" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Configure selected runner" }));
  await waitFor(() => expect(startRuntimeInstall).toHaveBeenCalledWith({ runtimeId: "runtime.mlx-lm", setupChoice: "install" }));
});

test("ready setup hides stale pipeline progress from previous setup work", async () => {
  const apiClient = clientFor({
    preview: preview("ready"),
    acceptance: { startedJobIds: [] },
    readiness: { state: "ready" },
    setupState: "ready",
    setupPipeline: { state: "runtime_installing", runtimeId: "runtime.ollama", modelId: "model.qwen-coder" },
  });

  render(
    <AppProviders apiClient={apiClient}>
      <SetupWizard onOpenRepository={vi.fn()} />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Open a repository" })).toBeInTheDocument();
  expect(screen.getByRole("heading", { name: "Local configuration" })).toBeInTheDocument();
  expect(screen.queryByText("Setup progress")).not.toBeInTheDocument();
  expect(screen.queryByText("Runtime install")).not.toBeInTheDocument();
});

test("recovers setup recommendations when the local API is still warming up", async () => {
  const setupPreview = vi
    .fn<() => Promise<SetupPlanPreview>>()
    .mockRejectedValueOnce(new Error("local API warming up"))
    .mockRejectedValueOnce(new Error("local API still warming up"))
    .mockResolvedValue(preview("ready"));
  const apiClient = clientFor({
    preview: preview("ready"),
    acceptance: { startedJobIds: [] },
    readiness: { state: "ready" },
    setupState: "ready",
  });
  apiClient.setupPreview = setupPreview;

  render(
    <AppProviders apiClient={apiClient}>
      <SetupWizard onOpenRepository={vi.fn()} />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Local configuration" }, { timeout: 3_000 })).toBeInTheDocument();
  expect(screen.queryByText("Setup data unavailable")).not.toBeInTheDocument();
  expect(setupPreview).toHaveBeenCalledTimes(3);
});

test("leads unfinished setup with a plain local setup decision", async () => {
  const apiClient = clientFor({
    preview: preview("ready"),
    acceptance: { startedJobIds: [] },
    readiness: { state: "blocked" },
    setupState: "not_started",
  });

  render(
    <AppProviders apiClient={apiClient}>
      <SetupWizard onOpenRepository={vi.fn()} />
    </AppProviders>,
  );

  const decision = await screen.findByRole("heading", { name: "Finish local setup" });
  const recommendation = screen.getByRole("heading", { name: "Recommended setup" });
  const setupPlan = screen.getByRole("heading", { name: "Setup plan" });
  const startSetup = screen.getByRole("button", { name: /start setup/i });
  const hardwareDisclosure = screen.getByRole("button", { name: /Your computer/i });

  expect(screen.getByText("Install the recommended local runner and coding model, or keep what is already installed.")).toBeInTheDocument();
  expect(decision.compareDocumentPosition(recommendation) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(setupPlan.compareDocumentPosition(recommendation) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(startSetup.compareDocumentPosition(hardwareDisclosure) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(screen.getByRole("button", { name: "Open Repository" })).toBeDisabled();
});

test("shows degraded setup readiness without calling it complete", async () => {
  const startCatalogRefresh = vi.fn<() => Promise<CatalogRefreshRequestResponse>>().mockResolvedValue({ jobId: "registry.refresh.manual" });
  const apiClient = clientFor({
    preview: preview("degraded"),
    acceptance: { startedJobIds: [] },
    readiness: { state: "degraded", degradedReasons: ["Using last-known-good compatibility catalog."] },
    startCatalogRefresh,
  });

  render(
    <AppProviders apiClient={apiClient}>
      <SetupWizard onOpenRepository={vi.fn()} />
    </AppProviders>,
  );

  expect(await screen.findByText("Degraded")).toBeInTheDocument();
  expect(screen.queryByText("DesktopLab is ready for a repository.")).not.toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: "Refresh compatibility catalog" }));

  await waitFor(() => expect(startCatalogRefresh).toHaveBeenCalled());
  expect(screen.getByText("Catalog refresh queued")).toBeInTheDocument();
  expect(screen.getByText("Track progress in Background.")).toBeInTheDocument();
});

test("keeps repository open disabled until backend setup state is ready", async () => {
  const apiClient = clientFor({
    preview: preview("ready"),
    acceptance: { startedJobIds: [] },
    readiness: { state: "ready" },
    setupState: "in_progress",
  });

  render(
    <AppProviders apiClient={apiClient}>
      <SetupWizard onOpenRepository={vi.fn()} />
    </AppProviders>,
  );

  const button = await screen.findByRole("button", { name: "Open Repository" });
  expect(button).toBeDisabled();
  expect(screen.queryByText("DesktopLab is ready for a repository.")).not.toBeInTheDocument();
});

test("renders persisted backend pipeline state after refresh", async () => {
  const apiClient = clientFor({
    preview: preview("ready"),
    acceptance: { startedJobIds: [] },
    readiness: { state: "blocked" },
    setupState: "in_progress",
    setupPipeline: { state: "runtime_installing", runtimeId: "runtime.ollama", modelId: "model.qwen-coder" },
  });

  render(
    <AppProviders apiClient={apiClient}>
      <SetupWizard onOpenRepository={vi.fn()} />
    </AppProviders>,
  );

  expect(await screen.findByText("Setup progress")).toBeInTheDocument();
  expect(screen.getByText("Runtime install")).toBeInTheDocument();
  expect(screen.getByText("Running")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Open Repository" })).toBeDisabled();
});

test("explains runtime install offline blocks without infrastructure jargon", async () => {
  const startRuntimeInstall = vi.fn<() => Promise<RuntimeInstallResponse>>().mockRejectedValue(new Error("NetworkUnavailable"));
  const apiClient = clientFor({
    preview: preview("degraded"),
    acceptance: { startedJobIds: [] },
    readiness: { state: "degraded", degradedReasons: ["Offline. Runtime download needs a verified cached installer."] },
    setupState: "not_started",
    startRuntimeInstall,
  });

  render(
    <AppProviders apiClient={apiClient}>
      <SetupWizard onOpenRepository={vi.fn()} />
    </AppProviders>,
  );

  fireEvent.click(await screen.findByRole("button", { name: /start setup/i }));

  expect(await screen.findByText("Runtime setup is waiting for a verified offline installer.")).toBeInTheDocument();
  expect(screen.getByText("Reconnect or add a verified cached installer, then start setup again.")).toBeInTheDocument();
  expect(screen.queryByText("NetworkUnavailable")).not.toBeInTheDocument();
});

test("replaces the running accept placeholder when runtime install fails", async () => {
  const startRuntimeInstall = vi.fn<() => Promise<RuntimeInstallResponse>>().mockResolvedValue({
    jobId: "job.3",
    runtimeId: "runtime.ollama",
    state: "failed",
    verificationState: "failed",
    retryClass: "retryable",
    remediation: "Network connection failed while downloading Ollama. Check the connection and retry.",
  });
  const startModelDownload = vi.fn<() => Promise<ModelDownloadResponse>>().mockResolvedValue({
    jobId: "job.4",
    modelId: "model.qwen-coder",
    familyId: "family.qwen",
    variantId: "model.qwen-coder",
    runtimeId: "runtime.ollama",
    state: "blocked",
    retryClass: "non_retryable",
    blockedReason: "runtime_not_verified",
  });
  const apiClient = clientFor({
    preview: preview("ready"),
    acceptance: {
      startedJobIds: ["job.1", "job.2"],
      jobs: [
        { jobId: "job.1", kind: "runtime.install", state: "running" },
        { jobId: "job.2", kind: "model.download", state: "blocked", blockedReason: "runtime_not_ready" },
      ],
    },
    readiness: { state: "blocked" },
    setupState: "not_started",
    startRuntimeInstall,
    startModelDownload,
  });

  render(
    <AppProviders apiClient={apiClient}>
      <SetupWizard onOpenRepository={vi.fn()} />
    </AppProviders>,
  );

  fireEvent.click(await screen.findByRole("button", { name: /start setup/i }));

  expect(await screen.findByText("Failed")).toBeInTheDocument();
  expect(screen.queryByText("Running")).not.toBeInTheDocument();
  expect(screen.getByText("Network connection failed while downloading Ollama. Check the connection and retry.")).toBeInTheDocument();
  expect(screen.getByText("Verify the local runner before downloading this model.")).toBeInTheDocument();
});

function clientFor(options: {
  preview: SetupPlanPreview;
  acceptance: SetupAcceptanceResponse;
  readiness: ReadinessResponse;
  startCatalogRefresh?: () => Promise<CatalogRefreshRequestResponse>;
  startRuntimeInstall?: () => Promise<RuntimeInstallResponse>;
  startModelDownload?: () => Promise<ModelDownloadResponse>;
  listModels?: () => Promise<ModelsListResponse>;
  listRuntimes?: () => Promise<RuntimesListResponse>;
  setupState?: "not_started" | "in_progress" | "ready" | "blocked";
  setupPipeline?: { state: "not_started" | "selected" | "runtime_detecting" | "runtime_installing" | "runtime_verifying" | "model_downloading" | "model_verifying" | "ready" | "blocked"; runtimeId?: string; modelId?: string; blockedReason?: string };
}): DesktopLabApiClient {
  return {
    appState: vi.fn().mockResolvedValue({
      readiness: options.setupState === "ready" || !options.setupState ? options.readiness : { state: "blocked" },
      setup: { state: options.setupState ?? "ready" },
      setupPipeline: options.setupPipeline ?? { state: options.setupState === "ready" || !options.setupState ? "ready" : "not_started" },
      currentWorkspace: null,
      routeInput: {
        readiness: options.setupState === "ready" || !options.setupState ? options.readiness.state : "blocked",
        setupState: options.setupState ?? "ready",
        hasWorkspace: false,
        activeApprovalCount: 0,
        activeSessionCount: 0,
      },
    }),
    health: vi.fn<() => Promise<HealthResponse>>().mockResolvedValue({ status: "healthy" }),
    readiness: vi.fn<() => Promise<ReadinessResponse>>().mockResolvedValue(options.readiness),
    version: vi.fn(),
    setupPreview: vi.fn<() => Promise<SetupPlanPreview>>().mockResolvedValue(options.preview),
    acceptSetupPlan: vi.fn<() => Promise<SetupAcceptanceResponse>>().mockResolvedValue(options.acceptance),
    catalogRefreshStatus: vi.fn<() => Promise<CatalogRefreshStatusResponse>>().mockResolvedValue(catalogStatus(options.preview.registryState)),
    listModels: options.listModels ?? vi.fn<() => Promise<ModelsListResponse>>().mockResolvedValue(modelsList()),
    listRuntimes: options.listRuntimes ?? vi.fn<() => Promise<RuntimesListResponse>>().mockResolvedValue(runtimesList()),
    startCatalogRefresh: options.startCatalogRefresh ?? vi.fn<() => Promise<CatalogRefreshRequestResponse>>().mockResolvedValue({ jobId: "registry.refresh.manual" }),
    startRuntimeInstall: options.startRuntimeInstall ?? vi.fn<() => Promise<RuntimeInstallResponse>>().mockResolvedValue({ jobId: "runtime.install.runtime.ollama", runtimeId: "runtime.ollama", state: "downloading", verificationState: "pending", retryClass: "retryable" }),
    startModelDownload: options.startModelDownload ?? vi.fn<() => Promise<ModelDownloadResponse>>().mockResolvedValue({ jobId: "model.download.model.qwen-coder", modelId: "model.qwen-coder", familyId: "family.qwen", variantId: "model.qwen-coder", runtimeId: "runtime.ollama", state: "downloading", retryClass: "retryable" }),
  } as unknown as DesktopLabApiClient;
}

function modelsList(): ModelsListResponse {
  return {
    models: [
      {
        modelId: "model.qwen-coder",
        displayName: "Qwen Coder",
        familyId: "family.qwen",
        familyName: "Qwen",
        runtimeId: "runtime.ollama",
        pullRef: "qwen2.5-coder:7b",
        channel: "stable",
        parameterClass: "small",
        parametersBillion: 7,
        quantization: "Q4",
        requiredMemoryGb: 8,
        installState: "installed",
        compatibility: "ready",
        sizeGb: 5,
        recommended: true,
        verification: "Found in Ollama",
        provenance: { catalogSource: "bundled_seed_catalog", runtimeId: "runtime.ollama", pullRef: "qwen2.5-coder:7b", verificationState: "verified_local_inventory", localVerification: "Found in Ollama" },
      },
      {
        modelId: "model.deepseek-coder",
        displayName: "DeepSeek Coder 7B",
        familyId: "family.deepseek",
        familyName: "DeepSeek",
        runtimeId: "runtime.ollama",
        pullRef: "deepseek-coder:7b",
        channel: "stable",
        parameterClass: "small",
        parametersBillion: 7,
        quantization: "Q4",
        requiredMemoryGb: 8,
        installState: "downloadable",
        compatibility: "compatible",
        sizeGb: 5,
        recommended: false,
        verification: "Ready to download through selected local runtime",
        provenance: { catalogSource: "bundled_seed_catalog", runtimeId: "runtime.ollama", pullRef: "deepseek-coder:7b", verificationState: "downloadable_not_installed", localVerification: "Ready to download through selected local runtime" },
      },
    ],
  };
}

function runtimesList(): RuntimesListResponse {
  return {
    runtimes: [
      {
        runtimeId: "runtime.ollama",
        displayName: "Ollama",
        status: "running",
        ownership: "user_owned",
        capabilities: ["Local chat"],
        install: { supported: true },
        repairActions: [],
      },
    ],
  };
}

function runtimesWithMlx(): RuntimesListResponse {
  return {
    ...runtimesList(),
    runtimes: [
      ...runtimesList().runtimes,
      {
        runtimeId: "runtime.mlx-lm",
        displayName: "MLX-LM Server",
        status: "not_installed",
        ownership: "desktoplab_managed",
        capabilities: ["Local chat"],
        install: { supported: true },
        repairActions: [],
      },
    ],
  };
}

function catalogStatus(state: SetupPlanPreview["registryState"]): CatalogRefreshStatusResponse {
  return {
    state,
    lastKnownGoodAvailable: state === "degraded",
    degradedReasons: state === "degraded" ? ["Using last-known-good compatibility catalog."] : [],
    manualRefresh: { available: state !== "blocked", jobId: "registry.refresh.manual" },
  };
}

function preview(registryState: SetupPlanPreview["registryState"]): SetupPlanPreview {
  return {
    registryState,
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
    expectedLimitations: registryState === "degraded" ? ["compatibility catalog refresh unavailable"] : [],
    hiddenReasons: [],
  };
}
