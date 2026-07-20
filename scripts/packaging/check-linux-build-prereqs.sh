#!/usr/bin/env bash
set -euo pipefail

if [ "$(uname -s)" != "Linux" ]; then
  printf 'Linux packaging prerequisites can only be checked on Linux.\n' >&2
  exit 1
fi

missing=0

for command in cc c++ make curl file pkg-config patchelf rpm rpmbuild wget; do
  if ! command -v "$command" >/dev/null 2>&1; then
    printf 'missing Linux packaging command: %s\n' "$command" >&2
    missing=1
  fi
done

for module in gtk+-3.0 webkit2gtk-4.1 ayatana-appindicator3-0.1 librsvg-2.0 openssl; do
  if ! pkg-config --exists "$module"; then
    printf 'missing Linux packaging pkg-config module: %s\n' "$module" >&2
    missing=1
  fi
done

if [ ! -f /usr/include/xdo.h ]; then
  printf 'missing Linux packaging header: /usr/include/xdo.h\n' >&2
  missing=1
fi

if [ "$missing" -ne 0 ]; then
  printf 'Provision the self-hosted runner as an administrator, then retry.\n' >&2
  exit 1
fi

printf 'Self-hosted Linux packaging prerequisites OK.\n'
