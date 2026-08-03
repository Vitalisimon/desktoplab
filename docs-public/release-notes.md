# DesktopLab Release Notes

Status: v0.1.0-beta.11 public prerelease
Date: 2026-08-03

[DesktopLab v0.1.0-beta.11](https://github.com/Vitalisimon/desktoplab/releases/tag/v0.1.0-beta.11)
is built from exact public tag `v0.1.0-beta.11` at commit
`e46835ec8fef9ddc2633c54aa4436515c80510a9`.

## Product Summary

DesktopLab is a local-first desktop environment and persistent control plane
for development agents. It keeps repositories, sessions, runtimes, tools,
approvals and evidence together while local execution remains primary.

## What Changed Since Beta.10

- automatic setup now exposes measured host capability, detects existing
  runtimes and shows a confirmation-gated plan before installation or launch;
- runtime ownership is explicit, so user-owned services are not silently
  adopted or stopped by DesktopLab;
- the supported Ollama path has stronger macOS vendor-signature, concurrent
  setup, health and recovery checks;
- connected LM Studio, managed LM Studio headless and managed MLX-LM are
  included as Preview lifecycle paths with explicit ownership boundaries;
- constrained local-model responses and recovery after malformed or repeated
  read-only tool output are grounded more strictly before completion;
- the public README now includes source-backed setup, workbench, approval and
  repository-tool captures.

## What Is Included

- local-first desktop control plane for development-agent sessions;
- repository selection, setup, workbench, approval and evidence flows;
- macOS Apple Silicon DMG;
- Linux x64 AppImage, deb and rpm;
- SHA-256 manifest, exact-source provenance, SBOM and updater-disabled proof;
- Sigstore bundles for Linux release files and a native OpenPGP signature on
  the rpm package.

## Runtime Boundary

The beta.11 public runtime claim is limited to the exact Ollama automatic-agent
route on macOS Apple Silicon and Linux x64. LM Studio and MLX-LM are labeled
Preview rather than inheriting that claim. Their exact ownership and lifecycle
paths have candidate evidence, but broader runtime/model support requires
additional evidence. The current managed MLX SmolLM3 model is inspection-only
and cannot mutate a workspace.

No cloud provider, external-agent bridge or arbitrary custom endpoint is
publicly certified.

## Platform Boundary

The macOS DMG passed Developer ID signing, notarization, stapling, Gatekeeper
and anonymous public-download verification. Linux packages passed exact-artifact
Sigstore/OpenPGP verification, physical x64 host smoke, agent parity and
anonymous public-download checksum verification.

Windows x64 is not publicly available. Development evidence does not substitute
for a publicly trusted Windows signing identity.

## Known Beta Boundaries

- the in-app updater is disabled; install future betas manually;
- runtime and model availability depends on host compatibility;
- Preview routes may expose narrower capabilities than the supported Ollama
  route;
- this is a prerelease intended to collect installation and workflow feedback.

See [installation](install.md), [platform support](platform-support.md),
[runtime/provider support](runtime-and-provider-support.md) and
[support](support.md).
