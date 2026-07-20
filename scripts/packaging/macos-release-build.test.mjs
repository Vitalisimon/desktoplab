import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const legacySource = readFileSync("scripts/packaging/build-macos-release.sh", "utf8");
const prepareSource = readFileSync("scripts/packaging/prepare-macos-candidate.sh", "utf8");
const source = readFileSync("scripts/packaging/promote-macos-candidate.sh", "utf8");
const metadataSource = readFileSync("scripts/packaging/prepare-build-metadata.mjs", "utf8");

test("release build is optimized, locked, signed, notarized and stapled", () => {
  assert.match(prepareSource, /release:verify-public-source/);
  assert.match(prepareSource, /--remap-path-prefix=\$\{repo_root\}=\/workspace/);
  assert.match(prepareSource, /--remap-path-prefix=\$\{HOME\}\/\.cargo=\/cargo/);
  assert.match(prepareSource, /CARGO_ENCODED_RUSTFLAGS/);
  assert.match(prepareSource, /tauri -- build/);
  assert.match(source, /macos-sign\.sh --app "\$app_path"/);
  assert.doesNotMatch(prepareSource, /--debug/);
  assert.match(prepareSource, /--locked/);
  assert.match(source, /macos-notarize\.sh/);
  assert.match(source, /stapler staple/);
  assert.match(source, /stapler validate/);
  assert.match(source, /spctl --assess/);
  assert.match(source, /packaging:verify:macos-metadata -- --app "\$app_path" --dmg "\$dmg_path" --mode notarized/);
  assert.match(source, /packaging:audit:macos-entitlements -- --app "\$app_path"/);
  assert.match(source, /record-artifacts\.mjs --bundle-dir apps\/desktop\/src-tauri\/target\/release\/bundle/);
  assert.doesNotMatch(source, /record-artifacts\.mjs --bundle-dir .*target\/debug\/bundle/);
});

test("release DMG contains the already notarized and stapled app", () => {
  const explicitSign = source.indexOf('macos-sign.sh --app "$app_path"');
  const initialVerification = source.indexOf('codesign --verify --deep --strict --verbose=2 "$app_path"');
  const archiveApp = source.indexOf('ditto -c -k --keepParent "$app_path" "$app_zip"');
  const notarizeApp = source.indexOf('macos-notarize.sh --artifact "$app_zip"');
  const stapleApp = source.indexOf('stapler staple "$app_path"');
  const rebuildDmg = source.indexOf('bash "$dmg_builder"');
  const notarizeDmg = source.indexOf('macos-notarize.sh --artifact "$dmg_path"');
  const stapleDmg = source.indexOf('stapler staple "$dmg_path"');
  const finalVerification = source.indexOf('packaging:verify:macos-metadata -- --app "$app_path" --dmg "$dmg_path" --mode notarized');

  assert.ok(explicitSign >= 0, "Tauri output must be explicitly signed by the reviewed signer");
  assert.ok(explicitSign < initialVerification, "explicit signing must precede the first strict verification");
  assert.ok(archiveApp >= 0, "release build must archive the signed app for notarization");
  assert.ok(archiveApp < notarizeApp, "the app archive must exist before app notarization");
  assert.ok(notarizeApp < stapleApp, "the app must be notarized before it is stapled");
  assert.ok(stapleApp < rebuildDmg, "the DMG must be rebuilt from the stapled app");
  assert.ok(rebuildDmg < notarizeDmg, "the rebuilt DMG must be notarized");
  assert.ok(notarizeDmg < stapleDmg, "the DMG must be notarized before it is stapled");
  assert.ok(stapleDmg < finalVerification, "the final app and DMG verification must run after all mutations");
});

test("release build refuses unsigned or non-release channels", () => {
  assert.match(source, /beta.*stable/);
  assert.match(source, /DESKTOPLAB_MACOS_SIGNING_IDENTITY/);
  assert.match(source, /APPLE_KEYCHAIN_PROFILE/);
});

test("legacy monolithic release command is fail closed", () => {
  assert.match(legacySource, /monolithic macOS release command is disabled/);
  assert.match(legacySource, /exit 1/);
  assert.doesNotMatch(legacySource, /tauri -- build|macos-sign\.sh|macos-notarize\.sh/);
});

test("embedded provenance names the release workflow for non-dev channels", () => {
  assert.match(prepareSource, /export DESKTOPLAB_RELEASE_CHANNEL="\$channel"/);
  assert.match(prepareSource, /export DESKTOPLAB_BUILD_WORKFLOW=/);
  assert.ok(
    prepareSource.indexOf("export DESKTOPLAB_RELEASE_CHANNEL") < prepareSource.indexOf("prepare-build-metadata.mjs"),
    "release channel must reach metadata generation",
  );
  assert.match(metadataSource, /channel === "dev"/);
  assert.match(metadataSource, /desktop:package:macos:release/);
});

test("dev metadata treats a blank macOS signing secret as ad hoc signing", () => {
  assert.match(
    metadataSource,
    /DESKTOPLAB_MACOS_SIGNING_IDENTITY\?\.trim\(\) \|\| "-"/,
  );
  assert.match(metadataSource, /macOS: \{ signingIdentity: macosSigningIdentity \}/);
});
