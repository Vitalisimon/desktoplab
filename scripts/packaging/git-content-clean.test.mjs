import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { isGitContentClean } from "./git-content-clean.mjs";

test("content clean check rejects tracked, staged, and untracked changes", () => {
  const root = repository();
  assert.equal(isGitContentClean(root), true);

  fs.writeFileSync(path.join(root, "tracked.txt"), "changed\n");
  assert.equal(isGitContentClean(root), false);

  git(root, ["add", "tracked.txt"]);
  assert.equal(isGitContentClean(root), false);

  git(root, ["reset", "--hard", "HEAD"]);
  fs.writeFileSync(path.join(root, "untracked.txt"), "new\n");
  assert.equal(isGitContentClean(root), false);
});

function repository() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "desktoplab-git-clean-"));
  git(root, ["init", "--quiet"]);
  git(root, ["config", "user.email", "tests@desktoplab.local"]);
  git(root, ["config", "user.name", "DesktopLab Tests"]);
  fs.writeFileSync(path.join(root, "tracked.txt"), "original\n");
  git(root, ["add", "tracked.txt"]);
  git(root, ["commit", "--quiet", "-m", "fixture"]);
  return root;
}

function git(cwd, args) {
  execFileSync("git", args, { cwd, stdio: "ignore" });
}
