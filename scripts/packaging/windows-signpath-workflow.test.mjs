import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { buildWindowsSignPathEvidence } from "./windows-signpath-evidence-core.mjs";

const workflow = readFileSync(".github/workflows/windows-signpath.yml", "utf8");
const policy = readFileSync(".signpath/policies/desktoplab/release-signing.yml", "utf8");
const owners = readFileSync(".github/CODEOWNERS", "utf8");

test("SignPath workflow stays inert before acceptance and binds signing to GitHub origin", () => {
  assert.match(workflow, /submit_for_signing:\n[\s\S]*default: false/);
  assert.match(workflow, /repository\.visibility != 'public'/);
  assert.match(workflow, /actions\/upload-artifact@v4/);
  assert.match(workflow, /signpath\/github-action-submit-signing-request@v2/);
  assert.match(workflow, /github-artifact-id:.*artifact-id/);
  assert.match(workflow, /SIGNPATH_API_TOKEN/);
  assert.match(workflow, /windows-install-smoke\.ps1/);
  assert.match(workflow, /Public SignPath evidence cannot use a self-signed certificate/);
  assert.match(workflow, /release_ref:/);
  assert.match(workflow, /ref: \$\{\{ inputs\.release_ref \}\}/);
  assert.match(workflow, /git cat-file -t "\$RELEASE_REF"/);
  assert.match(workflow, /git rev-parse "\$RELEASE_REF\^\{commit\}"/);
  assert.doesNotMatch(workflow, /self-hosted/);
});

test("SignPath source policy requires GitHub-hosted non-rerun builds and code ownership", () => {
  assert.match(policy, /require_github_hosted: true/);
  assert.match(policy, /disallow_reruns: true/);
  assert.match(owners, /\.signpath\/ @Vitalisimon/);
  assert.match(owners, /windows-signpath\.yml @Vitalisimon/);
});

test("SignPath provenance accepts only clean public-trust evidence", () => {
  const evidence = fixture();
  assert.equal(evidence.status, "pass");
  assert.equal(evidence.publicTrust, true);
  assert.throws(
    () => buildWindowsSignPathEvidence({
      ...fixtureInput(),
      signature: { status: "Valid", subject: "CN=Local", issuer: "CN=Local" },
    }),
    /refuses self-signed/,
  );
});

function fixture() {
  return buildWindowsSignPathEvidence(fixtureInput());
}

function fixtureInput() {
  return {
    build: {
      treeState: "clean",
      commitSha: "a".repeat(40),
      channel: "beta",
      architecture: "x64",
    },
    artifact: { fileName: "DesktopLab.exe", sha256: "b".repeat(64), sizeBytes: 10 },
    signature: { status: "Valid", subject: "CN=DesktopLab", issuer: "CN=Trusted CA" },
    origin: {
      organizationId: "org",
      projectSlug: "desktoplab",
      signingPolicySlug: "release-signing",
      artifactConfigurationSlug: "windows-v1",
      signingRequestId: "request",
      signingRequestUrl: "https://app.signpath.io/request",
      sourceArtifactId: "123",
      githubRunId: "456",
    },
  };
}
