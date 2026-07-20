const requiredPlatforms = ["macos-aarch64", "linux-x64", "windows-x64"];

export function assessPlatformCandidateConvergence({ candidate, evidence }) {
  const failures = [];
  if (candidate?.kind !== "desktoplab.release-candidate" || candidate?.schemaVersion !== 1) {
    failures.push("platform convergence candidate contract is invalid");
  }
  if (candidate?.state !== "post_sign_pass") failures.push("platform convergence requires post-sign candidate acceptance");

  const platforms = (evidence ?? []).flatMap(normalizeEvidence);
  for (const platform of requiredPlatforms) {
    const matches = platforms.filter((entry) => entry.platform === platform);
    if (matches.length !== 1) failures.push(`expected one ${platform} evidence, found ${matches.length}`);
  }
  for (const entry of platforms) {
    if (entry.commit !== candidate?.source?.commit) failures.push(`${entry.platform} commit differs from candidate`);
    if (entry.channel !== candidate?.release?.channel) failures.push(`${entry.platform} channel differs from candidate`);
    if (entry.status !== "pass" || entry.publicTrust !== true) failures.push(`${entry.platform} lacks passing public trust`);
  }

  return {
    kind: "desktoplab.platform-candidate-convergence",
    schemaVersion: 1,
    status: failures.length === 0 ? "pass" : "fail",
    candidateId: candidate?.candidateId ?? null,
    commit: candidate?.source?.commit ?? null,
    channel: candidate?.release?.channel ?? null,
    requiredPlatforms,
    platforms,
    failures,
  };
}

function normalizeEvidence(value) {
  if (value?.kind === "desktoplab.artifact-provenance" && value.schemaVersion === 2) {
    const mac = value.entries?.filter((entry) => entry.target === "macos-aarch64") ?? [];
    if (mac.length === 0) return [];
    return [{
      platform: "macos-aarch64",
      commit: value.build?.commitSha,
      channel: value.build?.channel,
      status: mac.every((entry) => entry.signatureState === "notarized") ? "pass" : "fail",
      publicTrust: mac.every((entry) => entry.signatureState === "notarized"),
    }];
  }
  if (value?.kind === "desktoplab.linux-signed-release") {
    return [{ platform: "linux-x64", commit: value.commit, channel: value.channel, status: value.status, publicTrust: value.publicTrust === true }];
  }
  if (value?.kind === "desktoplab.windows-signpath-provenance") {
    return [{ platform: "windows-x64", commit: value.commit, channel: value.channel, status: value.status, publicTrust: value.publicTrust === true }];
  }
  return [];
}
