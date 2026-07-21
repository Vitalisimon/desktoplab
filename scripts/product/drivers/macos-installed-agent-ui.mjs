#!/usr/bin/env node
import { createHash } from "node:crypto";
import { execFileSync, spawn, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, realpathSync, readdirSync, writeFileSync } from "node:fs";
import { homedir, hostname } from "node:os";
import { dirname, join, resolve } from "node:path";
import { DatabaseSync } from "node:sqlite";
import { fileURLToPath } from "node:url";

import { hashArtifact, readEmbeddedBuild } from "../../packaging/artifact-provenance-core.mjs";
import { installedAgentFixture, installedAgentPrompts } from "../installed-agent-recording-core.mjs";
import { localModelProvenance } from "../installed-agent-reliability-recording-core.mjs";
import { macosAccessibilityDriverEvidence, macosAccessibilityUi } from "./macos-native-accessibility.mjs";
import { installedAgentUiWaitModulePath, latestTerminalTurn, waitForActiveUi } from "./macos-installed-agent-ui-wait.mjs";

export { macosAccessibilityUi } from "./macos-native-accessibility.mjs";

const driverPath = fileURLToPath(import.meta.url);
const repoRoot = resolve(dirname(driverPath), "../../..");
const defaultStatePath = join(homedir(), ".config/desktoplab/desktoplab.sqlite");

export function driverPlan() {
  return {
    kind: "desktoplab.installed-agent-ui-driver-plan",
    schemaVersion: 1,
    certifying: false,
    platform: "darwin",
    cases: Object.entries(installedAgentPrompts).map(([caseId, prompt]) => ({ caseId, prompt, approvalMayBeRequired: ["create", "patch", "test_repair"].includes(caseId) })),
  };
}

export function installedAgentUiDriverEvidence() {
  return macosAccessibilityDriverEvidence(driverPath, [installedAgentUiWaitModulePath]);
}

export function parseArgs(argv) {
  const args = { app: null, workspace: null, evidence: null, candidate: null, state: defaultStatePath, printPlan: false };
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === "--app") args.app = argv[++index];
    else if (argv[index] === "--workspace") args.workspace = argv[++index];
    else if (argv[index] === "--evidence") args.evidence = argv[++index];
    else if (argv[index] === "--candidate") args.candidate = argv[++index];
    else if (argv[index] === "--state") args.state = argv[++index];
    else if (argv[index] === "--print-plan") args.printPlan = true;
    else throw new Error(`unknown argument ${argv[index]}`);
  }
  return args;
}

