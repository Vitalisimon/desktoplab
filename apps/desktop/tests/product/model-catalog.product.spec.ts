import { expect, test } from "@playwright/test";
import { desktopOnly, localApi, markSetupReady } from "./auditHelpers";

test("model catalog and composer routes come from the same backend-owned truth", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);

  await markSetupReady(request);
  const inventory = await localApi(request, "GET", "/v1/models");
  const downloadable = inventory.models.find((model: { installState: string }) => model.installState === "downloadable");
  const blocked = inventory.models.find((model: { installState: string }) => model.installState === "blocked");
  const readyModels = inventory.models.filter(
    (model: { installState: string; compatibility: string }) => model.installState === "installed" && model.compatibility === "ready",
  );

  expect(inventory.models.length).toBeGreaterThan(3);
  expect(downloadable, "catalog should expose at least one model that can be downloaded on this host").toBeTruthy();
  expect(blocked, "catalog should expose blocked heavy models honestly").toBeTruthy();

  await page.goto("/", { waitUntil: "domcontentloaded" });
  await page.getByText("Control center").click();
  await page.getByRole("button", { name: "Settings" }).click();
  await page.getByRole("button", { name: "Model catalog and setup" }).click();

  await expect(page.getByRole("heading", { name: "Local models" })).toBeVisible();
  const modelSelect = page.getByRole("combobox", { name: "Choose a model to download" });
  await expect(modelSelect).toBeVisible();
  await modelSelect.selectOption(blocked.modelId);
  await expect(page.getByText(blocked.blockedReason).first()).toBeVisible();
  await expect(page.getByText("Bundled catalog").first()).toBeVisible();

  const routeOptions = await localApi(request, "GET", "/v1/routing/options");
  const localRoutes = routeOptions.options.filter((route: { routeId: string }) => route.routeId.startsWith("route.local."));
  const availableLocalRoutes = localRoutes.filter((route: { status: string }) => route.status === "available");
  expect(availableLocalRoutes).toHaveLength(readyModels.length);
  expect(localRoutes.some((route: { modelId?: string }) => route.modelId === blocked.modelId)).toBe(false);
  expect(routeOptions.options.length).toBeGreaterThanOrEqual(localRoutes.length);
});
