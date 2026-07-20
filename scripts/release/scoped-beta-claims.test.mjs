import assert from "node:assert/strict";
import test from "node:test";
import { assessScopedBetaClaims } from "./scoped-beta-claims-core.mjs";
import { resolveEvidencePath } from "./scoped-beta-evidence-core.mjs";

function fixture(overrides = {}) {
  const head = "abc123";
  return {
    head,
    exportAudit: {
      sourceTree: { directPublicVisibility: "blocked_use_historyless_export" },
      sourceHistory: { directPublicVisibility: "blocked" },
      publicExportCandidate: { sourceCommit: head, sourceTreeState: "clean", findings: [] },
    },
    artifactManifest: {
      build: { commitSha: head, treeState: "clean" },
      entries: [{ kind: "app_bundle", fileName: "DesktopLab.app" }, { kind: "distribution_file", fileName: "DesktopLab.dmg" }],
    },
    supplyChain: { source: { commit: head }, localStatus: "pass", privateReporting: { status: "blocked" } },
    publicClaims: {
      schemaVersion: 1,
      binaryReleasePlatforms: ["macosAppleSilicon"],
      platforms: {
        macosAppleSilicon: {
          publicAvailability: "candidate_not_public",
          evidenceClaim: "signed_notarized_exact_candidate",
        },
        linuxX64: {
          publicAvailability: "not_public",
          evidenceClaim: "unsigned_physical_host_development",
        },
        windowsX64: {
          publicAvailability: "not_public",
          evidenceClaim: "test_signed_physical_host_development",
        },
      },
      claims: {
        installedLocalAgent: "exact_installed_evidence_required",
        cloudProviders: "not_publicly_certified",
        frontierLocal: "exact_certified_envelope_only",
        updater: "disabled",
      },
    },
    frontierGate: { status: "blocked", claimRequested: true },
    ...overrides,
  };
}

test("accepts an honest boundary while release remains blocked", () => {
  const report = assessScopedBetaClaims(fixture());
  assert.equal(report.status, "pass");
  assert.equal(report.sourceBoundary, "private_historyless_export");
  assert.equal(report.publicBetaAllowed, false);
  assert.equal(report.platforms.linuxX64.state, "blocked_current_head_host_unavailable");
  assert.equal(report.claims.cloudProviders.publiclyClaimed, false);
});

test("accepts a sanitized public checkout after historyless publication", () => {
  const input = fixture();
  input.exportAudit.sourceTree.directPublicVisibility = "allowed";
  input.exportAudit.sourceHistory.directPublicVisibility = "allowed";
  const report = assessScopedBetaClaims(input);
  assert.equal(report.status, "pass");
  assert.equal(report.sourceBoundary, "sanitized_public_checkout");
});

test("rejects a mixed source publication boundary", () => {
  const input = fixture();
  input.exportAudit.sourceTree.directPublicVisibility = "allowed";
  const report = assessScopedBetaClaims(input);
  assert.equal(report.status, "fail");
  assert.ok(report.failures.includes("source publication boundary is inconsistent or unsafe"));
});

test("fails stale artifacts, unsafe source publication, and overstated claims", () => {
  const input = fixture();
  input.artifactManifest.build.commitSha = "stale";
  input.exportAudit.sourceHistory.directPublicVisibility = "allowed";
  input.publicClaims.platforms.windowsX64.publicAvailability = "public";
  const report = assessScopedBetaClaims(input);
  assert.equal(report.status, "fail");
  assert.ok(report.failures.some((failure) => failure.includes("artifact")));
  assert.ok(report.failures.some((failure) => failure.includes("boundary")));
  assert.ok(report.failures.some((failure) => failure.includes("windowsX64")));
});

test("external evidence is accepted only for exact HEAD and never auto-published", () => {
  const report = assessScopedBetaClaims(fixture({ linuxEvidence: { kind: "linux-host", status: "pass", commit: "abc123" } }));
  assert.equal(report.platforms.linuxX64.state, "pass");
  assert.equal(report.platforms.linuxX64.publiclyClaimed, false);
});

test("derives a notarized macOS candidate and installed-agent proof from evidence", () => {
  const input = fixture({
    installedAgentEvidence: { kind: "installed-agent", status: "pass", provenance: { head: "abc123" } },
  });
  input.artifactManifest.entries = input.artifactManifest.entries.map((entry) => ({ ...entry, signatureState: "notarized" }));
  const report = assessScopedBetaClaims(input);
  assert.equal(report.status, "pass");
  assert.equal(report.platforms.macosAppleSilicon.evidence, "current_head_notarized_candidate");
  assert.equal(report.platforms.macosAppleSilicon.candidateEligibility, "eligible_pending_release_gate");
  assert.equal(report.claims.installedLocalAgent.state, "pass");
  assert.ok(!report.releaseBlockers.includes("Developer ID signing and notarization"));
  assert.ok(!report.releaseBlockers.includes("exact-candidate installed-agent recertification"));
});

test("accepts the macOS-only beta when exact evidence and private reporting pass", () => {
  const input = fixture({
    installedAgentEvidence: { kind: "installed-agent", status: "pass", provenance: { head: "abc123" } },
  });
  input.artifactManifest.entries = input.artifactManifest.entries.map((entry) => ({ ...entry, signatureState: "notarized" }));
  input.supplyChain.privateReporting.status = "pass";
  const report = assessScopedBetaClaims(input);
  assert.equal(report.publicBetaAllowed, true);
  assert.deepEqual(report.releaseBlockers, []);
});

test("requires host evidence only for platforms in the binary release scope", () => {
  const input = fixture();
  input.publicClaims.binaryReleasePlatforms.push("linuxX64");
  const report = assessScopedBetaClaims(input);
  assert.ok(report.releaseBlockers.includes("current-head Linux host evidence"));
  assert.ok(!report.releaseBlockers.includes("real Windows host evidence"));
});

test("discovers versioned evidence unless CI provides an explicit path", () => {
  const candidate = "dist/release/linux/abc12345/linux-current-head-evidence.json";
  assert.equal(resolveEvidencePath({
    candidates: [candidate],
    exists: (path) => path === candidate,
  }), candidate);
  assert.equal(resolveEvidencePath({
    explicitPath: "/ci/linux.json",
    candidates: [candidate],
    exists: () => true,
  }), "/ci/linux.json");
});
