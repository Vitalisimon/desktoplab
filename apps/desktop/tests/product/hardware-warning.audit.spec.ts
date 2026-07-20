import { expect, test } from "@playwright/test";
import { artifactDir, desktopOnly, localApi } from "./auditHelpers";

test("24.5 audit: hardware warnings come from live setup preview", async ({ page, request }, testInfo) => {
  desktopOnly(testInfo);
  await page.goto("/", { waitUntil: "domcontentloaded" });
  const preview = await localApi(request, "GET", "/v1/setup/preview");
  expect(preview.source).toBe("service_backed");
  expect(Array.isArray(preview.warnings)).toBe(true);
  expect(preview.hardware).toHaveProperty("unifiedMemoryGb");
  await page.screenshot({ path: `${artifactDir}/hardware-warning.png`, fullPage: true });
});
