import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";
import test from "node:test";

import { agentConfiguration } from "../test-fixtures/agent-configuration-fixture.mjs";
import { createReliabilityManifest, prepareReliabilityWorkspace, reliabilityDescriptors, reliabilityDriverPlan, snapshotReliabilityState, snapshotSqlite } from "../installed-agent-reliability-recording-core.mjs";
import { localModelProvenance } from "../ollama-model-provenance.mjs";
import { loadRunCheckpoints, parseReliabilityArgs, startMacosWakeLock } from "./macos-installed-agent-reliability-ui.mjs";

test("reliability plan creates twenty-five isolated descriptors across canonical cases and stress profiles", () => {
  const plan = reliabilityDriverPlan();
  assert.equal(plan.length, 25);
  assert.deepEqual([...new Set(plan.map((entry) => entry.caseId))], ["inspect", "create", "patch", "test_repair", "diff"]);
  assert.deepEqual([...new Set(plan.map((entry) => entry.profileId))], ["medium", "large_context", "long_session", "restart_resume", "deny_cancel_recovery"]);
  const manifest = createReliabilityManifest({ candidateId: sha("a"), appHash: sha("b"), configuration: agentConfiguration() });
  const descriptors = reliabilityDescriptors(manifest);
  assert.equal(new Set(descriptors.map((entry) => entry.runId)).size, 25);
  assert.ok(descriptors.every((entry) => typeof entry.profileId === "string"));
  assert.equal(manifest.minimumPassRate, 0.9);
});

test("canonical workspaces start clean except for the intentional diff fixture", () => {
  for (const caseId of ["inspect", "create", "patch", "test_repair", "diff"]) {
    const workspace = prepareReliabilityWorkspace(join(mkdtempSync(join(tmpdir(), "desktoplab-reliability-workspace-")), caseId), caseId);
    const status = execFileSync("git", ["status", "--short"], { cwd: workspace, encoding: "utf8" });
    assert.equal(status, caseId === "diff" ? " M release-note.md\n" : "", caseId);
  }
});

test("medium and large profiles create deterministic repository pressure", () => {
  const medium = prepareReliabilityWorkspace(join(mkdtempSync(join(tmpdir(), "desktoplab-medium-workspace-")), "repo"), "inspect", "medium");
  const large = prepareReliabilityWorkspace(join(mkdtempSync(join(tmpdir(), "desktoplab-large-workspace-")), "repo"), "inspect", "large_context");
  const mediumFiles = execFileSync("git", ["ls-files"], { cwd: medium, encoding: "utf8" }).trim().split("\n");
  const largeFiles = execFileSync("git", ["ls-files"], { cwd: large, encoding: "utf8" }).trim().split("\n");
  assert.ok(mediumFiles.length >= 500);
  assert.ok(largeFiles.length >= 2_000);
  assert.ok(largeFiles.length > mediumFiles.length);
});

test("SQLite snapshots include committed WAL state and remain independent", () => {
  const root = mkdtempSync(join(tmpdir(), "desktoplab-reliability-state-"));
  const source = join(root, "seed.sqlite");
  const target = join(root, "run", "desktoplab.sqlite");
  const database = new DatabaseSync(source);
  database.exec("pragma journal_mode = wal; create table proof (value text); insert into proof values ('seed')");
  snapshotSqlite(source, target);
  database.exec("insert into proof values ('later')");
  const snapshot = new DatabaseSync(target);
  assert.equal(snapshot.prepare("select value from proof").get().value, "seed");
  assert.equal(snapshot.prepare("select count(*) as count from proof").get().count, 1);
  snapshot.exec("insert into proof values ('run')");
  snapshot.close();
  assert.equal(database.prepare("select count(*) as count from proof").get().count, 2);
  database.close();
});

test("model provenance is derived offline from the content-addressed Ollama store", () => {
  const root = mkdtempSync(join(tmpdir(), "desktoplab-ollama-provenance-"));
  const state = join(root, "desktoplab.sqlite");
  const models = join(root, "models");
  const modelDigest = `sha256:${"a".repeat(64)}`;
  const configDigest = `sha256:${"b".repeat(64)}`;
  mkdirSync(join(models, "manifests/registry.ollama.ai/library/qwen2.5-coder"), { recursive: true });
  mkdirSync(join(models, "blobs"), { recursive: true });
  writeFileSync(join(models, `blobs/${modelDigest.replace(":", "-")}`), "model");
  writeFileSync(join(models, `blobs/${configDigest.replace(":", "-")}`), JSON.stringify({ file_type: "Q4_K_M" }));
  writeFileSync(join(models, "manifests/registry.ollama.ai/library/qwen2.5-coder/14b"), JSON.stringify({
    config: { digest: configDigest },
    layers: [{ mediaType: "application/vnd.ollama.image.model", digest: modelDigest, size: 5 }],
  }));
  const database = new DatabaseSync(state);
  database.exec("create table productization_state (kind text, subject_id text, payload text)");
  database.prepare("insert into productization_state values (?,?,?)").run("backend_readiness", "local", JSON.stringify({ modelCapabilities: { modelId: "qwen2.5-coder:14b" } }));
  database.close();
  assert.deepEqual(localModelProvenance(state, models), { modelId: "qwen2.5-coder:14b", quantization: "Q4_K_M", digest: modelDigest });
});

