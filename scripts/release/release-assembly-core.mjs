import { resolveReleaseScope } from "./release-scope-core.mjs";

export function validateReleaseSource({ releaseRef, channel, head, tagCommit, tagObjectType = "tag" }) {
  const match = releaseRef?.match(/^refs\/tags\/v(\d+\.\d+\.\d+)(?:-(beta)\.(\d+))?$/);
  if (!match) throw new Error("release assembly requires an existing version tag ref");
  const tagChannel = match[2] ? "beta" : "stable";
  if (channel !== tagChannel) throw new Error("release channel differs from version tag");
  if (!/^[a-f0-9]{40}$/.test(head ?? "") || head !== tagCommit) {
    throw new Error("release tag does not resolve to exact checkout HEAD");
  }
  if (tagObjectType !== "tag") throw new Error("release assembly requires an annotated version tag");
  return { releaseRef, tag: releaseRef.slice("refs/tags/".length), version: match[1], channel, commit: head };
}

export function buildReleaseAssembly({ source, platformEvidence, releaseClaims, sbom, updaterProof }) {
  const scope = resolveReleaseScope({ claims: releaseClaims, channel: source.channel });
  const normalized = platformEvidence.map((evidence) => ({
    platform: evidencePlatform(evidence),
    artifacts: normalizePlatformEvidence(evidence, source),
  }));
  for (const platform of scope.platforms) {
    const matches = normalized.filter((item) => item.platform === platform);
    if (matches.length !== 1) throw new Error(`release assembly expected one ${platform} evidence, found ${matches.length}`);
  }
  for (const item of normalized) {
    if (!scope.platforms.includes(item.platform)) throw new Error(`${item.platform} evidence is outside the declared release scope`);
  }
  const artifacts = normalized.flatMap((item) => item.artifacts);
  const verificationAssets = platformEvidence.flatMap((evidence) => verificationAssetsFor(evidence));
  if (artifacts.length === 0) throw new Error("release assembly has no distribution artifacts");
  const names = artifacts.map((artifact) => artifact.fileName);
  if (new Set(names).size !== names.length) throw new Error("release artifact filenames must be unique");
  verifySbom(sbom, source);
  verifyUpdaterProof(updaterProof, source);
  return {
    kind: "desktoplab.release-assembly",
    schemaVersion: 1,
    status: "draft-ready",
    source,
    releaseScope: scope,
    updater: {
      delivery: "disabled",
      hostedManifest: false,
      installPolicy: "manual-replacement",
      rollback: "existing-install-remains-usable-on-failure",
    },
    artifacts,
    verificationAssets,
    sbom: { format: "CycloneDX", specVersion: sbom.specVersion, sourceCommit: source.commit },
  };
}

function evidencePlatform(evidence) {
  if (evidence?.kind === "desktoplab.artifact-provenance" && evidence.schemaVersion === 2) {
    const targets = [...new Set(evidence.entries
      ?.filter((entry) => entry.kind === "distribution_file")
      .map((entry) => entry.target) ?? [])];
    if (targets.length !== 1) throw new Error("artifact provenance must contain one distribution platform");
    return targets[0];
  }
  if (evidence?.kind === "desktoplab.linux-signed-release") return "linux-x64";
  if (evidence?.kind === "desktoplab.windows-signpath-provenance") return "windows-x64";
  throw new Error("unsupported platform release evidence");
}

function verificationAssetsFor(evidence) {
  if (evidence?.kind !== "desktoplab.linux-signed-release") return [];
  const assets = evidence.artifacts.map((entry) => verificationAsset(entry.sigstoreBundle, entry.sigstoreBundleSha256, "sigstore-bundle"));
  assets.push(verificationAsset(evidence.rpmOpenPgp?.publicKey, evidence.rpmOpenPgp?.publicKeySha256, "rpm-public-key"));
  return assets;
}

function verificationAsset(fileName, sha256, role) {
  if (!fileName || !/^[a-f0-9]{64}$/.test(sha256 ?? "")) throw new Error(`Linux ${role} metadata is incomplete`);
  return { fileName, sha256, role };
}

export function normalizePlatformEvidence(evidence, source) {
  if (evidence?.kind === "desktoplab.artifact-provenance" && evidence.schemaVersion === 2) {
    requireExactBuild(evidence.build, source);
    return evidence.entries.filter((entry) => entry.kind === "distribution_file").map((entry) => {
      if (entry.target?.startsWith("macos-") && entry.signatureState !== "notarized") throw new Error(`${entry.fileName} is not notarized`);
      if (entry.target?.startsWith("windows-") && (entry.signatureState !== "signed" || entry.publicTrust !== true)) throw new Error(`${entry.fileName} lacks public Authenticode trust`);
      if (entry.target?.startsWith("linux-")) throw new Error("Linux artifacts require signed Sigstore and RPM release evidence");
      if (!entry.target?.startsWith("macos-") && !entry.target?.startsWith("windows-")) throw new Error(`unsupported release target: ${entry.target}`);
      return artifact(entry, entry.target, entry.signatureState);
    });
  }
  if (evidence?.kind === "desktoplab.linux-signed-release" && evidence.status === "pass") {
    requireExactEvidence(evidence, source);
    if (evidence.platform !== "linux-x64") throw new Error("Linux release evidence platform is invalid");
    if (evidence.publicTrust !== true) throw new Error("Linux release evidence lacks public trust");
    return evidence.artifacts.map((entry) => artifact(entry, evidence.platform, "signed"));
  }
  if (evidence?.kind === "desktoplab.windows-signpath-provenance" && evidence.status === "pass") {
    requireExactEvidence(evidence, source);
    if (evidence.publicTrust !== true || evidence.signature?.status !== "Valid") {
      throw new Error("Windows release evidence lacks trusted Authenticode");
    }
    return [artifact(evidence.artifact, "windows-x64", "signed")];
  }
  throw new Error("unsupported platform release evidence");
}

function requireExactBuild(build, source) {
  if (build?.commitSha !== source.commit || build.treeState !== "clean") throw new Error("artifact provenance is not exact-head clean");
  if (build.channel !== source.channel || build.version !== source.version) throw new Error("artifact version or channel differs from tag");
}

function requireExactEvidence(evidence, source) {
  if (evidence.commit !== source.commit || evidence.channel !== source.channel) throw new Error("signed evidence differs from release source");
}

function artifact(entry, target, signatureState) {
  if (!entry.fileName || !/^[a-f0-9]{64}$/.test(entry.sha256 ?? "") || !(entry.sizeBytes > 0)) {
    throw new Error("release artifact metadata is incomplete");
  }
  return { fileName: entry.fileName, target, sha256: entry.sha256, sizeBytes: entry.sizeBytes, signatureState };
}

function verifySbom(sbom, source) {
  const commit = sbom?.metadata?.properties?.find((item) => item.name === "desktoplab:sourceCommit")?.value;
  if (sbom?.bomFormat !== "CycloneDX" || sbom?.specVersion !== "1.5" || commit !== source.commit) {
    throw new Error("release SBOM is not bound to exact source commit");
  }
}

function verifyUpdaterProof(proof, source) {
  if (proof?.kind !== "desktoplab.updater-disabled-proof" || proof.status !== "passed" || proof.head !== source.commit) {
    throw new Error("updater proof is not bound to exact source commit");
  }
  if (proof.delivery !== "disabled" || proof.hostedManifest !== false || proof.installPolicy !== "manual-replacement") {
    throw new Error("release updater boundary is not fail-closed");
  }
}
