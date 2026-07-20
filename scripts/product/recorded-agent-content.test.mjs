import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { requestedTextMatches } from "./recorded-agent-content.mjs";

test("requested text accepts one optional terminal newline only", () => {
  const expected = "# Release proof\n\nDesktopLab completed this task locally.\n";
  assert.equal(requestedTextMatches(expected, expected), true);
  assert.equal(requestedTextMatches(expected.slice(0, -1), expected), true);
  assert.equal(requestedTextMatches(`${expected}\n`, expected), false);
  assert.equal(requestedTextMatches(expected.replace("locally", "remotely"), expected), false);
  assert.equal(requestedTextMatches(null, expected), false);
});

test("recorded content helpers stay below focused line guards", () => {
  for (const [path, limit] of [
    ["scripts/product/recorded-agent-content.mjs", 30],
    ["scripts/product/recorded-agent-content.test.mjs", 40],
  ]) {
    const logical = readFileSync(path, "utf8").split("\n").filter((line) => line.trim()).length;
    assert.ok(logical <= limit, `${path} has ${logical} logical lines, limit ${limit}`);
  }
});
