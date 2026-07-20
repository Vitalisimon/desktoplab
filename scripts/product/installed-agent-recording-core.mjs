import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, realpathSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { join, relative, resolve } from "node:path";
import { DatabaseSync } from "node:sqlite";

import { additionBehaviorMatches } from "./installed-agent-fixture-semantics.mjs";
import { requestedTextMatches } from "./recorded-agent-content.mjs";
import { versionedUiDriverFailures } from "./versioned-ui-driver-evidence.mjs";

export const installedAgentPrompts = Object.freeze({
  inspect: "Inspect this repository and summarize the files that define the test fixture.",
  create: "Create release-proof.md with exactly: # Release proof followed by a blank line and DesktopLab completed this task locally.",
  patch: "Read release-note.md, change Candidate state: pending. to Candidate state: verified., then show the diff.",
  test_repair: "Run npm test, repair only the implementation causing the failure, and rerun the test until it passes.",
  diff: "Show git status and the complete diff. Do not commit and do not push.",
});

export const installedAgentFixture = Object.freeze({
  trackedFiles: ["calculator.js", "calculator.test.js", "package.json", "release-note.md"],
  initialImplementationContent: "export function add(left, right) {\n  return left - right;\n}\n",
  initialPatchedContent: "# Release Candidate Note\n\nCandidate state: pending.\n",
  packageContent: '{"type":"module","scripts":{"test":"node calculator.test.js"}}\n',
  createdPath: "release-proof.md",
  createdContent: "# Release proof\n\nDesktopLab completed this task locally.\n",
  patchedPath: "release-note.md",
  patchedContent: "# Release Candidate Note\n\nCandidate state: verified.\n",
  implementationPath: "calculator.js",
  implementationContent: "export function add(left, right) {\n  return left + right;\n}\n",
  protectedPath: "calculator.test.js",
  protectedContent: "import assert from 'node:assert/strict';\nimport { add } from './calculator.js';\nassert.equal(add(2, 3), 5);\n",
  testCommand: ["npm", "test"],
});

export function verifyInstalledAgentRecording({
  evidence,
  appPath,
  workspacePath,
  repoRoot,
  verifyInstallation = installationFailures,
} = {}) {
  const failures = [];
  failures.push(...verifyInstallation(evidence?.installation, appPath, repoRoot));
  failures.push(...interactionEnvelopeFailures(evidence?.interactions, repoRoot));
  const recording = evidence?.recording;
  const canonicalWorkspace = canonicalDirectory(workspacePath);
  const recordedWorkspace = canonicalDirectory(recording?.workspacePath);
  if (!canonicalWorkspace || !existsSync(join(canonicalWorkspace, ".git"))) failures.push("real Git workspace missing");
  if (!recordedWorkspace || recordedWorkspace !== canonicalWorkspace) failures.push("recorded workspace differs from certification workspace");
  const statePath = canonicalFile(recording?.statePath);
  if (!statePath) failures.push("recorded state database missing");
  if (!nonEmpty(recording?.workspaceId) || !nonEmpty(recording?.sessionId)) failures.push("recorded workspace or session identity missing");
  if (failures.length > 0) return { status: "fail", failures, cases: [], metrics: emptyMetrics() };

  let session;
  try {
    session = readSession(statePath, recording.workspaceId, recording.sessionId);
  } catch (error) {
    return { status: "fail", failures: [error.message], cases: [], metrics: emptyMetrics() };
  }
  const trace = traceEnvelope(session, recording.sessionId);
  const interactionByCase = new Map(evidence.interactions.map((entry) => [entry.caseId, entry]));
  const promptEvents = trace.events.filter((event) => event.kind === "prompt_recorded");
  const cases = Object.keys(installedAgentPrompts).map((caseId, index) => deriveCase({
    caseId,
    sessionId: recording.sessionId,
    interaction: interactionByCase.get(caseId),
    events: traceWindow(trace.events, evidence.interactions, index),
    promptEvent: firstEventAtOrAfter(promptEvents, interactionByCase.get(caseId)?.sendActivatedAtUnixMs),
    workspacePath: canonicalWorkspace,
  }));
  failures.push(...cases.flatMap((entry) => entry.failures.map((failure) => `${entry.id}: ${failure}`)));
  failures.push(...cases.filter((entry) => entry.status !== "pass").map((entry) => `${entry.id}: deterministic verification failed`));
  const uniqueEvents = uniqueTraceEvents(trace.events);
  const metrics = {
    localModelRequestCount: uniqueEvents.filter((event) => event.kind === "model_response_recorded").length,
    realToolExecutionCount: uniqueEvents.filter((event) => event.kind === "tool_observed" && event.success === true).length,
    testControlRequests: uniqueEvents.filter((event) => /test[_-]?control/i.test(`${event.source} ${event.detail}`)).length,
  };
  if (metrics.localModelRequestCount < 1) failures.push("no real local model request observed in persisted trace");
  if (metrics.realToolExecutionCount < 1) failures.push("no real tool execution observed in persisted trace");
  if (metrics.testControlRequests !== 0) failures.push("test-control endpoint was observed in persisted trace");
  return { status: failures.length === 0 ? "pass" : "fail", failures, cases, metrics };
}

