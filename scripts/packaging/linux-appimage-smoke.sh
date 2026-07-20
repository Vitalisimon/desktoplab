#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: bash scripts/packaging/linux-appimage-smoke.sh --artifact <DesktopLab.AppImage>

Verifies AppImage launch without package-manager install:
  - requires Linux
  - verifies the artifact is executable
  - launches with isolated DesktopLab app data
  - waits for packaged local API discovery
  - probes /health
USAGE
}

artifact=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --artifact)
      artifact="${2:-}"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      printf 'unknown argument: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [ "$(uname -s)" != "Linux" ]; then
  printf '{"platform":"linux-x64","artifact":"%s","installState":"not_run","launchState":"not_run","localApiState":"not_run","cleanupState":"not_run"}\n' "${artifact:-DesktopLab.AppImage}"
  printf 'AppImage smoke must run on Linux.\n' >&2
  exit 1
fi

if [ -z "$artifact" ] || [ ! -f "$artifact" ]; then
  printf 'missing --artifact path to DesktopLab AppImage.\n' >&2
  exit 1
fi

command -v curl >/dev/null 2>&1 || {
  printf 'Linux AppImage smoke requires curl.\n' >&2
  exit 1
}

chmod +x "$artifact"

tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/desktoplab-appimage-smoke.XXXXXX")"
pid=""
cleanup() {
  if [ -n "$pid" ] && kill -0 "$pid" >/dev/null 2>&1; then
    kill "$pid" >/dev/null 2>&1 || true
    wait "$pid" >/dev/null 2>&1 || true
  fi
  rm -rf "$tmp_root"
}
trap cleanup EXIT

app_data_dir="$tmp_root/app-data"
mkdir -p "$app_data_dir"
DESKTOPLAB_APP_DATA_DIR="$app_data_dir" "$artifact" >/tmp/desktoplab-linux-appimage-smoke.log 2>&1 &
pid="$!"

discovery="$app_data_dir/local-api-discovery.json"
attempts="${DESKTOPLAB_LINUX_SMOKE_ATTEMPTS:-240}"
for _ in $(seq 1 "$attempts"); do
  if [ -f "$discovery" ]; then
    base_url="$(sed -n 's/.*"baseUrl":"\([^"]*\)".*/\1/p' "$discovery")"
    if [ -n "$base_url" ] && curl --fail --silent "$base_url/health" >/dev/null; then
      app_state_code="$(curl --silent --output /dev/null --write-out "%{http_code}" "$base_url/v1/app/state")"
      if [ "$app_state_code" != "401" ]; then
        printf 'Linux AppImage smoke failed: packaged app state route was not auth-protected.\n' >&2
        exit 1
      fi
      if ! grep -q '"tokenRedacted":"\[REDACTED_LOCAL_API_TOKEN\]"' "$discovery"; then
        printf 'Linux AppImage smoke failed: discovery document was not redacted.\n' >&2
        exit 1
      fi
      printf '{"platform":"linux-x64","artifact":"%s","installState":"passed","launchState":"passed","localApiState":"passed","setupState":"auth_required","cleanupState":"passed"}\n' "$artifact"
      printf 'Linux AppImage smoke passed: portable launch and local API health at %s.\n' "$base_url"
      exit 0
    fi
  fi
  sleep 0.25
done

printf 'Linux AppImage smoke failed: local API discovery/health did not become ready.\n' >&2
exit 1
