import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflow = readFileSync(".github/workflows/linux-release-signing.yml", "utf8");

test("Linux signing workflow requires public repo, OIDC and protected environment", () => {
  assert.match(workflow, /id-token: write/);
  assert.match(workflow, /Reject private repository signing/);
  assert.match(workflow, /if: github\.event\.repository\.private/);
  assert.match(workflow, /environment: linux-release-signing/);
  assert.match(workflow, /DESKTOPLAB_PUBLIC_REPOSITORY: "true"/);
});

test("Linux signing workflow accepts explicit tags and never auto-publishes", () => {
  assert.match(workflow, /\^refs\/tags\/v\[0-9\]/);
  assert.match(workflow, /test "\$GITHUB_REF" = "\$RELEASE_REF"/);
  assert.match(workflow, /persist-credentials: false/);
  assert.match(workflow, /Verify exact immutable source/);
  assert.match(workflow, /git rev-parse \"\$RELEASE_REF\^\{commit\}\"/);
  assert.match(workflow, /Upload signed candidate without publishing/);
  assert.doesNotMatch(workflow, /gh release|softprops\/action-gh-release|contents: write/);
});

test("published RPM trust root is immutable and documented", () => {
  const key = readFileSync("docs-public/desktoplab-rpm-signing-key.asc");
  assert.equal(createHash("sha256").update(key).digest("hex"), "5f9863c9164154be228d4b3408aa990aacbe6352cd6932ce47f689ad46b55904");
  const policy = readFileSync("docs-public/linux-code-signing-policy.md", "utf8");
  assert.match(policy, /EFEDA38FB0C5541C5639F7B41E6FC3BFC5B5A6E0/);
  assert.match(policy, /26088E36AD93318EDC39B9BE8A9ACBFA0830FF3F/);
});

test("Linux signing workflow imports protected RPM key and verifies Sigstore output", () => {
  assert.match(workflow, /LINUX_RPM_OPENPGP_PRIVATE_KEY_B64/);
  assert.doesNotMatch(workflow, /rpm --import/);
  assert.match(workflow, /sigstore\/cosign-installer@398d4b0eeef1380460a10c8013a76f728fb906ac/);
  assert.match(workflow, /install-dir: \$\{\{ runner\.temp \}\}\/cosign/);
  assert.match(workflow, /Verify Cosign installation[\s\S]*cosign version/);
  assert.match(workflow, /packaging:sign:linux/);
  assert.match(workflow, /packaging:verify:linux-signed/);
});
