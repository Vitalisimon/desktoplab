import assert from "node:assert/strict";
import test from "node:test";
import { hasUnexpectedSkip } from "./beta-gauntlet-output.mjs";

test("test titles containing skipped do not override a zero-skip TAP summary", () => {
  const output = [
    "✔ skipped verification remains explicit (0.2ms)",
    "ℹ tests 1",
    "ℹ pass 1",
    "ℹ skipped 0",
  ].join("\n");
  assert.equal(hasUnexpectedSkip(output), false);
});

test("a positive TAP skip count fails closed", () => {
  const output = ["ℹ tests 2", "ℹ pass 1", "ℹ skipped 1"].join("\n");
  assert.equal(hasUnexpectedSkip(output), true);
});

test("explicit non-TAP skips remain blocked except documented narrow skips", () => {
  assert.equal(hasUnexpectedSkip("runtime check skipped because model is absent"), true);
  assert.equal(hasUnexpectedSkip("narrow skipped by design"), false);
  assert.equal(
    hasUnexpectedSkip("Product surface audit skipped: private inventory is not present in this checkout"),
    true,
  );
});
