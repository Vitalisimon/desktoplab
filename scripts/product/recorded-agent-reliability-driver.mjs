#!/usr/bin/env node
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, realpathSync, statSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { hashArtifact } from "../packaging/artifact-provenance-core.mjs";
import { fingerprintAgentConfiguration } from "./agent-configuration-fingerprint.mjs";
import { additionBehaviorMatches } from "./installed-agent-fixture-semantics.mjs";
import { installedAgentFixture, installedAgentPrompts, isInstalledArtifactPath } from "./installed-agent-recording-core.mjs";
import { deriveReliabilityConfiguration } from "./installed-agent-reliability-recording-core.mjs";
import { versionedUiDriverFailures } from "./versioned-ui-driver-evidence.mjs";
import { verifyRecordedProfile } from "./recorded-agent-profile-verifier.mjs";
import { recordedFailureOutcome } from "./recorded-agent-failure.mjs";
import { readRecordedApprovals, readRecordedSession } from "./recorded-agent-state-reader.mjs";
import { requestedTextMatches } from "./recorded-agent-content.mjs";

const driverPath = fileURLToPath(import.meta.url);
const repoRoot = resolve(dirname(driverPath), "../..");
const digestPattern = /^sha256:[a-f0-9]{64}$/i;

export function runRecordedReliabilityCase(descriptor, { catalogPath, verifyInstallation = strictInstallation, verifyConfiguration = strictConfiguration } = {}) {
  const catalog = readJson(catalogPath);
  assert(catalog?.kind === "desktoplab.recorded-agent-reliability-catalog" && [2, 3, 4].includes(catalog.schemaVersion), "recorded reliability catalog is invalid or obsolete");
  assert(catalog.candidateId === descriptor.candidateId, "catalog belongs to another candidate");
  assert(catalog.appHash === descriptor.appHash, "catalog belongs to another app payload");
  verifyInstallation(catalog.installation, descriptor);
  const run = catalog.runs?.find((entry) =>
    entry.caseId === descriptor.caseId
    && entry.seed === descriptor.seed
    && entry.repetition === descriptor.repetition
  );
  assert(run, "recorded reliability run is missing");
  const failed = recordedFailureOutcome(run);
  if (failed) return failed;
  verifyRawEvidence(catalogPath, run, descriptor.caseId);
  verifyConfiguration(descriptor.configuration, run.statePath);
  const session = readRecordedSession(run.statePath, run.workspaceId, run.sessionId);
  const trace = traceEnvelope(session, run.sessionId);
  verifyInteraction(run.interaction, descriptor.caseId, trace.events);
  const checks = verifyCase(descriptor.caseId, run.workspacePath, trace, descriptor.timeoutMs);
  if (catalog.schemaVersion >= 3) {
    assert(run.profileId === descriptor.profileId, "recorded reliability profile differs from descriptor");
    const approvals = readRecordedApprovals(run.statePath);
    checks.push(...verifyRecordedProfile({ descriptor, run, workspacePath: run.workspacePath, session, approvals }));
  }
  const terminal = session.events?.at(-1)?.kind === "completed";
  const modelRequestCount = trace.events.filter((event) => event.kind === "model_response_recorded").length;
  const testControlRequests = trace.events.filter((event) => /test[_-]?control/i.test(`${event.source} ${event.detail}`)).length;
  const passed = terminal && checks.every((check) => check.passed) && modelRequestCount > 0 && testControlRequests === 0;
  return {
    status: passed ? "pass" : "failed",
    stopReason: passed ? null : "recorded deterministic verification failed",
    provenance: {
      executionKind: "installed_app_ui",
      candidateId: descriptor.candidateId,
      appHash: descriptor.appHash,
      modelRequestCount,
      testControlRequests,
      uiDriverSha256: catalog.installation.uiDriver.sha256,
      uiDriverBundleSha256: catalog.installation.uiDriver.bundleSha256,
      interactionSha256: digest(run.interaction),
      screenshotSha256: run.interaction.screenshot.sha256,
    },
    isolation: {
      workspaceId: run.workspaceId,
      workspacePath: run.workspacePath,
      sessionId: run.sessionId,
      statePath: run.statePath,
    },
    trace,
    verification: {
      kind: "desktoplab.deterministic-verification",
      schemaVersion: 1,
      status: checks.every((check) => check.passed) ? "pass" : "fail",
      checks,
    },
  };
}

function strictConfiguration(configuration, statePath) {
  const actual = deriveReliabilityConfiguration({ statePath, repoRoot });
  const declared = fingerprintAgentConfiguration(configuration);
  const observed = fingerprintAgentConfiguration(actual);
  assert(declared.status === "pass" && observed.status === "pass" && declared.fingerprint === observed.fingerprint, "campaign configuration differs from recorded runtime state");
}