function deriveCase({ caseId, sessionId, interaction, events, promptEvent, workspacePath }) {
  const failures = [];
  const prompt = installedAgentPrompts[caseId];
  if (!interaction || interaction.promptSha256 !== digest(prompt)) failures.push("UI prompt digest is missing or incorrect");
  if (!validInteractionTiming(interaction, promptEvent)) failures.push("UI interaction timing is not bound to the persisted prompt");
  const approvalExpected = ["create", "patch", "test_repair"].includes(caseId);
  const approvalEvent = events.find((event) => event.kind === "approval_resolved" && event.success === true);
  const approvalObserved = approvalTimingMatches(interaction?.approvalActivatedAtUnixMs, approvalEvent?.recordedAtUnixMs);
  if (approvalExpected && !approvalObserved) failures.push("approval click is not bound to persisted approval evidence");
  if (!approvalExpected && interaction?.approvalActivatedAtUnixMs != null) failures.push("unexpected approval click recorded");
  const verification = verifyFixtureCase(caseId, workspacePath, events);
  const completed = events.some((event) => event.kind === "completed" && event.success !== false);
  if (!completed) failures.push("case did not reach a completed persisted event");
  return {
    id: caseId,
    status: failures.length === 0 && verification.status === "pass" ? "pass" : "fail",
    promptEntered: failures.every((failure) => !failure.startsWith("UI prompt")),
    sendClicked: validInteractionTiming(interaction, promptEvent),
    sessionContinuous: true,
    approvalClicked: approvalExpected ? approvalObserved : false,
    latencyMs: promptEvent && events.at(-1) ? events.at(-1).recordedAtUnixMs - promptEvent.recordedAtUnixMs : null,
    evidence: {},
    verification,
    trace: { schemaVersion: 1, producer: "desktoplab-session-service/0.1.0", sessionId, events: normalizeTraceWindow(events) },
    failures,
  };
}

