import assert from "node:assert/strict";
import test from "node:test";
import { assessAppleSigningPreflight, parseDeveloperIdApplications } from "./apple-signing-preflight-core.mjs";

const identityOutput = `
  1) 0123456789ABCDEF0123456789ABCDEF01234567 "Developer ID Application: Example Person (TEAMID1234)"
     1 valid identities found
`;

test("parses Developer ID metadata while redacting the certificate owner", () => {
  const identities = parseDeveloperIdApplications(identityOutput);
  assert.equal(identities.length, 1);
  assert.equal(identities[0].teamId, "TEAMID1234");
  assert.equal(identities[0].displayName, "[REDACTED_SIGNING_IDENTITY]");
  assert.equal(JSON.stringify(identities.map(({ rawIdentity: _, ...identity }) => identity)).includes("Example Person"), false);
});

test("preflight passes only with one selected identity and validated notary profile", () => {
  const report = assessAppleSigningPreflight({
    identities: parseDeveloperIdApplications(identityOutput),
    selectedIdentityConfigured: true,
    selectedIdentityMatches: true,
    notarytoolAvailable: true,
    notaryProfileConfigured: true,
    notaryProfileValidated: true,
    appleServiceReachable: true,
    timestampServiceReachable: true,
    trackedSecretScanPassed: true,
  });
  assert.equal(report.status, "pass");
  assert.equal(report.safeToSign, false);
  assert.equal(report.signingPerformed, false);
});

test("preflight reports every missing external input without signing", () => {
  const report = assessAppleSigningPreflight({
    identities: [],
    selectedIdentityConfigured: false,
    selectedIdentityMatches: false,
    notarytoolAvailable: true,
    notaryProfileConfigured: false,
    notaryProfileValidated: false,
    appleServiceReachable: true,
    timestampServiceReachable: true,
    trackedSecretScanPassed: true,
  });
  assert.equal(report.status, "blocked");
  assert.equal(report.blockers.length, 3);
  assert.equal(report.signingPerformed, false);
});
