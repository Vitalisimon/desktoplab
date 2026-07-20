import { expect, test } from "@playwright/test";
import { desktopOnly, localApi, selectSetup } from "./auditHelpers";

test("24.5 audit: runtime verify persists readiness only with evidence", async ({ request }, testInfo) => {
  desktopOnly(testInfo);
  await selectSetup(request);
  const verified = await localApi(request, "POST", "/v1/runtimes/runtime.ollama/verify", {});
  expect(["verified", "blocked"]).toContain(verified.verificationState);
  expect(verified.readinessEvidence.runtimeVerification.state).toBe(verified.verificationState);
});