function verifyFixtureCase(caseId, workspacePath, events) {
  let checks;
  if (caseId === "inspect") {
    checks = [
      check("repository_files_observed", "filesystem", same(lines(git(workspacePath, "ls-files")), installedAgentFixture.trackedFiles), git(workspacePath, "ls-files")),
      check("answer_grounded", "session", events.some((event) => /read_file|list_files|search_text/.test(`${event.source} ${event.detail}`) && event.success !== false), events),
    ];
  } else if (caseId === "create") {
    const content = readWorkspaceFile(workspacePath, installedAgentFixture.createdPath);
    checks = [
      check("file_exists", "filesystem", content !== null, content ?? "missing"),
      check("content_digest_matches", "filesystem", requestedTextMatches(content, installedAgentFixture.createdContent), content ?? "missing"),
    ];
  } else if (caseId === "patch") {
    const contentChecks = exactFileChecks(workspacePath, installedAgentFixture.patchedPath, installedAgentFixture.patchedContent, ["expected_patch_applied"]);
    const diff = git(workspacePath, "diff", "--", installedAgentFixture.patchedPath);
    checks = [...contentChecks, check("diff_observed", "git", diff.includes("Candidate state: verified."), diff)];
  } else if (caseId === "test_repair") {
    const run = spawnSync(installedAgentFixture.testCommand[0], installedAgentFixture.testCommand.slice(1), { cwd: workspacePath, encoding: "utf8", timeout: 60_000 });
    const failed = events.some((event) => event.kind === "terminal_observed" && event.success === false);
    const implementation = readWorkspaceFile(workspacePath, installedAgentFixture.implementationPath);
    const protectedContent = readWorkspaceFile(workspacePath, installedAgentFixture.protectedPath);
    const behaviorMatches = implementation !== null && additionBehaviorMatches(resolve(workspacePath, installedAgentFixture.implementationPath));
    checks = [
      check("failing_test_observed", "process", failed, events),
      check("repair_applied", "filesystem", behaviorMatches && protectedContent === installedAgentFixture.protectedContent, `${implementation}\n${protectedContent}`),
      check("passing_rerun_observed", "process", run.status === 0 && events.some((event) => event.kind === "terminal_observed" && event.success === true), `${run.stdout}\n${run.stderr}`),
    ];
  } else {
    const diff = git(workspacePath, "diff");
    const status = git(workspacePath, "status", "--short");
    const noPush = git(workspacePath, "remote").trim() === "" && !events.some((event) => /git_push/.test(`${event.source} ${event.detail}`));
    checks = [check("diff_observed", "git", diff.includes("Candidate state: verified.") && status.length > 0, `${status}\n${diff}`), check("no_push_observed", "git", noPush, events)];
  }
  return { kind: "desktoplab.deterministic-verification", schemaVersion: 1, status: checks.every((entry) => entry.passed) ? "pass" : "fail", checks };
}

function installationFailures(installation, appPath, repoRoot) {
  const failures = [];
  const artifact = canonicalDirectory(appPath);
  if (installation?.kind !== "installed_application" || installation?.platform !== process.platform) failures.push("installed application provenance missing");
  if (!artifact || canonicalDirectory(installation?.artifactPath) !== artifact) failures.push("recorded artifact differs from installed application");
  if (process.platform === "darwin" && artifact && !isInstalledArtifactPath(artifact, process.platform)) failures.push("macOS certification app is not installed in an Applications directory");
  const executable = canonicalFile(installation?.executablePath);
  if (!executable || !artifact || relative(artifact, executable).startsWith("..")) failures.push("installed executable is outside the certified artifact");
  failures.push(...versionedUiDriverFailures(installation?.uiDriver, repoRoot));
  return failures;
}

function interactionEnvelopeFailures(interactions, repoRoot) {
  const failures = [];
  const evidenceRoot = canonicalDirectory(repoRoot);
  const expected = Object.keys(installedAgentPrompts);
  if (!Array.isArray(interactions) || !same(interactions.map((entry) => entry.caseId), expected)) return ["installed UI interaction set is incomplete or out of order"];
  for (const entry of interactions) {
    const screenshot = canonicalFile(entry.screenshot?.path);
    if (!screenshot || entry.screenshot.sha256 !== digest(readFileSync(screenshot))) failures.push(`${entry.caseId}: UI screenshot missing or hash mismatch`);
    if (screenshot && evidenceRoot && relative(evidenceRoot, screenshot).startsWith("..")) failures.push(`${entry.caseId}: UI screenshot is outside release evidence root`);
  }
  return failures;
}

function readSession(statePath, workspaceId, sessionId) {
  const database = new DatabaseSync(statePath, { readOnly: true });
  try {
    const row = database.prepare("select payload from productization_state where kind = ? and subject_id = ?").get("agent_session", "sessions");
    const payload = row?.payload ? JSON.parse(row.payload) : null;
    const session = payload?.records?.find((record) => record.workspaceId === workspaceId && record.events?.[0]?.sessionId === sessionId);
    if (!session) throw new Error("recorded session is absent from the installed app state database");
    return session;
  } finally {
    database.close();
  }
}

function traceEnvelope(session, sessionId) {
  const events = Array.isArray(session.trace) ? session.trace : [];
  if (events.length === 0) throw new Error("installed session trace is empty");
  return { schemaVersion: 1, producer: "desktoplab-session-service/0.1.0", sessionId, events };
}