function traceEnvelope(session, sessionId) {
  const events = (session.trace ?? []).map((event) => ({ ...event }));
  assert(events.length > 0, "recorded session trace is empty");
  assert(events.every((event, index) => event.eventId?.startsWith(`${sessionId}:trace:`)
    && event.parentEventId === (index === 0 ? null : events[index - 1].eventId)), "persisted trace identity or parentage is invalid");
  return { schemaVersion: 1, producer: "desktoplab-session-service/0.1.0", sessionId, events };
}

function verifyCase(caseId, workspacePath, trace, timeoutMs) {
  if (caseId === "inspect") return verifyInspect(workspacePath, trace);
  if (caseId === "create") return verifyCreate(workspacePath);
  if (caseId === "patch") return verifyPatch(workspacePath);
  if (caseId === "test_repair") return verifyTestRepair(workspacePath, trace, timeoutMs);
  if (caseId === "diff") return verifyDiff(workspacePath, trace);
  throw new Error(`unsupported recorded reliability case ${caseId}`);
}

function verifyInspect(workspacePath, trace) {
  const files = lines(git(workspacePath, "ls-files"));
  const grounded = trace.events.some((event) => event.kind === "completed" && event.success !== false);
  return [
    check("repository_files_observed", "filesystem", installedAgentFixture.trackedFiles.every((path) => files.includes(path)), files),
    check("answer_grounded", "session", grounded, trace.events.at(-1)),
  ];
}

function verifyCreate(workspacePath) {
  const target = workspaceFile(workspacePath, installedAgentFixture.createdPath);
  const content = existsSync(target) ? readFileSync(target, "utf8") : null;
  return [
    check("file_exists", "filesystem", content !== null, installedAgentFixture.createdPath),
    check("content_digest_matches", "filesystem", requestedTextMatches(content, installedAgentFixture.createdContent), content ?? "missing"),
  ];
}

function verifyPatch(workspacePath) {
  const target = workspaceFile(workspacePath, installedAgentFixture.patchedPath);
  const content = existsSync(target) ? readFileSync(target, "utf8") : null;
  const diff = git(workspacePath, "diff", "--", installedAgentFixture.patchedPath);
  return [
    check("expected_patch_applied", "filesystem", content === installedAgentFixture.patchedContent, content ?? "missing"),
    check("diff_observed", "git", diff.includes("Candidate state: verified."), diff),
  ];
}

function verifyTestRepair(workspacePath, trace, timeoutMs) {
  const target = workspaceFile(workspacePath, installedAgentFixture.implementationPath);
  const protectedFile = workspaceFile(workspacePath, installedAgentFixture.protectedPath);
  const command = installedAgentFixture.testCommand;
  const independent = spawnSync(command[0], command.slice(1), {
    cwd: workspacePath,
    encoding: "utf8",
    timeout: timeoutMs,
    maxBuffer: 16 * 1024 * 1024,
  });
  const failedIndex = trace.events.findIndex((event) => event.kind === "terminal_observed" && event.success === false);
  const passedAfter = trace.events.findIndex((event, index) => index > failedIndex && event.kind === "terminal_observed" && event.success === true);
  const implementation = existsSync(target) ? readFileSync(target, "utf8") : null;
  const protectedContent = existsSync(protectedFile) ? readFileSync(protectedFile, "utf8") : null;
  const behaviorMatches = implementation !== null && additionBehaviorMatches(target, timeoutMs);
  return [
    check("failing_test_observed", "process", failedIndex >= 0, trace.events[failedIndex] ?? "missing"),
    check("repair_applied", "filesystem", behaviorMatches && protectedContent === installedAgentFixture.protectedContent, implementation ?? "missing"),
    check("passing_rerun_observed", "process", passedAfter > failedIndex && independent.status === 0, `${independent.stdout}\n${independent.stderr}`),
  ];
}

function verifyDiff(workspacePath, trace) {
  const status = lines(git(workspacePath, "status", "--short"));
  const diff = git(workspacePath, "diff");
  const remotes = git(workspacePath, "remote").trim();
  const noPush = !trace.events.some((event) => /git_push/.test(`${event.source} ${event.detail}`));
  return [
    check("diff_observed", "git", status.includes(" M release-note.md") && diff.includes("Candidate state: verified."), `${status.join("\n")}\n${diff}`),
    check("no_push_observed", "git", remotes === "" && noPush, `${remotes}\nno_push=${noPush}`),
  ];
}