test("reliability state snapshots remove unrelated user history before recording", () => {
  const root = mkdtempSync(join(tmpdir(), "desktoplab-reliability-sanitized-"));
  const source = join(root, "seed.sqlite");
  const target = join(root, "run", "desktoplab.sqlite");
  const database = new DatabaseSync(source);
  database.exec("create table event_log (payload text); create table productization_state (kind text, subject_id text, payload text, updated_at text); create table settings (key text, value_kind text, value text, updated_at text); create table support_records (value text); create table support_records_fts (value text); create table support_sync_state (value text); create table support_tombstones (value text)");
  database.exec("insert into event_log values ('private'); insert into productization_state values ('agent_session','sessions','private','now'); insert into productization_state values ('backend_readiness','local','ready','now'); insert into settings values ('provider.secret','string','private','now'); insert into settings values ('approval.default_mode','string','require_approval','now'); insert into support_records values ('private'); insert into support_records_fts values ('private'); insert into support_sync_state values ('private'); insert into support_tombstones values ('private')");
  database.close();
  snapshotReliabilityState(source, target);
  const snapshot = new DatabaseSync(target, { readOnly: true });
  assert.equal(snapshot.prepare("select count(*) as count from event_log").get().count, 0);
  assert.deepEqual(snapshot.prepare("select kind from productization_state").all().map((row) => row.kind), ["backend_readiness"]);
  assert.deepEqual(snapshot.prepare("select key from settings").all().map((row) => row.key), ["approval.default_mode"]);
  assert.equal(snapshot.prepare("select count(*) as count from support_records").get().count, 0);
  snapshot.close();
});

test("reliability snapshots remove and re-migrate FTS without requiring Node FTS5", () => {
  const root = mkdtempSync(join(tmpdir(), "desktoplab-reliability-fts-"));
  const source = join(root, "seed.sqlite");
  const target = join(root, "run", "desktoplab.sqlite");
  const database = new DatabaseSync(source);
  database.exec("create table event_log (payload text); create table productization_state (kind text, subject_id text, payload text, updated_at text); create table settings (key text, value_kind text, value text, updated_at text); create table support_records (body text); create table support_sync_state (value text); create table support_tombstones (value text); create table schema_migrations (version integer primary key, checksum text, applied_at text); create table support_records_fts_content (id integer primary key, body text); insert into schema_migrations values (4,'fts','now'); insert into support_records values ('private support text'); insert into support_records_fts_content values (1,'private support text');");
  database.exec("pragma writable_schema = on; insert into sqlite_schema(type,name,tbl_name,rootpage,sql) values('table','support_records_fts','support_records_fts',0,'CREATE VIRTUAL TABLE support_records_fts USING fts5(body)'); pragma writable_schema = off; pragma schema_version = 2;");
  database.close();
  snapshotReliabilityState(source, target);
  const snapshot = new DatabaseSync(target, { readOnly: true });
  assert.equal(snapshot.prepare("select count(*) as count from sqlite_schema where name glob 'support_records_fts*'").get().count, 0);
  assert.equal(snapshot.prepare("select count(*) as count from schema_migrations where version = 4").get().count, 0);
  snapshot.close();
  assert.equal(readFileSync(target).includes(Buffer.from("private support text")), false);
});

