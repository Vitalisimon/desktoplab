#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

encoded_flags="${CARGO_ENCODED_RUSTFLAGS:-}"
for remap in "--remap-path-prefix=${repo_root}=/workspace" "--remap-path-prefix=${HOME}/.cargo=/cargo"; do
  if [ -n "$encoded_flags" ]; then
    encoded_flags+=$'\x1f'
  fi
  encoded_flags+="$remap"
done
export CARGO_ENCODED_RUSTFLAGS="$encoded_flags"

channel="${DESKTOPLAB_RELEASE_CHANNEL:-beta}"
if [ "$(uname -s)" != "Darwin" ]; then
  printf 'macOS candidate preparation must run on macOS.\n' >&2
  exit 1
fi
if [ "$channel" != "beta" ] && [ "$channel" != "stable" ]; then
  printf 'macOS candidate preparation requires beta or stable channel.\n' >&2
  exit 1
fi

export DESKTOPLAB_RELEASE_CHANNEL="$channel"
export DESKTOPLAB_BUILD_WORKFLOW="npm run desktop:package:macos:prepare"
npm run release:verify-public-source

source_admission="${DESKTOPLAB_CANDIDATE_SOURCE_ADMISSION:-dist/release/candidate/source-admission.json}"
candidate_admission="${DESKTOPLAB_CANDIDATE_ADMISSION:-dist/release/candidate/admission.json}"
app_path="apps/desktop/src-tauri/target/release/bundle/macos/DesktopLab.app"

npm run release:candidate -- admit-source --channel "$channel" --output "$source_admission"
npm --prefix apps/desktop run build
tauri_metadata_config="$(DESKTOPLAB_MACOS_SIGNING_IDENTITY="-" node scripts/packaging/prepare-build-metadata.mjs)"
(
  cd apps/desktop
  npm exec tauri -- build --config "$tauri_metadata_config" --bundles app,dmg -- --locked
)
npm run packaging:verify:macos-metadata -- --app "$app_path" --mode dev
npm run release:candidate -- bind-payload --candidate "$source_admission" --app "$app_path" --output "$candidate_admission"
npm run release:candidate -- verify --candidate "$candidate_admission" --app "$app_path"
bash scripts/packaging/verify-lockfiles-clean.sh
printf 'Prepared immutable macOS candidate at %s. No Developer ID signing or notarization was performed.\n' "$app_path"
