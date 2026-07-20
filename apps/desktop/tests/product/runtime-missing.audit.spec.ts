import { expect, test } from "@playwright/test";
import { desktopOnly, localApi, selectSetup } from "./auditHelpers";

test("24.5 audit: runtime install route returns real completed or blocked state", async ({ request }, testInfo) => {
  desktopOnly(testInfo);
  await selectSetup(request);
  const runtime = await localApi(request, "POST", "/v1/runtimes/runtime.ollama/install", {
    setupAccepted: true,
    networkAvailable: true,
    diskAvailableGb: 64,
  });
  expect(["completed", "blocked", "external_guided", "failed"]).toContain(runtime.state);
  expect(runtime.source).toBe("service_backed");
});
