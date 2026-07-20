// @vitest-environment jsdom
import { readFileSync } from "node:fs";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import { themePreferenceValues } from "../../design/tokens";
import type { DesktopLabApiClient } from "../../api/client";
import type {
  ApprovalModesResponse,
  ExternalBackendsResponse,
  DiagnosticsSnapshot,
  HealthResponse,
  PluginsListResponse,
  ProvidersListResponse,
  ReadinessResponse,
  RoutePreference,
  RuntimesListResponse,
  RuntimeInstallResponse,
  ModelsListResponse,
  SetupPlanPreview,
  VersionResponse,
} from "../../api/types";
import { SettingsFeature } from "./SettingsFeature";

beforeEach(() => {
  window.localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.removeAttribute("data-theme-preference");
  Element.prototype.scrollIntoView = vi.fn();
});

test("renders user-facing local configuration without support bundle noise", async () => {
  render(
    <AppProviders apiClient={clientFor()}>
      <SettingsFeature />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
  expect(screen.getByRole("heading", { name: "Settings" }).closest("[data-ui-route='settings']")).toHaveClass("pb-16");
  expect(screen.getByRole("heading", { name: "Current setup" })).toBeInTheDocument();
  expect(screen.queryByText("API v1")).not.toBeInTheDocument();
  expect(screen.queryByText("Local services ready")).not.toBeInTheDocument();
  expect(await screen.findByText("Ollama is running")).toBeInTheDocument();
  expect(screen.getByText("Qwen Coder is ready now")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Updates" })).toBeInTheDocument();
  expect(screen.queryByText("Update checks are prepared but public release updates are not enabled yet.")).not.toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Setup diagnostics" })).not.toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Diagnostics" })).not.toBeInTheDocument();
  expect(screen.queryByText("Diagnostic bundle")).not.toBeInTheDocument();
  expect(screen.queryByText("Bundle preview")).not.toBeInTheDocument();
  expect(screen.queryByText("runtime.ollama")).not.toBeInTheDocument();
  expect(screen.queryByText("model.qwen-coder")).not.toBeInTheDocument();
  expect(screen.queryByText("Installing runner")).not.toBeInTheDocument();
  expect(screen.queryByText("Runtime install: Running")).not.toBeInTheDocument();
  expect(screen.queryByText("/Users/example/secret")).not.toBeInTheDocument();
  expect(screen.getByTestId("control-surface-header")).not.toHaveClass("rounded-desktop");
  for (const disclosure of screen.getAllByTestId("settings-disclosure")) {
    expect(disclosure).not.toHaveClass("bg-panel");
    expect(disclosure).not.toHaveClass("rounded-desktop");
  }
});

test("theme preference contract defaults to system and resolves on boot", async () => {
  render(
    <AppProviders apiClient={clientFor()}>
      <div>Theme booted</div>
    </AppProviders>,
  );

  expect(await screen.findByText("Theme booted")).toBeInTheDocument();
  expect(themePreferenceValues).toEqual(["system", "light", "dark"]);
  expect(document.documentElement.dataset.themePreference).toBe("system");
  expect(["light", "dark"]).toContain(document.documentElement.dataset.theme);
});

test("theme preference persists and applies light and dark without restart", async () => {
  window.localStorage.setItem("desktoplab.themePreference", "dark");
  const { unmount } = render(
    <AppProviders apiClient={clientFor()}>
      <div>Dark theme booted</div>
    </AppProviders>,
  );

  expect(await screen.findByText("Dark theme booted")).toBeInTheDocument();
  expect(document.documentElement.dataset.themePreference).toBe("dark");
  expect(document.documentElement.dataset.theme).toBe("dark");
  unmount();

  window.localStorage.setItem("desktoplab.themePreference", "light");
  render(
    <AppProviders apiClient={clientFor()}>
      <div>Light theme booted</div>
    </AppProviders>,
  );

  expect(await screen.findByText("Light theme booted")).toBeInTheDocument();
  expect(document.documentElement.dataset.themePreference).toBe("light");
  expect(document.documentElement.dataset.theme).toBe("light");
});

