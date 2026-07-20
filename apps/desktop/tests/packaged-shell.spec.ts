import { expect, test } from "@playwright/test";

const apiBase = "http://127.0.0.1:1421";
const token = "desktoplab-packaged-smoke-token";

test("packaged shell boots setup-first and keeps protected API boundary", async ({
  page,
  request,
}, testInfo) => {
  test.skip(testInfo.project.name !== "desktop", "packaged shell smoke mutates shared local API state");

  await expect
    .poll(async () => (await request.get(`${apiBase}/health`)).status(), { timeout: 5_000 })
    .toBe(200);

  const unauthorized = await request.get(`${apiBase}/v1/app/state`);
  expect(unauthorized.status()).toBe(401);

  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("desktoplab-root")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Setup" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Open Repository" })).toHaveCount(0);

  const authorized = await request.get(`${apiBase}/v1/app/state`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(authorized.status()).toBe(200);

  const state = await authorized.json();
  expect(state.setup.state).toBe("not_started");
  expect(state.currentWorkspace).toBeNull();
});