export async function runMacosInstalledAgentDriver(args, dependencies = {}) {
  const platform = dependencies.platform ?? process.platform;
  if (platform !== "darwin") throw new Error("macOS installed UI driver can only run on macOS");
  for (const key of ["app", "workspace", "evidence", "candidate"]) if (!args[key]) throw new Error(`missing --${key}`);
  const appPath = realpathSync(args.app);
  if (appPath !== "/Applications/DesktopLab.app" && appPath !== join(homedir(), "Applications/DesktopLab.app")) throw new Error("driver requires a natively installed DesktopLab.app");
  const executablePath = join(appPath, "Contents/MacOS/desktoplab-desktop");
  if (!existsSync(executablePath)) throw new Error("installed DesktopLab executable missing");
  const candidate = JSON.parse(readFileSync(args.candidate, "utf8"));
  const appHash = `sha256:${hashArtifact(appPath).sha256}`;
  const appBuild = readEmbeddedBuild(appPath);
  if (candidate.source?.commit !== appBuild.commitSha || `sha256:${candidate.payload?.sha256}` !== appHash) throw new Error("installed app differs from release candidate");
  const workspacePath = prepareWorkspace(args.workspace);
  const evidencePath = resolve(args.evidence);
  const screenshotRoot = join(dirname(evidencePath), "installed-agent-ui");
  mkdirSync(screenshotRoot, { recursive: true });
  const statePath = resolve(args.state);
  const ui = dependencies.ui ?? macosAccessibilityUi;
  if (!ui.trusted()) throw new Error("Accessibility permission is not available to the installed UI driver");
  await stopExistingDesktopLab(ui);
  const logPath = join(dirname(evidencePath), "installed-agent-ui-driver.log");
  const log = await import("node:fs").then(({ openSync }) => openSync(logPath, "a", 0o600));
  const child = spawn(executablePath, [], { env: { ...process.env, DESKTOPLAB_TEST_CONTROLS: "0" }, stdio: ["ignore", log, log] });
  const interactions = [];
  let workspaceId = null;
  let sessionId = null;
  try {
    await waitForActiveUi(ui, () => ui.ready(), 45_000, "DesktopLab Accessibility window");
    await waitForActiveUi(ui, () => ui.hasButton("Open project"), 45_000, "DesktopLab Open project command");
    ui.openProject(workspacePath);
    workspaceId = await waitFor(() => workspaceIdentity(statePath, workspacePath), 30_000, "persisted workspace selection");
    for (const [caseId, prompt] of Object.entries(installedAgentPrompts)) {
      const previousPromptCount = sessionPromptCount(statePath, workspaceId);
      const enteredAtUnixMs = Date.now();
      await waitForActiveUi(ui, () => ui.hasButton("Send prompt"), 30_000, `${caseId} composer`);
      ui.setPrompt(prompt);
      await waitForActiveUi(ui, () => ui.buttonEnabled("Send prompt"), 30_000, `${caseId} enabled Send prompt command`);
      const sendActivatedAtUnixMs = Date.now();
      ui.send(caseId === "inspect" ? "keyboard" : "button");
      const interaction = { caseId, promptSha256: digest(prompt), enteredAtUnixMs, sendActivatedAtUnixMs, approvalActivatedAtUnixMs: null, screenshot: null };
      const completion = await completeCase({ ui, statePath, workspaceId, previousPromptCount, interaction, timeoutMs: 12 * 60_000 });
      sessionId ??= completion.sessionId;
      if (completion.sessionId !== sessionId) throw new Error("visible prompts did not remain in one continuous session");
      const screenshotPath = join(screenshotRoot, `${String(interactions.length + 1).padStart(2, "0")}-${caseId}.png`);
      ui.capture(screenshotPath);
      interaction.screenshot = { path: screenshotPath, sha256: digest(readFileSync(screenshotPath)) };
      interactions.push(interaction);
    }
    const model = modelProvenance(statePath);
    const evidence = {
      kind: "desktoplab.installed-agent-evidence",
      schemaVersion: 2,
      appHash,
      commit: appBuild.commitSha,
      appBuild,
      modelId: model.modelId,
      quantization: model.quantization,
      host: `${hostname()}; ${process.platform}; ${process.arch}; local runtime`,
      installation: {
        kind: "installed_application",
        platform: process.platform,
        artifactPath: appPath,
        executablePath,
        uiDriver: installedAgentUiDriverEvidence(),
      },
      recording: { statePath, workspacePath, workspaceId, sessionId },
      interactions,
    };
    mkdirSync(dirname(evidencePath), { recursive: true });
    writeFileSync(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`, { mode: 0o600 });
    return evidence;
  } finally {
    ui.quit();
    await waitFor(() => child.exitCode !== null, 15_000, "DesktopLab shutdown").catch(() => child.kill("SIGTERM"));
  }
}

function prepareWorkspace(path) {
  const target = resolve(path);
  if (existsSync(target) && readdirSync(target).length > 0) throw new Error("installed-agent workspace must be absent or empty");
  mkdirSync(target, { recursive: true });
  writeFileSync(join(target, "calculator.js"), installedAgentFixture.initialImplementationContent);
  writeFileSync(join(target, "calculator.test.js"), installedAgentFixture.protectedContent);
  writeFileSync(join(target, "package.json"), installedAgentFixture.packageContent);
  writeFileSync(join(target, "release-note.md"), installedAgentFixture.initialPatchedContent);
  git(target, "init", "-b", "main");
  git(target, "add", ...installedAgentFixture.trackedFiles);
  git(target, "-c", "user.name=DesktopLab", "-c", "user.email=fixture@desktoplab.local", "commit", "-m", "installed certification fixture");
  return realpathSync(target);
}

export async function completeCase({ ui, statePath, workspaceId, previousPromptCount, interaction, timeoutMs, allowApprovals = true }) {
  const startedAt = Date.now();
  let lastApprovalCount = approvalCount(statePath, workspaceId);
  while (Date.now() - startedAt < timeoutMs) {
    const session = currentSession(statePath, workspaceId);
    const prompts = session?.trace?.filter((event) => event.kind === "prompt_recorded").length ?? 0;
    const terminal = latestTerminalTurn(session, interaction.sendActivatedAtUnixMs);
    if (prompts > previousPromptCount && terminal?.kind === "completed") return { sessionId: session.events[0].sessionId };
    if (prompts > previousPromptCount && terminal && terminal.kind !== "completed") {
      throw new Error(`installed UI case ${interaction.caseId} ${terminal.kind}: ${terminal.reason}`);
    }
    if (pendingApproval(statePath, session?.events?.[0]?.sessionId)) {
      if (!allowApprovals) throw new Error(`installed UI case ${interaction.caseId} requested an unexpected approval`);
      await waitForActiveUi(ui, () => ui.hasButton("Approve"), 30_000, "visible approval command");
      const activatedAt = Date.now();
      ui.clickButton("Approve");
      interaction.approvalActivatedAtUnixMs ??= activatedAt;
      await waitFor(() => approvalCount(statePath, workspaceId) > lastApprovalCount, 30_000, "persisted approval resolution");
      lastApprovalCount = approvalCount(statePath, workspaceId);
    }
    await sleep(500);
  }
  throw new Error(`installed UI case ${interaction.caseId} did not complete before timeout`);
}

export function workspaceIdentity(statePath, workspacePath) {
  const payload = statePayload(statePath, "workspace_registry", "local");
  return payload?.workspaces?.find((entry) => canonical(entry.rootPath) === canonical(workspacePath))?.workspaceId ?? null;
}

export function currentSession(statePath, workspaceId) {
  const payload = statePayload(statePath, "agent_session", "sessions");
  return [...(payload?.records ?? [])].reverse().find((entry) => entry.workspaceId === workspaceId) ?? null;
}

function sessionPromptCount(statePath, workspaceId) {
  return currentSession(statePath, workspaceId)?.trace?.filter((event) => event.kind === "prompt_recorded").length ?? 0;
}

function approvalCount(statePath, workspaceId) {
  return currentSession(statePath, workspaceId)?.trace?.filter((event) => event.kind === "approval_resolved" && event.success === true).length ?? 0;
}

export function approvalForSession(statePath, sessionId, state) {
  if (!sessionId) return null;
  const payload = statePayload(statePath, "approval_record", "local");
  return payload?.approvals?.find((approval) => approval.sessionId === sessionId && approval.state === state && approval.consumed !== true) ?? null;
}

export function pendingApproval(statePath, sessionId) {
  return approvalForSession(statePath, sessionId, "pending");
}

export function modelProvenance(statePath) {
  return localModelProvenance(statePath);
}

function statePayload(path, kind, subjectId) {
  if (!existsSync(path)) return null;
  const database = new DatabaseSync(path, { readOnly: true });
  try {
    configureStateReader(database);
    const row = database.prepare("select payload from productization_state where kind = ? and subject_id = ?").get(kind, subjectId);
    return row?.payload ? JSON.parse(row.payload) : null;
  } finally { database.close(); }
}

export function configureStateReader(database) {
  database.exec("PRAGMA busy_timeout = 5000");
}

export async function stopExistingDesktopLab(ui, dependencies = {}) {
  const processCommand = dependencies.processCommand ?? spawnSync;
  const wait = dependencies.wait ?? waitFor;
  const stopped = () => processCommand("pgrep", ["-x", "desktoplab-desktop"]).status !== 0;
  if (stopped()) return;
  try { ui.quit(); } catch {}
  try {
    await wait(stopped, 5_000, "graceful DesktopLab shutdown");
  } catch {
    const terminated = processCommand("pkill", ["-TERM", "-x", "desktoplab-desktop"]);
    if (terminated.status !== 0) throw new Error("DesktopLab SIGTERM fallback failed");
    await wait(stopped, 10_000, "DesktopLab SIGTERM shutdown");
  }
}

export async function waitFor(probe, timeoutMs, label) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    try { const value = await probe(); if (value) return value; } catch {}
    await sleep(250);
  }
  throw new Error(`timed out waiting for ${label}`);
}

function git(cwd, ...args) { execFileSync("git", args, { cwd, stdio: "ignore" }); }
function canonical(path) { try { return realpathSync(path); } catch { return null; } }
function digest(value) { return `sha256:${createHash("sha256").update(value).digest("hex")}`; }
function sleep(ms) { return new Promise((resolvePromise) => setTimeout(resolvePromise, ms)); }

if (process.argv[1] && resolve(process.argv[1]) === driverPath) {
  try {
    const args = parseArgs(process.argv.slice(2));
    if (args.printPlan) console.log(JSON.stringify(driverPlan(), null, 2));
    else console.log(JSON.stringify(await runMacosInstalledAgentDriver(args), null, 2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
