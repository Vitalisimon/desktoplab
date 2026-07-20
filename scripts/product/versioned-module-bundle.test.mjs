import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { versionedModuleBundle } from "./versioned-module-bundle.mjs";

test("module bundle follows and hashes transitive local imports", async () => {
  const root = mkdtempSync(join(tmpdir(), "desktoplab-module-bundle-"));
  mkdirSync(join(root, "nested"));
  const entry = join(root, "entry.mjs");
  const helper = join(root, "helper.mjs");
  writeFileSync(entry, 'import { helper } from "./helper.mjs";\nexport default helper;\n');
  writeFileSync(helper, 'import value from "./nested/value.mjs";\nexport const helper = value;\n');
  writeFileSync(join(root, "nested/value.mjs"), "export default 1;\n");

  const first = await versionedModuleBundle(entry, root);
  writeFileSync(join(root, "nested/value.mjs"), "export default 2;\n");
  const second = await versionedModuleBundle(entry, root);

  assert.equal(first.schemaVersion, 1);
  assert.equal(first.sourceCount, 3);
  assert.deepEqual(first.sources.map((source) => source.path), ["entry.mjs", "helper.mjs", "nested/value.mjs"]);
  assert.notEqual(first.bundleSha256, second.bundleSha256);
});

test("module bundle fails closed on imports outside its boundary", async () => {
  const parent = mkdtempSync(join(tmpdir(), "desktoplab-module-boundary-"));
  const root = join(parent, "root");
  mkdirSync(root);
  writeFileSync(join(parent, "outside.mjs"), "export default 1;\n");
  writeFileSync(join(root, "entry.mjs"), 'import value from "../outside.mjs";\nexport default value;\n');

  await assert.rejects(versionedModuleBundle(join(root, "entry.mjs"), root), /outside the attested module boundary/);
});

test("module bundle rejects computed dynamic imports", async () => {
  const root = mkdtempSync(join(tmpdir(), "desktoplab-module-dynamic-"));
  const entry = join(root, "entry.mjs");
  writeFileSync(entry, 'const name = "helper";\nexport default import(`./${name}.mjs`);\n');

  await assert.rejects(versionedModuleBundle(entry, root), /computed dynamic import/);
});
