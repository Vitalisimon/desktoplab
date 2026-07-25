import assert from "node:assert/strict";
import test from "node:test";
import { resolveReleaseScope } from "./release-scope-core.mjs";

function claims({ macOS = "candidate_not_public", linux = "candidate_not_public", windows = "not_public" } = {}) {
  return {
    schemaVersion: 1,
    binaryReleasePlatforms: ["macosAppleSilicon", "linuxX64"],
    platforms: {
      macosAppleSilicon: {
        publicAvailability: macOS,
        evidenceClaim: "signed_notarized_exact_candidate",
      },
      linuxX64: {
        publicAvailability: linux,
        evidenceClaim: "sigstore_signed_exact_candidate",
      },
      windowsX64: {
        publicAvailability: windows,
        evidenceClaim: "test_signed_physical_host_development",
      },
    },
  };
}

test("release scope accepts candidate and already-public in-scope platforms", () => {
  assert.deepEqual(resolveReleaseScope({ claims: claims(), channel: "beta" }).claimKeys, [
    "macosAppleSilicon",
    "linuxX64",
  ]);
  assert.deepEqual(resolveReleaseScope({
    claims: claims({ macOS: "public", linux: "public" }),
    channel: "beta",
  }).claimKeys, ["macosAppleSilicon", "linuxX64"]);
});

test("release scope rejects public availability outside the declared scope", () => {
  assert.throws(
    () => resolveReleaseScope({ claims: claims({ windows: "public" }), channel: "beta" }),
    /windowsX64 public availability/,
  );
});
