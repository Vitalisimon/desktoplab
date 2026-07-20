#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: bash scripts/packaging/linux-rpm-smoke.sh --artifact <desktoplab.rpm>

Verifies RPM package behavior:
  - requires Linux with rpm and dnf or zypper
  - installs the rpm package
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
  printf 'rpm smoke must run on Linux.\n' >&2
  exit 1
fi

command -v rpm >/dev/null 2>&1 || {
  printf 'rpm smoke requires rpm.\n' >&2
  exit 1
}

command -v curl >/dev/null 2>&1 || {
  printf 'Linux rpm smoke requires curl.\n' >&2
  exit 1
}

if command -v dnf >/dev/null 2>&1; then
  installer=(sudo dnf install -y -q)
  remover=(sudo dnf remove -y -q)
elif command -v zypper >/dev/null 2>&1; then
  installer=(sudo zypper --non-interactive install)
  remover=(sudo zypper --non-interactive remove)
else
  printf 'rpm smoke requires dnf or zypper.\n' >&2
  exit 1
fi

if [ -z "$artifact" ] || [ ! -f "$artifact" ]; then
  printf 'missing --artifact path to DesktopLab rpm package.\n' >&2
  exit 1
fi

package_name="$(rpm -qp --queryformat '%{NAME}' "$artifact")"
app_data_dir="$(mktemp -d "${TMPDIR:-/tmp}/desktoplab-rpm-smoke-app-data.XXXXXX")"
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
  rpm -ql "$package_name" |
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
  run_sudo "${remover[@]:1}" "$package_name" >/dev/null 2>&1 || true
  rm -rf "$app_data_dir"
}
trap cleanup EXIT

run_sudo "${installer[@]:1}" "$artifact"
binary="$(resolve_packaged_binary || true)"
if [ -z "$binary" ]; then
  printf 'installed DesktopLab binary not found on PATH or in package file list.\n' >&2
  exit 1
fi

DESKTOPLAB_APP_DATA_DIR="$app_data_dir" "$binary" >/tmp/desktoplab-linux-rpm-smoke.log 2>&1 &
pid="$!"

discovery="$app_data_dir/local-api-discovery.json"
for _ in $(seq 1 "$attempts"); do
  if [ -f "$discovery" ]; then
    base_url="$(sed -n 's/.*"baseUrl":"\([^"]*\)".*/\1/p' "$discovery")"
    if [ -n "$base_url" ] && curl --fail --silent "$base_url/health" >/dev/null; then
      app_state_code="$(curl --silent --output /dev/null --write-out "%{http_code}" "$base_url/v1/app/state")"
      if [ "$app_state_code" != "401" ]; then
        printf 'Linux rpm smoke failed: packaged app state route was not auth-protected.\n' >&2
        exit 1
      fi
      if ! grep -q '"tokenRedacted":"\[REDACTED_LOCAL_API_TOKEN\]"' "$discovery"; then
        printf 'Linux rpm smoke failed: discovery document was not redacted.\n' >&2
        exit 1
      fi
      run_sudo "${remover[@]:1}" "$package_name"
      printf '{"platform":"linux-x64","artifact":"%s","packageFormat":"rpm","installState":"passed","launchState":"passed","localApiState":"passed","setupState":"auth_required","cleanupState":"passed"}\n' "$artifact"
      printf 'Linux rpm smoke passed: install, launch, local API health, uninstall.\n'
      exit 0
    fi
  fi
  sleep 0.25
done

printf 'Linux rpm smoke failed: local API discovery/health did not become ready.\n' >&2
exit 1
