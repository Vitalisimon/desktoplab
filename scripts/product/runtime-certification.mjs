#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

import {
  assessRuntimeCertification,
  runtimeCertificationTemplate,
} from "./runtime-certification-core.mjs";

const args = parseArgs(process.argv.slice(2));
if (!args.expected || !existsSync(resolve(args.expected))) {
  throw new Error("runtime certification requires an existing --expected contract");
}
const expected = JSON.parse(readFileSync(resolve(args.expected), "utf8"));
let result;
if (args.template) {
  result = runtimeCertificationTemplate(expected, args.evidenceKind);
  write(args.template, result);
} else {
  const evidence =
    args.evidence && existsSync(resolve(args.evidence))
      ? JSON.parse(readFileSync(resolve(args.evidence), "utf8"))
      : null;
  result = assessRuntimeCertification(expected, evidence);
  if (args.report) write(args.report, result);
}
console.log(JSON.stringify(result, null, 2));
if (!args.template) process.exitCode = result.status === "pass" ? 0 : 1;

function write(path, value) {
  const target = resolve(path);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, `${JSON.stringify(value, null, 2)}\n`);
}

function parseArgs(argv) {
  const parsed = { evidenceKind: "live_installed_app" };
  const names = {
    "--expected": "expected",
    "--evidence": "evidence",
    "--report": "report",
    "--template": "template",
    "--evidence-kind": "evidenceKind",
  };
  for (let index = 0; index < argv.length; index += 1) {
    const key = names[argv[index]];
    if (!key || index + 1 >= argv.length) throw new Error(`unknown argument ${argv[index]}`);
    parsed[key] = argv[++index];
  }
  return parsed;
}
