#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tauri_root="$repo_root/apps/desktop/src-tauri"
binary="$tauri_root/target/debug/desktoplab-desktop"
version="$(node -e "console.log(require('$repo_root/apps/desktop/package.json').version)")"
release="${DESKTOPLAB_RPM_RELEASE:-1}"
arch="$(rpm --eval '%{_arch}')"
out_dir="$tauri_root/target/debug/bundle/rpm"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/desktoplab-rpm-build.XXXXXX")"

cleanup() {
  rm -rf "$work_dir"
}
trap cleanup EXIT

if [ "$(uname -s)" != "Linux" ]; then
  printf 'DesktopLab RPM build must run on Linux.\n' >&2
  exit 1
fi

command -v rpmbuild >/dev/null 2>&1 || {
  printf 'DesktopLab RPM build requires rpmbuild.\n' >&2
  exit 1
}

if [ ! -x "$binary" ]; then
  printf 'missing built DesktopLab binary: %s\n' "$binary" >&2
  printf 'run Tauri Linux build before RPM packaging.\n' >&2
  exit 1
fi

payload="$work_dir/payload"
topdir="$work_dir/rpmbuild"
mkdir -p "$payload/usr/bin" \
  "$payload/usr/share/applications" \
  "$payload/usr/share/icons/hicolor/32x32/apps" \
  "$payload/usr/share/icons/hicolor/128x128/apps" \
  "$payload/usr/share/icons/hicolor/256x256@2/apps" \
  "$payload/usr/share/icons/hicolor/1024x1024/apps" \
  "$topdir/BUILD" "$topdir/RPMS" "$topdir/SOURCES" "$topdir/SPECS" "$topdir/SRPMS" \
  "$out_dir"

install -m 0755 "$binary" "$payload/usr/bin/desktoplab-desktop"
install -m 0644 "$tauri_root/icons/32x32.png" "$payload/usr/share/icons/hicolor/32x32/apps/desktoplab-desktop.png"
install -m 0644 "$tauri_root/icons/128x128.png" "$payload/usr/share/icons/hicolor/128x128/apps/desktoplab-desktop.png"
install -m 0644 "$tauri_root/icons/128x128@2x.png" "$payload/usr/share/icons/hicolor/256x256@2/apps/desktoplab-desktop.png"
install -m 0644 "$tauri_root/icons/icon.png" "$payload/usr/share/icons/hicolor/1024x1024/apps/desktoplab-desktop.png"
cat > "$payload/usr/share/applications/DesktopLab.desktop" <<'DESKTOP'
[Desktop Entry]
Categories=Development;
Comment=Local-first AI development agent workbench
Exec=desktoplab-desktop
StartupWMClass=desktoplab-desktop
Icon=desktoplab-desktop
Name=DesktopLab
Terminal=false
Type=Application
DESKTOP

spec="$topdir/SPECS/desktoplab.spec"
cat > "$spec" <<SPEC
Name: DesktopLab
Version: $version
Release: $release
Summary: Local-first AI development agent workbench
License: Apache-2.0
URL: https://desktoplab.ai
Requires: webkit2gtk4.1

%description
DesktopLab is a local-first AI development agent workbench.

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}
cp -a %{desktoplab_payload}/usr %{buildroot}/

%files
%defattr(-,root,root,-)
/usr/bin/desktoplab-desktop
/usr/share/applications/DesktopLab.desktop
/usr/share/icons/hicolor/32x32/apps/desktoplab-desktop.png
/usr/share/icons/hicolor/128x128/apps/desktoplab-desktop.png
/usr/share/icons/hicolor/256x256@2/apps/desktoplab-desktop.png
/usr/share/icons/hicolor/1024x1024/apps/desktoplab-desktop.png
SPEC

rpmbuild -bb \
  --define "_topdir $topdir" \
  --define "_build_id_links none" \
  --define "desktoplab_payload $payload" \
  "$spec"

rpm_path="$topdir/RPMS/$arch/DesktopLab-$version-$release.$arch.rpm"
if [ ! -f "$rpm_path" ]; then
  printf 'rpmbuild did not produce expected artifact: %s\n' "$rpm_path" >&2
  exit 1
fi

cp "$rpm_path" "$out_dir/DesktopLab-$version-$release.$arch.rpm"
printf 'DesktopLab RPM artifact created: %s\n' "$out_dir/DesktopLab-$version-$release.$arch.rpm"
