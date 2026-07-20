import { expect, test } from "@playwright/test";
import { artifactDir, desktopOnly } from "./auditHelpers";

test("24.6 product: settings appearance controls persist light and dark themes", async ({ page }, testInfo) => {
  desktopOnly(testInfo);
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await openSettings(page);

  await expect(page.getByRole("heading", { name: "Appearance" })).toBeVisible();
  await page.getByRole("radio", { name: "Dark" }).check();
  await expect(page.locator("html")).toHaveAttribute("data-theme-preference", "dark");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.screenshot({ path: `${artifactDir}/theme-selection-dark.png`, fullPage: true });

  await page.reload({ waitUntil: "domcontentloaded" });
  await openSettings(page);
  await expect(page.locator("html")).toHaveAttribute("data-theme-preference", "dark");
  await expect(page.getByRole("radio", { name: "Dark" })).toBeChecked();

  await page.getByRole("radio", { name: "Light" }).check();
  await expect(page.locator("html")).toHaveAttribute("data-theme-preference", "light");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await page.screenshot({ path: `${artifactDir}/theme-selection-light.png`, fullPage: true });
});

async function openSettings(page: import("@playwright/test").Page) {
  await page.getByText("Control center").click();
  await page.getByRole("button", { name: "Settings" }).click();
}
