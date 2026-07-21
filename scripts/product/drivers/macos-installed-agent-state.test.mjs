import assert from "node:assert/strict";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";
import test from "node:test";

import { approvalForSession, stopExistingDesktopLab } from "./macos-installed-agent-ui.mjs";
import { macosAppLaunchSpec, terminalSessionState } from "./macos-installed-agent-reliability-run.mjs";

test("terminal session state is derived from persisted lifecycle events", () => {
  const session = { events: [
    { kind: "created", sessionId: "session.1" },
    { kind: "job_observation" },
    { kind: "cancelled" },
  ] };
  assert.equal(terminalSessionState(session, "session.1"), "cancelled");
  assert.equal(terminalSessionState(session, "session.other"), null);
});

test("non-terminal event streams remain active", () => {
  const session = { events: [{ kind: "created", sessionId: "session.1" }, { kind: "job_heartbeat" }] };
  assert.equal(terminalSessionState(session, "session.1"), null);
});

test("approval state follows the persisted approval record contract", () => {
  const path = join(mkdtempSync(join(tmpdir(), "desktoplab-approval-contract-")), "state.sqlite");
  const database = new DatabaseSync(path);
  database.exec("create table productization_state (kind text, subject_id text, payload text)");
  database.prepare("insert into productization_state values ('approval_record','local',?)").run(JSON.stringify({ approvals: [
    { approvalId: "approval.1", sessionId: "session.1", state: "denied", consumed: false },
  ] }));
  database.close();
  assert.equal(approvalForSession(path, "session.1", "denied")?.approvalId, "approval.1");
  assert.equal(approvalForSession(path, "session.1", "pending"), null);
});

test("macOS reliability launches the installed executable with isolated app data", () => {
  const spec = macosAppLaunchSpec("/Applications/DesktopLab.app", "/tmp/run/app-data");
  assert.equal(spec.executablePath, "/Applications/DesktopLab.app/Contents/MacOS/desktoplab-desktop");
  assert.deepEqual(spec.environment, { DESKTOPLAB_APP_DATA_DIR: "/tmp/run/app-data", DESKTOPLAB_TEST_CONTROLS: "0" });
});

test("DesktopLab shutdown escalates from graceful quit to exact-process SIGTERM", async () => {
  const commands = [];
  let running = true;
  let waits = 0;
  const processCommand = (command, args) => {
    commands.push([command, ...args]);
    if (command === "pkill") running = false;
    return { status: command === "pgrep" ? (running ? 0 : 1) : 0 };
  };
  const wait = async (probe) => {
    if (++waits === 1) throw new Error("graceful timeout");
    assert.equal(probe(), true);
  };

  await stopExistingDesktopLab({ quit() {} }, { processCommand, wait });

  assert.deepEqual(commands.find(([command]) => command === "pkill"), ["pkill", "-TERM", "-x", "desktoplab-desktop"]);
});
