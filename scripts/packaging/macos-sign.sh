#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
app_path=""
dry_run=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --app)
      app_path="${2:-}"
      shift 2
      ;;
    --dry-run)
      dry_run=1
      shift
      ;;
    *)
      printf 'unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
done

identity="${DESKTOPLAB_MACOS_SIGNING_IDENTITY:-}"
entitlements="$repo_root/apps/desktop/src-tauri/entitlements/macos.plist"

node "$repo_root/scripts/packaging/verify-empty-macos-entitlements.mjs" "$entitlements"

if [ "$dry_run" -eq 1 ]; then
  printf 'dry-run: macOS signing boundary OK.\n'
  printf 'dry-run: set DESKTOPLAB_MACOS_SIGNING_IDENTITY before real signing.\n'
  printf 'dry-run: codesign will use hardened runtime without entitlements.\n'
  exit 0
fi

if [ -z "$identity" ]; then
  printf 'missing DESKTOPLAB_MACOS_SIGNING_IDENTITY; refusing to sign.\n' >&2
  exit 1
fi

if [ -z "$app_path" ] || [ ! -d "$app_path" ]; then
  printf 'missing --app path to DesktopLab.app; refusing to sign.\n' >&2
  exit 1
fi

codesign_nested_macho() {
  local candidate
  while IFS= read -r -d '' candidate; do
    if ! file -b "$candidate" | grep -q '^Mach-O'; then
      continue
    fi
    codesign \
      --force \
      --timestamp \
      --options runtime \
      --sign "$identity" \
      "$candidate"
  done < <(find "$app_path/Contents" -depth -type f -print0)
}

codesign_nested_macho

codesign \
  --force \
  --timestamp \
  --options runtime \
  --sign "$identity" \
  "$app_path"

codesign --verify --deep --strict --verbose=2 "$app_path"
