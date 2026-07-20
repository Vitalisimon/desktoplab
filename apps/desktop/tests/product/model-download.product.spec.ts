import { expect, test } from "@playwright/test";
import { desktopOnly, localApi, selectSetup } from "./auditHelpers";

test("24.5 product: model download states are backend-owned from host state to verified", async ({ request }, testInfo) => {
  desktopOnly(testInfo);
  const selection = await selectSetup(request);

  const runtimeBlocked = await localApi(request, "POST", `/v1/models/${selection.modelId}/download`, {
    setupAccepted: true,
    networkAvailable: true,
    diskAvailableMb: 100_000,
  });
  expect(runtimeBlocked.source).toBe("service_backed");
  expect(["blocked", "running", "completed"]).toContain(runtimeBlocked.state);
  if (runtimeBlocked.state === "blocked") {
    expect(runtimeBlocked.blockedReason).toBe("runtime_not_verified");
  }

  await localApi(request, "POST", `/v1/runtimes/${selection.runtimeId}/verify`, {});

  const diskBlocked = await localApi(request, "POST", `/v1/models/${selection.modelId}/download`, {
    setupAccepted: true,
    networkAvailable: true,
    diskAvailableMb: 128,
  });
  expect(diskBlocked.state).toBe("blocked");
  expect(diskBlocked.blockedReason).toBe("insufficient disk");
  expect(diskBlocked.requiredDiskMb).toBeGreaterThan(diskBlocked.availableDiskMb);

  const started = await localApi(request, "POST", `/v1/models/${selection.modelId}/download`, {
    setupAccepted: true,
    networkAvailable: true,
    diskAvailableMb: 100_000,
  });
  expect(["running", "completed"]).toContain(started.state);
  if (started.state === "running") {
    expect(started.progressPercent).toBe(5);
    expect(started.executionEvidence).toMatch(/^ollama pull \S+$/);
    expect(started.jobId).toMatch(/^job\./);
  } else {
    expect(started.executionEvidence).toMatch(/existing model detected|^ollama pull \S+$/);
  }

  const replay = await localApi(request, "GET", "/v1/events/replay");
  const payloads = replay.frames.map((frame: { payload: string }) => frame.payload).join("\n");
  expect(payloads).toContain('"kind":"model.download"');
  expect(payloads).toMatch(/"state":"(running|completed)"/);

  const verified = await localApi(request, "POST", `/v1/models/${selection.modelId}/verify`, {});
  expect(["verified", "blocked"]).toContain(verified.verificationState);
  expect(verified.readinessEvidence.modelVerification.state).toBe(verified.verificationState);
});