test("driver accepts only explicit release paths and writes no claimed outcomes", () => {
  const args = parseReliabilityArgs(["--app", "/Applications/DesktopLab.app", "--candidate", "/tmp/candidate.json", "--output-root", "/tmp/runs", "--manifest", "/tmp/manifest.json", "--catalog", "/tmp/catalog.json"]);
  assert.equal(args.seedState.endsWith("desktoplab.sqlite"), true); assert.equal(parseReliabilityArgs(["--resume"]).resume, true);
  assert.throws(() => parseReliabilityArgs(["--shortcut"]), /unknown argument/);
  const source = readFileSync("scripts/product/drivers/macos-installed-agent-reliability-ui.mjs", "utf8");
  assert.match(source, /macosAccessibilityUi/);
  assert.match(source, /macosAccessibilityDriverEvidence/);
  assert.doesNotMatch(source, /promptEntered:\s*true|sendClicked:\s*true|approvalClicked:\s*true|verification:\s*\{/);
  const runSource = readFileSync("scripts/product/drivers/macos-installed-agent-reliability-run.mjs", "utf8");
  assert.match(runSource, /DESKTOPLAB_APP_DATA_DIR/);
  assert.match(runSource, /denyFirstApproval/);
  assert.match(runSource, /cancelFirstReadOnly/); assert.match(runSource, /stopExistingDesktopLab/); assert.match(runSource, /captureWhenReady/); assert.match(runSource, /macosAppLaunchArguments/); assert.match(runSource, /ui\.hasButton\("Send prompt"\).*composer after cancellation/); assert.doesNotMatch(runSource, /ui\.buttonEnabled\("Send prompt"\).*composer after cancellation/);
  const core = readFileSync("scripts/product/installed-agent-reliability-recording-core.mjs", "utf8");
  assert.doesNotMatch(core, /(?:execFileSync|spawnSync)\(\s*["']sqlite3["']/);
});

test("wake lock follows the recording process lifetime", () => {
  const calls = [];
  const child = { exitCode: null, kill: (signal) => calls.push(["kill", signal]) };
  const lock = startMacosWakeLock((command, args, options) => {
    calls.push([command, args, options]);
    return child;
  });
  lock.stop();
  assert.deepEqual(calls[0], ["/usr/bin/caffeinate", ["-dimsu", "-w", String(process.pid)], { stdio: "ignore" }]);
  assert.deepEqual(calls[1], ["kill", "SIGTERM"]);
});

test("resume deletes failed evidence roots and retains completed checkpoints", () => {
  const root = mkdtempSync(join(tmpdir(), "desktoplab-reliability-resume-"));
  const identity = { candidateId: sha("a"), appHash: sha("b"), uiDriverBundleSha256: sha("c") };
  const descriptors = [{ runId: "run-pass", caseId: "inspect", seed: 7, profileId: "medium", repetition: 1 }, { runId: "run-fail", caseId: "patch", seed: 43, profileId: "long_session", repetition: 1 }];
  for (const descriptor of descriptors) {
    const directory = join(root, descriptor.runId);
    mkdirSync(directory, { recursive: true });
    writeFileSync(join(directory, "run-result.json"), JSON.stringify({ kind: "desktoplab.reliability-run-checkpoint", schemaVersion: 1, ...identity, run: { ...descriptor, recordingStatus: descriptor.runId === "run-pass" ? "completed" : "failed" } }));
  }
  const runs = loadRunCheckpoints(root, descriptors, identity);
  assert.deepEqual(runs.map((run) => run.runId), ["run-pass"]);
  assert.equal(existsSync(join(root, "run-fail")), false);
});

test("reliability recording sources stay within focused line guards", () => {
  for (const [path, limit] of [
    ["scripts/product/installed-agent-reliability-recording-core.mjs", 180],
    ["scripts/product/agent-reliability-configuration.mjs", 120],
    ["scripts/product/agent-reliability-profiles.mjs", 90],
    ["scripts/product/ollama-model-provenance.mjs", 80],
    ["scripts/product/drivers/macos-installed-agent-reliability-ui.mjs", 240],
    ["scripts/product/drivers/macos-installed-agent-reliability-run.mjs", 240], ["scripts/product/drivers/macos-installed-agent-completion.test.mjs", 45], ["scripts/product/drivers/macos-installed-agent-state.test.mjs", 55], ["scripts/product/drivers/reliability-run-collector.mjs", 65], ["scripts/product/drivers/reliability-run-collector.test.mjs", 105],
    ["scripts/product/drivers/memory-pressure-helper.mjs", 20], ["scripts/product/drivers/process-memory-observation.mjs", 40],
    ["scripts/product/drivers/macos-installed-agent-reliability-ui.test.mjs", 180],
    ["scripts/product/drivers/macos-native-accessibility.mjs", 90],
    ["scripts/product/drivers/macos-native-accessibility.swift", 280],
    ["scripts/product/drivers/macos-system-keyboard-events.mjs", 60],
    ["scripts/product/versioned-ui-driver-evidence.mjs", 65],
  ]) {
    const logical = readFileSync(path, "utf8").split("\n").filter((line) => line.trim()).length;
    assert.ok(logical <= limit, `${path} has ${logical} logical lines, limit ${limit}`);
  }
});

function sha(character) { return `sha256:${character.repeat(64)}`; }
