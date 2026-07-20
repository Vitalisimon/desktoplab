import { execFileSync } from "node:child_process";

export function isGitContentClean(cwd) {
  return gitDiffIsQuiet(cwd, ["diff", "--quiet", "--ignore-submodules", "--"])
    && gitDiffIsQuiet(cwd, ["diff", "--cached", "--quiet", "--ignore-submodules", "--"])
    && git(cwd, ["ls-files", "--others", "--exclude-standard"]) === "";
}

function gitDiffIsQuiet(cwd, args) {
  try {
    execFileSync("git", args, { cwd, stdio: "ignore" });
    return true;
  } catch (error) {
    if (error?.status === 1) return false;
    throw error;
  }
}

function git(cwd, args) {
  return execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
}
