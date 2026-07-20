#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

candidate="${DESKTOPLAB_CANDIDATE_ADMISSION:-dist/release/candidate/admission.json}"
source_app="apps/desktop/src-tauri/target/release/bundle/macos/DesktopLab.app"
install_root="/Applications"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --candidate) candidate="${2:-}"; shift 2 ;;
    --app) source_app="${2:-}"; shift 2 ;;
    --install-root) install_root="${2:-}"; shift 2 ;;
    *) printf 'unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
done

if [ "$(uname -s)" != "Darwin" ]; then
  printf 'macOS candidate installation must run on macOS.\n' >&2
  exit 1
fi
if [ ! -d "$source_app" ] || [ ! -f "$candidate" ] || [ ! -d "$install_root" ]; then
  printf 'candidate, prepared app or install root is missing.\n' >&2
  exit 1
fi

target="$install_root/DesktopLab.app"
staged="$install_root/.DesktopLab.candidate.$$"
backup="$install_root/.DesktopLab.previous.$$"
committed=0

cleanup() {
  rm -rf "$staged"
  if [ "$committed" -ne 1 ] && [ -d "$backup" ]; then
    rm -rf "$target"
    mv "$backup" "$target"
  fi
}
trap cleanup EXIT

npm run release:verify-public-source
node scripts/release/candidate-admission.mjs verify --candidate "$candidate" --app "$source_app"
node scripts/packaging/verify-macos-candidate-install.mjs \
  --candidate "$candidate" --source-app "$source_app" --installed-app "$source_app"

duplicate_roots=("/Applications")
if [ -d "$HOME/Applications" ]; then duplicate_roots+=("$HOME/Applications"); fi
installed_copies=()
while IFS= read -r copy; do installed_copies+=("$copy"); done < <(find "${duplicate_roots[@]}" -maxdepth 1 -type d -name 'DesktopLab.app' -print)
for copy in "${installed_copies[@]}"; do
  if [ "$(cd "$(dirname "$copy")" && pwd -P)/$(basename "$copy")" != "$(cd "$install_root" && pwd -P)/DesktopLab.app" ]; then
    printf 'another DesktopLab.app exists at %s; refusing ambiguous installation.\n' "$copy" >&2
    exit 1
  fi
done

osascript -e 'tell application id "ai.desktoplab.desktop" to quit' >/dev/null 2>&1 || true
for _ in $(seq 1 40); do
  if ! pgrep -x desktoplab-desktop >/dev/null 2>&1; then break; fi
  sleep 0.25
done
if pgrep -x desktoplab-desktop >/dev/null 2>&1; then
  printf 'DesktopLab is still running; close it before candidate installation.\n' >&2
  exit 1
fi

rm -rf "$staged" "$backup"
ditto "$source_app" "$staged"
node scripts/packaging/verify-macos-candidate-install.mjs \
  --candidate "$candidate" --source-app "$source_app" --installed-app "$staged"
if [ -d "$target" ]; then mv "$target" "$backup"; fi
mv "$staged" "$target"
node scripts/packaging/verify-macos-candidate-install.mjs \
  --candidate "$candidate" --source-app "$source_app" --installed-app "$target"

final_copies=()
while IFS= read -r copy; do final_copies+=("$copy"); done < <(find "${duplicate_roots[@]}" -maxdepth 1 -type d -name 'DesktopLab.app' -print)
if [ "${#final_copies[@]}" -ne 1 ] || [ "${final_copies[0]}" != "$target" ]; then
  printf 'single-install verification failed after candidate installation.\n' >&2
  exit 1
fi

rm -rf "$backup"
committed=1
printf 'Installed exact pre-sign candidate at %s.\n' "$target"
