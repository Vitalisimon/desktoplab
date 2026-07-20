#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import { runReliabilityCampaign } from "./agent-reliability-campaign-core.mjs";
import { versionedModuleBundle } from "./versioned-module-bundle.mjs";

function parseArgs(argv) {
  const args = { manifest: null, report: null, driver: process.env.DESKTOPLAB_RELIABILITY_DRIVER ?? null };
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === "--manifest") args.manifest = argv[++index];
    else if (argv[index] === "--report") args.report = argv[++index];
    else if (argv[index] === "--driver") args.driver = argv[++index];
  }
  return args;
}

async function driverExecutor(driver) {
  if (!driver || !existsSync(resolve(driver))) return null;
  const path = resolve(driver);
  const bundle = await versionedModuleBundle(path, resolve("scripts"));
  return {
    provenance: {
      kind: "versioned_external_driver",
      schemaVersion: 2,
      id: basename(path),
      sha256: bundle.entrySha256,
      bundleSha256: bundle.bundleSha256,
      sourceCount: bundle.sourceCount,
      sources: bundle.sources,
    },
    run: async (descriptor) => {
    const result = spawnSync(path, ["--run", JSON.stringify(descriptor)], {
      encoding: "utf8",
      timeout: descriptor.timeoutMs,
      env: { ...process.env, DESKTOPLAB_TEST_CONTROLS: "0" },
      maxBuffer: 16 * 1024 * 1024,
    });
    if (result.error?.code === "ETIMEDOUT") return { status: "timeout", reason: "campaign driver timeout" };
    if (result.status !== 0) return { status: "infrastructure_failure", reason: (result.stderr || result.stdout || "driver failed").trim() };
    return JSON.parse(result.stdout);
    },
  };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const args = parseArgs(process.argv.slice(2));
  const manifest = args.manifest && existsSync(resolve(args.manifest))
    ? JSON.parse(readFileSync(resolve(args.manifest), "utf8"))
    : null;
  const driver = await driverExecutor(args.driver);
  const report = await runReliabilityCampaign(manifest, { executor: driver?.run, executorProvenance: driver?.provenance });
  if (args.report) {
    mkdirSync(dirname(resolve(args.report)), { recursive: true });
    writeFileSync(resolve(args.report), `${JSON.stringify(report, null, 2)}\n`);
  }
  console.log(JSON.stringify(report, null, 2));
  process.exitCode = report.status === "pass" ? 0 : 1;
}
