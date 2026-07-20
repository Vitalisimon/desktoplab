# Install DesktopLab

Status: macOS beta installation candidate
Date: 2026-07-20

DesktopLab is not publicly released yet.

When a beta is accepted, download it only from the official
[DesktopLab GitHub Releases](https://github.com/Vitalisimon/desktoplab/releases)
page. DesktopLab does not distribute installers through mirrors or third-party
download sites.

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
| `beta` | Candidate channel after the public beta gate accepts platform evidence. |
| `stable` | Public release channel after signing, notarization, update and platform gates pass. |

## Current Availability

DesktopLab is not available as a public beta yet.

Current platform evidence is tracked in `platform-support.md`.

Current platform status before a public beta installer can be shared:

- Windows NSIS packaging has current physical-host development evidence with a current-user self-signed certificate; it is not publicly trusted or distributable.
- Linux AppImage, deb and rpm packaging have current unsigned development smoke evidence.
- macOS has historical Developer ID signing and notarization evidence; the next candidate must be rebuilt and recertified from the exact new public HEAD.

## macOS Apple Silicon

The first binary beta is limited to Apple Silicon Macs.

1. Download the `.dmg` and `SHA256SUMS.txt` from the same GitHub release.
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

Windows and Linux downloads are intentionally absent until their public signing
and exact-package verification gates pass.

## Updates

In-app update checks are disabled in the beta candidate scope. Until DesktopLab has a real hosted channel, a securely managed updater key and signed channel manifests, future builds must be installed manually. A failed or unavailable future update channel must never make the currently installed app unusable.
