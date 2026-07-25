# DesktopLab

Status: public beta available for macOS Apple Silicon and Linux x64

DesktopLab is a local-first desktop environment for development agents. It is designed so a user can install the app, open a repository and reach the first useful prompt with minimal setup.

DesktopLab is not just a chat window for a model. It is a local control plane for agent sessions, runtimes, providers, tools, approvals, repositories and evidence.

## Download

The current prerelease is
[DesktopLab v0.1.0-beta.10](https://github.com/Vitalisimon/desktoplab/releases/tag/v0.1.0-beta.10):

- macOS Apple Silicon: signed, notarized and stapled DMG;
- Linux x64: AppImage, deb and rpm with Sigstore bundles; the rpm also carries
  a native OpenPGP package signature;
- Windows x64: not publicly available.

Download only from the official GitHub Release, verify `SHA256SUMS.txt`, and
follow the [installation guide](docs-public/install.md). Never bypass
Gatekeeper or package-signature checks.

## Current Release State

The audited historyless source and the scoped macOS/Linux beta are public in the
[canonical GitHub repository](https://github.com/Vitalisimon/desktoplab).
Release assets are bound to the exact public tag and include provenance,
checksums, an SBOM and updater-disabled evidence.

The private development repository and its history are never published
directly. Public source is produced through the audited export described in
[the public export gate](docs-public/public-export-gate.md).

Start here:

- [Install DesktopLab](docs-public/install.md)
- [v0.1.0-beta.10 release notes](docs-public/release-notes.md)
- [Platform support](docs-public/platform-support.md)
- [Runtime and provider support](docs-public/runtime-and-provider-support.md)
- [Support](docs-public/support.md)

## Code Signing Policy

The macOS beta is Developer ID signed, notarized and stapled. Linux release
files carry keyless Sigstore bundles, and the rpm carries a native OpenPGP
package signature. Platform policies and verification boundaries are
documented here:

- [Windows code signing policy](docs-public/windows-code-signing-policy.md)
- [Linux code signing policy](docs-public/linux-code-signing-policy.md)

No Windows artifact is public or represented as SignPath-signed. Public Windows
distribution remains blocked until trusted signing and exact-artifact
verification are available.

## Product Direction

- local-first;
- offline-first where possible;
- cloud optional;
- runtime agnostic;
- provider agnostic;
- repository focused;
- open-source product first, enterprise governance second.

## Runtime And Model Setup

DesktopLab does not bundle large runtime installers or model weights.

Runtime installers and compatible models are downloaded on demand through setup flows owned by the local backend. The hardware wizard selects compatible options based on host capabilities and blocks unsupported choices with explicit reasons.

## Cloud Optional

Cloud provider bridges are designed to be optional, policy-gated execution paths. They are not public support claims until live account, egress, vault and backend execution evidence exists. Local runtimes remain first-class and must not be hidden behind enterprise or cloud-only gates.
