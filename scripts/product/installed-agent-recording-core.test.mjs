import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";
import test from "node:test";

import {
  installedAgentFixture,
  installedAgentPrompts,
  isInstalledArtifactPath,
  verifyInstalledAgentRecording,
} from "./installed-agent-recording-core.mjs";

test("installed location policy covers native roots and rejects build bundles", () => {
  assert.equal(isInstalledArtifactPath("/Applications/DesktopLab.app", "darwin", "/Users/test"), true);
  assert.equal(isInstalledArtifactPath("/Users/test/Applications/DesktopLab.app", "darwin", "/Users/test"), true);
  assert.equal(isInstalledArtifactPath("/tmp/DesktopLab.app", "darwin", "/Users/test"), false);
  assert.equal(isInstalledArtifactPath("C:\\Users\\test\\AppData\\Local\\DesktopLab\\DesktopLab.exe", "win32"), true);
  assert.equal(isInstalledArtifactPath("/opt/DesktopLab", "linux"), true);
  assert.equal(isInstalledArtifactPath("/tmp/DesktopLab.AppImage", "linux", "/home/test"), false);
});

test("installed recording is derived from SQLite filesystem Git process and hashed UI records", () => {
  const fixture = recordingFixture();
  const report = verifyInstalledAgentRecording({
    evidence: fixture.evidence,
    appPath: fixture.appPath,
    workspacePath: fixture.workspace,
    repoRoot: fixture.root,
    verifyInstallation: () => [],
  });

  assert.equal(report.status, "pass", report.failures.join("\n"));
  assert.ok(report.cases.every((entry) => entry.status === "pass"));
  assert.equal(report.cases.length, 5);
  assert.equal(report.metrics.localModelRequestCount, 5);
  assert.ok(report.metrics.realToolExecutionCount >= 6);
  assert.equal(report.metrics.testControlRequests, 0);
});

test("installed recording accepts equivalent text and implementation formatting", () => {
  const fixture = recordingFixture();
  writeFileSync(join(fixture.workspace, installedAgentFixture.createdPath), installedAgentFixture.createdContent.trimEnd());
  writeFileSync(join(fixture.workspace, installedAgentFixture.implementationPath), "export function add(left, right) {\n  return Number(left) + Number(right);\n}");

  const report = verifyInstalledAgentRecording({
    evidence: fixture.evidence,
    appPath: fixture.appPath,
    workspacePath: fixture.workspace,
    repoRoot: fixture.root,
    verifyInstallation: () => [],
  });

  assert.equal(report.status, "pass", report.failures.join("\n"));
});

test("installed recording rejects a repair that only hardcodes the fixture assertion", () => {
  const fixture = recordingFixture();
  writeFileSync(join(fixture.workspace, installedAgentFixture.implementationPath), "export function add() { return 5; }");

  const report = verifyInstalledAgentRecording({
    evidence: fixture.evidence,
    appPath: fixture.appPath,
    workspacePath: fixture.workspace,
    repoRoot: fixture.root,
    verifyInstallation: () => [],
  });

  assert.equal(report.status, "fail");
  assert.equal(report.cases.find((entry) => entry.id === "test_repair")?.verification.status, "fail");
});

test("claimed UI actions cannot replace missing persisted execution", () => {
  const fixture = recordingFixture();
  const database = new DatabaseSync(fixture.statePath);
  database.prepare("delete from productization_state").run();
  database.close();

  const report = verifyInstalledAgentRecording({
    evidence: fixture.evidence,
    appPath: fixture.appPath,
    workspacePath: fixture.workspace,
    repoRoot: fixture.root,
    verifyInstallation: () => [],
  });
  assert.equal(report.status, "fail");
  assert.ok(report.failures.includes("recorded session is absent from the installed app state database"));
});

test("UI screenshot and driver evidence fail closed when hashes drift", () => {
  const fixture = recordingFixture();
  fixture.evidence.interactions[0].screenshot.sha256 = `sha256:${"f".repeat(64)}`;
  const report = verifyInstalledAgentRecording({
    evidence: fixture.evidence,
    appPath: fixture.appPath,
    workspacePath: fixture.workspace,
    repoRoot: fixture.root,
    verifyInstallation: () => [],
  });
  assert.equal(report.status, "fail");
  assert.ok(report.failures.some((failure) => failure.includes("screenshot missing or hash mismatch")));
});

