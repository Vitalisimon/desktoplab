import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { assertNoConfiguredEntitlements } from "./macos-entitlements-contract.mjs";

const source = readFileSync("scripts/packaging/macos-sign.sh", "utf8");
const auditSource = readFileSync("scripts/packaging/audit-macos-entitlements.mjs", "utf8");
const tauriConfig = JSON.parse(readFileSync("apps/desktop/src-tauri/tauri.conf.json", "utf8"));

test("Developer ID signing covers nested Mach-O executables before the app bundle", () => {
  assert.match(source, /find "\$app_path\/Contents" -depth -type f -print0/);
  assert.match(source, /file -b "\$candidate"/);
  assert.match(source, /codesign_nested_macho/);
  assert.match(source, /codesign_nested_macho\n\ncodesign \\/s);
});

test("empty entitlements are validated but never embedded in the signature", () => {
  assert.match(source, /verify-empty-macos-entitlements\.mjs/);
  assert.doesNotMatch(source, /--entitlements/);
  assert.equal("entitlements" in tauriConfig.bundle.macOS, false);
  assert.match(auditSource, /invalid entitlements blob/);
  assert.match(auditSource, /xmlStart !== -1/);
  assert.match(auditSource, /entitlementBlob: "absent"/);
});

test("the entitlement contract rejects keys until signing support is reviewed", () => {
  assert.doesNotThrow(() => assertNoConfiguredEntitlements({}));
  assert.throws(
    () => assertNoConfiguredEntitlements({ "com.apple.security.app-sandbox": true }),
    /does not yet support configured entitlements: com\.apple\.security\.app-sandbox/,
  );
});
