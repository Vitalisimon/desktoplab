import { expect, test } from "@playwright/test";

const apiBase = "http://127.0.0.1:1421";
const token = "desktoplab-packaged-smoke-token";

test("packaged local api requires auth and exposes app state with explicit token", async ({ request }, testInfo) => {
  test.skip(testInfo.project.name !== "desktop", "packaged local API smoke uses shared backend state");

  await expect
    .poll(async () => (await request.get(`${apiBase}/health`)).status(), { timeout: 5_000 })
    .toBe(200);

  const unauthorized = await request.get(`${apiBase}/v1/app/state`);
  expect(unauthorized.status()).toBe(401);

  const runtimeInstall = await request.post(`${apiBase}/v1/runtimes/runtime.ollama/install`, {
    data: {},
  });
  expect(runtimeInstall.status()).toBe(401);

  const modelDownload = await request.post(`${apiBase}/v1/models/model.auth-probe/download`, {
    data: { runtimeId: "runtime.ollama" },
  });
  expect(modelDownload.status()).toBe(401);

  const authorized = await request.get(`${apiBase}/v1/app/state`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(authorized.status()).toBe(200);

  const body = await authorized.json();
  expect(body.setup.state).toBe("not_started");
});
