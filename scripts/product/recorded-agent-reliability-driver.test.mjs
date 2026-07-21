import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";
import test from "node:test";

import { hashArtifact } from "../packaging/artifact-provenance-core.mjs";
import { scoreExecutableCase } from "./agent-trace-score-core.mjs";
import { runRecordedReliabilityCase } from "./recorded-agent-reliability-driver.mjs";
import { passingExecutableCase } from "./test-fixtures/agent-trace-score-fixture.mjs";
import { installedAgentFixture, installedAgentPrompts } from "./installed-agent-recording-core.mjs";

test("recorded driver derives every executable case from isolated state and repository evidence", () => {
  for (const caseId of ["inspect", "create", "patch", "test_repair", "diff"]) {
    const fixture = createFixture(caseId);
    const result = runFixture(fixture);
    assert.equal(result.status, "pass", caseId);
    assert.equal(result.trace.sessionId, fixture.descriptorSessionId, caseId);
    assert.equal(result.provenance.executionKind, "installed_app_ui", caseId);
    assert.equal(result.verification.status, "pass", caseId);
    const scored = scoreExecutableCase(caseId, result);
    assert.equal(scored.status, "pass", `${caseId}: ${scored.failures.join("; ")}`);
  }
});

test("recorded driver rejects another app payload and absent database session", () => {
  const fixture = createFixture("inspect");
  assert.throws(
    () => runRecordedReliabilityCase({ ...fixture.descriptor, appHash: `sha256:${"f".repeat(64)}` }, fixtureOptions(fixture)),
    /app payload/,
  );
  const catalog = JSON.parse(readFileSync(fixture.catalogPath, "utf8"));
  catalog.runs[0].sessionId = "session.absent";
  writeFileSync(fixture.catalogPath, JSON.stringify(catalog));
  assert.throws(
    () => runFixture(fixture),
    /absent from the isolated state database/,
  );
});

test("recorded driver rejects evidence outside its root and unbound UI timing", () => {
  const fixture = createFixture("create");
  const catalog = JSON.parse(readFileSync(fixture.catalogPath, "utf8"));
  catalog.runs[0].workspacePath = fixture.outsidePath;
  writeFileSync(fixture.catalogPath, JSON.stringify(catalog));
  assert.throws(
    () => runFixture(fixture),
    /escapes the evidence root/,
  );

  const timingFixture = createFixture("create");
  const timingCatalog = JSON.parse(readFileSync(timingFixture.catalogPath, "utf8"));
  timingCatalog.runs[0].interaction.sendActivatedAtUnixMs = 50_000;
  writeFileSync(timingFixture.catalogPath, JSON.stringify(timingCatalog));
  assert.throws(() => runFixture(timingFixture), /not bound to the persisted prompt/);
});

test("recorded verifier accepts semantically equivalent repair formatting", () => {
  const fixture = createFixture("test_repair");
  const catalog = JSON.parse(readFileSync(fixture.catalogPath, "utf8"));
  const workspacePath = catalog.runs[0].workspacePath;
  writeFileSync(join(workspacePath, installedAgentFixture.implementationPath), "export function add(left, right) {\n  return Number(left) + Number(right);\n}");

  const result = runFixture(fixture);

  assert.equal(result.status, "pass", result.stopReason);
});

test("schema v3 binds each run to its declared reliability profile", () => {
  const fixture = createFixture("inspect");
  const catalog = JSON.parse(readFileSync(fixture.catalogPath, "utf8"));
  catalog.schemaVersion = 3;
  catalog.runs[0].profileId = "large_context";
  fixture.descriptor.profileId = "medium";
  writeFileSync(fixture.catalogPath, JSON.stringify(catalog));
  assert.throws(() => runFixture(fixture), /profile differs/);
});

test("schema v4 exposes recorded failures without hiding later campaign runs", () => {
  const fixture = createFixture("inspect");
  const catalog = JSON.parse(readFileSync(fixture.catalogPath, "utf8"));
  catalog.schemaVersion = 4;
  Object.assign(catalog.runs[0], { recordingStatus: "failed", operationalStatus: "timeout", stopReason: "installed case did not complete before timeout" });
  writeFileSync(fixture.catalogPath, JSON.stringify(catalog));
  const result = runFixture(fixture);
  assert.equal(result.status, "timeout");
  assert.match(result.reason, /timeout/);
});

test("schema v4 keeps model protocol failures distinct from infrastructure", () => {
  const fixture = createFixture("inspect");
  const catalog = JSON.parse(readFileSync(fixture.catalogPath, "utf8"));
  catalog.schemaVersion = 4;
  Object.assign(catalog.runs[0], { recordingStatus: "failed", outcomeStatus: "agent_failure", stopReason: "model_failure:model_protocol_error:provider_canonical_tool_call_required" });
  writeFileSync(fixture.catalogPath, JSON.stringify(catalog));
  assert.equal(runFixture(fixture).status, "agent_failure");
});

test("recorded driver sources stay below focused line guards", () => {
  for (const [path, limit] of [
    ["scripts/product/recorded-agent-reliability-driver.mjs", 240],
    ["scripts/product/recorded-agent-failure.mjs", 25],
    ["scripts/product/recorded-agent-profile-verifier.mjs", 100],
    ["scripts/product/recorded-agent-state-reader.mjs", 50], ["scripts/product/recorded-agent-profile-verifier.test.mjs", 80],
    ["scripts/product/recorded-agent-reliability-driver.test.mjs", 210],
  ]) {
    const logical = readFileSync(path, "utf8").split("\n").filter((line) => line.trim()).length;
    assert.ok(logical <= limit, `${path} has ${logical} logical lines, limit ${limit}`);
  }
});

