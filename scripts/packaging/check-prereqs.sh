#!/usr/bin/env bash
set -euo pipefail

missing=0

require_command() {
  local name="$1"
  local guidance="$2"
  if ! command -v "$name" >/dev/null 2>&1; then
    printf 'missing: %s\n  %s\n' "$name" "$guidance"
    missing=1
  fi
}

require_command cargo "Install Rust from https://rustup.rs/."
require_command npm "Install Node.js and npm."

if [ "$(uname -s)" = "Linux" ]; then
  require_command rpmbuild "Install rpm-build/rpmbuild to generate DesktopLab RPM dev artifacts."
fi

if ! npm --prefix apps/desktop exec tauri -- --version >/dev/null 2>&1; then
  printf 'missing: @tauri-apps/cli\n  Install project dependencies, then rerun npm run desktop:package:dev.\n'
  missing=1
fi

if [ "$missing" -ne 0 ]; then
  exit 1
fi

printf 'DesktopLab packaging prerequisites OK.\n'
