// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { ExternalBackendsResponse, PluginsListResponse } from "../../api/types";
import { ExtensionsFeature } from "./ExtensionsFeature";

test("renders plugins and external backends with trust capability and ownership boundaries", async () => {
  renderExtensions();

  expect(await screen.findByRole("heading", { name: "Extensions" })).toBeInTheDocument();
  expect(screen.getByText("ACP bridge")).toBeInTheDocument();
  expect(screen.getAllByText("Unverified").length).toBeGreaterThan(0);
  expect(screen.getByText("agent.external")).toBeInTheDocument();
  expect(screen.getAllByText("Descriptor metadata is present, but executable plugin runtime is disabled until provenance, signature and sandbox gates exist.").length).toBeGreaterThan(0);
  expect(screen.getByText("unverified_plugin_requires_trust_approval")).toBeInTheDocument();
  expect(screen.getByText("runtime_registration_missing")).toBeInTheDocument();
  expect(screen.getByText("missing_signature")).toBeInTheDocument();
  expect(screen.getByText("not_registered")).toBeInTheDocument();
  expect(screen.getByText("disabled")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Enable plugin" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Install ACP bridge" })).not.toBeInTheDocument();
  expect(screen.getByText("Codex bridge")).toBeInTheDocument();
  expect(screen.getByText("External backend routes are contract-ready but not certified for execution yet.")).toBeInTheDocument();
  expect(screen.getByText("external_agent_bridge_v2_contract_ready")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Deny external route" })).not.toBeInTheDocument();
});

test("trust elevation calls backend while blocked external routes are display-only", async () => {
  const trustPlugin = vi.fn().mockResolvedValue({ status: "approved" });
  const approveExternalBackendRoute = vi.fn().mockResolvedValue({ status: "blocked", reason: "external_route_resolution_not_connected" });
  renderExtensions({ trustPlugin, approveExternalBackendRoute });

  await screen.findByRole("heading", { name: "Extensions" });
  fireEvent.click(screen.getByRole("button", { name: "Trust ACP bridge" }));

  await waitFor(() => expect(trustPlugin).toHaveBeenCalledWith("plugin.acp", { decision: "approve" }));
  expect(approveExternalBackendRoute).not.toHaveBeenCalled();
});

function renderExtensions(overrides: Partial<DesktopLabApiClient> = {}) {
  const client = {
    listPlugins: vi.fn().mockResolvedValue(plugins()),
    trustPlugin: vi.fn(),
    listExternalBackends: vi.fn().mockResolvedValue(backends()),
    approveExternalBackendRoute: vi.fn(),
    ...overrides,
  } as unknown as DesktopLabApiClient;

  return render(
    <AppProviders apiClient={client}>
      <ExtensionsFeature />
    </AppProviders>,
  );
}

function plugins(): PluginsListResponse {
  return {
    plugins: [
      {
        pluginId: "plugin.acp",
        displayName: "ACP bridge",
        status: "blocked",
        trust: "unverified",
        capabilities: ["agent.external"],
        descriptorState: "present",
        coldManifestState: "present",
        runtimeRegistration: "not_registered",
        installSource: "bundled_descriptor",
        integrityStatus: "missing_signature",
        executionEligibility: "disabled",
        provenance: {
          descriptorState: "present",
          coldManifestState: "present",
          runtimeRegistration: "not_registered",
          installSource: "bundled_descriptor",
          integrityStatus: "missing_signature",
          executionEligibility: "disabled",
          blockedReasons: ["runtime_registration_missing", "plugin_integrity_missing_signature"],
        },
        executionBoundary: {
          kind: "display-only",
          reason: "Descriptor metadata is present, but executable plugin runtime is disabled until provenance, signature and sandbox gates exist.",
        },
        blockedReasons: ["unverified_plugin_requires_trust_approval", "runtime_registration_missing", "plugin_integrity_missing_signature"],
        trustActions: [{ id: "trust", label: "Trust plugin", description: "Request approval after review." }],
      },
    ],
  };
}

function backends(): ExternalBackendsResponse {
  return {
    backends: [
      {
        backendId: "backend.codex",
        displayName: "Codex bridge",
        kind: "external",
        status: "blocked",
        capabilities: ["agent.events.stream"],
        routes: [{
          routeId: "route.codex",
          status: "blocked",
          reason: "External backend routes are contract-ready but not certified for execution yet.",
          blockedReasons: ["external_agent_bridge_v2_contract_ready"],
        }],
      },
    ],
  };
}
