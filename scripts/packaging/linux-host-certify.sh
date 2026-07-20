#!/usr/bin/env bash
set -euo pipefail

if [ "$(uname -s)" != "Linux" ]; then
  printf 'Linux host certification must run on Linux.\n' >&2
  exit 1
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"
evidence_dir="dist/release"
mkdir -p "$evidence_dir"

npm run packaging:verify
DESKTOPLAB_LINUX_SMOKE_ATTEMPTS="${DESKTOPLAB_LINUX_SMOKE_ATTEMPTS:-480}" \
  xvfb-run -a bash scripts/packaging/linux-appimage-smoke.sh \
  --artifact apps/desktop/src-tauri/target/debug/bundle/appimage/DesktopLab_0.1.0_amd64.AppImage \
  | tee "$evidence_dir/linux-appimage-smoke.log"
DESKTOPLAB_LINUX_SMOKE_ATTEMPTS="${DESKTOPLAB_LINUX_SMOKE_ATTEMPTS:-480}" \
  xvfb-run -a bash scripts/packaging/linux-deb-smoke.sh \
  --artifact apps/desktop/src-tauri/target/debug/bundle/deb/DesktopLab_0.1.0_amd64.deb \
  | tee "$evidence_dir/linux-deb-smoke.log"
docker run --rm -v "$PWD:/work" -w /work fedora:41 bash -lc \
  "dnf install -y -q curl xorg-x11-server-Xvfb rpm >/tmp/desktoplab-fedora-prep.log && DESKTOPLAB_LINUX_SMOKE_ATTEMPTS=${DESKTOPLAB_LINUX_SMOKE_ATTEMPTS:-480} xvfb-run -a bash scripts/packaging/linux-rpm-smoke.sh --artifact apps/desktop/src-tauri/target/debug/bundle/rpm/DesktopLab-0.1.0-1.x86_64.rpm" \
  | tee "$evidence_dir/linux-rpm-smoke.log"
node scripts/packaging/linux-host-evidence.mjs
