import { execFileSync } from "node:child_process";
import process from "node:process";

import { normalizeRepositoryOrigin } from "../release/public-release-source-core.mjs";

const CANONICAL_PUBLIC_REPOSITORY = "github.com/vitalisimon/desktoplab";

export function defaultRepositoryVisibilityMode(origin) {
  return normalizeRepositoryOrigin(origin) === CANONICAL_PUBLIC_REPOSITORY
    ? "public-export"
    : "internal";
}

export function currentRepositoryVisibilityMode(cwd = process.cwd()) {
  try {
    const origin = execFileSync("git", ["remote", "get-url", "origin"], {
      cwd,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    return defaultRepositoryVisibilityMode(origin);
  } catch {
    return "internal";
  }
}
