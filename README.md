# DesktopLab

Status: audited public-source candidate prepared, not published; no public binary release

DesktopLab is a local-first desktop environment for development agents. It is designed so a user can install the app, open a repository and reach the first useful prompt with minimal setup.

DesktopLab is not just a chat window for a model. It is a local control plane for agent sessions, runtimes, providers, tools, approvals, repositories and evidence.

## Current Release State

DesktopLab's audited historyless public-source candidate is prepared locally.
The canonical public repository is intentionally not live yet, and no public
binary has been released. Windows, Linux and macOS package-development evidence
exists, but installers are not public distribution claims until the exact
release-head signing, provenance and platform recertification gates pass.

The private development repository and its history are never published
directly. Public source is produced through the audited export described in
`docs-public/public-export-gate.md`.

See:

- `docs-public/README.md`
- `docs-public/install.md`
- `docs-public/release-notes.md`
- `docs-public/platform-support.md`
- `docs-public/public-export-gate.md`
- `docs-public/support.md`

## Code Signing Policy

Public binaries are not available yet. The platform signing policies and the
controls required before any binary release are documented here:

- [Windows code signing policy](docs-public/windows-code-signing-policy.md)
- [Linux code signing policy](docs-public/linux-code-signing-policy.md)

The Windows policy identifies the signing roles, privacy boundary and planned
SignPath Foundation trusted-build integration. No artifact is represented as
SignPath-signed until Foundation acceptance and exact-artifact verification.

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
