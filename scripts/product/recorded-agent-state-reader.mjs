import { DatabaseSync } from "node:sqlite";

export function readRecordedSession(statePath, workspaceId, sessionId) {
  const payload = readPayload(statePath, "agent_session", "sessions");
  const session = payload?.records?.find((record) =>
    record.workspaceId === workspaceId && record.events?.[0]?.sessionId === sessionId
  );
  assert(session, "recorded session is absent from the isolated state database");
  return session;
}

export function readRecordedApprovals(statePath) {
  return readPayload(statePath, "approval_record", "local")?.approvals ?? [];
}

function readPayload(statePath, kind, subjectId) {
  const database = new DatabaseSync(statePath, { readOnly: true });
  try {
    const row = database.prepare("select payload from productization_state where kind = ? and subject_id = ?").get(kind, subjectId);
    return row?.payload ? JSON.parse(row.payload) : null;
  } finally {
    database.close();
  }
}

function assert(condition, message) { if (!condition) throw new Error(message); }
