# Platform Support

Status: scoped macOS and Linux public beta
Date: 2026-08-03

This page describes what DesktopLab can publicly claim.

It must not be stronger than collected packaging and smoke evidence.

## Current State

| Platform | Public availability | Evidence state |
| --- | --- | --- |
| macOS Apple Silicon | Public beta | v0.1.0-beta.11 passed Developer ID signing, notarization, stapling, Gatekeeper and public-download consumer verification. |
| Linux x64 | Public beta | v0.1.0-beta.11 AppImage, deb and rpm passed exact-artifact Sigstore/OpenPGP verification, physical-host smoke and public-download checksum verification. |
| Windows x64 | Not publicly available | Physical-host development evidence exists, but public publisher trust remains blocked. Windows is outside the scoped beta release and will be reconsidered after trusted signing is available. |

The current binary release scope is macOS Apple Silicon and Linux x64. Stable
releases still require macOS, Linux and Windows convergence.

## Important Boundaries

- Unsigned artifacts are not trusted public packages.
- macOS Developer ID signing and notarization alone does not by itself authorize publication or certify a later source HEAD. The current beta is public because the exact tagged artifacts passed the complete release gate.
- Windows physical-host development verification is complete. Public distribution still requires publicly trusted signing evidence; current-user self-signed test evidence does not satisfy that gate. See the [Windows code signing policy](windows-code-signing-policy.md).
- Linux public distribution follows the active [Linux code signing policy](linux-code-signing-policy.md); each later release still requires fresh exact-candidate evidence.
- Public release readiness is separate from local packaging evidence.
- Packaging mechanics do not prove setup, runtime, model, workbench, provider, file drawer or terminal product readiness. Those claims keep separate evidence gates.
- The scoped public candidate does not imply Windows availability, even though development evidence exists.
