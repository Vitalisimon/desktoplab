import assert from "node:assert/strict";
import test from "node:test";

import { cancellationRecoveryObserved, denialRecoveryObserved } from "./recorded-agent-profile-verifier.mjs";

test("denial recovery follows persisted denial in the same session", () => {
  const run = fixtureRun({ deniedAtUnixMs: 100 });
  const session = fixtureSession([
    event("blocked", 110, false),
    event("prompt_recorded", 120, null),
    event("completed", 130, true),
  ]);
  const approvals = [{ sessionId: "session.1", state: "denied" }];

  assert.equal(denialRecoveryObserved(run, session, approvals), true);
  assert.equal(denialRecoveryObserved(run, session, []), false);
});

test("cancellation recovery preserves thread identity and completes a later prompt", () => {
  const run = fixtureRun({ cancelledAtUnixMs: 100, cancelledSessionId: "session.1" });
  const session = fixtureSession([
    event("cancelled", 110, false),
    event("prompt_recorded", 120, null),
    event("completed", 130, true),
  ]);

  assert.equal(cancellationRecoveryObserved(run, session), true);
  assert.equal(cancellationRecoveryObserved({ ...run, sessionId: "session.2" }, session), false);
});

function fixtureRun(lifecycle) {
  return { sessionId: "session.1", lifecycle };
}

function fixtureSession(trace) {
  return { events: [{ sessionId: "session.1" }], trace };
}

function event(kind, recordedAtUnixMs, success) {
  return { kind, recordedAtUnixMs, success };
}
