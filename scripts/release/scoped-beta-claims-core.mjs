export function assessScopedBetaClaims({
  head,
  exportAudit,
  artifactManifest,
  supplyChain,
  publicClaims,
  frontierGate,
  linuxEvidence = null,
  windowsEvidence = null,
  providerEvidence = null,
  installedAgentEvidence = null,
}) {
  const failures = [];
  const candidate = exportAudit?.publicExportCandidate;
  if (!candidate || candidate.findings?.length > 0) failures.push("public export has findings");
  if (candidate?.sourceCommit !== head || candidate?.sourceTreeState !== "clean") failures.push("public export is not bound to clean HEAD");
  const sourceBoundary = classifySourceBoundary(exportAudit);
  if (sourceBoundary === "invalid") failures.push("source publication boundary is inconsistent or unsafe");
  if (artifactManifest?.build?.commitSha !== head || artifactManifest?.build?.treeState !== "clean") failures.push("macOS dev artifact is not current-head clean evidence");
  if (!artifactManifest?.entries?.some((entry) => entry.kind === "app_bundle")) failures.push("macOS app bundle evidence is missing");
  if (!artifactManifest?.entries?.some((entry) => entry.fileName?.endsWith(".dmg"))) failures.push("macOS DMG evidence is missing");
  if (supplyChain?.source?.commit !== head || supplyChain?.localStatus !== "pass") failures.push("supply-chain local evidence is stale or failed");

  const macosEntries = artifactManifest?.entries?.filter((entry) =>
    entry.kind === "app_bundle" || entry.fileName?.endsWith(".dmg"),
  ) ?? [];
  const macosNotarized = macosEntries.length >= 2 && macosEntries.every((entry) => entry.signatureState === "notarized");
  const releasePlatforms = validatePublicClaims(publicClaims, failures);
  if (!frontierGate || !["pass", "blocked"].includes(frontierGate.status) || frontierGate.claimRequested !== true) {
    failures.push("frontier-local claim was not evaluated independently");
  }

  const platforms = {
    macosAppleSilicon: {
      publicAvailability: "not_public",
      evidence: macosNotarized ? "current_head_notarized_candidate" : "current_head_unsigned_dev",
      candidateEligibility: macosNotarized ? "eligible_pending_release_gate" : "blocked_signing_notarization",
    },
    linuxX64: externalState(linuxEvidence, head, "blocked_current_head_host_unavailable"),
    windowsX64: externalState(windowsEvidence, head, "unavailable_real_host_not_run"),
  };
  const claims = {
    installedLocalAgent: externalState(installedAgentEvidence, head, "private_installed_evidence_required"),
    cloudProviders: externalState(providerEvidence, head, "no_live_provider_certification"),
    frontierLocal: {
      state: frontierGate?.status === "pass" ? "certified_exact_envelope_only" : "blocked_live_high_end_evidence",
      publiclyClaimed: false,
    },
    updater: { state: "disabled_out_of_beta_claims", publiclyClaimed: false },
  };
  const releaseBlockers = [];
  if (supplyChain?.privateReporting?.status !== "pass") {
    releaseBlockers.push("GitHub private vulnerability test report");
  }
  if (releasePlatforms.includes("macosAppleSilicon") && !macosNotarized) {
    releaseBlockers.push("Developer ID signing and notarization");
  }
  if (claims.installedLocalAgent.state !== "pass") releaseBlockers.push("exact-candidate installed-agent recertification");
  if (releasePlatforms.includes("linuxX64") && platforms.linuxX64.state !== "pass") {
    releaseBlockers.push("current-head Linux host evidence");
  }
  if (releasePlatforms.includes("windowsX64") && platforms.windowsX64.state !== "pass") {
    releaseBlockers.push("real Windows host evidence");
  }

  return {
    kind: "desktoplab.scoped-beta-claims",
    schemaVersion: 1,
    status: failures.length === 0 ? "pass" : "fail",
    boundaryAccepted: failures.length === 0,
    publicBetaAllowed: failures.length === 0 && releaseBlockers.length === 0,
    directPrivateSourcePublicationAllowed: false,
    sourceBoundary,
    releasePlatforms,
    head,
    platforms,
    claims,
    releaseBlockers,
    failures,
  };
}

function classifySourceBoundary(exportAudit) {
  const treeVisibility = exportAudit?.sourceTree?.directPublicVisibility;
  const historyVisibility = exportAudit?.sourceHistory?.directPublicVisibility;
  if (treeVisibility === "blocked_use_historyless_export" && historyVisibility === "blocked") {
    return "private_historyless_export";
  }
  if (treeVisibility === "allowed" && historyVisibility === "allowed") {
    return "sanitized_public_checkout";
  }
  return "invalid";
}

function validatePublicClaims(publicClaims, failures) {
  const supportedPlatforms = new Set(["macosAppleSilicon", "linuxX64", "windowsX64"]);
  if (publicClaims?.schemaVersion !== 1) failures.push("public release claims schema is missing or unsupported");
  const releasePlatforms = Array.isArray(publicClaims?.binaryReleasePlatforms)
    ? publicClaims.binaryReleasePlatforms
    : [];
  if (releasePlatforms.length === 0) failures.push("public release platform scope is empty");
  for (const platform of releasePlatforms) {
    if (!supportedPlatforms.has(platform)) failures.push(`unsupported public release platform: ${platform}`);
  }

  const expectedPlatforms = {
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
  };
  for (const [platform, expected] of Object.entries(expectedPlatforms)) {
    for (const [field, value] of Object.entries(expected)) {
      if (publicClaims?.platforms?.[platform]?.[field] !== value) {
        failures.push(`public release claims overstate or omit ${platform}.${field}`);
      }
    }
  }
  const expectedClaims = {
    installedLocalAgent: "exact_installed_evidence_required",
    cloudProviders: "not_publicly_certified",
    frontierLocal: "exact_certified_envelope_only",
    updater: "disabled",
  };
  for (const [claim, expected] of Object.entries(expectedClaims)) {
    if (publicClaims?.claims?.[claim] !== expected) failures.push(`public capability claim mismatch: ${claim}`);
  }
  return [...new Set(releasePlatforms.filter((platform) => supportedPlatforms.has(platform)))];
}

function externalState(evidence, head, blockedState) {
  const evidenceHead = evidence?.commit ?? evidence?.provenance?.head;
  if (evidence?.status === "pass" && evidenceHead === head) {
    return { state: "pass", publiclyClaimed: false, evidenceKind: evidence.kind ?? null };
  }
  return { state: blockedState, publiclyClaimed: false, evidenceKind: evidence?.kind ?? null };
}