function strictInstallation(installation, descriptor) {
  assert(installation?.kind === "installed_application" && installation.platform === process.platform, "native installed application provenance missing");
  const appPath = canonicalDirectory(installation.artifactPath);
  assert(appPath && isInstalledArtifactPath(appPath), "catalog app is not installed in a native application location");
  assert(`sha256:${hashArtifact(appPath).sha256}` === descriptor.appHash, "catalog app bytes differ from descriptor");
  const executable = canonicalFile(installation.executablePath);
  assert(executable && !relative(appPath, executable).startsWith(".."), "catalog executable is outside the installed app");
  const driverFailures = versionedUiDriverFailures(installation.uiDriver, repoRoot);
  assert(driverFailures.length === 0, driverFailures.join("; "));
}

function verifyRawEvidence(catalogPath, run, caseId) {
  const evidenceRoot = canonicalDirectory(dirname(resolve(catalogPath)));
  const workspace = canonicalDirectory(run.workspacePath);
  const state = canonicalFile(run.statePath);
  const screenshot = canonicalFile(run.interaction?.screenshot?.path);
  assert(evidenceRoot && workspace && state && screenshot, "recorded run files are incomplete");
  for (const path of [workspace, state, screenshot]) assert(!relative(evidenceRoot, path).startsWith(".."), "recorded run escapes the evidence root");
  assert(existsSync(join(workspace, ".git")), "recorded workspace is not a Git repository");
  assert(run.interaction.screenshot.sha256 === digest(readFileSync(screenshot)), "recorded UI screenshot hash mismatch");
  assert(run.interaction.caseId === caseId, "recorded UI interaction belongs to another case");
}

function verifyInteraction(interaction, caseId, events) {
  const prompt = installedAgentPrompts[caseId];
  assert(interaction?.promptSha256 === digest(prompt), "recorded UI prompt digest mismatch");
  const promptEvent = events.find((event) => event.kind === "prompt_recorded" && event.recordedAtUnixMs >= interaction.sendActivatedAtUnixMs);
  assert(Number.isInteger(interaction.enteredAtUnixMs) && Number.isInteger(interaction.sendActivatedAtUnixMs), "recorded UI interaction timing missing");
  assert(interaction.enteredAtUnixMs <= interaction.sendActivatedAtUnixMs, "recorded UI interaction timing invalid");
  assert(promptEvent && promptEvent.recordedAtUnixMs - interaction.sendActivatedAtUnixMs <= 15_000, "UI send is not bound to the persisted prompt");
  const requiresApproval = ["create", "patch", "test_repair"].includes(caseId);
  const approval = events.find((event) => event.kind === "approval_resolved" && event.success === true && event.recordedAtUnixMs >= interaction.approvalActivatedAtUnixMs);
  if (requiresApproval) assert(Number.isInteger(interaction.approvalActivatedAtUnixMs) && approval && approval.recordedAtUnixMs - interaction.approvalActivatedAtUnixMs <= 15_000, "UI approval is not bound to persisted evidence");
}

function workspaceFile(workspacePath, value) {
  assert(typeof value === "string" && value.length > 0 && !isAbsolute(value), "expected workspace path is invalid");
  const target = resolve(workspacePath, value);
  assert(relative(resolve(workspacePath), target) && !relative(resolve(workspacePath), target).startsWith(".."), "expected path escapes workspace");
  return target;
}

function check(id, source, passed, material) {
  return { id, passed, source, evidenceId: digest(material) };
}

function git(cwd, ...args) {
  const result = spawnSync("git", args, { cwd, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 });
  assert(result.status === 0, `Git verifier failed: ${(result.stderr || result.stdout).trim()}`);
  return result.stdout;
}

function lines(value) { return value.trimEnd().split(/\r?\n/).filter(Boolean); }

function digest(value) {
  const material = Buffer.isBuffer(value) ? value : Buffer.from(typeof value === "string" ? value : JSON.stringify(value));
  return `sha256:${createHash("sha256").update(material).digest("hex")}`;
}

function readJson(path) {
  assert(path && existsSync(resolve(path)), "recorded reliability catalog is missing");
  return JSON.parse(readFileSync(resolve(path), "utf8"));
}

function canonicalDirectory(path) {
  try { const value = realpathSync(path); return statSync(value).isDirectory() ? value : null; } catch { return null; }
}

function canonicalFile(path) {
  try { const value = realpathSync(path); return statSync(value).isFile() ? value : null; } catch { return null; }
}

function assert(condition, message) { if (!condition) throw new Error(message); }

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const runIndex = process.argv.indexOf("--run");
  try {
    assert(runIndex >= 0 && process.argv[runIndex + 1], "recorded driver requires --run");
    const result = runRecordedReliabilityCase(JSON.parse(process.argv[runIndex + 1]), {
      catalogPath: process.env.DESKTOPLAB_RELIABILITY_CATALOG,
    });
    console.log(JSON.stringify(result));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
