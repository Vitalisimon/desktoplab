# DesktopLab Public Documentation

Status: public beta available for macOS Apple Silicon and Linux x64
Date: 2026-07-25

This directory contains the public documentation for the DesktopLab source
repository.

Documentation outside `docs-public/` is internal by default unless a later public export gate explicitly promotes it.

DesktopLab v0.1.0-beta.10 is available from the official GitHub Release for
macOS Apple Silicon and Linux x64. Windows is not publicly available.

## Public Reader Path

- `install.md`: how users install and start DesktopLab.
- `release-notes.md`: current public beta contents and boundaries.
- `launch-kit-v0.1.0-beta.10.md`: reusable announcement copy and truthful launch boundaries.
- `release-claims.json`: machine-readable platform and capability claim boundary used by release gates.
- `platform-support.md`: current platform availability and evidence level.
- `linux-code-signing-policy.md`: active Sigstore and RPM trust boundary for Linux releases.
- `windows-code-signing-policy.md`: local test-signing boundary and planned public Windows signing policy.
- `runtime-and-provider-support.md`: honest runtime, model and provider claims.
- `supply-chain.md`: dependency, license and artifact hygiene boundary.
- `troubleshooting.md`: user-facing support steps.
- `support.md`: assistance, bug, feature and security-reporting channels.
- `triage.md`: public issue priorities, labels and lifecycle.
- `security.md`: how to report security issues.
- `public-export-gate.md`: rules that must pass before this repository can become public.

## Boundary

DesktopLab can be open source without publishing the internal planning method.

Public docs explain how to use, install, inspect, contribute to and trust the product. Internal plans, audits, task waves, competitive research, freeze reports and operating notes remain private project material.

The private working repository must not be made public by simply changing repository visibility. A clean public export with fresh history is required.

Private competitor analysis may inform implementation, but public docs must cite only DesktopLab behavior and release evidence.