function createFixture(caseId) {
  const root = mkdtempSync(join(tmpdir(), "desktoplab-recorded-reliability-"));
  const workspacePath = join(root, "workspace");
  const statePath = join(root, "desktoplab.sqlite");
  const appPath = join(root, "DesktopLab.app");
  const catalogPath = join(root, "catalog.json");
  const screenshotPath = join(root, `${caseId}.png`);
  const outsidePath = mkdtempSync(join(tmpdir(), "desktoplab-recorded-outside-"));
  mkdirSync(workspacePath);
  mkdirSync(appPath);
  writeFileSync(join(appPath, "binary"), "candidate");
  writeFileSync(join(workspacePath, "calculator.js"), installedAgentFixture.initialImplementationContent);
  writeFileSync(join(workspacePath, "calculator.test.js"), installedAgentFixture.protectedContent);
  writeFileSync(join(workspacePath, "package.json"), installedAgentFixture.packageContent);
  writeFileSync(join(workspacePath, "release-note.md"), installedAgentFixture.initialPatchedContent);
  execFileSync("git", ["init", "-b", "main"], { cwd: workspacePath });
  execFileSync("git", ["add", "."], { cwd: workspacePath });
  execFileSync("git", ["-c", "user.name=DesktopLab", "-c", "user.email=desktoplab@example.invalid", "commit", "-m", "fixture"], { cwd: workspacePath });
  if (caseId === "create") writeFileSync(join(workspacePath, installedAgentFixture.createdPath), installedAgentFixture.createdContent);
  if (caseId === "patch" || caseId === "diff") writeFileSync(join(workspacePath, installedAgentFixture.patchedPath), installedAgentFixture.patchedContent);
  if (caseId === "test_repair") writeFileSync(join(workspacePath, installedAgentFixture.implementationPath), installedAgentFixture.implementationContent);
  const sessionId = `session.recorded-${caseId}`;
  const actual = passingExecutableCase(caseId);
  relabelTrace(actual.trace, sessionId);
  const completedSequence = actual.trace.events.length + 1;
  const database = new DatabaseSync(statePath);
  database.exec("create table productization_state (kind text, subject_id text, payload text, updated_at text, primary key(kind, subject_id))");
  const record = {
    workspaceId: `workspace.recorded-${caseId}`,
    events: [{ kind: "completed", sessionId }],
    trace: [
      ...actual.trace.events.slice(0, -1),
      { ...actual.trace.events.at(-1), kind: "model_response_recorded", source: "provider.ollama" },
      { ...actual.trace.events.at(-1), eventId: `${sessionId}:trace:${completedSequence}`, sequence: completedSequence, kind: "completed", source: "agent", success: true },
    ],
  };
  relink(record.trace);
  database.prepare("insert into productization_state values (?, ?, ?, ?)").run("agent_session", "sessions", JSON.stringify({ nextSessionNumber: 2, records: [record] }), "1970-01-01T00:00:00Z");
  database.close();
  writeFileSync(screenshotPath, `screenshot-${caseId}`);
  const appHash = `sha256:${hashArtifact(appPath).sha256}`;
  const descriptor = {
    runId: `run-recorded-${caseId}`,
    candidateId: `sha256:${"a".repeat(64)}`,
    appHash,
    caseId,
    seed: 1,
    repetition: 1,
    timeoutMs: 60_000,
  };
  writeFileSync(catalogPath, JSON.stringify({
    kind: "desktoplab.recorded-agent-reliability-catalog",
    schemaVersion: 2,
    candidateId: descriptor.candidateId,
    appHash,
    installation: {
      kind: "installed_application",
      platform: process.platform,
      artifactPath: appPath,
      executablePath: join(appPath, "binary"),
      uiDriver: { path: "scripts/product/drivers/macos-installed-agent-reliability-ui.mjs", sha256: `sha256:${"9".repeat(64)}`, technology: "macos_accessibility" },
    },
    runs: [{
      caseId,
      seed: 1,
      repetition: 1,
      workspaceId: record.workspaceId,
      workspacePath,
      statePath,
      sessionId,
      interaction: {
        caseId,
        promptSha256: digest(installedAgentPrompts[caseId]),
        enteredAtUnixMs: 999,
        sendActivatedAtUnixMs: 1_000,
        approvalActivatedAtUnixMs: ["create", "patch", "test_repair"].includes(caseId) ? 1_000 : null,
        screenshot: { path: screenshotPath, sha256: digest(readFileSync(screenshotPath)) },
      },
    }],
  }));
  return { descriptor, descriptorSessionId: sessionId, catalogPath, outsidePath };
}

function relabelTrace(trace, sessionId) {
  trace.sessionId = sessionId;
  trace.events.forEach((event, index) => {
    event.sessionId = sessionId;
    event.eventId = `${sessionId}:trace:${index + 1}`;
    event.parentEventId = index === 0 ? null : `${sessionId}:trace:${index}`;
  });
}

function runFixture(fixture) {
  return runRecordedReliabilityCase(fixture.descriptor, fixtureOptions(fixture));
}

function fixtureOptions(fixture) {
  return { catalogPath: fixture.catalogPath, verifyInstallation: () => {}, verifyConfiguration: () => {} };
}

function relink(events) {
  events.forEach((event, index) => {
    event.sequence = index + 1;
    event.parentEventId = index === 0 ? null : events[index - 1].eventId;
  });
}

function digest(value) {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
}
