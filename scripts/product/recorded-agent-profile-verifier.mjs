import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { execFileSync } from "node:child_process";

import { reliabilityProfile } from "./agent-reliability-profiles.mjs";

export function verifyRecordedProfile({ descriptor, run, workspacePath, session, approvals }) {
  const profile = reliabilityProfile(descriptor.profileId);
  const checks = [];
  const tracked = gitLines(workspacePath, "ls-files");
  checks.push(check("profile_workspace_scale", tracked.length >= profile.fileCount + 4, tracked.length));
  checks.push(check("profile_identity", run.profileId === profile.id, run.profileId ?? "missing"));
  const preludes = run.preludeInteractions ?? [];
  const expectedPreludeDigests = profile.preludePrompts.map(digest);
  checks.push(check("profile_preludes", same(preludes.map((entry) => entry.promptSha256), expectedPreludeDigests)
    && preludes.every((entry) => entry.sessionId === run.sessionId), preludes));
  if (profile.id === "large_context") {
    const sentinel = readFileSync(join(workspacePath, "docs/generated/context-2047.md"), "utf8");
    checks.push(check("profile_extended_context", sentinel.includes("RELEASE_CONTEXT_SENTINEL_2047=verified"), sentinel));
  }
  if (profile.restartAfterPrelude) checks.push(check("profile_restart_resume", Number.isInteger(run.lifecycle?.restartedAtUnixMs) && preludes.length > 0, run.lifecycle));
  if (profile.denyFirstApproval && approvalExpected(descriptor.caseId)) {
    checks.push(check("profile_denial_recovery", denialRecoveryObserved(run, session, approvals), { lifecycle: run.lifecycle, approvals }));
  }
  if (profile.cancelFirstReadOnly && !approvalExpected(descriptor.caseId)) {
    checks.push(check("profile_cancel_recovery", cancellationRecoveryObserved(run, session), { lifecycle: run.lifecycle, sessionId: run.sessionId }));
  }
  if (profile.memoryPressureMb > 0) {
    const pressure = run.lifecycle?.memoryPressure;
    checks.push(check("profile_memory_pressure", pressure?.requestedMb === profile.memoryPressureMb
      && ["physical_footprint", "rss"].includes(pressure.measurement)
      && pressure.observedMemoryKb >= profile.memoryPressureMb * 900, pressure ?? "missing"));
  }
  return checks;
}

export function denialRecoveryObserved(run, session, approvals = []) {
  const deniedAt = run.lifecycle?.deniedAtUnixMs;
  const denied = approvals.some((approval) => approval.sessionId === run.sessionId && approval.state === "denied");
  return Number.isInteger(deniedAt) && denied && recoveredAfter(session?.trace, "blocked", deniedAt);
}

export function cancellationRecoveryObserved(run, session) {
  const cancelledAt = run.lifecycle?.cancelledAtUnixMs;
  return Number.isInteger(cancelledAt)
    && run.lifecycle?.cancelledSessionId === run.sessionId
    && session?.events?.[0]?.sessionId === run.sessionId
    && recoveredAfter(session?.trace, "cancelled", cancelledAt);
}

function recoveredAfter(trace = [], terminalKind, boundary) {
  const terminal = trace.findIndex((event) => event.kind === terminalKind && event.recordedAtUnixMs >= boundary);
  const resumed = trace.findIndex((event, index) => index > terminal && event.kind === "prompt_recorded");
  return terminal >= 0 && resumed > terminal
    && trace.some((event, index) => index > resumed && event.kind === "completed" && event.success === true);
}

function approvalExpected(caseId) { return ["create", "patch", "test_repair"].includes(caseId); }
function gitLines(cwd, ...args) { return execFileSync("git", args, { cwd, encoding: "utf8" }).trimEnd().split(/\r?\n/).filter(Boolean); }
function check(id, passed, material) { return { id, passed, source: "session", evidenceId: digest(material) }; }
function digest(value) { return `sha256:${createHash("sha256").update(typeof value === "string" ? value : JSON.stringify(value)).digest("hex")}`; }
function same(left, right) { return JSON.stringify(left) === JSON.stringify(right); }
