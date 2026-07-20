#!/usr/bin/env bash
set -euo pipefail

artifact_path=""
dry_run=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --artifact)
      artifact_path="${2:-}"
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

profile="${APPLE_KEYCHAIN_PROFILE:-}"
apple_id="${APPLE_ID:-}"
team_id="${APPLE_TEAM_ID:-}"
password="${APPLE_APP_SPECIFIC_PASSWORD:-}"

if [ "$dry_run" -eq 1 ]; then
  printf 'dry-run: macOS notarization boundary OK.\n'
  printf 'dry-run: use APPLE_KEYCHAIN_PROFILE or APPLE_ID, APPLE_TEAM_ID and APPLE_APP_SPECIFIC_PASSWORD.\n'
  printf 'dry-run: xcrun notarytool submit will run only for a signed artifact.\n'
  exit 0
fi

if [ -z "$artifact_path" ] || [ ! -f "$artifact_path" ]; then
  printf 'missing --artifact path to signed .dmg or .zip; refusing notarization.\n' >&2
  exit 1
fi

if ! command -v xcrun >/dev/null 2>&1; then
  printf 'missing xcrun; notarization requires Apple command line tools.\n' >&2
  exit 1
fi

if [ -n "$profile" ]; then
  xcrun notarytool submit "$artifact_path" --wait --keychain-profile "$profile"
elif [ -n "$apple_id" ] && [ -n "$team_id" ] && [ -n "$password" ]; then
  xcrun notarytool submit "$artifact_path" --wait --apple-id "$apple_id" --team-id "$team_id" --password "$password"
else
  printf 'missing Apple notarization credentials; refusing notarization.\n' >&2
  exit 1
fi
