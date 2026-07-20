#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { syncDistribution, validateDistribution } from "./skill-distribution-core.mjs";

const manifestPath = resolve(argument("--manifest") ?? ".desktoplab-agent-distribution.json");
if (!existsSync(manifestPath)) {
  console.error("skill distribution manifest not found");
  process.exit(2);
}
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const report = process.argv.includes("--sync")
  ? syncDistribution(manifest, manifestPath)
  : validateDistribution(manifest, manifestPath);
console.log(JSON.stringify(report, null, 2));
process.exitCode = report.status === "pass" ? 0 : 1;

function argument(name) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : null;
}
