# Install DesktopLab

Status: public beta installation guide
Date: 2026-08-03

Download v0.1.0-beta.11 only from the official
[DesktopLab GitHub Release](https://github.com/Vitalisimon/desktoplab/releases/tag/v0.1.0-beta.11).
DesktopLab does not distribute installers through mirrors or third-party sites.
Windows binaries are not available.

## What DesktopLab Installs

DesktopLab installs the desktop application and its local control plane.

It does not bundle model weights.

Runtime installers and model downloads are handled on demand by DesktopLab after setup, based on the machine's hardware and the selected runtime/provider path.

## First Launch

On first launch, DesktopLab opens setup before the workbench.

Setup checks the local machine and guides the user toward compatible choices. The user should not need to understand ports, inference servers, environment variables or model formats to reach the first useful prompt.

## Release Channels

| Channel | Meaning |
| --- | --- |
| `dev` | Local development evidence. Not public-ready. |
| `beta` | Public prerelease after the beta gate accepts exact platform evidence. |
| `stable` | Public release channel after signing, notarization, update and platform gates pass. |

## Current Availability

| Platform | Public beta package |
| --- | --- |
| macOS Apple Silicon | `DesktopLab_0.1.0_aarch64.dmg` |
| Linux x64 | AppImage, deb and rpm |
| Windows x64 | Not publicly available |

## macOS Apple Silicon

1. Download `DesktopLab_0.1.0_aarch64.dmg` and `SHA256SUMS.txt` from the same release.
2. Verify the checksum before opening the image:

   ```bash
   shasum -a 256 DesktopLab_0.1.0_aarch64.dmg
   ```

3. Compare the complete output with the matching line in `SHA256SUMS.txt`.
4. Open the DMG and move `DesktopLab.app` to Applications.
5. Launch DesktopLab from Applications. macOS should identify the Developer ID
   publisher without requiring a Gatekeeper bypass.

Do not use `xattr`, `spctl --add`, ad-hoc signing or a Gatekeeper bypass to make
an untrusted download run. Report a rejected official artifact through the
support channels instead.

## Linux x64

Download one package and its adjacent `.sigstore.json` bundle:

- `DesktopLab_0.1.0_amd64.AppImage`
- `DesktopLab_0.1.0_amd64.deb`
- `DesktopLab-0.1.0-1.x86_64.rpm`

Verify the selected package against `SHA256SUMS.txt`, then verify its Sigstore
bundle using the command in the
[Linux signing policy](linux-code-signing-policy.md#verification).

For the AppImage:

```bash
chmod +x DesktopLab_0.1.0_amd64.AppImage
./DesktopLab_0.1.0_amd64.AppImage
```

For Debian or Ubuntu:

```bash
sudo apt install ./DesktopLab_0.1.0_amd64.deb
```

For rpm-based distributions, first verify the public-key fingerprint and native
rpm signature as described in the signing policy, then install with the
distribution package manager.

Report checksum, signature, install or first-launch failures through the
[installation problem form](https://github.com/Vitalisimon/desktoplab/issues/new?template=installation_problem.yml).

## Updates

In-app update checks are disabled in the beta candidate scope. Until DesktopLab has a real hosted channel, a securely managed updater key and signed channel manifests, future builds must be installed manually. A failed or unavailable future update channel must never make the currently installed app unusable.