test("dark theme primary controls avoid white text on theme-dependent ink backgrounds", () => {
  const sources = [
    "src/features/productization/AgentComposer.tsx",
    "src/design/AppDrawer.tsx",
    "src/design/OperationalPrimitives.tsx",
    "src/features/setup/SetupWizard.tsx",
    "src/features/workspaces/WorkspaceOpenView.tsx",
    "src/features/productization/ModelRow.tsx",
    "src/features/productization/ProviderConnectPanel.tsx",
    "src/features/approvals/ApprovalCard.tsx",
    "src/features/terminal/TerminalDrawer.tsx",
  ];

  for (const sourcePath of sources) {
    expect(readFileSync(sourcePath, "utf8"), sourcePath).not.toContain("bg-ink text-white");
    expect(readFileSync(sourcePath, "utf8"), sourcePath).not.toMatch(/bg-ink[^"]*text-white|text-white[^"]*bg-ink/);
  }
});

test("appearance settings let the user choose system light or dark theme", async () => {
  render(
    <AppProviders apiClient={clientFor()}>
      <SettingsFeature />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Appearance" })).toBeInTheDocument();
  expect(screen.getByRole("radio", { name: "System" })).toBeChecked();

  fireEvent.click(screen.getByRole("radio", { name: "Dark" }));
  expect(document.documentElement.dataset.themePreference).toBe("dark");
  expect(document.documentElement.dataset.theme).toBe("dark");
  expect(window.localStorage.getItem("desktoplab.themePreference")).toBe("dark");

  fireEvent.click(screen.getByRole("radio", { name: "Light" }));
  expect(document.documentElement.dataset.themePreference).toBe("light");
  expect(document.documentElement.dataset.theme).toBe("light");
  expect(window.localStorage.getItem("desktoplab.themePreference")).toBe("light");
});

test("keeps degraded engine diagnostics out of the model catalog", async () => {
  render(
    <AppProviders
      apiClient={clientFor({
        readiness: { state: "degraded", degradedReasons: ["Using last-known-good compatibility catalog."] },
      })}
    >
      <SettingsFeature />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Model catalog and setup" })).not.toBeInTheDocument();
  expect(screen.queryByText("Local services limited")).not.toBeInTheDocument();
  expect(screen.queryByText("Using last-known-good compatibility catalog.")).not.toBeInTheDocument();
  expect(screen.queryByText("Local services ready")).not.toBeInTheDocument();
});

test("keeps governance settings read-only until backend command endpoints exist", async () => {
  render(
    <AppProviders apiClient={clientFor()}>
      <SettingsFeature />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Safety and approvals" }));
  expect(await screen.findByText("File changes wait for you")).toBeInTheDocument();
  expect(screen.getByText("Community plugins start unverified")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: /save/i })).not.toBeInTheDocument();
});

test("shows redacted diagnostics export guidance only inside diagnostics settings", async () => {
  render(
    <AppProviders apiClient={clientFor()}>
      <SettingsFeature />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Setup diagnostics" })).not.toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: "Diagnostics" }));

  expect(await screen.findByRole("heading", { name: "Setup diagnostics" })).toBeInTheDocument();
  expect(screen.getByRole("heading", { name: "Local status" })).toBeInTheDocument();
  expect(screen.getByText("Stability snapshot: ready; route decision current.")).toBeInTheDocument();
  expect(screen.getByText("Export bundle is local. Review it before sharing.")).toBeInTheDocument();
  expect(screen.getByText("Redacted")).toBeInTheDocument();
  expect(screen.getByText("Ollama")).toBeInTheDocument();
  expect(screen.getByText("Qwen Coder")).toBeInTheDocument();
  expect(screen.queryByText("runtime.ollama")).not.toBeInTheDocument();
  expect(screen.queryByText("model.qwen-coder")).not.toBeInTheDocument();
  expect(screen.queryByText("/Users/example/secret")).not.toBeInTheDocument();
  expect(screen.queryByText("sk-live-secret")).not.toBeInTheDocument();
});

test("explains local safety defaults in plain language without policy controls", async () => {
  render(
    <AppProviders apiClient={clientFor()}>
      <SettingsFeature />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Safety and approvals" }));
  expect(await screen.findByText("File changes wait for you")).toBeInTheDocument();
  expect(screen.getByText("Command runs wait for you")).toBeInTheDocument();
  expect(screen.getByText("Git actions wait for you")).toBeInTheDocument();
  expect(screen.getByText("Cloud model use is shown first")).toBeInTheDocument();
  expect(screen.getByText("Protected data stays on this device")).toBeInTheDocument();
  expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
  expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
  expect(screen.queryByText("policy", { exact: false })).not.toBeInTheDocument();
});

test("settings persist the default approval mode through the backend", async () => {
  const updateDefaultApprovalMode = vi.fn().mockResolvedValue(approvalModes("full_access"));

  render(
    <AppProviders apiClient={clientFor({ updateDefaultApprovalMode })}>
      <SettingsFeature />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Safety and approvals" }));
  expect(await screen.findByText("Default for new sessions")).toBeInTheDocument();
  expect(screen.getByRole("radio", { name: "Ask for approval" })).toBeChecked();
  expect(screen.getByText("Recommended for small local models and careful first runs.")).toBeInTheDocument();
  expect(screen.getByText("Workspace file writes can continue in this session while commands and git actions still stop.")).toBeInTheDocument();
  expect(screen.getByText("Commits, pushes, external providers and protected data still stop for you.")).toBeInTheDocument();
  expect(screen.queryByText("External providers, pushes and protected data still stop for you.")).not.toBeInTheDocument();

  fireEvent.click(screen.getByRole("radio", { name: "Full local access" }));

  expect(await screen.findByText("Full local access will be used for new sessions.")).toBeInTheDocument();
  expect(updateDefaultApprovalMode).toHaveBeenCalledWith({ mode: "full_access" });
  expect(screen.queryByText("policy", { exact: false })).not.toBeInTheDocument();
});

test("summarizes productized configuration without duplicating technical routes", async () => {
  render(
    <AppProviders apiClient={clientFor()}>
      <SettingsFeature />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
  expect(await screen.findByText("Active local setup")).toBeInTheDocument();
  expect(screen.getByText("Available changes")).toBeInTheDocument();
  expect(screen.queryByText("Connected accounts")).not.toBeInTheDocument();
  expect(screen.queryByText("Routing")).not.toBeInTheDocument();
  expect(screen.queryByText("Agent bridges")).not.toBeInTheDocument();
  expect(screen.queryByText("Plugins")).not.toBeInTheDocument();
  expect(screen.queryByText("1 unverified")).not.toBeInTheDocument();
  expect(screen.queryByLabelText("API key")).not.toBeInTheDocument();
});

test("explains the active local setup and available replacements in plain language", async () => {
  render(
    <AppProviders apiClient={clientFor()}>
      <SettingsFeature />
    </AppProviders>,
  );

  expect(await screen.findByText("Active local setup")).toBeInTheDocument();
  expect(screen.getByText("Ollama is running")).toBeInTheDocument();
  expect(screen.getByText("Qwen Coder is ready now")).toBeInTheDocument();
  expect(screen.getByText("Available changes")).toBeInTheDocument();
  expect(screen.getByText("1 optional local runner can be configured")).toBeInTheDocument();
  expect(screen.getByText("1 compatible model can be downloaded")).toBeInTheDocument();
});

test("pluralizes available local setup changes", async () => {
  const availableModels = models();
  availableModels.models.push({
    ...availableModels.models[1],
    modelId: "model.codestral",
    displayName: "Codestral",
  });
  render(
    <AppProviders
      apiClient={clientFor({
        listRuntimes: vi.fn().mockResolvedValue(runtimesWithMlx()),
        listModels: vi.fn().mockResolvedValue(availableModels),
      })}
    >
      <SettingsFeature />
    </AppProviders>,
  );

  expect(await screen.findByText("2 optional local runners can be configured")).toBeInTheDocument();
  expect(screen.getByText("2 compatible models can be downloaded")).toBeInTheDocument();
});

test("shows stable version labels and deduplicates identical setup jobs", async () => {
  const repeatedJobs = diagnostics();
  repeatedJobs.bundlePreview.jobs = [
    { kind: "model.download", state: "succeeded" },
    { kind: "model.download", state: "succeeded" },
  ];
  render(
    <AppProviders apiClient={clientFor({ diagnostics: vi.fn().mockResolvedValue(repeatedJobs) })}>
      <SettingsFeature />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Diagnostics" }));
  expect(await screen.findByText("DesktopLab 0.1.0")).toBeInTheDocument();
  expect(screen.getByText("API v1")).toBeInTheDocument();
  expect(screen.getAllByText("Model download: Completed")).toHaveLength(1);
});

test("organizes settings into user-readable control center groups", async () => {
  render(
    <AppProviders apiClient={clientFor()}>
      <SettingsFeature />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
  const groupNames = screen.getAllByTestId("settings-group").map((group) => group.getAttribute("aria-label"));
  const disclosureNames = screen.getAllByTestId("settings-disclosure").map((group) => group.getAttribute("aria-label"));

  expect(groupNames).toEqual([
    "Current setup",
    "Appearance",
  ]);
  expect(disclosureNames).toEqual([
    "Safety and approvals",
    "Providers",
    "Updates",
    "Diagnostics",
  ]);
  expect(screen.queryByRole("button", { name: "Model catalog and setup" })).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Safety and approvals" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Providers" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Updates" })).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Extensions" })).not.toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Diagnostics" })).not.toBeInTheDocument();
});

test("brings an opened settings disclosure into view", async () => {
  render(
    <AppProviders apiClient={clientFor()}>
      <SettingsFeature />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Diagnostics" }));

  await waitFor(() =>
    expect(Element.prototype.scrollIntoView).toHaveBeenCalledWith({ block: "start" }),
  );
});

test("provider account setup lives inside settings and uses backend provider state", async () => {
  render(
    <AppProviders apiClient={clientFor()}>
      <SettingsFeature />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Accounts" })).not.toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: "Providers" }));

  expect(await screen.findByRole("heading", { name: "Accounts" })).toBeInTheDocument();
  expect(screen.getAllByText("OpenAI").length).toBeGreaterThan(0);
  expect(screen.getByRole("heading", { name: "Connect account" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Connect account" })).toBeDisabled();
});

test("keeps provider settings reachable when diagnostics are temporarily unavailable", async () => {
  render(
    <AppProviders apiClient={clientFor({ diagnostics: vi.fn().mockRejectedValue(new Error("diagnostics unavailable")) })}>
      <SettingsFeature />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Providers" }));
  expect(await screen.findByRole("heading", { name: "Accounts" })).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Settings unavailable" })).not.toBeInTheDocument();
});

test("recovers settings diagnostics while the local API is warming up", async () => {
  const readDiagnostics = vi.fn().mockRejectedValueOnce(new Error("api booting")).mockResolvedValue(diagnostics());
  render(
    <AppProviders apiClient={clientFor({ diagnostics: readDiagnostics })}>
      <SettingsFeature />
    </AppProviders>,
  );

  expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Diagnostics" }));
  expect(await screen.findByRole("heading", { name: "Setup diagnostics" }, { timeout: 2_000 })).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Diagnostics unavailable" })).not.toBeInTheDocument();
  expect(readDiagnostics).toHaveBeenCalledTimes(2);
});

test("keeps the settings feature container small enough to preserve ownership boundaries", () => {
  const source = readFileSync("src/features/settings/SettingsFeature.tsx", "utf8");

  expect(source.split("\n").length).toBeLessThan(140);
});

function clientFor(
  overrides: {
    readiness?: ReadinessResponse;
    setupPreview?: DesktopLabApiClient["setupPreview"];
    updateDefaultApprovalMode?: DesktopLabApiClient["updateDefaultApprovalMode"];
    startModelDownload?: DesktopLabApiClient["startModelDownload"];
    startRuntimeInstall?: DesktopLabApiClient["startRuntimeInstall"];
    listRuntimes?: DesktopLabApiClient["listRuntimes"];
    listModels?: DesktopLabApiClient["listModels"];
    diagnostics?: DesktopLabApiClient["diagnostics"];
  } = {},
): DesktopLabApiClient {
  return {
    health: vi.fn<() => Promise<HealthResponse>>().mockResolvedValue({ status: "healthy" }),
    readiness: vi.fn<() => Promise<ReadinessResponse>>().mockResolvedValue(overrides.readiness ?? { state: "ready" }),
    version: vi.fn<() => Promise<VersionResponse>>().mockResolvedValue({ productVersion: "0.1.0", apiVersion: "v1" }),
    setupPreview: overrides.setupPreview ?? vi.fn<() => Promise<SetupPlanPreview>>().mockResolvedValue(preview()),
    listProviders: vi.fn<() => Promise<ProvidersListResponse>>().mockResolvedValue(providers()),
    routePreference: vi.fn<() => Promise<RoutePreference>>().mockResolvedValue(routePreference()),
    listRuntimes: overrides.listRuntimes ?? vi.fn<() => Promise<RuntimesListResponse>>().mockResolvedValue(runtimes()),
    listModels: overrides.listModels ?? vi.fn<() => Promise<ModelsListResponse>>().mockResolvedValue(models()),
    startRuntimeInstall: overrides.startRuntimeInstall ?? vi.fn(),
    startModelDownload: overrides.startModelDownload ?? vi.fn(),
    listPlugins: vi.fn<() => Promise<PluginsListResponse>>().mockResolvedValue(plugins()),
    listExternalBackends: vi.fn<() => Promise<ExternalBackendsResponse>>().mockResolvedValue({ backends: [] }),
    approvalModes: vi.fn<() => Promise<ApprovalModesResponse>>().mockResolvedValue(approvalModes()),
    updateDefaultApprovalMode: overrides.updateDefaultApprovalMode ?? vi.fn().mockResolvedValue(approvalModes()),
    diagnostics: overrides.diagnostics ?? vi.fn<() => Promise<DiagnosticsSnapshot>>().mockResolvedValue(diagnostics()),
  } as unknown as DesktopLabApiClient;
}

function approvalModes(
  defaultMode:
    | "require_approval"
    | "approve_for_me"
    | "approve_workspace_writes_for_session"
    | "full_access" = "require_approval",
): ApprovalModesResponse {
  return {
    defaultMode,
    sessionMode: "require_approval",
    modes: [
      {
        mode: "require_approval",
        label: "Ask for approval",
        description: "Recommended for small local models and careful first runs.",
      },
      {
        mode: "approve_for_me",
        label: "Approve routine actions",
        description: "DesktopLab can approve routine local steps while provider egress still stops.",
      },
      {
        mode: "approve_workspace_writes_for_session",
        label: "Allow workspace writes",
        description: "Workspace file writes can continue while commands and git actions still stop.",
      },
      {
        mode: "full_access",
        label: "Full local access",
        description: "External providers and protected data still stop for you.",
      },
    ],
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
    runtimeRecommendations: [{ manifestId: "runtime.ollama", displayName: "Ollama", channel: "stable" }],
    modelRecommendations: [{ manifestId: "model.qwen-coder", displayName: "Qwen Coder", channel: "stable" }],
    warnings: [],
    expectedLimitations: [],
    hiddenReasons: [],
  };
}

function providers(): ProvidersListResponse {
  return {
    providers: [
      {
        providerId: "provider.openai",
        displayName: "OpenAI",
        status: "connected",
        trust: "verified",
        egress: "requires_approval",
        capabilities: ["Chat"],
        diagnostic: { state: "ready", message: "Ready", redactedEvidence: "Bearer [REDACTED]" },
      },
      {
        providerId: "provider.anthropic",
        displayName: "Anthropic",
        status: "missing_credential",
        trust: "verified",
        egress: "requires_approval",
        capabilities: ["Chat"],
        diagnostic: { state: "missing_credential", message: "Credential missing", redactedEvidence: "Bearer [REDACTED]" },
      },
    ],
  };
}

function routePreference(): RoutePreference {
  return {
    mode: "local_first",
    cloudAllowed: false,
    lockedByPolicy: true,
    explanation: "Cloud routes require approval.",
  };
}

function runtimes(): RuntimesListResponse {
  return {
    runtimes: [
      {
        runtimeId: "runtime.ollama",
        displayName: "Ollama",
        ownership: "user_owned",
        status: "running",
        capabilities: ["Local chat"],
        install: { supported: true },
        repairActions: [],
      },
      {
        runtimeId: "runtime.lm-studio",
        displayName: "LM Studio",
        ownership: "externally_managed",
        status: "not_installed",
        capabilities: ["OpenAI-compatible endpoint"],
        install: { supported: false, blockedReason: "Guided setup" },
        repairActions: [],
      },
    ],
  };
}

function runtimesWithMlx(): RuntimesListResponse {
  return {
    runtimes: [
      runtimes().runtimes[0],
      {
        runtimeId: "runtime.mlx-lm",
        displayName: "MLX-LM Server",
        ownership: "desktoplab_managed",
        status: "not_installed",
        capabilities: ["Local chat"],
        install: { supported: true },
        repairActions: [],
      },
      runtimes().runtimes[1],
    ],
  };
}

function models(): ModelsListResponse {
  return {
    models: [
      {
        modelId: "model.qwen-coder",
        displayName: "Qwen Coder",
        runtimeId: "runtime.ollama",
        channel: "stable",
        installState: "installed",
        compatibility: "ready",
        sizeGb: 8,
        recommended: true,
      },
      {
        modelId: "model.deepseek-coder",
        displayName: "DeepSeek Coder 7B",
        runtimeId: "runtime.ollama",
        pullRef: "deepseek-coder:7b",
        channel: "stable",
        familyName: "DeepSeek",
        parametersBillion: 7,
        quantization: "Q4_K_M",
        requiredMemoryGb: 8,
        installState: "downloadable",
        compatibility: "compatible",
        sizeGb: 5,
        recommended: false,
        provenance: {
          catalogSource: "bundled_seed_catalog",
          runtimeId: "runtime.ollama",
          pullRef: "deepseek-coder:7b",
          verificationState: "downloadable_not_installed",
          localVerification: "Ready to download through selected local runtime",
        },
      },
    ],
  };
}

function plugins(): PluginsListResponse {
  return {
    plugins: [
      {
        pluginId: "plugin.community",
        displayName: "Community Tools",
        status: "available",
        trust: "unverified",
        capabilities: ["tool.filesystem.write"],
        blockedReasons: [],
        trustActions: [{ id: "trust", label: "Trust plugin", description: "Request approval after review." }],
      },
    ],
  };
}

function diagnostics(): DiagnosticsSnapshot {
  return {
    state: "ready",
    services: [],
    repairActions: [],
    bundlePreview: {
      summary: "ready token=[REDACTED]",
      setup: {
        runtimeId: "runtime.ollama",
        modelId: "model.qwen-coder",
        pipelineState: "runtime_installing",
      },
      hardware: [{ label: "OS", value: "macos", confidence: "confirmed" }],
      jobs: [{ kind: "runtime.install", state: "running" }],
      redactedErrors: [{ kind: "model.download", message: "Background work needs attention.", redacted: true }],
      sizeBytes: 512,
      maxBytes: 64000,
      redacted: true,
    },
    updateStatus: {
      channel: "dev",
      currentVersion: "0.1.0",
      state: "disabled",
      message: "Update checks are prepared but public release updates are not enabled yet.",
      canInstall: false,
    },
    stability: {
      kind: "desktoplab.stability.snapshot",
      schemaVersion: 1,
      redacted: true,
      payloadFree: true,
      startupPhase: "ready",
      uptimeMs: 1250,
      localApiHealth: { state: "responding", scope: "loopback_router", payloadFree: true },
      routeDecisionRecency: {
        state: "current",
        selectedRouteId: "route.local.qwen-coder",
        lastChangedAgoMs: 25,
      },
      queueBackpressure: {
        state: "idle",
        queued: 0,
        running: 0,
        awaitingApproval: 0,
        blocked: 0,
        failed: 0,
        active: 0,
        payloadFree: true,
      },
      budgets: {
        memory: { budgetMb: 512, sampleState: "not_sampled" },
        disk: { minimumFreeMb: 2048, sampleState: "not_sampled" },
      },
      degradedReasons: [],
      jobStates: [],
    },
  };
}
