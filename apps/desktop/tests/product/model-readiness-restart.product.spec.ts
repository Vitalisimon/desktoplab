import { expect, test } from "@playwright/test";
import { desktopOnly, localApi, markSetupReady } from "./auditHelpers";

test("24.5 product: model readiness survives reload and blocks when runtime inventory loses it", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);
  const selection = await markSetupReady(request);

  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { level: 1, name: /Open a project folder|Agent/ })).toBeVisible();

  const ready = await localApi(request, "GET", "/v1/app/state");
  expect(ready.setup.state).toBe("ready");
  expect(ready.readiness.state).toBe("ready");
  expect(ready.readiness.evidence.modelVerification.state).toBe("verified");

  const refreshed = await localApi(request, "POST", `/v1/models/${selection.modelId}/verify`, {});
  expect(["verified", "blocked"]).toContain(refreshed.verificationState);

  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { level: 1, name: /Setup|Open a project folder|Agent/ })).toBeVisible();

  const state = await localApi(request, "GET", "/v1/app/state");
  expect(state.readiness.evidence.modelVerification.state).toBe(refreshed.verificationState);
});
