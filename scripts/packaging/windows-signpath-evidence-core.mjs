export function buildWindowsSignPathEvidence({ build, artifact, signature, origin }) {
  if (build?.treeState !== "clean" || !/^[a-f0-9]{40}$/.test(build?.commitSha ?? "")) {
    throw new Error("SignPath evidence requires a clean exact commit");
  }
  if (!['beta', 'stable'].includes(build.channel) || build.architecture !== 'x64') {
    throw new Error("SignPath evidence requires a Windows x64 beta or stable build");
  }
  if (signature?.status !== "Valid" || !signature.subject || !signature.issuer) {
    throw new Error("SignPath evidence requires a valid Authenticode signature");
  }
  if (signature.subject === signature.issuer) {
    throw new Error("SignPath evidence refuses self-signed certificates");
  }
  for (const [key, value] of Object.entries(origin)) {
    if (typeof value !== "string" || value.trim() === "") {
      throw new Error(`SignPath evidence is missing ${key}`);
    }
  }
  return {
    kind: "desktoplab.windows-signpath-provenance",
    schemaVersion: 1,
    status: "pass",
    publicTrust: true,
    commit: build.commitSha,
    channel: build.channel,
    artifact,
    signature,
    origin,
  };
}
