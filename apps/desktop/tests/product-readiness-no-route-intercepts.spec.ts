import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "@playwright/test";

const appDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const productReadinessFiles = [
  "tests/local-api-product.spec.ts",
  "tests/fresh-user-product-flow.spec.ts",
  "tests/packaged-local-api.spec.ts",
  "tests/packaged-product-launch.spec.ts",
  "tests/product/fresh-first-launch.audit.spec.ts",
  "tests/product/hardware-warning.audit.spec.ts",
  "tests/product/catalog-selection.audit.spec.ts",
  "tests/product/runtime-missing.audit.spec.ts",
  "tests/product/runtime-present.audit.spec.ts",
  "tests/product/model-download.audit.spec.ts",
  "tests/product/model-catalog.product.spec.ts",
  "tests/product/setup-restart.audit.spec.ts",
  "tests/product/first-prompt.audit.spec.ts",
  "tests/product/file-drawer.audit.spec.ts",
  "tests/product/terminal-drawer.audit.spec.ts",
  "tests/product/provider-surface.audit.spec.ts",
  "tests/product/external-route-egress.product.spec.ts",
];

test("product readiness smoke files do not intercept critical local api routes", () => {
  for (const relativePath of productReadinessFiles) {
    const source = readFileSync(path.join(appDir, relativePath), "utf8");
    expect(source, `${relativePath} must use the live local API`).not.toMatch(/page\.route|route\.fulfill/);
  }
});
