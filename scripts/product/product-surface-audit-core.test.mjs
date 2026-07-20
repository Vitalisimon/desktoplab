import assert from "node:assert/strict";
import test from "node:test";
import {
  classifyAvailability,
  validatePlatformClaims,
  validateSourcePublicationClaims,
  validateSecurityReportingClaims,
} from "./product-surface-audit-core.mjs";

const claims = {
  platforms: {
    macosAppleSilicon: { publicAvailability: "candidate_not_public" },
    linuxX64: { publicAvailability: "not_public" },
    windowsX64: { publicAvailability: "not_public" },
  },
};

test("availability wording is normalized instead of matched as fixed copy", () => {
  assert.equal(classifyAvailability("First beta candidate, not public yet"), "candidate_not_public");
  assert.equal(classifyAvailability("Not publicly available"), "not_public");
  assert.equal(classifyAvailability("Unavailable"), "not_public");
  assert.equal(classifyAvailability("Public beta"), "public");
});

test("human platform table must agree with machine-readable release claims", () => {
  const source = platformDocument({ windows: "Not publicly available" });
  assert.deepEqual(validatePlatformClaims(source, claims), []);
});

test("public Windows wording and missing macOS boundary fail closed", () => {
  const publicWindows = platformDocument({ windows: "Public beta" });
  assert.match(validatePlatformClaims(publicWindows, claims).join("\n"), /Windows x64.*not_public/);

  const missingBoundary = platformDocument({ windows: "Unavailable" })
    .replace("this does not by itself authorize publication", "publication is authorized");
  assert.match(validatePlatformClaims(missingBoundary, claims).join("\n"), /macOS publication boundary/);
});

test("source publication claims reject stale live-repository wording", () => {
  const candidate = "Status: audited public-source candidate prepared, not published\nThe canonical public repository is intentionally not live yet.";
  assert.deepEqual(validateSourcePublicationClaims(candidate, { sourceAvailability: "candidate_not_public" }), []);

  const stale = "Status: public source published\nDesktopLab's public source repository is live.";
  assert.match(validateSourcePublicationClaims(stale, { sourceAvailability: "candidate_not_public" }).join("\n"), /source publication/);
});

test("published source claims reject stale publication-pending wording", () => {
  const published = `
Status: public source published; no public binary release
DesktopLab's public source repository is live.
Cloud providers are not public support claims yet.
`;
  assert.deepEqual(validateSourcePublicationClaims(published, { sourceAvailability: "public" }), []);

  const stale = "Status: public-source candidate prepared, not published\nPublication is pending.";
  assert.match(validateSourcePublicationClaims(stale, { sourceAvailability: "public" }).join("\n"), /source publication/);
});

test("security reporting distinguishes channel activation from external proof", () => {
  const activeButUnverified = `
Use GitHub Private Vulnerability Reporting for confidential reports. The channel is enabled.
No external private test report has been verified yet.
Public beta binaries remain blocked until that end-to-end report passes.
`;
  assert.deepEqual(validateSecurityReportingClaims(activeButUnverified), []);

  const overstated = `
GitHub Private Vulnerability Reporting is planned.
Public beta binaries are available.
`;
  assert.match(validateSecurityReportingClaims(overstated).join("\n"), /enabled/);
  assert.match(validateSecurityReportingClaims(overstated).join("\n"), /external private test report/);
  assert.match(validateSecurityReportingClaims(overstated).join("\n"), /binaries remain blocked/);
});

test("security reporting accepts completed external proof without implying binary release", () => {
  const verified = `
GitHub Private Vulnerability Reporting is enabled.
The private end-to-end path has been verified by an external non-collaborator report.
The authorized test report was received, triaged and closed without public disclosure.
This page does not claim released-binary support.
`;
  assert.deepEqual(validateSecurityReportingClaims(verified), []);
});

test("security reporting accepts a historical proof only while repository publication is pending", () => {
  const pendingRepublication = `
GitHub Private Vulnerability Reporting is not currently available because public repository publication is pending.
A historical external non-collaborator report completed the private end-to-end path.
The authorized test report was received, triaged and closed without public disclosure.
The channel must be enabled and reverified after the new repository is published.
No public binary is released.
`;
  assert.deepEqual(validateSecurityReportingClaims(pendingRepublication), []);
});

test("security reporting rejects incomplete or contradictory verified proof", () => {
  const incomplete = `
GitHub Private Vulnerability Reporting is enabled.
An external report was received, but its private closure was not verified.
No public binary is released.
`;
  assert.match(validateSecurityReportingClaims(incomplete).join("\n"), /external private test report/);

  const contradictory = `
GitHub Private Vulnerability Reporting is enabled.
The private end-to-end path has been verified by an external non-collaborator report.
The authorized test report was received, triaged and closed without public disclosure.
Public beta binaries are available.
`;
  assert.match(validateSecurityReportingClaims(contradictory).join("\n"), /binary release/);
});

function platformDocument({ windows }) {
  return `
| Platform | Public availability | Evidence state |
| --- | --- | --- |
| macOS Apple Silicon | First beta candidate, not public yet | Signed and notarized. |
| Linux x64 | Not publicly available | Unsigned development evidence. |
| Windows x64 | ${windows} | Test-signed development evidence. |

- macOS Developer ID signing and notarization evidence exists; this does not by itself authorize publication.
`;
}
