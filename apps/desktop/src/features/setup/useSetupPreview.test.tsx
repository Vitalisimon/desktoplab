// @vitest-environment jsdom
import { renderHook, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import { DesktopLabApiClient } from "../../api/client";
import type { HardwareFact, SetupPlanPreview } from "../../api/types";
import type { ApiTransport } from "../../api/transport";
import { useSetupPreview } from "./useSetupPreview";

test("exposes ready setup preview from backend contract", async () => {
  const { result } = renderHook(() => useSetupPreview(), {
    wrapper: ({ children }) => <AppProviders apiClient={clientFor(preview("ready"))}>{children}</AppProviders>,
  });

  await waitFor(() => expect(result.current.preview.data?.registryState).toBe("ready"));
  expect(result.current.isReady).toBe(true);
  expect(result.current.isBlocked).toBe(false);
});

test("exposes degraded last-known-good setup state", async () => {
  const { result } = renderHook(() => useSetupPreview(), {
    wrapper: ({ children }) => <AppProviders apiClient={clientFor(preview("degraded"))}>{children}</AppProviders>,
  });

  await waitFor(() => expect(result.current.isDegraded).toBe(true));
  expect(result.current.preview.data?.expectedLimitations).toContain(
    "compatibility catalog refresh unavailable; using last-known-good catalog",
  );
});

test("exposes blocked no-catalog setup state", async () => {
  const blocked = preview("blocked", []);
  const { result } = renderHook(() => useSetupPreview(), {
    wrapper: ({ children }) => <AppProviders apiClient={clientFor(blocked)}>{children}</AppProviders>,
  });

  await waitFor(() => expect(result.current.isBlocked).toBe(true));
  expect(result.current.isReady).toBe(false);
});

function clientFor(setupPreview: SetupPlanPreview) {
  return new DesktopLabApiClient({
    authToken: "local-test-token",
    transport: {
      async request(request) {
        if (request.path === "/v1/setup/preview") return { status: 200, body: setupPreview };
        return { status: 200, body: { startedJobIds: ["runtime.install:runtime.ollama"] } };
      },
    } satisfies ApiTransport,
  });
}

function preview(
  registryState: SetupPlanPreview["registryState"],
  runtimeRecommendations = [{ manifestId: "runtime.ollama", displayName: "Ollama", channel: "stable" as const }],
): SetupPlanPreview {
  return {
    registryState,
    hardware: {
      cpu: fact("CPU", "Apple M4 Pro"),
      ramGb: fact("RAM", 48),
      gpu: fact("GPU", null, "unknown"),
      vramGb: fact("VRAM", null, "unknown"),
      unifiedMemoryGb: fact("Unified memory", 48),
      operatingSystem: fact("OS", "macOS"),
      architecture: fact("Architecture", "arm64"),
      storageAvailableGb: fact("Storage", 900),
    },
    runtimeRecommendations,
    modelRecommendations: [{ manifestId: "model.qwen-coder", displayName: "Qwen Coder", channel: "stable" }],
    warnings: [],
    expectedLimitations:
      registryState === "degraded"
        ? ["compatibility catalog refresh unavailable; using last-known-good catalog"]
        : ["small local models are recommended"],
    hiddenReasons: [],
  };
}

function fact<T>(label: string, value: T | null, confidence: HardwareFact["confidence"] = "confirmed") {
  return { label, value, confidence };
}
