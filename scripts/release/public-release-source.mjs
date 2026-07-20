#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import process from "node:process";

import { assessPublicReleaseSource } from "./public-release-source-core.mjs";

const report = assessPublicReleaseSource({
  origin: git(["remote", "get-url", "origin"], true),
  head: git(["rev-parse", "HEAD"]),
  publishedHead: git(["ls-remote", "--refs", "origin", "refs/heads/main"], true).split(/\s+/)[0] || null,
  treeState: git(["status", "--porcelain=v1"]) ? "dirty" : "clean",
  trackedPaths: git(["ls-files", "-z"]).split("\0").filter(Boolean),
  canonicalRepository: process.env.DESKTOPLAB_CANONICAL_PUBLIC_REPOSITORY,
});

console.log(JSON.stringify(report, null, 2));
if (report.status !== "pass") process.exitCode = 1;

function git(args, optional = false) {
  try {
    return execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
  } catch (error) {
    if (optional) return "";
    throw error;
  }
}
