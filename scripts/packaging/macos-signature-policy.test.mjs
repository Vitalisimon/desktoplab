import assert from "node:assert/strict";
import test from "node:test";
import { assertSignatureMode, signatureState } from "./macos-signature-policy.mjs";

test("development accepts a valid ad-hoc state while release rejects it", () => {
  const state = signatureState("Signature=adhoc\nTeamIdentifier=not set\n");
  assert.equal(state, "adhoc_dev");
  assert.doesNotThrow(() => assertSignatureMode(state, "dev"));
  assert.throws(() => assertSignatureMode(state, "release"), /rejects adhoc_dev/);
});

test("Developer ID requires identity, team and hardened runtime", () => {
  const valid = "Authority=Developer ID Application: DesktopLab\nTeamIdentifier=TEAM123\nCodeDirectory flags=0x10000(runtime)";
  assert.equal(signatureState(valid), "developer_id");
  assert.doesNotThrow(() => assertSignatureMode("developer_id", "release"));
  assert.equal(signatureState(valid.replace("runtime", "none")), "invalid");
});

test("notarized mode accepts only stapled notarized evidence", () => {
  assert.equal(signatureState("Authority=Developer ID Application: DesktopLab", true), "notarized");
  assert.throws(() => assertSignatureMode("developer_id", "notarized"), /rejects developer_id/);
});
