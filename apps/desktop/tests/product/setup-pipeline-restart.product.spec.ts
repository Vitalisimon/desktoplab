import { expect, test } from "@playwright/test";
import { desktopOnly, localApi, markSetupReady, resetProductState, selectSetup } from "./auditHelpers";

test("24.5 product: setup pipeline survives reload before and after readiness", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);

  await resetProductState(request);
  await selectSetup(request);
  const inProgress = await localApi(request, "GET", "/v1/app/state");
  expect(inProgress.setup.state).toBe("in_progress");
  expect(inProgress.setupPipeline.state).toBe("runtime_installing");

  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Setup" })).toBeVisible();
  await expect(page.getByText("Setup progress")).toBeVisible();
  await expect(page.getByText("Runtime install")).toBeVisible();
  await expect(page.getByRole("button", { name: "Open Repository" })).toBeDisabled();

  await markSetupReady(request);
  await page.reload({ waitUntil: "domcontentloaded" });

  const ready = await localApi(request, "GET", "/v1/app/state");
  expect(ready.setup.state).toBe("ready");
  expect(ready.setupPipeline.state).toBe("ready");
  await expect(page.getByRole("heading", { name: /Open a project folder|Agent/ })).toBeVisible();
});
