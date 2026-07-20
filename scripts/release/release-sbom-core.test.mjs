import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { npmPackagesFromLock } from "./release-sbom-core.mjs";

test("release SBOM inventory derives scoped package names without installed manifests", () => {
  const inventory = npmPackagesFromLock({ packages: {
    "": { name: "desktoplab", version: "0.1.0", license: "Apache-2.0" },
    "node_modules/@scope/tool": { version: "2.0.0", license: "MIT" },
  } });
  assert.deepEqual(inventory.map(({ name, version }) => ({ name, version })), [
    { name: "@scope/tool", version: "2.0.0" },
    { name: "desktoplab", version: "0.1.0" },
  ]);
});

test("installed package metadata takes precedence over incomplete lock entries", () => {
  const inventory = npmPackagesFromLock(
    { packages: { "node_modules/pkg": { version: "1.0.0" } } },
    () => ({ name: "real-pkg", version: "1.1.0", license: "ISC" }),
  );
  assert.deepEqual(inventory, [{ name: "real-pkg", version: "1.1.0", license: "ISC" }]);
});

test("release SBOM helpers stay reviewable", () => {
  for (const [file, limit] of [
    ["scripts/release/release-sbom-core.mjs", 80],
    ["scripts/release/generate-release-sbom.mjs", 120],
  ]) {
    const logical = readFileSync(file, "utf8").split("\n").filter((line) => line.trim() && !line.trim().startsWith("//")).length;
    assert.ok(logical <= limit, `${file} has ${logical} logical lines, limit ${limit}`);
  }
});
