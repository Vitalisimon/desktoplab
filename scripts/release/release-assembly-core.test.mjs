import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { buildReleaseAssembly, validateReleaseSource } from "./release-assembly-core.mjs";

const head = "a".repeat(40);
const source = validateReleaseSource({ releaseRef: "refs/tags/v0.1.0-beta.1", channel: "beta", head, tagCommit: head });

test("release source requires an existing exact tag and matching channel", () => {
  assert.equal(source.tag, "v0.1.0-beta.1");
  assert.throws(() => validateReleaseSource({ releaseRef: "refs/heads/main", channel: "beta", head, tagCommit: head }), /version tag/);
  assert.throws(() => validateReleaseSource({ releaseRef: "refs/tags/v0.1.0", channel: "beta", head, tagCommit: head }), /channel/);
  assert.throws(() => validateReleaseSource({ releaseRef: "refs/tags/v0.1.0-beta.1", channel: "beta", head, tagCommit: "b".repeat(40) }), /exact/);
  assert.throws(() => validateReleaseSource({ releaseRef: "refs/tags/v0.1.0-beta.1", channel: "beta", head, tagCommit: head, tagObjectType: "commit" }), /annotated/);
});

test("assembly binds trusted artifact SBOM and disabled updater to one commit", () => {
  const assembly = buildReleaseAssembly({
    source,
    platformEvidence: [artifactEvidence("notarized")],
    releaseClaims: claims(["macosAppleSilicon"]),
    sbom: sbom(head),
    updaterProof: updater(head),
  });
  assert.equal(assembly.status, "draft-ready");
  assert.equal(assembly.artifacts[0].fileName, "DesktopLab.dmg");
  assert.equal(assembly.updater.rollback, "existing-install-remains-usable-on-failure");
});

test("assembly rejects unsigned artifacts and stale supporting evidence", () => {
  assert.throws(() => buildReleaseAssembly({ source, platformEvidence: [artifactEvidence("unsigned_dev")], releaseClaims: claims(["macosAppleSilicon"]), sbom: sbom(head), updaterProof: updater(head) }), /not notarized/);
  assert.throws(() => buildReleaseAssembly({ source, platformEvidence: [artifactEvidence("notarized")], releaseClaims: claims(["macosAppleSilicon"]), sbom: sbom("b".repeat(40)), updaterProof: updater(head) }), /SBOM/);
  assert.throws(() => buildReleaseAssembly({ source, platformEvidence: [artifactEvidence("notarized")], releaseClaims: claims(["macosAppleSilicon"]), sbom: sbom(head), updaterProof: updater("b".repeat(40)) }), /updater proof/);
});

test("generic evidence cannot substitute for Linux Sigstore or Windows public trust", () => {
  const linux = artifactEvidence("signed");
  linux.entries[0].target = "linux-x64";
  assert.throws(() => buildReleaseAssembly({ source, platformEvidence: [linux], releaseClaims: claims(["macosAppleSilicon"]), sbom: sbom(head), updaterProof: updater(head) }), /Sigstore/);
  const windows = artifactEvidence("signed");
  windows.entries[0].target = "windows-x64";
  windows.entries[0].fileName = "DesktopLab.exe";
  assert.throws(() => buildReleaseAssembly({ source, platformEvidence: [windows], releaseClaims: claims(["macosAppleSilicon"]), sbom: sbom(head), updaterProof: updater(head) }), /public Authenticode/);
});

test("Linux signed evidence retains Sigstore bundles and RPM trust root", () => {
  const linux = {
    kind: "desktoplab.linux-signed-release", schemaVersion: 1, status: "pass", publicTrust: true,
    commit: head, channel: "beta", platform: "linux-x64",
    rpmOpenPgp: { publicKey: "desktoplab-rpm.asc", publicKeySha256: "4".repeat(64) },
    artifacts: [{ fileName: "DesktopLab.AppImage", sha256: "2".repeat(64), sizeBytes: 12, sigstoreBundle: "DesktopLab.AppImage.sigstore.json", sigstoreBundleSha256: "3".repeat(64) }],
  };
  const assembly = buildReleaseAssembly({
    source,
    platformEvidence: [artifactEvidence("notarized"), linux],
    releaseClaims: claims(["macosAppleSilicon", "linuxX64"]),
    sbom: sbom(head),
    updaterProof: updater(head),
  });
  assert.deepEqual(assembly.verificationAssets.map((asset) => asset.role), ["sigstore-bundle", "rpm-public-key"]);
});

test("assembly rejects missing or out-of-scope platform evidence", () => {
  const scoped = claims(["macosAppleSilicon", "linuxX64"]);
  assert.throws(() => buildReleaseAssembly({
    source,
    platformEvidence: [artifactEvidence("notarized")],
    releaseClaims: scoped,
    sbom: sbom(head),
    updaterProof: updater(head),
  }), /expected one linux-x64 evidence/);
  const windows = {
    kind: "desktoplab.windows-signpath-provenance",
    status: "pass",
    publicTrust: true,
    commit: head,
    channel: "beta",
    signature: { status: "Valid" },
    artifact: { fileName: "DesktopLab.exe", sha256: "5".repeat(64), sizeBytes: 10 },
  };
  assert.throws(() => buildReleaseAssembly({
    source,
    platformEvidence: [artifactEvidence("notarized"), windows],
    releaseClaims: claims(["macosAppleSilicon"]),
    sbom: sbom(head),
    updaterProof: updater(head),
  }), /outside the declared release scope/);
});

test("release assembly core stays reviewable", () => {
  const logical = readFileSync("scripts/release/release-assembly-core.mjs", "utf8").split("\n").filter((line) => line.trim() && !line.trim().startsWith("//")).length;
  assert.ok(logical <= 170, `release assembly core has ${logical} logical lines, limit 170`);
});

function artifactEvidence(signatureState) {
  return {
    kind: "desktoplab.artifact-provenance", schemaVersion: 2,
    build: { commitSha: head, treeState: "clean", channel: "beta", version: "0.1.0" },
    entries: [{ kind: "distribution_file", fileName: "DesktopLab.dmg", target: "macos-aarch64", sha256: "1".repeat(64), sizeBytes: 10, signatureState }],
  };
}

function sbom(commit) {
  return { bomFormat: "CycloneDX", specVersion: "1.5", metadata: { properties: [{ name: "desktoplab:sourceCommit", value: commit }] } };
}

function updater(commit) {
  return { kind: "desktoplab.updater-disabled-proof", status: "passed", head: commit, delivery: "disabled", hostedManifest: false, installPolicy: "manual-replacement" };
}

function claims(binaryReleasePlatforms) {
  return {
    schemaVersion: 1,
    binaryReleasePlatforms,
    platforms: {
      macosAppleSilicon: {
        publicAvailability: binaryReleasePlatforms.includes("macosAppleSilicon") ? "candidate_not_public" : "not_public",
        evidenceClaim: "signed_notarized_exact_candidate",
      },
      linuxX64: {
        publicAvailability: binaryReleasePlatforms.includes("linuxX64") ? "candidate_not_public" : "not_public",
        evidenceClaim: "sigstore_signed_exact_candidate",
      },
      windowsX64: {
        publicAvailability: binaryReleasePlatforms.includes("windowsX64") ? "candidate_not_public" : "not_public",
        evidenceClaim: binaryReleasePlatforms.includes("windowsX64")
          ? "trusted_authenticode_exact_candidate"
          : "test_signed_physical_host_development",
      },
    },
  };
}