function traceWindow(events, interactions, index) {
  const start = interactions[index]?.sendActivatedAtUnixMs ?? Number.MAX_SAFE_INTEGER;
  const end = interactions[index + 1]?.sendActivatedAtUnixMs ?? Number.MAX_SAFE_INTEGER;
  return events.filter((event) => event.recordedAtUnixMs >= start && event.recordedAtUnixMs < end);
}

function normalizeTraceWindow(events) {
  return events.map((event, index) => ({
    ...event,
    parentEventId: index === 0 ? null : events[index - 1].eventId,
  }));
}

function firstEventAtOrAfter(events, timestamp) {
  return Number.isInteger(timestamp) ? events.find((event) => event.recordedAtUnixMs >= timestamp) : null;
}

function validInteractionTiming(interaction, promptEvent) {
  return Number.isInteger(interaction?.enteredAtUnixMs)
    && Number.isInteger(interaction?.sendActivatedAtUnixMs)
    && interaction.enteredAtUnixMs <= interaction.sendActivatedAtUnixMs
    && Number.isInteger(promptEvent?.recordedAtUnixMs)
    && promptEvent.recordedAtUnixMs >= interaction.sendActivatedAtUnixMs
    && promptEvent.recordedAtUnixMs - interaction.sendActivatedAtUnixMs <= 15_000;
}

function approvalTimingMatches(activatedAt, recordedAt) {
  return Number.isInteger(activatedAt)
    && Number.isInteger(recordedAt)
    && recordedAt >= activatedAt
    && recordedAt - activatedAt <= 15_000;
}

function exactFileChecks(workspacePath, path, expected, ids) {
  const content = readWorkspaceFile(workspacePath, path);
  return ids.map((id) => check(id, "filesystem", content === expected, content ?? "missing"));
}

function readWorkspaceFile(workspacePath, value) {
  const target = resolve(workspacePath, value);
  if (!relative(workspacePath, target) || relative(workspacePath, target).startsWith("..") || !existsSync(target)) return null;
  return readFileSync(target, "utf8");
}

function check(id, source, passed, material) {
  return { id, passed, source, evidenceId: digest(typeof material === "string" ? material : JSON.stringify(material)) };
}

function git(cwd, ...args) {
  const result = spawnSync("git", args, { cwd, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 });
  return result.status === 0 ? result.stdout : "";
}

function canonicalDirectory(path) {
  try { const value = realpathSync(path); return statSync(value).isDirectory() ? value : null; } catch { return null; }
}

function canonicalFile(path) {
  try { const value = realpathSync(path); return statSync(value).isFile() ? value : null; } catch { return null; }
}

export function isInstalledArtifactPath(path, platform = process.platform, home = homedir()) {
  if (!isAbsolutePath(path)) return false;
  if (platform === "darwin") return ["/Applications", join(home, "Applications")].some((root) => path === join(root, "DesktopLab.app"));
  if (platform === "win32") return /^(?:[A-Za-z]:\\Program Files|[A-Za-z]:\\Users\\[^\\]+\\AppData\\Local)\\DesktopLab(?:\\|$)/i.test(path);
  if (platform === "linux") return [
    "/opt/DesktopLab",
    "/usr/bin/desktoplab-desktop",
    "/usr/local/bin/desktoplab-desktop",
    join(home, ".local/bin/DesktopLab.AppImage"),
    join(home, "Applications/DesktopLab.AppImage"),
  ].includes(path);
  return false;
}

function isAbsolutePath(path) {
  return typeof path === "string" && (path.startsWith("/") || /^[A-Za-z]:\\/.test(path));
}

function uniqueTraceEvents(events) {
  return [...new Map(events.map((event) => [event.eventId, event])).values()];
}

function lines(value) { return value.trimEnd().split(/\r?\n/).filter(Boolean); }
function digest(value) { return `sha256:${createHash("sha256").update(value).digest("hex")}`; }
function same(left, right) { return JSON.stringify(left) === JSON.stringify(right); }
function nonEmpty(value) { return typeof value === "string" && value.length > 0; }
function emptyMetrics() { return { localModelRequestCount: 0, realToolExecutionCount: 0, testControlRequests: null }; }
