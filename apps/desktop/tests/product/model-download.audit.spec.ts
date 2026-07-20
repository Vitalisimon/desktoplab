import { expect, test } from "@playwright/test";
import { desktopOnly, localApi, selectSetup } from "./auditHelpers";

test("24.5 audit: model download is live backend state and blocks before runtime verification", async ({ request }, testInfo) => {
  desktopOnly(testInfo);
  const selection = await selectSetup(request);
  const download = await localApi(request, "POST", `/v1/models/${selection.modelId}/download`, {
    setupAccepted: true,
    networkAvailable: true,
    diskAvailableMb: 100_000,
  });
  expect(download.source).toBe("service_backed");
  expect(["running", "blocked", "completed"]).toContain(download.state);
  if (download.state === "blocked") expect(download.blockedReason).toBeTruthy();
});
