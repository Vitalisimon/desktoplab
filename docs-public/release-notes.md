# DesktopLab Release Notes

Status: v0.1.0-beta.10 public prerelease
Date: 2026-07-25

[DesktopLab v0.1.0-beta.10](https://github.com/Vitalisimon/desktoplab/releases/tag/v0.1.0-beta.10)
is the first public binary beta. It is built from exact public tag
`v0.1.0-beta.10` at commit `90aa61ba417030934fe9caadc8ea3944d1898ba9`.

## Product Summary

DesktopLab is a local-first desktop environment for development agents.

It helps users open a repository, complete setup, connect a compatible local runtime or cloud provider path and work from an agent-focused desktop workbench.

## What Is Included

- local-first desktop control plane for development-agent sessions;
- repository selection, setup, workbench, approval and evidence flows;
- macOS Apple Silicon DMG;
- Linux x64 AppImage, deb and rpm;
- SHA-256 manifest, exact-source provenance, SBOM and updater-disabled proof;
- Sigstore bundles for Linux release files and a native OpenPGP signature on
  the rpm package.

## Platform Boundary

The macOS DMG passed Developer ID signing, notarization, stapling, Gatekeeper
and clean consumer-install smoke. Linux packages passed exact-artifact
Sigstore/OpenPGP verification and consumer smoke on a physical x64 Ubuntu host.

Windows x64 is not publicly available. Development evidence does not substitute
for a publicly trusted Windows signing identity.

No cloud provider, frontier-local host/model envelope or automatic application update channel is included in the current public claims.

## Known Beta Boundaries

- the in-app updater is disabled; install future betas manually;
- runtime and model availability depends on host compatibility;
- cloud providers and external-agent bridges remain outside public support
  until their live certification gates pass;
- this is a prerelease intended to collect installation and workflow feedback.

See [installation](install.md), [platform support](platform-support.md),
[runtime/provider support](runtime-and-provider-support.md) and
[support](support.md).
