import { expect, test } from "@playwright/test";
import { desktopOnly, localApi, selectSetup } from "./auditHelpers";

test("24.5 product: runtime install exposes blocked, running evidence and phase events", async ({ request }, testInfo) => {
  desktopOnly(testInfo);
  await selectSetup(request);

  const offline = await localApi(request, "POST", "/v1/runtimes/runtime.ollama/install", {
    setupAccepted: true,
    networkAvailable: false,
    diskAvailableGb: 64,
  });
  expect(offline.state).toBe("blocked");
  expect(offline.retryClass).toBe("offline");
  expect(offline.blockedReason).toBe("network unavailable");

  const install = await localApi(request, "POST", "/v1/runtimes/runtime.ollama/install", {
    setupAccepted: true,
    networkAvailable: true,
    diskAvailableGb: 64,
  });
  expect(install.source).toBe("service_backed");
  expect(["completed", "blocked", "external_guided", "failed"]).toContain(install.state);
  expect(install.executionEvidence).toContain("ollama");
  expect(install.jobId).toMatch(/^job\./);

  const replay = await localApi(request, "GET", "/v1/events/replay");
  const payloads = replay.frames.map((frame: { payload: string }) => frame.payload).join("\n");
  expect(payloads).toContain('"kind":"runtime.install"');
  expect(payloads).toContain('"phase":"detect"');
  expect(payloads).toContain('"phase":"download"');
  expect(payloads).toContain('"nextAction":');
});

test("24.5 product: installed runtime proof updates readiness through verify route", async ({ request }, testInfo) => {
  desktopOnly(testInfo);
  await selectSetup(request);

  const verified = await localApi(request, "POST", "/v1/runtimes/runtime.ollama/verify", {});

  expect(["verified", "blocked"]).toContain(verified.verificationState);
  expect(verified.readinessEvidence.runtimeVerification.state).toBe(verified.verificationState);
});
