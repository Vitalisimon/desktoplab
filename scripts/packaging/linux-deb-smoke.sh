#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: bash scripts/packaging/linux-deb-smoke.sh --artifact <desktoplab.deb>

Verifies Debian package behavior:
  - requires Linux with dpkg
  - installs the deb package
  - launches DesktopLab
  - verifies packaged local API /health
  - uninstalls the package
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
  printf 'deb smoke must run on Linux.\n' >&2
  exit 1
fi

command -v dpkg >/dev/null 2>&1 || {
  printf 'deb smoke requires dpkg.\n' >&2
  exit 1
}

command -v curl >/dev/null 2>&1 || {
  printf 'Linux deb smoke requires curl.\n' >&2
  exit 1
}

if [ -z "$artifact" ] || [ ! -f "$artifact" ]; then
  printf 'missing --artifact path to DesktopLab deb package.\n' >&2
  exit 1
fi

package_name="$(dpkg-deb -f "$artifact" Package)"
app_data_dir="$(mktemp -d "${TMPDIR:-/tmp}/desktoplab-deb-smoke-app-data.XXXXXX")"
attempts="${DESKTOPLAB_LINUX_SMOKE_ATTEMPTS:-240}"
pid=""
run_sudo() {
  if [ "$(id -u)" -eq 0 ]; then
    "$@"
  elif [ -n "${DESKTOPLAB_LINUX_SMOKE_SUDO_PASSWORD:-}" ]; then
    printf '%s\n' "$DESKTOPLAB_LINUX_SMOKE_SUDO_PASSWORD" | sudo -S -p '' "$@"
  else
    sudo "$@"
  fi
}
resolve_packaged_binary() {
  for candidate in desktoplab desktoplab-desktop desktop-lab DesktopLab; do
    candidate_path="$(command -v "$candidate" || true)"
    if [ -n "$candidate_path" ]; then
      printf '%s\n' "$candidate_path"
      return 0
    fi
  done
  dpkg -L "$package_name" |
    while IFS= read -r package_path; do
      if [ -x "$package_path" ] && [ "${package_path#/usr/bin/}" != "$package_path" ]; then
        printf '%s\n' "$package_path"
        return 0
      fi
    done
}
cleanup() {
  if [ -n "$pid" ] && kill -0 "$pid" >/dev/null 2>&1; then
    kill "$pid" >/dev/null 2>&1 || true
    wait "$pid" >/dev/null 2>&1 || true
  fi
  run_sudo dpkg -r "$package_name" >/dev/null 2>&1 || true
  rm -rf "$app_data_dir"
}
trap cleanup EXIT

run_sudo dpkg -i "$artifact"
binary="$(resolve_packaged_binary || true)"
if [ -z "$binary" ]; then
  printf 'installed DesktopLab binary not found on PATH or in package file list.\n' >&2
  exit 1
fi

DESKTOPLAB_APP_DATA_DIR="$app_data_dir" "$binary" >/tmp/desktoplab-linux-deb-smoke.log 2>&1 &
pid="$!"

discovery="$app_data_dir/local-api-discovery.json"
for _ in $(seq 1 "$attempts"); do
  if [ -f "$discovery" ]; then
    base_url="$(sed -n 's/.*"baseUrl":"\([^"]*\)".*/\1/p' "$discovery")"
    if [ -n "$base_url" ] && curl --fail --silent "$base_url/health" >/dev/null; then
      app_state_code="$(curl --silent --output /dev/null --write-out "%{http_code}" "$base_url/v1/app/state")"
      if [ "$app_state_code" != "401" ]; then
        printf 'Linux deb smoke failed: packaged app state route was not auth-protected.\n' >&2
        exit 1
      fi
      if ! grep -q '"tokenRedacted":"\[REDACTED_LOCAL_API_TOKEN\]"' "$discovery"; then
        printf 'Linux deb smoke failed: discovery document was not redacted.\n' >&2
        exit 1
      fi
      run_sudo dpkg -r "$package_name"
      printf '{"platform":"linux-x64","artifact":"%s","packageFormat":"deb","installState":"passed","launchState":"passed","localApiState":"passed","setupState":"auth_required","cleanupState":"passed"}\n' "$artifact"
      printf 'Linux deb smoke passed: install, launch, local API health, uninstall.\n'
      exit 0
    fi
  fi
  sleep 0.25
done

printf 'Linux deb smoke failed: local API discovery/health did not become ready.\n' >&2
exit 1
