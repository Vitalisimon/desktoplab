# Platform Support

Status: macOS-first public beta candidate, not public
Date: 2026-07-20

This page describes what DesktopLab can publicly claim.

It must not be stronger than collected packaging and smoke evidence.

## Current State

| Platform | Public availability | Evidence state |
| --- | --- | --- |
| macOS Apple Silicon | First beta candidate, not public yet | Historical Developer ID signing, notarization and installed-app evidence exists, but later source and verifier changes invalidate it for the next candidate. An exact new public HEAD rebuild and recertification are required. |
| Linux x64 | Not publicly available | Historical AppImage, deb and rpm physical-host development evidence exists. Exact new public HEAD signing and recertification are required before distribution. |
| Windows x64 | Not publicly available | Historical self-signed NSIS physical-host evidence exists. Exact new public HEAD recertification and public publisher trust remain blocked. |

The first binary release has a macOS-only candidate scope. This is a scope
boundary, not beta acceptance: no artifact becomes public until the exact tagged
candidate passes its release, security-reporting and installed-agent gates.

## Important Boundaries

- Unsigned artifacts are not trusted public packages.
- Historical macOS Developer ID signing and notarization evidence exists; this does not by itself authorize publication or certify the next source HEAD.
- Windows physical-host development verification is complete. Public distribution still requires publicly trusted signing evidence; current-user self-signed test evidence does not satisfy that gate. See the [Windows code signing policy](windows-code-signing-policy.md).
- Linux public distribution requires activation and exact-candidate evidence for the [prepared Linux code signing policy](linux-code-signing-policy.md); package-format-specific development smoke does not replace that gate.
- Public release readiness is separate from local packaging evidence.
- Packaging mechanics do not prove setup, runtime, model, workbench, provider, file drawer or terminal product readiness. Those claims keep separate evidence gates.
- The macOS-only public candidate scope does not imply Linux or Windows availability, even where development evidence exists.
