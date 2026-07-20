import { expect, test } from "@playwright/test";
import { desktopOnly, localApi, openWorkspaceThroughUi } from "./auditHelpers";

test("24.5 product: provider setup is secondary and every visible action is backend-owned or blocked", async ({
  page,
  request,
}, testInfo) => {
  desktopOnly(testInfo);
  await openWorkspaceThroughUi(page, request);
  await expect(page.getByRole("heading", { name: "Agent" })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Prompt" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Accounts" })).toHaveCount(0);

  const providers = await localApi(request, "GET", "/v1/providers");
  expect(providers.source).toBe("service_backed");
  expect(providers.providers[0].supportedAccountModes).toContain("api_key_billing");

  await page.getByText("Control center").click();
  await page.getByRole("button", { name: "Settings" }).click();
  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
  await page.getByRole("button", { name: "Providers" }).click();
  await expect(page.getByRole("heading", { name: "Accounts" })).toBeVisible();
  await expect(page.getByText("OpenAI").first()).toBeVisible();
  await expect(page.getByRole("heading", { name: "Connect account" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Connect account" })).toBeDisabled();

  await page.getByRole("textbox", { name: "API key" }).fill("sk-product-secret");
  await page.getByRole("button", { name: "Connect account" }).click();
  await expect(page.getByText("Credential reference connected")).toBeVisible();
  await expect(page.getByText("sk-product-secret")).toHaveCount(0);

  await page.getByRole("button", { name: "Test credential" }).click();
  await expect(page.getByText(/live provider calls are not certified yet/i)).toBeVisible();

  await page.getByRole("button", { name: "Remove credential" }).click();
  await expect(page.getByText("Credential reference removed from DesktopLab provider state.")).toBeVisible();

  await page.getByLabel("Account mode").selectOption("subscription_account");
  await expect(page.getByRole("textbox", { name: "API key" })).toHaveCount(0);
  await expect(page.getByText("Connect with your OpenAI Codex subscription through a local bridge.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Connect OpenAI Codex" })).toBeEnabled();
});
