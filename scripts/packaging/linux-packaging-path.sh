#!/usr/bin/env bash
set -euo pipefail

packaging_path=""

add_directory() {
  local directory="$1"
  [ -d "$directory" ] || return
  case ":$packaging_path:" in
    *":$directory:"*) return ;;
  esac
  if [ -n "$packaging_path" ]; then
    packaging_path+=":"
  fi
  packaging_path+="$directory"
}

# Tauri resolves Cargo by command name. Keep the selected Rust toolchain ahead
# of system directories that may also contain an older distro Cargo binary.
for command in cargo rustc node npm; do
  resolved="$(command -v "$command" 2>/dev/null || true)"
  if [ -z "$resolved" ]; then
    printf 'missing active Linux packaging toolchain command: %s\n' "$command" >&2
    exit 1
  fi
  add_directory "$(dirname "$resolved")"
done

for directory in /usr/bin /bin /usr/sbin /sbin; do
  add_directory "$directory"
done

printf '%s\n' "$packaging_path"
