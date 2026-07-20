import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { gitTreeState } from "./git-tree-state.mjs";

test("tree state follows content and untracked files instead of file timestamps", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "desktoplab-tree-state-"));
  run(root, ["init"]);
  run(root, ["config", "user.name", "DesktopLab Test"]);
  run(root, ["config", "user.email", "desktoplab-test@example.invalid"]);
  fs.writeFileSync(path.join(root, ".gitignore"), "ignored/\n");
  fs.writeFileSync(path.join(root, "Cargo.toml"), "[package]\nname = \"fixture\"\n");
  run(root, ["add", ".gitignore", "Cargo.toml"]);
  run(root, ["commit", "-m", "fixture"]);

  fs.writeFileSync(path.join(root, "Cargo.toml"), "[package]\nname = \"fixture\"\n");
  assert.equal(gitTreeState(root), "clean");

  fs.writeFileSync(path.join(root, "Cargo.toml"), "[package]\nname = \"changed\"\n");
  assert.equal(gitTreeState(root), "dirty");
  run(root, ["restore", "Cargo.toml"]);

  fs.writeFileSync(path.join(root, "new.txt"), "new\n");
  assert.equal(gitTreeState(root), "dirty");
  fs.rmSync(path.join(root, "new.txt"));
  fs.mkdirSync(path.join(root, "ignored"));
  fs.writeFileSync(path.join(root, "ignored", "generated.txt"), "generated\n");
  assert.equal(gitTreeState(root), "clean");
});

function run(root, args) {
  return execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
}
