#!/usr/bin/env node
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { hashArtifact, readEmbeddedBuild } from "../../packaging/artifact-provenance-core.mjs";
import { createReliabilityManifest, deriveReliabilityConfiguration, reliabilityDescriptors, reliabilityDriverPlan } from "../installed-agent-reliability-recording-core.mjs";
import { macosAccessibilityUi, stopExistingDesktopLab } from "./macos-installed-agent-ui.mjs";
import { installedAgentUiWaitModulePath } from "./macos-installed-agent-ui-wait.mjs";
import { macosAccessibilityDriverEvidence } from "./macos-native-accessibility.mjs";
import { recordReliabilityRun } from "./macos-installed-agent-reliability-run.mjs";
import { collectReliabilityRuns } from "./reliability-run-collector.mjs";

const driverPath = fileURLToPath(import.meta.url);
const repoRoot = resolve(dirname(driverPath), "../../..");
const defaultStatePath = join(homedir(), ".config/desktoplab/desktoplab.sqlite");
const pressureHelperPath = fileURLToPath(new URL("./memory-pressure-helper.mjs", import.meta.url));

export function parseReliabilityArgs(argv) {
  const args = { app: null, candidate: null, outputRoot: null, manifest: null, catalog: null, seedState: defaultStatePath, printPlan: false, resume: false };
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === "--app") args.app = argv[++index];
    else if (argv[index] === "--candidate") args.candidate = argv[++index];
    else if (argv[index] === "--output-root") args.outputRoot = argv[++index];
    else if (argv[index] === "--manifest") args.manifest = argv[++index];
    else if (argv[index] === "--catalog") args.catalog = argv[++index];
    else if (argv[index] === "--seed-state") args.seedState = argv[++index];
    else if (argv[index] === "--print-plan") args.printPlan = true;
    else if (argv[index] === "--resume") args.resume = true;
    else throw new Error(`unknown argument ${argv[index]}`);
  }
  return args;
}

export function reliabilityUiDriverEvidence() {
  return macosAccessibilityDriverEvidence(driverPath, [fileURLToPath(new URL("./macos-installed-agent-ui.mjs", import.meta.url)), installedAgentUiWaitModulePath, fileURLToPath(new URL("./macos-installed-agent-reliability-run.mjs", import.meta.url)), fileURLToPath(new URL("./reliability-run-collector.mjs", import.meta.url)), pressureHelperPath]);
}

export async function runMacosReliabilityUi(args, dependencies = {}) {
  if ((dependencies.platform ?? process.platform) !== "darwin") throw new Error("installed reliability UI driver requires macOS");
  for (const name of ["app", "candidate", "outputRoot", "manifest", "catalog"]) if (!args[name]) throw new Error(`missing --${name.replace(/[A-Z]/g, (value) => `-${value.toLowerCase()}`)}`);
  const appPath = realpathSync(args.app);
  if (!["/Applications/DesktopLab.app", join(homedir(), "Applications/DesktopLab.app")].includes(appPath)) throw new Error("reliability driver requires a natively installed DesktopLab.app");
  const executablePath = join(appPath, "Contents/MacOS/desktoplab-desktop");
  if (!existsSync(executablePath)) throw new Error("installed DesktopLab executable missing");
  const candidate = JSON.parse(readFileSync(resolve(args.candidate), "utf8"));
  const appHash = `sha256:${hashArtifact(appPath).sha256}`;
  const appBuild = readEmbeddedBuild(appPath);
  if (candidate.source?.commit !== appBuild.commitSha || `sha256:${candidate.payload?.sha256}` !== appHash) throw new Error("installed app differs from release candidate");
  const root = requireEvidenceRoot(args.outputRoot, args.resume);
  requireEvidencePath(root, args.manifest, "manifest");
  requireEvidencePath(root, args.catalog, "catalog");
  const seedState = realpathSync(args.seedState);
  const configuration = deriveReliabilityConfiguration({ statePath: seedState, repoRoot });
  const manifest = createReliabilityManifest({ candidateId: candidate.candidateId, appHash, configuration });
  const ui = dependencies.ui ?? macosAccessibilityUi;
  const uiDriver = reliabilityUiDriverEvidence();
  const checkpointIdentity = { candidateId: candidate.candidateId, appHash, uiDriverBundleSha256: uiDriver.bundleSha256 };
  if (!ui.trusted()) throw new Error("Accessibility permission is not available to the reliability UI driver");
  const wakeLock = dependencies.wakeLock ?? startMacosWakeLock(dependencies.spawnProcess);
  try {
    requireDesktopSession(ui);
    await stopExistingDesktopLab(ui);
    const descriptors = reliabilityDescriptors(manifest);
    const runs = await collectReliabilityRuns({
      descriptors,
      root,
      existingRuns: args.resume ? loadRunCheckpoints(root, descriptors, checkpointIdentity) : [],
      record: (descriptor) => recordReliabilityRun({ descriptor, root, appPath, seedState, ui, pressureHelperPath }),
      checkpoint: (run) => writeRunCheckpoint(root, run, checkpointIdentity),
      onProgress: dependencies.onProgress ?? (({ index, total, run, resumed }) => process.stderr.write(`[${index}/${total}] ${run.caseId}/${run.profileId} ${run.recordingStatus}${resumed ? " resumed" : ""}\n`)),
    });
    const catalog = {
      kind: "desktoplab.recorded-agent-reliability-catalog",
      schemaVersion: 4,
      candidateId: candidate.candidateId,
      appHash,
      installation: { kind: "installed_application", platform: process.platform, artifactPath: appPath, executablePath, uiDriver },
      runs,
    };
    writeJson(args.manifest, manifest);
    writeJson(args.catalog, catalog);
    return { kind: "desktoplab.recorded-agent-reliability-output", schemaVersion: 1, runCount: runs.length, failedRunCount: runs.filter((run) => run.recordingStatus === "failed").length, manifestPath: resolve(args.manifest), catalogPath: resolve(args.catalog) };
  } finally {
    wakeLock.stop();
  }
}

