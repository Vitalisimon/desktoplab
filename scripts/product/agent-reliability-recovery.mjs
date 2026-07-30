#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

import {
  planAgentReliabilityRecovery,
  reaggregateAgentReliabilityRecovery,
} from "./agent-reliability-recovery-core.mjs";

const args = parseArgs(process.argv.slice(2));
if (!args.source || !existsSync(resolve(args.source))) {
  throw new Error("agent reliability recovery requires an existing --source report");
}
const sourcePath = resolve(args.source);
const outputPath = args.output ? resolve(args.output) : null;
if (outputPath === sourcePath) throw new Error("recovery output must not overwrite its source report");
const source = JSON.parse(readFileSync(sourcePath, "utf8"));
const result = args.replacement
  ? reaggregateAgentReliabilityRecovery(
      source,
      JSON.parse(readFileSync(resolve(args.replacement), "utf8")),
    )
  : planAgentReliabilityRecovery(source);
if (outputPath) {
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, `${JSON.stringify(result, null, 2)}\n`);
}
console.log(JSON.stringify(result, null, 2));
process.exitCode = ["ready", "pass"].includes(result.status) ? 0 : 1;

function parseArgs(argv) {
  const parsed = {};
  for (let index = 0; index < argv.length; index += 1) {
    const key = { "--source": "source", "--replacement": "replacement", "--output": "output" }[
      argv[index]
    ];
    if (!key || index + 1 >= argv.length) {
      throw new Error(`unknown or incomplete argument ${argv[index]}`);
    }
    parsed[key] = argv[++index];
  }
  return parsed;
}
