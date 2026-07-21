import { createHash } from "node:crypto";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { spawn, spawnSync } from "node:child_process";

import { installedAgentPrompts } from "../installed-agent-recording-core.mjs";
import { reliabilityProfile } from "../agent-reliability-profiles.mjs";
import { prepareReliabilityWorkspace, snapshotReliabilityState } from "../installed-agent-reliability-recording-core.mjs";
import { approvalForSession, completeCase, currentSession, stopExistingDesktopLab, waitFor, workspaceIdentity } from "./macos-installed-agent-ui.mjs";
import { observeProcessMemory } from "./process-memory-observation.mjs";

export async function recordReliabilityRun({ descriptor, root, appPath, seedState, ui, pressureHelperPath }) {
  const profile = reliabilityProfile(descriptor.profileId);
  const runRoot = join(root, descriptor.runId);
  const workspacePath = prepareReliabilityWorkspace(join(runRoot, "workspace"), descriptor.caseId, descriptor.profileId);
  const appDataPath = join(runRoot, "app-data");
  const statePath = snapshotReliabilityState(seedState, join(appDataPath, "desktoplab.sqlite"));
  const screenshotPath = join(runRoot, "visible-ui.png");
  const logPath = join(runRoot, "desktoplab.log");
  const pressure = await startMemoryPressure(profile.memoryPressureMb, pressureHelperPath);
  let workspaceId = null;
  let sessionId = null;
  const preludes = [];
  const lifecycle = { profileId: profile.id, restartedAtUnixMs: null, deniedAtUnixMs: null, cancelledAtUnixMs: null, cancelledSessionId: null, memoryPressure: pressure.evidence };
  const launch = async (selectWorkspace) => {
    requireDesktopSession(ui, descriptor.runId);
    await stopExistingDesktopLab(ui);
    for (let attempt = 1; attempt <= 2; attempt += 1) {
      const opened = spawnSync("/usr/bin/open", macosAppLaunchArguments(appPath, appDataPath, logPath), {
        encoding: "utf8",
        timeout: 30_000,
      });
      if (opened.status !== 0) {
        throw new Error(`${descriptor.runId} could not launch DesktopLab: ${(opened.stderr || opened.stdout || "open failed").trim()}`);
      }
      try {
        await waitFor(() => {
          try { return ui.ready(); } catch { return false; }
        }, 45_000, `${descriptor.runId} DesktopLab window`);
        break;
      } catch (error) {
        await stopExistingDesktopLab(ui);
        if (attempt === 2) throw error;
      }
    }
    ui.activate();
    if (selectWorkspace) {
      await waitFor(() => ui.hasButton("Open project"), 45_000, `${descriptor.runId} Open project command`);
      ui.openProject(workspacePath);
      workspaceId = await waitFor(() => workspaceIdentity(statePath, workspacePath), 30_000, `${descriptor.runId} workspace selection`);
      await waitFor(() => ui.hasButton("Send prompt"), 45_000, `${descriptor.runId} mounted composer`);
    } else {
      await waitFor(() => ui.hasButton("Send prompt"), 45_000, `${descriptor.runId} restored composer`);
    }
  };
  const stop = async () => {
    await stopExistingDesktopLab(ui);
  };
  try {
    await launch(true);
    for (const [index, prompt] of profile.preludePrompts.entries()) {
      const interaction = await sendAndComplete({ ui, statePath, workspaceId, prompt, caseId: `prelude_${index + 1}`, timeoutMs: descriptor.timeoutMs, allowApprovals: true });
      sessionId ??= interaction.sessionId;
      if (interaction.sessionId !== sessionId) throw new Error("prelude prompts did not remain in one session");
      preludes.push({ ...interaction.record, sessionId: interaction.sessionId });
    }
    if (profile.restartAfterPrelude) {
      await stop();
      lifecycle.restartedAtUnixMs = Date.now();
      await launch(false);
    }
    const prompt = installedAgentPrompts[descriptor.caseId];
    if (profile.denyFirstApproval && approvalExpected(descriptor.caseId)) {
      const denied = await sendForDenial({ ui, statePath, workspaceId, prompt, caseId: descriptor.caseId, timeoutMs: descriptor.timeoutMs });
      lifecycle.deniedAtUnixMs = denied.deniedAtUnixMs;
    } else if (profile.cancelFirstReadOnly && !approvalExpected(descriptor.caseId)) {
      const cancelled = await sendForCancellation({ ui, statePath, workspaceId, prompt, timeoutMs: descriptor.timeoutMs });
      lifecycle.cancelledAtUnixMs = cancelled.cancelledAtUnixMs;
      lifecycle.cancelledSessionId = cancelled.sessionId;
    }
    const final = await sendAndComplete({ ui, statePath, workspaceId, prompt, caseId: descriptor.caseId, timeoutMs: descriptor.timeoutMs, allowApprovals: true });
    sessionId = final.sessionId;
    await captureWhenReady(ui, screenshotPath, descriptor.runId);
    final.record.screenshot = { path: screenshotPath, sha256: digest(readFileSync(screenshotPath)) };
    return { caseId: descriptor.caseId, seed: descriptor.seed, profileId: descriptor.profileId, repetition: descriptor.repetition, workspaceId, workspacePath, statePath, sessionId, interaction: final.record, preludeInteractions: preludes, lifecycle };
  } catch (error) {
    const failure = desktopSessionFailure(ui, descriptor.runId, error);
    const diagnostics = captureFailureDiagnostics(ui, runRoot, statePath, workspaceId);
    failure.reliabilityDiagnostics = diagnostics;
    throw failure;
  } finally {
    await stop().catch(() => {});
    pressure.stop();
  }
}

