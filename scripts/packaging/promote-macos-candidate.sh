#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

channel="${DESKTOPLAB_RELEASE_CHANNEL:-beta}"
identity="${DESKTOPLAB_MACOS_SIGNING_IDENTITY:-}"
profile="${APPLE_KEYCHAIN_PROFILE:-}"
candidate="${DESKTOPLAB_CANDIDATE_ADMISSION:-dist/release/candidate/admission.json}"
certification="${DESKTOPLAB_CANDIDATE_AGENT_CERTIFICATION:-dist/release/candidate/installed-agent-certification.json}"
safe_signing="${DESKTOPLAB_CANDIDATE_SAFE_SIGNING_REPORT:-dist/release/candidate/safe-signing-regression.json}"
app_path="apps/desktop/src-tauri/target/release/bundle/macos/DesktopLab.app"
dmg_path="apps/desktop/src-tauri/target/release/bundle/dmg/DesktopLab_0.1.0_aarch64.dmg"
dmg_dir="$(dirname "$dmg_path")"
dmg_builder="$dmg_dir/bundle_dmg.sh"
app_zip="dist/desktoplab-packaging/DesktopLab.app.zip"

if [ "$(uname -s)" != "Darwin" ]; then
  printf 'macOS candidate promotion must run on macOS.\n' >&2
  exit 1
fi
if [ "$channel" != "beta" ] && [ "$channel" != "stable" ]; then
  printf 'macOS candidate promotion requires beta or stable channel.\n' >&2
  exit 1
fi
if [ -z "$identity" ] || [ -z "$profile" ]; then
  printf 'macOS candidate promotion requires Developer ID identity and notary keychain profile.\n' >&2
  exit 1
fi

cleanup() {
  rm -f "$app_zip"
}
trap cleanup EXIT

export DESKTOPLAB_RELEASE_CHANNEL="$channel"
export DESKTOPLAB_BUILD_WORKFLOW="npm run desktop:package:macos:promote"
npm run release:verify-public-source
npm run packaging:preflight:apple
npm run release:candidate -- verify --candidate "$candidate" --app "$app_path"
node scripts/release/verify-macos-promotion.mjs --candidate "$candidate" --certification "$certification" --safe-signing "$safe_signing" --app "$app_path"

bash scripts/packaging/macos-sign.sh --app "$app_path"
codesign --verify --deep --strict --verbose=2 "$app_path"
rm -f "$app_zip"
ditto -c -k --keepParent "$app_path" "$app_zip"
bash scripts/packaging/macos-notarize.sh --artifact "$app_zip"
xcrun stapler staple "$app_path"
xcrun stapler validate "$app_path"
codesign --verify --deep --strict --verbose=2 "$app_path"

rm -f "$dmg_path"
bash "$dmg_builder" \
  --volname "DesktopLab" \
  --volicon "$dmg_dir/icon.icns" \
  --window-size 660 400 \
  --icon-size 128 \
  --icon "DesktopLab.app" 180 170 \
  --hide-extension "DesktopLab.app" \
  --app-drop-link 480 170 \
  --no-internet-enable \
  --codesign "$identity" \
  "$dmg_path" \
  "$(dirname "$app_path")"
codesign --verify --deep --strict --verbose=2 "$dmg_path"
bash scripts/packaging/macos-notarize.sh --artifact "$dmg_path"
xcrun stapler staple "$dmg_path"
xcrun stapler validate "$dmg_path"
codesign --verify --deep --strict --verbose=2 "$app_path"
codesign --verify --deep --strict --verbose=2 "$dmg_path"
npm run packaging:verify:macos-metadata -- --app "$app_path" --dmg "$dmg_path" --mode notarized
npm run packaging:audit:macos-entitlements -- --app "$app_path"
spctl --assess --type execute --verbose=2 "$app_path"
node scripts/packaging/record-artifacts.mjs --bundle-dir apps/desktop/src-tauri/target/release/bundle
npm run release:candidate -- transition --candidate "$candidate" --to signed --evidence dist/desktoplab-packaging/artifact-manifest.json --output "$candidate"
bash scripts/packaging/verify-lockfiles-clean.sh
printf 'Promoted the preverified macOS payload without recompiling it.\n'