function recordingFixture() {
  const root = mkdtempSync(join(tmpdir(), "desktoplab-installed-recording-"));
  const workspace = join(root, "workspace");
  const evidenceRoot = join(root, "evidence");
  mkdirSync(workspace);
  mkdirSync(evidenceRoot);
  writeFileSync(join(workspace, "calculator.js"), "export function add(left, right) {\n  return left - right;\n}\n");
  writeFileSync(join(workspace, "calculator.test.js"), installedAgentFixture.protectedContent);
  writeFileSync(join(workspace, "package.json"), '{"type":"module","scripts":{"test":"node calculator.test.js"}}\n');
  writeFileSync(join(workspace, "release-note.md"), "# Release Candidate Note\n\nCandidate state: pending.\n");
  execFileSync("git", ["init", "-b", "main"], { cwd: workspace });
  execFileSync("git", ["add", "calculator.js", "calculator.test.js", "package.json", "release-note.md"], { cwd: workspace });
  execFileSync("git", ["-c", "user.name=DesktopLab", "-c", "user.email=fixture@desktoplab.local", "commit", "-m", "fixture"], { cwd: workspace });
  writeFileSync(join(workspace, "calculator.js"), installedAgentFixture.implementationContent);
  writeFileSync(join(workspace, "release-note.md"), installedAgentFixture.patchedContent);
  writeFileSync(join(workspace, installedAgentFixture.createdPath), installedAgentFixture.createdContent);
  const statePath = join(root, "desktoplab.sqlite");
  const sessionId = "session.recorded";
  const workspaceId = "workspace.recorded";
  const interactions = interactionRecords(evidenceRoot);
  const trace = traceEvents(interactions, sessionId);
  writeState(statePath, { workspaceId, sessionId, trace });
  const appPath = join(root, "DesktopLab.app");
  mkdirSync(appPath);
  return {
    root,
    workspace,
    statePath,
    appPath,
    evidence: {
      kind: "desktoplab.installed-agent-evidence",
      schemaVersion: 2,
      installation: {},
      recording: { workspacePath: workspace, workspaceId, sessionId, statePath },
      interactions,
    },
  };
}

function interactionRecords(root) {
  return Object.entries(installedAgentPrompts).map(([caseId, prompt], index) => {
    const path = join(root, `${caseId}.png`);
    writeFileSync(path, `screenshot:${caseId}`);
    const send = (index + 1) * 10_000;
    return {
      caseId,
      promptSha256: digest(prompt),
      enteredAtUnixMs: send - 100,
      sendActivatedAtUnixMs: send,
      approvalActivatedAtUnixMs: ["create", "patch", "test_repair"].includes(caseId) ? send + 1 : null,
      screenshot: { path, sha256: digest(`screenshot:${caseId}`) },
    };
  });
}

function traceEvents(interactions, sessionId) {
  const templates = {
    inspect: [["prompt_recorded", "user", false, null], ["tool_observed", "desktoplab.list_files", false, true], ["model_response_recorded", "provider.ollama", false, true], ["completed", "agent", false, true]],
    create: [["prompt_recorded", "user", false, null], ["approval_resolved", "policy", false, true], ["tool_observed", "desktoplab.write_file", true, true], ["model_response_recorded", "provider.ollama", false, true], ["completed", "agent", false, true]],
    patch: [["prompt_recorded", "user", false, null], ["tool_observed", "desktoplab.read_file", false, true], ["approval_resolved", "policy", false, true], ["tool_observed", "desktoplab.patch_file", true, true], ["tool_observed", "desktoplab.git_diff", false, true], ["model_response_recorded", "provider.ollama", false, true], ["completed", "agent", false, true]],
    test_repair: [["prompt_recorded", "user", false, null], ["tool_observed", "desktoplab.read_file", false, true], ["terminal_observed", "desktoplab.run_tests", false, false], ["approval_resolved", "policy", false, true], ["tool_observed", "desktoplab.patch_file", true, true], ["terminal_observed", "desktoplab.run_tests", false, true], ["model_response_recorded", "provider.ollama", false, true], ["completed", "agent", false, true]],
    diff: [["prompt_recorded", "user", false, null], ["tool_observed", "desktoplab.git_status", false, true], ["tool_observed", "desktoplab.git_diff", false, true], ["model_response_recorded", "provider.ollama", false, true], ["completed", "agent", false, true]],
  };
  const events = [];
  for (const interaction of interactions) {
    for (const [kind, source, mutation, success] of templates[interaction.caseId]) {
      const sequence = events.length + 1;
      events.push({
        eventId: `${sessionId}:trace:${sequence}`,
        parentEventId: sequence === 1 ? null : `${sessionId}:trace:${sequence - 1}`,
        sequence,
        recordedAtUnixMs: interaction.sendActivatedAtUnixMs + sequence,
        durationMs: null,
        correlationId: null,
        truncated: false,
        redacted: false,
        kind,
        source,
        mutation,
        success,
        detail: `${kind} tool=${source}`,
      });
    }
  }
  return events;
}

function writeState(path, { workspaceId, sessionId, trace }) {
  const database = new DatabaseSync(path);
  database.exec("create table productization_state (kind text not null, subject_id text not null, payload text not null)");
  const payload = { records: [{ workspaceId, events: [{ sessionId, kind: "started" }, { sessionId, kind: "completed" }], trace }] };
  database.prepare("insert into productization_state values (?, ?, ?)").run("agent_session", "sessions", JSON.stringify(payload));
  database.close();
}

function digest(value) {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
}
