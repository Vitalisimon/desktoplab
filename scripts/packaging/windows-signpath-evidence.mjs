#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

import { hashArtifact, writeArtifactEvidence } from "./artifact-provenance-core.mjs";
import { buildWindowsSignPathEvidence } from "./windows-signpath-evidence-core.mjs";

const root = process.cwd();
const outputDir = path.join(root, "dist", "signpath-signed");
const artifactPath = requiredPath(process.env.SIGNPATH_SIGNED_ARTIFACT, "signed artifact");
const signaturePath = requiredPath(path.join(outputDir, "authenticode.json"), "signature evidence");
const build = JSON.parse(
  fs.readFileSync(path.join(root, "dist", "desktoplab-packaging", "DesktopLabBuild.json"), "utf8"),
);
const signature = JSON.parse(fs.readFileSync(signaturePath, "utf8"));
const digest = hashArtifact(artifactPath);
const origin = {
  organizationId: required("SIGNPATH_ORGANIZATION_ID"),
  projectSlug: required("SIGNPATH_PROJECT_SLUG"),
  signingPolicySlug: required("SIGNPATH_SIGNING_POLICY_SLUG"),
  artifactConfigurationSlug: required("SIGNPATH_ARTIFACT_CONFIGURATION_SLUG"),
  signingRequestId: required("SIGNPATH_SIGNING_REQUEST_ID"),
  signingRequestUrl: required("SIGNPATH_SIGNING_REQUEST_URL"),
  sourceArtifactId: required("SIGNPATH_SOURCE_ARTIFACT_ID"),
  githubRunId: required("GITHUB_RUN_ID"),
};
const artifact = {
  fileName: path.basename(artifactPath),
  sha256: digest.sha256,
  sizeBytes: digest.sizeBytes,
};
const evidence = buildWindowsSignPathEvidence({ build, artifact, signature, origin });
const manifest = writeArtifactEvidence({
  root,
  evidenceDir: outputDir,
  artifactPaths: [artifactPath],
  build,
  signatureStateFor: () => "signed",
});
Object.assign(manifest.entries[0], {
  publicTrust: true,
  signingIdentity: signature.subject,
  signatureRef: origin.signingRequestId,
});
fs.writeFileSync(
  path.join(outputDir, "artifact-manifest.json"),
  `${JSON.stringify(manifest, null, 2)}\n`,
);
fs.writeFileSync(
  path.join(outputDir, "signpath-provenance.json"),
  `${JSON.stringify(evidence, null, 2)}\n`,
);
console.log(`Recorded SignPath provenance for ${artifact.fileName}`);

function required(name) {
  const value = process.env[name]?.trim();
  if (!value) throw new Error(`missing ${name}`);
  return value;
}

function requiredPath(value, label) {
  const resolved = path.resolve(root, value ?? "");
  if (!value || !fs.existsSync(resolved)) throw new Error(`missing ${label}`);
  return resolved;
}