export function startMacosWakeLock(spawnProcess = spawn) {
  const child = spawnProcess("/usr/bin/caffeinate", ["-dimsu", "-w", String(process.pid)], { stdio: "ignore" });
  return { stop() { if (child.exitCode === null) child.kill("SIGTERM"); } };
}

function requireDesktopSession(ui) {
  if (typeof ui.sessionAvailable === "function" && ui.sessionAvailable()) return;
  throw new Error("macOS desktop session is unavailable; unlock the console before recording UI reliability evidence");
}

function requireEvidenceRoot(path, resume) {
  const target = resolve(path);
  if (!resume && existsSync(target) && readdirSync(target).length > 0) throw new Error("reliability evidence root must be absent or empty");
  mkdirSync(target, { recursive: true });
  return realpathSync(target);
}

function writeRunCheckpoint(root, run, identity) {
  writeJson(join(root, run.runId, "run-result.json"), { kind: "desktoplab.reliability-run-checkpoint", schemaVersion: 1, ...identity, run });
}

export function loadRunCheckpoints(root, descriptors, identity) {
  return descriptors.flatMap((descriptor) => {
    const path = join(root, descriptor.runId, "run-result.json");
    if (!existsSync(path)) return [];
    const checkpoint = JSON.parse(readFileSync(path, "utf8"));
    if (checkpoint.kind !== "desktoplab.reliability-run-checkpoint" || checkpoint.schemaVersion !== 1
      || checkpoint.candidateId !== identity.candidateId || checkpoint.appHash !== identity.appHash
      || checkpoint.uiDriverBundleSha256 !== identity.uiDriverBundleSha256 || checkpoint.run?.runId !== descriptor.runId
      || checkpoint.run?.caseId !== descriptor.caseId || checkpoint.run?.seed !== descriptor.seed || checkpoint.run?.repetition !== descriptor.repetition) {
      throw new Error(`reliability checkpoint identity mismatch for ${descriptor.runId}`);
    }
    if (checkpoint.run.recordingStatus !== "completed") {
      rmSync(join(root, descriptor.runId), { recursive: true, force: true });
      return [];
    }
    return [checkpoint.run];
  });
}

function requireEvidencePath(root, path, label) {
  const target = resolve(path);
  if (target === root || target.startsWith(`${root}/`) === false) throw new Error(`${label} path must remain inside the reliability evidence root`);
}

function writeJson(path, value) {
  const target = resolve(path);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, `${JSON.stringify(value, null, 2)}\n`, { mode: 0o600 });
}

if (process.argv[1] && resolve(process.argv[1]) === driverPath) {
  try {
    const args = parseReliabilityArgs(process.argv.slice(2));
    console.log(JSON.stringify(args.printPlan ? { kind: "desktoplab.installed-agent-reliability-plan", schemaVersion: 1, certifying: false, runs: reliabilityDriverPlan() } : await runMacosReliabilityUi(args), null, 2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
