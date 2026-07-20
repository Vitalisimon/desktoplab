import { expect, test } from "@playwright/test";

const apiBase = "http://127.0.0.1:1421";
const token = "desktoplab-packaged-smoke-token";

test("packaged product launch uses setup-first UI and protected local API", async ({ page, request }, testInfo) => {
  test.skip(testInfo.project.name !== "desktop", "packaged product launch mutates shared local API state");

  const unauthorized = await request.get(`${apiBase}/v1/app/state`);
  expect(unauthorized.status()).toBe(401);

  await page.goto("/");
  await expect(page.getByTestId("desktoplab-root")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Setup" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Open Repository" })).toHaveCount(0);

  const authorized = await request.get(`${apiBase}/v1/app/state`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(authorized.status()).toBe(200);
  const state = await authorized.json();
  expect(state.setup.state).toBe("not_started");
});
