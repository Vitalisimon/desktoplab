import assert from "node:assert/strict";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { currentRepositoryVisibilityMode } from "./repository-visibility-mode.mjs";

test("a checkout without a readable canonical origin remains internal", () => {
  const directory = mkdtempSync(join(tmpdir(), "desktoplab-repository-mode-"));
  assert.equal(currentRepositoryVisibilityMode(directory), "internal");
});
