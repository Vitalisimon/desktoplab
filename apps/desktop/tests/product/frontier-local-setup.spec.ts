import { expect, test } from "@playwright/test";
import { localApi, resetProductState } from "./auditHelpers";

test("high-capacity setup discovers a real local model route without exposing ports by default", async ({ page, request }, testInfo) => {
  test.skip(testInfo.project.name !== "desktop", "shared high-capacity setup state uses one backend");
  await resetProductState(request);
  await page.goto("/", { waitUntil: "domcontentloaded" });

  await expect(page.getByRole("heading", { name: "High-capacity local setup" })).toBeVisible();
  await expect(page.getByText("High-memory AI workstation")).toBeVisible();
  await expect(page.getByText("NVIDIA NIM").first()).toBeVisible();
  await expect(page.getByText("Live certification remains separate")).toBeVisible();
  await expect(page.getByLabel("Local or private network address")).not.toBeVisible();

  await page.getByText("Connection details").click();
  await expect(page.getByLabel("Local or private network address")).toHaveValue("http://127.0.0.1:18000");

  await page.getByRole("button", { name: "Check local runner" }).click();
  await expect(page.getByLabel("Available model")).toHaveValue("frontier-test-model-600b");
  await page.getByRole("button", { name: "Use this local route" }).click();
  await expect(page.getByRole("heading", { name: "Open a project folder" })).toBeVisible();

  const routes = await localApi(request, "GET", "/v1/routing/options");
  expect(routes.selectedRouteId).toBe("route.high-end-local");
  expect(routes.options).toEqual(expect.arrayContaining([
    expect.objectContaining({ routeId: "route.high-end-local", backendId: "backend.high-end-local", status: "available" }),
  ]));
});
