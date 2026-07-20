import { DesktopLabApiClient } from "./client";
import type { ApiTransport, TransportRequest } from "./transport";

test("maps diagnostics and repair methods to local api paths", async () => {
  const requests: TransportRequest[] = [];
  const client = new DesktopLabApiClient({
    authToken: "local-test-token",
    transport: transportFor(requests),
  });

  await client.diagnostics();
  await client.diagnosticsExport();
  await client.runDiagnosticRepair("repair.runtime");

  expect(requests.map((request) => `${request.method} ${request.path}`)).toEqual([
    "GET /v1/diagnostics",
    "GET /v1/diagnostics/export",
    "POST /v1/diagnostics/repairs/repair.runtime/run",
  ]);
});

function transportFor(requests: TransportRequest[]): ApiTransport {
  return {
    async request(request) {
      requests.push(request);
      return { status: 200, body: responseFor(request.path) };
    },
  };
}

function responseFor(path: string) {
  if (path === "/v1/diagnostics") {
    return {
      state: "degraded",
      bundlePreview: {
        summary: "Runtime stopped. token=[REDACTED]",
        sizeBytes: 9000,
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
      services: [],
      repairActions: [],
    };
  }
  if (path === "/v1/diagnostics/export") return { manifest: { kind: "desktoplab.diagnostics.export", schemaVersion: 1 } };
  if (path.endsWith("/run")) return { status: "blocked", repairId: "repair.runtime", reason: "diagnostic_repair_not_connected" };
  return {};
}
