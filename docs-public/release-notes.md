# DesktopLab Release Notes

Status: macOS beta scope, new exact-head candidate pending
Date: 2026-07-20

DesktopLab is not publicly released yet.

These notes define the boundary of the first accepted beta candidate.

## Product Summary

DesktopLab is a local-first desktop environment for development agents.

It helps users open a repository, complete setup, connect a compatible local runtime or cloud provider path and work from an agent-focused desktop workbench.

## Current Beta Boundary

No public beta artifact is available yet.

Before a beta artifact is shared, DesktopLab must pass the public beta gate for
the claimed platform, verify the private vulnerability reporting channel and
publish accurate installation, security, troubleshooting, runtime and provider
support docs.

## Platform Boundary

Current platform status is documented in `platform-support.md`.

The first binary candidate scope is macOS Apple Silicon only. Historical
Developer ID signing, notarization, stapling and Gatekeeper evidence exists,
but later source and verifier changes invalidate that payload for release.
Release acceptance requires a new final tagged artifact and fresh
installed-agent evidence from the exact same public source commit.

Linux is not part of the first binary candidate scope. AppImage, deb and rpm
development smoke passed on a physical x64 host, but development smoke is not
public release certification and the Linux signing/package-trust policy is not
yet active.

Windows is not part of the first binary candidate scope. A physical Windows 11
x64 host passed test-signed build, install, launch, local API, agent parity and
uninstall verification. Test signing does not establish public publisher trust,
so the installer must not be distributed before the public signing policy is
active.

No cloud provider, frontier-local host/model envelope or automatic application update channel is included in the current public claims.
