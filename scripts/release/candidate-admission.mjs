#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, relative, resolve } from "node:path";
import process from "node:process";

import { hashArtifact, sha256File } from "../packaging/artifact-provenance-core.mjs";
import {
  admitCandidateSource,
  bindCandidatePayload,
  rejectCandidate,
  transitionCandidate,
  verifyCandidate,
} from "./candidate-admission-core.mjs";

const args = parseArgs(process.argv.slice(2));
const root = resolve(execFileSync("git", ["rev-parse", "--show-toplevel"], { encoding: "utf8" }).trim());
const source = JSON.parse(execFileSync("node", ["scripts/release/public-release-source.mjs"], { cwd: root, encoding: "utf8" }));
const tauri = JSON.parse(readFileSync(resolve(root, "apps/desktop/src-tauri/tauri.conf.json"), "utf8"));
const lockfiles = currentLockfiles(root);

if (args.command === "admit-source") {
  const candidate = admitCandidateSource({ source, version: tauri.version, channel: args.channel, lockfiles });
  writeJson(args.output ?? "dist/release/candidate/admission.json", candidate);
  console.log(JSON.stringify(candidate, null, 2));
} else if (args.command === "bind-payload") {
  requireFile(args.candidate, "candidate admission");
  requireFile(args.app, "candidate app");
  const candidate = JSON.parse(readFileSync(resolve(args.candidate), "utf8"));
  const payload = hashArtifact(resolve(args.app));
  const bound = bindCandidatePayload(candidate, {
    platform: "macos-aarch64",
    relativePath: relative(root, resolve(args.app)),
    ...payload,
  });
  writeJson(args.output ?? "dist/release/candidate/admission.json", bound);
  console.log(JSON.stringify(bound, null, 2));
} else if (args.command === "verify") {
  requireFile(args.candidate, "candidate admission");
  const candidate = JSON.parse(readFileSync(resolve(args.candidate), "utf8"));
  const payload = args.app ? hashArtifact(resolve(args.app)) : null;
  const report = verifyCandidate({ candidate, source, lockfiles, payload });
  console.log(JSON.stringify(report, null, 2));
  if (report.status !== "pass") process.exitCode = 1;
} else if (args.command === "transition") {
  requireFile(args.candidate, "candidate admission");
  requireFile(args.evidence, "transition evidence");
  const candidate = JSON.parse(readFileSync(resolve(args.candidate), "utf8"));
  const evidence = JSON.parse(readFileSync(resolve(args.evidence), "utf8"));
  const transitioned = transitionCandidate(candidate, { to: args.to, evidence });
  writeJson(args.output ?? args.candidate, transitioned);
  console.log(JSON.stringify(transitioned, null, 2));
} else if (args.command === "reject") {
  requireFile(args.candidate, "candidate admission");
  const candidate = JSON.parse(readFileSync(resolve(args.candidate), "utf8"));
  const rejected = rejectCandidate(candidate, { reason: args.reason });
  writeJson(args.output ?? args.candidate, rejected);
  console.log(JSON.stringify(rejected, null, 2));
} else {
  throw new Error("candidate admission requires admit-source, bind-payload, verify, transition or reject");
}

function currentLockfiles(repoRoot) {
  return ["Cargo.lock", "package-lock.json", "apps/desktop/src-tauri/Cargo.lock"].map((path) => ({
    path,
    sha256: sha256File(resolve(repoRoot, path)),
  }));
}

function writeJson(path, value) {
  const target = resolve(path);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, `${JSON.stringify(value, null, 2)}\n`);
}

function requireFile(path, label) {
  if (!path || !existsSync(resolve(path))) throw new Error(`${label} is missing`);
}

function parseArgs(values) {
  const parsed = { command: values[0], channel: "beta", output: null };
  for (let index = 1; index < values.length; index += 1) {
    if (values[index] === "--channel") parsed.channel = values[++index];
    else if (values[index] === "--output") parsed.output = values[++index];
    else if (values[index] === "--candidate") parsed.candidate = values[++index];
    else if (values[index] === "--app") parsed.app = values[++index];
    else if (values[index] === "--to") parsed.to = values[++index];
    else if (values[index] === "--evidence") parsed.evidence = values[++index];
    else if (values[index] === "--reason") parsed.reason = values[++index];
  }
  return parsed;
}
