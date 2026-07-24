# Platform Support

Status: scoped macOS and Linux public beta candidate, not public
Date: 2026-07-24

This page describes what DesktopLab can publicly claim.

It must not be stronger than collected packaging and smoke evidence.

## Current State

| Platform | Public availability | Evidence state |
| --- | --- | --- |
| macOS Apple Silicon | Scoped beta candidate, not public yet | A previous exact candidate passed Developer ID signing, notarization and installed-app certification. This release-policy change requires a new public HEAD build and recertification. |
| Linux x64 | Scoped beta candidate, not public yet | A previous exact candidate passed Sigstore/OpenPGP signing and physical-host verification. This release-policy change requires a new public HEAD build and recertification. |
| Windows x64 | Not publicly available | Physical-host development evidence exists, but public publisher trust remains blocked. Windows is outside the scoped beta release and will be reconsidered after trusted signing is available. |

The first binary release has a macOS Apple Silicon and Linux x64 candidate
scope. This is a scope boundary, not beta acceptance: no artifact becomes public
until the exact tagged candidate passes every release, public-trust,
security-reporting and installed-agent gate required for the platforms in scope.
Stable releases still require macOS, Linux and Windows convergence.

## Important Boundaries

- Unsigned artifacts are not trusted public packages.
- Historical macOS Developer ID signing and notarization evidence exists; this does not by itself authorize publication or certify the next source HEAD.
- Windows physical-host development verification is complete. Public distribution still requires publicly trusted signing evidence; current-user self-signed test evidence does not satisfy that gate. See the [Windows code signing policy](windows-code-signing-policy.md).
- Linux public distribution requires activation and exact-candidate evidence for the [prepared Linux code signing policy](linux-code-signing-policy.md); package-format-specific development smoke does not replace that gate.
- Public release readiness is separate from local packaging evidence.
- Packaging mechanics do not prove setup, runtime, model, workbench, provider, file drawer or terminal product readiness. Those claims keep separate evidence gates.
- The scoped public candidate does not imply Windows availability, even though development evidence exists.
