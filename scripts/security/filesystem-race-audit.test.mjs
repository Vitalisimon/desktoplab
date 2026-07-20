import assert from "node:assert/strict";
import test from "node:test";

import { auditFilesystemSurfaces, surfaces } from "./filesystem-race-audit.mjs";

test("every packaged agent filesystem surface has an owned race classification", () => {
  const records = auditFilesystemSurfaces();

  assert.deepEqual(records.map((record) => record.id), [
    "read",
    "write",
    "patch",
    "multi_file_patch",
    "terminal",
    "git",
  ]);
  assert.ok(records.every((record) => record.audited), JSON.stringify(records, null, 2));
  assert.ok(records.every((record) => ["executor", "platform", "policy"].includes(record.owner)));
  assert.equal(new Set(surfaces.map((record) => record.id)).size, surfaces.length);
});

test("archive move and delete are absent from the current model-facing tool schema", async () => {
  const { readFile } = await import("node:fs/promises");
  const source = await readFile(new URL("../../crates/desktoplab-agent-engine/src/tool_schema.rs", import.meta.url), "utf8");

  for (const undeclaredMutation of ["delete_file", "move_file", "extract_archive"]) {
    assert.equal(source.includes(undeclaredMutation), false);
  }
});
