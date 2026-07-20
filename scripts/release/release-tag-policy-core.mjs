const stateRank = new Map([
  ["source_admitted", 0],
  ["payload_built", 1],
  ["pre_sign_pass", 2],
  ["signed", 3],
  ["post_sign_pass", 4],
  ["cross_platform_pass", 5],
  ["draft_ready", 6],
]);

export function assessReleaseTag({ candidate, releaseRef, objectType, tagCommit }) {
  const failures = [];
  if (candidate?.kind !== "desktoplab.release-candidate" || candidate?.schemaVersion !== 1) {
    failures.push("release tag candidate contract is invalid");
  }
  if ((stateRank.get(candidate?.state) ?? -1) < stateRank.get("pre_sign_pass")) {
    failures.push("semantic release tag requires pre-sign candidate acceptance");
  }
  if (objectType !== "tag") failures.push("release tag must be annotated");
  const expected = expectedPattern(candidate?.release?.version, candidate?.release?.channel);
  if (!expected?.test(releaseRef ?? "")) failures.push("release ref does not match candidate version and channel");
  if (tagCommit !== candidate?.source?.commit) failures.push("release tag commit differs from candidate source");
  return {
    kind: "desktoplab.release-tag-policy",
    schemaVersion: 1,
    status: failures.length === 0 ? "pass" : "fail",
    releaseRef: releaseRef ?? null,
    candidateId: candidate?.candidateId ?? null,
    commit: tagCommit ?? null,
    failures,
  };
}

function expectedPattern(version, channel) {
  if (!/^\d+\.\d+\.\d+$/.test(version ?? "")) return null;
  const escaped = version.replaceAll(".", "\\.");
  if (channel === "beta") return new RegExp(`^refs/tags/v${escaped}-beta\\.[1-9][0-9]*$`);
  if (channel === "stable") return new RegExp(`^refs/tags/v${escaped}$`);
  return null;
}
