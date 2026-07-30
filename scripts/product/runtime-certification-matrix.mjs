#!/usr/bin/env node
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

import { hashArtifact } from "../packaging/artifact-provenance-core.mjs";
import {
  assessRuntimeCertificationMatrix,
  requiredRuntimeRoutes,
} from "./runtime-certification-matrix-core.mjs";

const args = parseArgs(process.argv.slice(2));
for (const name of ["candidate", "app", "report", ...requiredRuntimeRoutes.map(({ key }) => key)]) {
  if (!args[name]) throw new Error(`runtime certification matrix requires --${name.replaceAll("_", "-")}`);
}
for (const path of [args.candidate, args.app, ...requiredRuntimeRoutes.map(({ key }) => args[key])]) {
  if (!existsSync(resolve(path))) throw new Error(`runtime certification matrix path is missing: ${path}`);
}

const reports = {};
const reportDigests = {};
for (const { key } of requiredRuntimeRoutes) {
  const bytes = readFileSync(resolve(args[key]));
  reports[key] = JSON.parse(bytes.toString("utf8"));
  reportDigests[key] = `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}
const result = assessRuntimeCertificationMatrix({
  candidate: JSON.parse(readFileSync(resolve(args.candidate), "utf8")),
  appHash: `sha256:${hashArtifact(resolve(args.app)).sha256}`,
  sourceHead: gitHead(),
  reports,
  reportDigests,
});
write(args.report, result);
console.log(JSON.stringify(result, null, 2));
process.exitCode = result.status === "pass" ? 0 : 1;

function gitHead() {
  const result = spawnSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" });
  if (result.status !== 0) throw new Error("runtime certification matrix cannot read source HEAD");
  return result.stdout.trim();
}

function write(path, value) {
  const target = resolve(path);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, `${JSON.stringify(value, null, 2)}\n`);
}

function parseArgs(argv) {
  const parsed = {};
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token.startsWith("--") || index + 1 >= argv.length) {
      throw new Error(`unknown or incomplete argument ${token}`);
    }
    parsed[token.slice(2).replaceAll("-", "_")] = argv[++index];
  }
  return parsed;
}
