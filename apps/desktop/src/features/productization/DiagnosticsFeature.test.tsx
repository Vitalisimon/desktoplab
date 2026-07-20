// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { CatalogRefreshRequestResponse, CatalogRefreshStatusResponse, DiagnosticsSnapshot } from "../../api/types";
import { DiagnosticsFeature } from "./DiagnosticsFeature";

test("renders backend-owned diagnostics without exposing secrets", async () => {
  renderDiagnostics();

  expect(await screen.findByRole("heading", { name: "Diagnostics" })).toBeInTheDocument();
  expect(screen.getByRole("heading", { name: "Diagnostics" }).closest("[data-ui-route='diagnostics']")).toHaveClass("pb-16");
  expect(screen.getByText("Runtime")).toBeInTheDocument();
  expect(screen.getAllByText("Ollama is stopped").length).toBeGreaterThan(0);
  expect(screen.getByRole("button", { name: "Restart local runner" })).toBeEnabled();
  expect(screen.getByText("Registry")).toBeInTheDocument();
  expect(screen.getByText("Use cached catalog until network returns")).toBeInTheDocument();
  expect(screen.getByText("Guidance only")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Guidance only" })).not.toBeInTheDocument();
  expect(screen.getByText("Recent local decisions")).toBeInTheDocument();
  expect(screen.getByText("Provider egress allowed")).toBeInTheDocument();
  expect(screen.getByText("Tool execution denied")).toBeInTheDocument();
  expect(screen.getByText("terminal denied bearer [REDACTED]")).toBeInTheDocument();
  expect(screen.getByText("Redacted audit export")).toBeInTheDocument();
  expect(screen.getByText(/provider_egress allowed: sent after approval token=\[REDACTED\]/)).toBeInTheDocument();
  expect(screen.queryByText(/compliance/i)).not.toBeInTheDocument();
  expect(screen.queryByText("sk-live-secret")).not.toBeInTheDocument();
  expect(screen.queryByText("raw-bearer-token")).not.toBeInTheDocument();
});

test("runs executable repair actions through backend command", async () => {
  const runDiagnosticRepair = vi.fn().mockResolvedValue({ status: "blocked", repairId: "repair.runtime", reason: "external_manual_repair_required", repairKind: "external_manual" });
  renderDiagnostics({ runDiagnosticRepair });

  await screen.findByRole("heading", { name: "Diagnostics" });
  fireEvent.click(screen.getByRole("button", { name: "Restart local runner" }));

  await waitFor(() => expect(runDiagnosticRepair).toHaveBeenCalledWith("repair.runtime"));
  expect(await screen.findByText("Repair blocked")).toBeInTheDocument();
  expect(screen.getByText("Complete this repair outside DesktopLab.")).toBeInTheDocument();
  expect(screen.getByText("Manual repair")).toBeInTheDocument();
  expect(screen.queryByText("external_manual_repair_required")).not.toBeInTheDocument();
  expect(screen.queryByText("Repair queued")).not.toBeInTheDocument();
});

test("shows runtime install failure retry and blocks unsupported os repair", async () => {
  const runDiagnosticRepair = vi.fn().mockResolvedValue({ status: "blocked", repairId: "repair.runtime-install", reason: "diagnostic_repair_not_connected" });
  renderDiagnostics({ runDiagnosticRepair }, runtimeInstallFailureSnapshot());

  await screen.findByRole("heading", { name: "Diagnostics" });
  expect(screen.getByText("Runtime install verification failed")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Retry runtime install" }));

  await waitFor(() => expect(runDiagnosticRepair).toHaveBeenCalledWith("repair.runtime-install"));
  expect(screen.getByText("Driver repair must be handled outside DesktopLab.")).toBeInTheDocument();
  expect(screen.getByText("Guidance only")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Guidance only" })).not.toBeInTheDocument();
});

test("shows model download recovery without fake repairs for local constraints", async () => {
  const runDiagnosticRepair = vi.fn().mockResolvedValue({ status: "blocked", repairId: "repair.model-download.qwen", reason: "diagnostic_repair_not_connected" });
  renderDiagnostics({ runDiagnosticRepair }, modelDownloadFailureSnapshot());

  await screen.findByRole("heading", { name: "Diagnostics" });
  expect(screen.getByText("Qwen Coder download failed")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Retry model download" }));

  await waitFor(() => expect(runDiagnosticRepair).toHaveBeenCalledWith("repair.model-download.qwen"));
  expect(screen.getByText("Free disk space before downloading NVIDIA Nemotron.")).toBeInTheDocument();
  expect(screen.getByText("Guidance only")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Guidance only" })).not.toBeInTheDocument();
  expect(screen.getByText("Redacted audit export")).toBeInTheDocument();
  expect(screen.queryByText("token=secret")).not.toBeInTheDocument();
});

test("starts compatibility catalog refresh as a quiet support action", async () => {
  const startCatalogRefresh = vi.fn<() => Promise<CatalogRefreshRequestResponse>>().mockResolvedValue({ jobId: "registry.refresh.manual" });
  renderDiagnostics({ startCatalogRefresh });

  await screen.findByRole("heading", { name: "Diagnostics" });
  fireEvent.click(screen.getByRole("button", { name: "Refresh compatibility catalog" }));

  await waitFor(() => expect(startCatalogRefresh).toHaveBeenCalled());
  expect(screen.getByText("Catalog refresh queued")).toBeInTheDocument();
  expect(screen.getByText("Track progress in Background.")).toBeInTheDocument();
});

test("recovers diagnostics when local API readiness is briefly unavailable", async () => {
  const diagnostics = vi.fn().mockRejectedValueOnce(new Error("api booting")).mockResolvedValueOnce(snapshot());
  renderDiagnostics({ diagnostics });

  expect(await screen.findByRole("heading", { name: "Diagnostics" })).toBeInTheDocument();
  expect(screen.queryByText("Diagnostics unavailable")).not.toBeInTheDocument();
});

test("shows a compact healthy state when no repairs are available", async () => {
  renderDiagnostics({}, {
    ...snapshot(),
    state: "ready",
    repairActions: [],
  });

  expect(await screen.findByText("No repairs are needed.")).toBeInTheDocument();
  expect(screen.getByText("Ready")).toBeInTheDocument();
});

test("shows doctor lint checks as read-only backend evidence", async () => {
  renderDiagnostics({}, doctorLintSnapshot());

  expect(await screen.findByRole("heading", { name: "Diagnostics" })).toBeInTheDocument();
  expect(screen.getByText("Doctor lint")).toBeInTheDocument();
  expect(screen.getAllByText("Blocked").length).toBeGreaterThan(0);
  expect(screen.getByText("Runtime and model setup")).toBeInTheDocument();
  expect(screen.getByText("Finish setup before repository work.")).toBeInTheDocument();
  expect(screen.getAllByText("Blocked").length).toBeGreaterThan(0);
  expect(screen.queryByText("sk-live-secret")).not.toBeInTheDocument();
});

function renderDiagnostics(overrides: Partial<DesktopLabApiClient> = {}, diagnostics: DiagnosticsSnapshot = snapshot()) {
  const client = {
    diagnostics: vi.fn().mockResolvedValue(diagnostics),
    localAuditTransparency: vi.fn().mockResolvedValue(localAudit()),
    securityAudit: vi.fn().mockResolvedValue(securityAudit()),
    catalogRefreshStatus: vi.fn<() => Promise<CatalogRefreshStatusResponse>>().mockResolvedValue(catalogStatus()),
    startCatalogRefresh: vi.fn<() => Promise<CatalogRefreshRequestResponse>>().mockResolvedValue({ jobId: "registry.refresh.manual" }),
    runDiagnosticRepair: vi.fn(),
    ...overrides,
  } as unknown as DesktopLabApiClient;

  return render(
    <AppProviders apiClient={client}>
      <DiagnosticsFeature />
    </AppProviders>,
  );
}

function runtimeInstallFailureSnapshot(): DiagnosticsSnapshot {
  return {
    state: "degraded",
    services: [
      {
        family: "runtime",
        label: "Runtime",
        state: "degraded",
        message: "Runtime install verification failed",
      },
    ],
    repairActions: [
      {
        repairId: "repair.runtime-install",
        family: "runtime",
        label: "Retry runtime install",
        reason: "Checksum mismatch blocked install completion",
        mode: "executable",
      },
      {
        repairId: "repair.os-driver",
        family: "runtime",
        label: "Driver repair must be handled outside DesktopLab.",
        reason: "OS-level repair is unsupported",
        mode: "guidance_only",
      },
    ],
    bundlePreview: {
      summary: "runtime install failed token=[REDACTED]",
      sizeBytes: 5000,
      maxBytes: 64000,
      redacted: true,
    },
    updateStatus: updateStatus(),
  };
}

function modelDownloadFailureSnapshot(): DiagnosticsSnapshot {
  return {
    state: "degraded",
    services: [
      {
        family: "model",
        label: "Model",
        state: "degraded",
        message: "Qwen Coder download failed",
      },
    ],
    repairActions: [
      {
        repairId: "repair.model-download.qwen",
        family: "model",
        label: "Retry model download",
        reason: "Network interrupted during model download",
        mode: "executable",
      },
      {
        repairId: "repair.model-download.nemotron-disk",
        family: "model",
        label: "Free disk space before downloading NVIDIA Nemotron.",
        reason: "Insufficient disk space",
        mode: "guidance_only",
      },
    ],
    bundlePreview: {
      summary: "model download failed token=[REDACTED]",
      sizeBytes: 7000,
      maxBytes: 64000,
      redacted: true,
    },
    updateStatus: updateStatus(),
  };
}

function catalogStatus(): CatalogRefreshStatusResponse {
  return {
    state: "degraded",
    lastKnownGoodAvailable: true,
    degradedReasons: ["Using last-known-good compatibility catalog."],
    manualRefresh: { available: true, jobId: "registry.refresh.manual" },
  };
}

function localAudit() {
  return {
    scope: "local_single_user",
    records: [
      {
        sequence: 1,
        action: "provider_egress",
        outcome: "allowed",
        redactedDetails: "sent after approval token=[REDACTED]",
      },
      {
        sequence: 2,
        action: "tool_execution",
        outcome: "denied",
        redactedDetails: "terminal denied bearer [REDACTED]",
      },
    ],
    redactedExport:
      "provider_egress allowed: sent after approval token=[REDACTED]\ntool_execution denied: terminal denied bearer [REDACTED]",
  };
}

function snapshot(): DiagnosticsSnapshot {
  return {
    state: "degraded",
    services: [
      {
        family: "runtime",
        label: "Runtime",
        state: "degraded",
        message: "Ollama is stopped",
      },
      {
        family: "registry",
        label: "Registry",
        state: "degraded",
        message: "Offline mode",
      },
    ],
    repairActions: [
      {
        repairId: "repair.runtime",
        family: "runtime",
        label: "Restart local runner",
        reason: "Ollama is stopped",
        mode: "executable",
      },
      {
        repairId: "repair.registry",
        family: "registry",
        label: "Use cached catalog until network returns",
        reason: "Network unavailable",
        mode: "guidance_only",
      },
    ],
    bundlePreview: {
      summary: "Runtime stopped. token=[REDACTED]",
      sizeBytes: 9000,
      maxBytes: 64000,
      redacted: true,
    },
    updateStatus: updateStatus(),
  };
}

function doctorLintSnapshot(): DiagnosticsSnapshot {
  return {
    ...snapshot(),
    doctorLint: {
      source: "service_backed",
      mode: "lint",
      repairable: false,
      summary: { state: "blocked", blocked: 1, degraded: 0, ready: 0 },
      checks: [
        {
          checkId: "doctor.setup.runtime_model_ready",
          label: "Runtime and model setup",
          severity: "blocked",
          source: "runtime",
          message: "Setup must verify runtime and model before repository work.",
          fixHint: "Finish setup before repository work.",
          repairId: "repair.setup",
        },
      ],
    },
  } as unknown as DiagnosticsSnapshot;
}

test("shows security audit as read-only redacted posture", async () => {
  renderDiagnostics({}, securityAuditSnapshot());

  expect(await screen.findByRole("heading", { name: "Diagnostics" })).toBeInTheDocument();
  expect(screen.getByText("Security audit")).toBeInTheDocument();
  expect(screen.getByText("Plugin provenance")).toBeInTheDocument();
  expect(screen.getByText("Keep plugin execution disabled until provenance and sandbox gates exist.")).toBeInTheDocument();
  expect(screen.queryByText("sk-live-secret")).not.toBeInTheDocument();
});

function updateStatus() {
  return {
    channel: "dev" as const,
    currentVersion: "0.1.0",
    state: "disabled" as const,
    message: "Update checks are prepared but public release updates are not enabled yet.",
    canInstall: false,
  };
}

function securityAudit() {
  return {
    source: "service_backed",
    kind: "security_audit",
    redacted: true,
    exportSafe: true,
    summary: { state: "blocked", blocked: 1, degraded: 0, ready: 1 },
    remediationPolicy: "safe_remediation_routes_through_doctor_repair_contract",
    findings: [
      {
        checkId: "security.plugins.provenance",
        label: "Plugin provenance",
        severity: "blocked",
        source: "plugin_policy",
        message: "Executable plugin runtime is not certified.",
        fixHint: "Keep plugin execution disabled until provenance and sandbox gates exist.",
        repairId: "none",
        suppressed: false,
      },
    ],
  };
}

function securityAuditSnapshot(): DiagnosticsSnapshot {
  return {
    ...snapshot(),
    securityAudit: securityAudit(),
  } as unknown as DiagnosticsSnapshot;
}
