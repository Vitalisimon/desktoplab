const FORMAT_BY_EXTENSION = new Map([
  [".AppImage", "appimage"],
  [".deb", "deb"],
  [".rpm", "rpm"],
]);

export function linuxSigningPlan(manifest, head) {
  if (manifest?.kind !== "desktoplab.artifact-provenance" || manifest?.schemaVersion !== 2) {
    throw new Error("Linux signing requires the artifact provenance v2 manifest");
  }
  if (manifest.build?.commitSha !== head || manifest.build?.treeState !== "clean") {
    throw new Error("Linux signing requires clean exact-head artifact provenance");
  }
  const artifacts = manifest.entries.map((entry) => {
    const format = formatFor(entry.fileName);
    if (entry.target !== "linux-x64" || entry.channel !== "dev" || entry.signatureState !== "unsigned_dev") {
      throw new Error(`${entry.fileName} is not an unsigned linux-x64 development artifact`);
    }
    return {
      format,
      sourcePath: entry.relativePath,
      fileName: entry.fileName,
      sourceSha256: entry.sha256,
      sourceSizeBytes: entry.sizeBytes,
      signatures: format === "rpm" ? ["openpgp-rpm", "sigstore-keyless"] : ["sigstore-keyless"],
    };
  });
  const formats = artifacts.map((artifact) => artifact.format).sort();
  if (JSON.stringify(formats) !== JSON.stringify(["appimage", "deb", "rpm"])) {
    throw new Error("Linux signing requires exactly one AppImage, deb and rpm artifact");
  }
  return { commit: head, artifacts };
}

export function signedReleaseManifest({ plan, signedArtifacts, identity, issuer, runner, releaseChannel, rpmTrustRoot }) {
  if (!["beta", "stable"].includes(releaseChannel)) throw new Error("Linux signed release channel must be beta or stable");
  if (signedArtifacts.length !== plan.artifacts.length) throw new Error("signed artifact count differs from signing plan");
  for (const artifact of signedArtifacts) {
    if (!artifact.sha256 || artifact.sizeBytes <= 0 || !artifact.sigstoreBundleSha256) {
      throw new Error(`${artifact.fileName} has incomplete signed evidence`);
    }
    if (artifact.format === "rpm" && artifact.rpmSignatureState !== "valid") {
      throw new Error("rpm native OpenPGP signature is not valid");
    }
  }
  return {
    kind: "desktoplab.linux-signed-release",
    schemaVersion: 1,
    status: "pass",
    commit: plan.commit,
    platform: "linux-x64",
    channel: releaseChannel,
    publicTrust: true,
    sigstore: { mode: "keyless", identity, issuer },
    rpmOpenPgp: rpmTrustRoot,
    runner,
    artifacts: signedArtifacts,
  };
}

export function validateSignedReleaseShape(release) {
  if (release?.kind !== "desktoplab.linux-signed-release" || release?.schemaVersion !== 1 || release?.status !== "pass") {
    throw new Error("unexpected Linux signed release manifest");
  }
  if (release.platform !== "linux-x64" || release.publicTrust !== true || release.sigstore?.mode !== "keyless") {
    throw new Error("Linux signed release trust boundary is incomplete");
  }
  if (!["beta", "stable"].includes(release.channel)) throw new Error("Linux signed release channel is invalid");
  if (!release.rpmOpenPgp?.fingerprint || !release.rpmOpenPgp?.publicKeySha256) {
    throw new Error("Linux signed release RPM trust root is missing");
  }
  const formats = release.artifacts?.map((artifact) => artifact.format).sort();
  if (JSON.stringify(formats) !== JSON.stringify(["appimage", "deb", "rpm"])) {
    throw new Error("Linux signed release requires AppImage, deb and rpm evidence");
  }
  return release;
}

export function validateRpmSigningSubkey(listing, expectedFingerprint) {
  let key = null;
  for (const line of listing.split(/\r?\n/)) {
    const fields = line.split(":");
    if (fields[0] === "sec" || fields[0] === "ssb") {
      key = { type: fields[0], capabilities: fields[11] ?? "" };
    } else if (fields[0] === "fpr" && fields[9]?.toUpperCase() === expectedFingerprint.toUpperCase()) {
      if (key?.type !== "ssb") throw new Error("RPM signing fingerprint must identify a dedicated secret subkey");
      if (!key.capabilities.toLowerCase().includes("s")) throw new Error("RPM signing subkey does not have signing capability");
      return { fingerprint: fields[9], capabilities: key.capabilities };
    }
  }
  throw new Error("RPM signing key fingerprint does not match an available secret subkey");
}

function formatFor(fileName) {
  for (const [extension, format] of FORMAT_BY_EXTENSION) {
    if (fileName.endsWith(extension)) return format;
  }
  throw new Error(`unsupported Linux artifact: ${fileName}`);
}
