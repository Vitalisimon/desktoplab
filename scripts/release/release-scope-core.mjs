const PLATFORM_TARGETS = {
  macosAppleSilicon: "macos-aarch64",
  linuxX64: "linux-x64",
  windowsX64: "windows-x64",
};

const TRUSTED_EVIDENCE_CLAIMS = {
  macosAppleSilicon: "signed_notarized_exact_candidate",
  linuxX64: "sigstore_signed_exact_candidate",
  windowsX64: "trusted_authenticode_exact_candidate",
};

export function resolveReleaseScope({ claims, channel }) {
  if (claims?.schemaVersion !== 1) throw new Error("release scope claims schema is missing or unsupported");
  if (!["beta", "stable"].includes(channel)) throw new Error("release scope channel is invalid");

  const claimKeys = claims.binaryReleasePlatforms;
  if (!Array.isArray(claimKeys) || claimKeys.length === 0) throw new Error("release platform scope is empty");
  if (new Set(claimKeys).size !== claimKeys.length) throw new Error("release platform scope contains duplicates");
  for (const key of claimKeys) {
    if (!PLATFORM_TARGETS[key]) throw new Error(`unsupported release platform scope: ${key}`);
  }

  const canonicalKeys = Object.keys(PLATFORM_TARGETS);
  if (channel === "stable" && canonicalKeys.some((key) => !claimKeys.includes(key))) {
    throw new Error("stable release scope requires macOS, Linux and Windows");
  }
  for (const key of canonicalKeys) {
    const platform = claims.platforms?.[key];
    const included = claimKeys.includes(key);
    const expectedAvailability = included ? "candidate_not_public" : "not_public";
    if (platform?.publicAvailability !== expectedAvailability) {
      throw new Error(`${key} public availability does not match release scope`);
    }
    if (included && platform.evidenceClaim !== TRUSTED_EVIDENCE_CLAIMS[key]) {
      throw new Error(`${key} release scope lacks its exact public-trust evidence claim`);
    }
  }

  const orderedClaimKeys = canonicalKeys.filter((key) => claimKeys.includes(key));
  return {
    claimKeys: orderedClaimKeys,
    platforms: orderedClaimKeys.map((key) => PLATFORM_TARGETS[key]),
  };
}
