import assert from "node:assert/strict";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";
import test from "node:test";

import { configureStateReader, pendingApproval } from "./macos-installed-agent-ui.mjs";
import { terminalUiReady } from "./macos-installed-agent-reliability-run.mjs";

test("state readers wait for transient SQLite locks", () => {
  const database = new DatabaseSync(":memory:");
  configureStateReader(database);
  assert.equal(database.prepare("PRAGMA busy_timeout").get().timeout, 5_000);
  database.close();
});

test("pending approvals are correlated to the active agent session", () => {
  const statePath = join(mkdtempSync(join(tmpdir(), "desktoplab-approval-state-")), "desktoplab.sqlite");
  const database = new DatabaseSync(statePath);
  database.exec("create table productization_state (kind text, subject_id text, payload text)");
  const write = database.prepare("insert into productization_state values ('approval_record', 'local', ?)");
  write.run(JSON.stringify({ approvals: [
    { approvalId: "approval.1", sessionId: "session.other", state: "pending", consumed: false },
    { approvalId: "approval.2", sessionId: "session.active", state: "pending", consumed: false },
  ] }));
  database.close();

  assert.equal(pendingApproval(statePath, "session.active")?.approvalId, "approval.2");
  assert.equal(pendingApproval(statePath, "session.missing"), null);
});

test("resolved or consumed approvals do not block completion", () => {
  const statePath = join(mkdtempSync(join(tmpdir(), "desktoplab-resolved-approval-")), "desktoplab.sqlite");
  const database = new DatabaseSync(statePath);
  database.exec("create table productization_state (kind text, subject_id text, payload text)");
  database.prepare("insert into productization_state values ('approval_record', 'local', ?)").run(JSON.stringify({ approvals: [
    { approvalId: "approval.1", sessionId: "session.active", state: "approved", consumed: false },
    { approvalId: "approval.2", sessionId: "session.active", state: "pending", consumed: true },
  ] }));
  database.close();

  assert.equal(pendingApproval(statePath, "session.active"), null);
});

test("visible evidence waits for composer recovery after terminal persistence", () => {
  const ui = { hasButton: (name) => name === "Send prompt" };
  assert.equal(terminalUiReady(ui), true);
  assert.equal(terminalUiReady({ hasButton: () => true }), false);
});
