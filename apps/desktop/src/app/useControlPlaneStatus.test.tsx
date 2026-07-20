// @vitest-environment jsdom
import { renderHook, waitFor } from "@testing-library/react";
import { AppProviders } from "./AppProviders";
import { useControlPlaneStatus } from "./useControlPlaneStatus";
import { DesktopLabApiClient } from "../api/client";
import type { ApiTransport } from "../api/transport";

test("reads backend health and readiness through the api client", async () => {
  const paths: string[] = [];
  const client = new DesktopLabApiClient({
    authToken: "local-test-token",
    transport: {
      async request(request) {
        paths.push(request.path);
        if (request.path === "/health") return { status: 200, body: { status: "healthy" } };
        return {
          status: 200,
          body: { state: "degraded", degradedReasons: ["registry:refresh_unavailable"] },
        };
      },
    } satisfies ApiTransport,
  });

  const { result } = renderHook(() => useControlPlaneStatus(), {
    wrapper: ({ children }) => <AppProviders apiClient={client}>{children}</AppProviders>,
  });

  await waitFor(() => expect(result.current.health.data?.status).toBe("healthy"));
  expect(result.current.isDegraded).toBe(true);
  expect(paths).toEqual(expect.arrayContaining(["/health", "/v1/readiness"]));
});
