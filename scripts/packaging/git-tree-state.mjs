import { execFileSync, spawnSync } from "node:child_process";

export function gitTreeState(root) {
  const diff = spawnSync("git", ["diff", "--quiet", "HEAD", "--"], {
    cwd: root,
    stdio: "ignore",
  });
  if (diff.error) throw diff.error;
  if (diff.status !== 0 && diff.status !== 1) {
    throw new Error(`git diff failed with status ${diff.status}`);
  }

  const untracked = execFileSync(
    "git",
    ["ls-files", "--others", "--exclude-standard"],
    { cwd: root, encoding: "utf8" },
  ).trim();
  return diff.status === 1 || untracked ? "dirty" : "clean";
}
