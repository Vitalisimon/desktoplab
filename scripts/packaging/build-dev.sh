#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

encoded_flags="${CARGO_ENCODED_RUSTFLAGS:-}"
for remap in "--remap-path-prefix=${repo_root}=/workspace" "--remap-path-prefix=${HOME}/.cargo=/cargo"; do
  if [ -n "$encoded_flags" ]; then
    encoded_flags+=$'\x1f'
  fi
  encoded_flags+="$remap"
done
export CARGO_ENCODED_RUSTFLAGS="$encoded_flags"

configure_windows_msvc_linker() {
  case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) ;;
    *) return ;;
  esac
  if [ -n "${CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER:-}" ]; then
    return
  fi

  local candidate
  while IFS= read -r candidate; do
    candidate="${candidate%$'\r'}"
    case "$candidate" in
      *"Microsoft Visual Studio"*\\bin\\Hostx64\\x64\\link.exe)
        export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="$candidate"
        return
        ;;
    esac
  done < <(where.exe link.exe 2>/dev/null || true)

  printf 'The x64 MSVC linker is missing from the Visual Studio developer environment.\n' >&2
  exit 1
}

windows_rustc_wrapper=""
cleanup_windows_rustc_wrapper() {
  if [ -n "$windows_rustc_wrapper" ]; then
    rm -f "$windows_rustc_wrapper"
  fi
}
trap cleanup_windows_rustc_wrapper EXIT

configure_windows_test_rustc_signing() {
  case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) ;;
    *) return ;;
  esac
  if [ "${WINDOWS_SIGNING_TRUST_MODE:-}" != "Test" ]; then
    return
  fi
  if [ -n "${RUSTC_WRAPPER:-}" ]; then
    printf 'RUSTC_WRAPPER is already configured; refusing to replace an unknown compiler boundary.\n' >&2
    exit 1
  fi

  windows_rustc_wrapper="$repo_root/target/desktoplab-windows-rustc-sign-wrapper.exe"
  local wrapper_windows
  wrapper_windows="$(cygpath -w "$windows_rustc_wrapper")"
  pwsh -NoProfile -File scripts/packaging/windows-rustc-signing-bootstrap.ps1 \
    -OutputPath "$wrapper_windows"
  export RUSTC_WRAPPER="$wrapper_windows"
}

bash scripts/packaging/check-prereqs.sh
configure_windows_msvc_linker
configure_windows_test_rustc_signing
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    if [ -n "${WINDOWS_SIGNING_CERTIFICATE_THUMBPRINT:-}" ]; then
      pwsh -NoProfile -File scripts/packaging/windows-sign.ps1 \
        -TrustMode "${WINDOWS_SIGNING_TRUST_MODE:-Test}" \
        -Preflight
    fi
    ;;
esac
npm --prefix apps/desktop run build
tauri_metadata_config="$(node scripts/packaging/prepare-build-metadata.mjs)"
case "$(uname -s)" in
  Linux)
    linux_packaging_path="$(bash scripts/packaging/linux-packaging-path.sh)"
    (
      cd apps/desktop
      PATH="$linux_packaging_path" npm exec tauri -- build --verbose --debug --config "$tauri_metadata_config" --bundles appimage,deb -- --locked
    )
    bash scripts/packaging/linux-rpm-build.sh
    ;;
  *)
    (
      cd apps/desktop
      npm exec tauri -- build --verbose --debug --config "$tauri_metadata_config" -- --locked
    )
    ;;
esac

node scripts/packaging/record-artifacts.mjs

bash scripts/packaging/verify-lockfiles-clean.sh

if [ -n "${WINDOWS_SIGNING_CERTIFICATE_THUMBPRINT:-}" ]; then
  printf 'Locally signed dev packaging artifacts recorded in dist/desktoplab-packaging/dev-artifacts.txt\n'
else
  printf 'Unsigned dev packaging artifacts recorded in dist/desktoplab-packaging/dev-artifacts.txt\n'
fi