function requireDesktopSession(ui, runId) {
  if (desktopSessionAvailable(ui)) return;
  const error = new Error(`${runId} macOS desktop session unavailable`);
  error.reliabilityAbortCampaign = true;
  throw error;
}

function desktopSessionFailure(ui, runId, error) {
  if (desktopSessionAvailable(ui) && error instanceof Error) return error;
  const failure = new Error(`${runId} macOS desktop session unavailable`);
  failure.cause = error;
  failure.reliabilityAbortCampaign = true;
  return failure;
}

function desktopSessionAvailable(ui) {
  try { return typeof ui.sessionAvailable !== "function" || ui.sessionAvailable(); } catch { return false; }
}

function captureFailureDiagnostics(ui, runRoot, statePath, workspaceId) {
  const screenshotPath = join(runRoot, "failure-ui.png");
  let accessibility = null;
  let screenshot = null;
  try { ui.activate(); accessibility = ui.diagnostics(); } catch {}
  try { ui.capture(screenshotPath); screenshot = { path: screenshotPath, sha256: digest(readFileSync(screenshotPath)) }; } catch {}
  let session = null;
  try {
    const current = currentSession(statePath, workspaceId);
    session = current ? { sessionId: current.events?.[0]?.sessionId ?? null, eventKind: current.events?.at(-1)?.kind ?? null, traceKind: current.trace?.at(-1)?.kind ?? null, promptCount: current.trace?.filter((event) => event.kind === "prompt_recorded").length ?? 0 } : null;
  } catch {}
  const diagnostics = { capturedAtUnixMs: Date.now(), accessibility, session, screenshot };
  writeFileSync(join(runRoot, "failure-diagnostics.json"), `${JSON.stringify(diagnostics, null, 2)}\n`, { mode: 0o600 });
  return diagnostics;
}

export function macosAppLaunchArguments(appPath, appDataPath, logPath) {
  return [
    "-F", "-n", "-a", appPath,
    "--stdout", logPath,
    "--stderr", logPath,
    "--env", `DESKTOPLAB_APP_DATA_DIR=${appDataPath}`,
    "--env", "DESKTOPLAB_TEST_CONTROLS=0",
  ];
}

async function sendAndComplete({ ui, statePath, workspaceId, prompt, caseId, timeoutMs, allowApprovals }) {
  const previousPromptCount = currentSession(statePath, workspaceId)?.trace?.filter((event) => event.kind === "prompt_recorded").length ?? 0;
  const record = interactionRecord(caseId, prompt);
  await setPromptWhenReady(ui, prompt, `${caseId} prompt`);
  record.sendActivatedAtUnixMs = Date.now();
  ui.send(caseId === "inspect" ? "keyboard" : "button");
  const completion = await completeCase({ ui, statePath, workspaceId, previousPromptCount, interaction: record, timeoutMs, allowApprovals });
  return { sessionId: completion.sessionId, record };
}

