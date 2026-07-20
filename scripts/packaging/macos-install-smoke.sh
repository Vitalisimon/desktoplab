#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source_app=""
mode=""
pid=""
tmp_root=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --dev-artifact)
      mode="dev-artifact"
      shift
      ;;
    --app)
      source_app="${2:-}"
      shift 2
      ;;
    *)
      printf 'unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
done

if [ "$(uname -s)" != "Darwin" ]; then
  printf '{"platform":"macos-aarch64","artifact":"%s","installState":"not_run","launchState":"not_run","localApiState":"not_run","cleanupState":"not_run"}\n' "${source_app:-DesktopLab.app}"
  printf 'macOS install smoke can only run on macOS.\n' >&2
  exit 1
fi

if [ "$mode" != "dev-artifact" ]; then
  printf 'missing --dev-artifact; this smoke validates local unsigned dev artifacts only.\n' >&2
  exit 1
fi

if [ -z "$source_app" ]; then
  source_app="$repo_root/apps/desktop/src-tauri/target/debug/bundle/macos/DesktopLab.app"
fi

if [ ! -d "$source_app" ]; then
  printf 'missing DesktopLab.app dev artifact: %s\n' "$source_app" >&2
  printf 'run npm run desktop:package:dev first.\n' >&2
  exit 1
fi

cleanup() {
  if [ -n "$pid" ] && kill -0 "$pid" >/dev/null 2>&1; then
    kill "$pid" >/dev/null 2>&1 || true
    wait "$pid" >/dev/null 2>&1 || true
  fi
  if [ -n "$tmp_root" ]; then
    rm -rf "$tmp_root"
  fi
}
trap cleanup EXIT

tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/desktoplab-macos-install-smoke.XXXXXX")"
app_data_dir="$tmp_root/app-data"
install_dir="$tmp_root/Applications"
mkdir -p "$app_data_dir" "$install_dir"
cp -R "$source_app" "$install_dir/DesktopLab.app"

binary="$install_dir/DesktopLab.app/Contents/MacOS/desktoplab-desktop"
if [ ! -x "$binary" ]; then
  printf 'missing executable in copied app: %s\n' "$binary" >&2
  exit 1
fi

DESKTOPLAB_APP_DATA_DIR="$app_data_dir" "$binary" >/tmp/desktoplab-macos-install-smoke.log 2>&1 &
pid="$!"

discovery="$app_data_dir/local-api-discovery.json"
base_url=""
for _ in $(seq 1 80); do
  if [ -f "$discovery" ]; then
    base_url="$(sed -n 's/.*"baseUrl":"\([^"]*\)".*/\1/p' "$discovery")"
    if [ -n "$base_url" ] && curl --fail --silent "$base_url/health" >/dev/null; then
      app_state_code="$(curl --silent --output /dev/null --write-out "%{http_code}" "$base_url/v1/app/state")"
      if [ "$app_state_code" != "401" ]; then
        printf 'macOS install smoke failed: packaged app state route was not auth-protected.\n' >&2
        exit 1
      fi
      if ! grep -q '"tokenRedacted":"\[REDACTED_LOCAL_API_TOKEN\]"' "$discovery"; then
        printf 'macOS install smoke failed: discovery document was not redacted.\n' >&2
        exit 1
      fi
      printf '{"platform":"macos-aarch64","artifact":"%s","installState":"passed","launchState":"passed","localApiState":"passed","setupState":"auth_required","cleanupState":"passed"}\n' "$source_app"
      printf 'macOS install smoke passed: copied app launched and local API health responded at %s.\n' "$base_url"
      exit 0
    fi
  fi
  sleep 0.25
done

printf 'macOS install smoke failed: local API discovery/health did not become ready.\n' >&2
printf 'log: /tmp/desktoplab-macos-install-smoke.log\n' >&2
exit 1
