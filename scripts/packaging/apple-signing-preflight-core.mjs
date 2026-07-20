export function parseDeveloperIdApplications(output) {
  const identities = [];
  const pattern = /^\s*\d+\)\s+([0-9A-F]{40})\s+"Developer ID Application:\s+(.+?)\s+\(([A-Z0-9]{10})\)"\s*$/gim;
  for (const match of output.matchAll(pattern)) {
    identities.push({
      fingerprint: match[1].toUpperCase(),
      teamId: match[3].toUpperCase(),
      displayName: "[REDACTED_SIGNING_IDENTITY]",
      rawIdentity: match[0].replace(/^\s*\d+\)\s+/, "").trim(),
    });
  }
  return identities;
}

export function assessAppleSigningPreflight({
  identities,
  selectedIdentityConfigured,
  selectedIdentityMatches,
  notarytoolAvailable,
  notaryProfileConfigured,
  notaryProfileValidated,
  appleServiceReachable,
  timestampServiceReachable,
  trackedSecretScanPassed,
}) {
  const blockers = [];
  if (identities.length !== 1) blockers.push(`expected exactly one Developer ID Application identity, found ${identities.length}`);
  if (!selectedIdentityConfigured) blockers.push("DESKTOPLAB_MACOS_SIGNING_IDENTITY is not configured");
  else if (!selectedIdentityMatches) blockers.push("configured signing identity does not match the available Developer ID identity");
  if (!notarytoolAvailable) blockers.push("notarytool is unavailable");
  if (!notaryProfileConfigured) blockers.push("APPLE_KEYCHAIN_PROFILE is not configured");
  else if (!notaryProfileValidated) blockers.push("notarytool keychain profile validation failed");
  if (!appleServiceReachable) blockers.push("Apple service connectivity failed");
  if (!timestampServiceReachable) blockers.push("Apple timestamp service connectivity failed");
  if (!trackedSecretScanPassed) blockers.push("tracked source secret scan failed");
  return {
    kind: "desktoplab.apple-signing-preflight",
    schemaVersion: 1,
    status: blockers.length === 0 ? "pass" : "blocked",
    safeToSign: false,
    signingPerformed: false,
    identityCount: identities.length,
    identities: identities.map(({ fingerprint, teamId, displayName }) => ({ fingerprint, teamId, displayName })),
    checks: {
      selectedIdentityConfigured,
      selectedIdentityMatches,
      notarytoolAvailable,
      notaryProfileConfigured,
      notaryProfileValidated,
      appleServiceReachable,
      timestampServiceReachable,
      trackedSecretScanPassed,
    },
    blockers,
  };
}