async function sendForDenial({ ui, statePath, workspaceId, prompt, caseId, timeoutMs }) {
  await setPromptWhenReady(ui, prompt, `${caseId} denial prompt`);
  ui.send("button");
  await waitFor(() => ui.hasButton("Deny"), timeoutMs, `${caseId} denial approval`);
  const deniedAtUnixMs = Date.now();
  ui.clickButton("Deny");
  await waitFor(() => denialObserved(statePath, workspaceId), 30_000, `${caseId} persisted denial`);
  await waitFor(() => ui.hasButton("Send prompt"), timeoutMs, `${caseId} denial completion`);
  return { deniedAtUnixMs };
}

async function sendForCancellation({ ui, statePath, workspaceId, prompt, timeoutMs }) {
  await setPromptWhenReady(ui, prompt, "cancellation prompt");
  ui.send("button");
  await waitFor(() => ui.hasButton("Stop agent"), 30_000, "Stop agent command");
  const sessionId = await waitFor(() => currentSession(statePath, workspaceId)?.events?.[0]?.sessionId, 30_000, "running cancellation session");
  const cancelledAtUnixMs = Date.now();
  ui.clickButton("Stop agent");
  await waitFor(() => sessionState(statePath, workspaceId, sessionId) === "cancelled", timeoutMs, "persisted cancellation");
  await waitFor(() => ui.hasButton("Send prompt"), 30_000, "composer after cancellation");
  return { cancelledAtUnixMs, sessionId };
}

function sessionState(statePath, workspaceId, sessionId) {
  return terminalSessionState(currentSession(statePath, workspaceId), sessionId);
}

export function terminalSessionState(session, sessionId) {
  if (session?.events?.[0]?.sessionId !== sessionId) return null;
  return [...(session.events ?? [])].reverse().find((event) => ["cancelled", "completed", "failed"].includes(event.kind))?.kind ?? null;
}

function denialObserved(statePath, workspaceId) {
  const sessionId = currentSession(statePath, workspaceId)?.events?.[0]?.sessionId;
  return Boolean(approvalForSession(statePath, sessionId, "denied"));
}

async function setPromptWhenReady(ui, prompt, label) {
  await waitFor(() => ui.hasButton("Send prompt"), 45_000, `${label} composer`);
  ui.activate();
  ui.setPrompt(prompt);
  await waitFor(() => ui.buttonEnabled("Send prompt"), 30_000, `${label} send command`);
}

async function captureWhenReady(ui, path, runId) {
  ui.activate();
  await waitFor(() => terminalUiReady(ui), 30_000, `${runId} terminal UI`);
  await waitFor(() => {
    try { ui.capture(path); return existsSync(path); } catch { return false; }
  }, 30_000, `${runId} visible UI capture`);
}

export function terminalUiReady(ui) {
  try { return ui.hasButton("Send prompt") && !ui.hasButton("Stop agent"); } catch { return false; }
}

function interactionRecord(caseId, prompt) {
  return { caseId, promptSha256: digest(prompt), enteredAtUnixMs: Date.now(), sendActivatedAtUnixMs: null, approvalActivatedAtUnixMs: null, screenshot: null };
}

function approvalExpected(caseId) { return ["create", "patch", "test_repair"].includes(caseId); }

async function startMemoryPressure(megabytes, helperPath) {
  if (!megabytes) return { evidence: null, stop() {} };
  const child = spawn(process.execPath, [helperPath, String(megabytes)], { stdio: ["ignore", "pipe", "pipe"] });
  let readyOutput = "";
  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk) => { readyOutput += chunk; });
  try {
    const observed = await waitFor(() => {
      if (!readyOutput.includes(`ready ${megabytes * 1024 * 1024}`)) return null;
      const memory = observeProcessMemory(child.pid);
      return memory?.observedMemoryKb >= megabytes * 900 ? memory : null;
    }, 30_000, "memory pressure allocation");
    return { evidence: { requestedMb: megabytes, ...observed, startedAtUnixMs: Date.now() }, stop() { child.kill("SIGTERM"); } };
  } catch (error) {
    child.kill("SIGTERM");
    throw error;
  }
}

function digest(value) { return `sha256:${createHash("sha256").update(value).digest("hex")}`; }
