#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

lockfiles=(
  Cargo.lock
  apps/desktop/src-tauri/Cargo.lock
)

for lockfile in "${lockfiles[@]}"; do
  if [ ! -f "$lockfile" ]; then
    printf 'missing tracked lockfile: %s\n' "$lockfile" >&2
    exit 1
  fi
done

if ! git diff --quiet HEAD -- "${lockfiles[@]}"; then
  printf 'packaging changed or started with an uncommitted Cargo lockfile:\n' >&2
  git status --short -- "${lockfiles[@]}" >&2
  exit 1
fi

printf 'Cargo lockfiles match the committed dependency graphs.\n'
