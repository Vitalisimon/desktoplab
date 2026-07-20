import { expect, test } from "@playwright/test";
import { desktopOnly, localApi, markSetupReady, selectSetup } from "./auditHelpers";

test("24.5 audit: setup completion is derived and survives reload", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);
  await selectSetup(request);
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Setup" })).toBeVisible();

  await markSetupReady(request);
  await page.reload();
  await expect(page.getByRole("heading", { name: /Open a project folder|Agent/ })).toBeVisible();

  const state = await localApi(request, "GET", "/v1/app/state");
  expect(state.setup.state).toBe("ready");
});
