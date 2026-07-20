import assert from "node:assert/strict";
import test from "node:test";
import { linuxSigningPlan, signedReleaseManifest, validateRpmSigningSubkey, validateSignedReleaseShape } from "./linux-release-signing-core.mjs";

const source = {
  kind: "desktoplab.artifact-provenance",
  schemaVersion: 2,
  build: { commitSha: "abc", treeState: "clean" },
  entries: [
    entry("DesktopLab.AppImage", "bundle/DesktopLab.AppImage"),
    entry("DesktopLab.deb", "bundle/DesktopLab.deb"),
    entry("DesktopLab.rpm", "bundle/DesktopLab.rpm"),
  ],
};

test("Linux signing plan requires exact-head AppImage, deb and rpm", () => {
  const plan = linuxSigningPlan(source, "abc");
  assert.deepEqual(plan.artifacts.map((artifact) => artifact.format), ["appimage", "deb", "rpm"]);
  assert.deepEqual(plan.artifacts[2].signatures, ["openpgp-rpm", "sigstore-keyless"]);
  assert.deepEqual(plan.artifacts[1].signatures, ["sigstore-keyless"]);
});

test("Linux signing plan rejects stale or pre-signed source artifacts", () => {
  assert.throws(() => linuxSigningPlan(source, "new"), /exact-head/);
  const signed = structuredClone(source);
  signed.entries[0].signatureState = "signed";
  assert.throws(() => linuxSigningPlan(signed, "abc"), /not an unsigned/);
});

test("signed release manifest requires Sigstore evidence and native rpm signature", () => {
  const plan = linuxSigningPlan(source, "abc");
  const artifacts = plan.artifacts.map((artifact) => ({ ...artifact, sha256: "b".repeat(64), sizeBytes: 20, sigstoreBundleSha256: "c".repeat(64), rpmSignatureState: artifact.format === "rpm" ? "valid" : null }));
  const release = signedReleaseManifest({ plan, signedArtifacts: artifacts, identity: "workflow", issuer: "issuer", runner: "test", releaseChannel: "beta", rpmTrustRoot: { fingerprint: "d".repeat(40), publicKeySha256: "e".repeat(64) } });
  assert.equal(release.status, "pass");
  assert.equal(release.publicTrust, true);
  assert.equal(validateSignedReleaseShape(release), release);
  artifacts[2].rpmSignatureState = "invalid";
  assert.throws(() => signedReleaseManifest({ plan, signedArtifacts: artifacts, identity: "workflow", issuer: "issuer", runner: "test", releaseChannel: "beta", rpmTrustRoot: { fingerprint: "d".repeat(40), publicKeySha256: "e".repeat(64) } }), /rpm native/);
});

test("RPM signing accepts only a signing-capable secret subkey fingerprint", () => {
  const fingerprint = "d".repeat(40).toUpperCase();
  const listing = `sec:u:4096:1:PRIMARY:0:0:::::cC:\nfpr:::::::::PRIMARYFINGERPRINT:\nssb:u:4096:1:SUBKEY:0:0:::::s:\nfpr:::::::::${fingerprint}:`;
  assert.equal(validateRpmSigningSubkey(listing, fingerprint).fingerprint, fingerprint);
  assert.throws(() => validateRpmSigningSubkey(listing, "PRIMARYFINGERPRINT"), /dedicated secret subkey/);
  assert.throws(() => validateRpmSigningSubkey(listing.replace(":::::s:", ":::::e:"), fingerprint), /signing capability/);
});

test("Linux signing implementation stays reviewable", async () => {
  const fs = await import("node:fs");
  for (const [file, limit] of [["scripts/packaging/linux-sign-release.mjs", 180], ["scripts/packaging/linux-verify-release.mjs", 100], ["scripts/packaging/linux-release-signing-core.mjs", 100]]) {
    assert.ok(fs.readFileSync(file, "utf8").split(/\r?\n/).length <= limit, `${file} exceeds ${limit} lines`);
  }
});

test("Linux signer and verifier use the Cosign version subcommand", async () => {
  const fs = await import("node:fs");
  for (const file of ["scripts/packaging/linux-sign-release.mjs", "scripts/packaging/linux-verify-release.mjs"]) {
    const implementation = fs.readFileSync(file, "utf8");
    assert.match(implementation, /requireCommand\("cosign", \["version"\]\)/, `${file} must probe cosign with its version subcommand`);
  }
});

test("Linux signer and verifier isolate RPM trust databases", async () => {
  const fs = await import("node:fs");
  for (const file of ["scripts/packaging/linux-sign-release.mjs", "scripts/packaging/linux-verify-release.mjs"]) {
    const implementation = fs.readFileSync(file, "utf8");
    assert.match(implementation, /rpmDatabase/);
    assert.match(implementation, /run\("rpmkeys", \["--dbpath", rpmDatabase, "--import"/);
    assert.match(implementation, /run\("rpmkeys", \["--dbpath", rpmDatabase, "--define", "_pkgverify_level signature", "--checksig"/);
    assert.doesNotMatch(implementation, /signatures OK/);
    assert.match(implementation, /fs\.rmSync\(rpmDatabase, \{ recursive: true, force: true \}\)/);
  }
});

function entry(fileName, relativePath) {
  return { fileName, relativePath, target: "linux-x64", channel: "dev", signatureState: "unsigned_dev", sha256: "a".repeat(64), sizeBytes: 10 };
}
