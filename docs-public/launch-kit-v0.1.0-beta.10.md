# DesktopLab v0.1.0-beta.10 Launch Kit

Status: approved public copy; not a record of external posting
Date: 2026-07-25

Canonical release:
[v0.1.0-beta.10](https://github.com/Vitalisimon/desktoplab/releases/tag/v0.1.0-beta.10)

## Truth Boundary

- DesktopLab is an open-source, local-first desktop control plane for
  development agents.
- The public beta supports macOS Apple Silicon and Linux x64.
- Windows is not publicly available.
- Cloud providers, external-agent bridges and frontier-local model envelopes
  are not publicly certified.
- The in-app updater is disabled.
- Feedback should contain no credentials, private paths, repository content,
  prompts or raw tool output.

## Short Announcement

DesktopLab v0.1.0-beta.10 is now public for macOS Apple Silicon and Linux x64.
It is an open-source, local-first desktop control plane for development agents:
repositories, sessions, runtimes, tools, approvals and evidence in one
persistent environment.

This first beta is deliberately scoped. Windows and uncertified provider/model
paths are not claimed yet. Try it, verify the release assets, and tell us where
installation or the first useful workflow breaks:
https://github.com/Vitalisimon/desktoplab/releases/tag/v0.1.0-beta.10

## Community Post

I have published the first DesktopLab binary beta.

DesktopLab is not another chat wrapper. It is an open-source, local-first
desktop control plane for development agents. The product owns the persistent
session and coordinates repositories, runtimes, execution backends, tools,
approvals and evidence while keeping cloud use optional.

v0.1.0-beta.10 is available for macOS Apple Silicon and Linux x64. The macOS
DMG is Developer ID signed, notarized and stapled. Linux AppImage, deb and rpm
packages ship with Sigstore bundles, and the rpm also has a native OpenPGP
signature. Checksums, provenance, an SBOM and updater-disabled evidence are
included in the release.

This is a focused public beta, not a stable release. Windows is not available,
the updater is disabled, and provider/model support remains limited to the
documented certified boundary.

The most useful feedback right now is concrete: installation, setup, first
repository, first prompt and repeatable workflow failures. Please review logs
before sharing and remove private material.

Release:
https://github.com/Vitalisimon/desktoplab/releases/tag/v0.1.0-beta.10

Issues:
https://github.com/Vitalisimon/desktoplab/issues/new/choose

Q&A:
https://github.com/Vitalisimon/desktoplab/discussions/categories/q-a

## Suggested Distribution

Use the short announcement for a personal social post and the community post
where technical context is expected. Adapt the opening sentence to each
community, but do not change the truth boundary.

Good first audiences:

1. developers already experimenting with local or hybrid coding agents;
2. open-source and local-first software communities;
3. macOS Apple Silicon and Linux desktop users willing to test a prerelease;
4. agent-runtime and developer-tooling communities interested in control-plane
   architecture.

Do not cross-post the same text everywhere at once. Start with maintainer-owned
channels, answer early reports, then expand only after the installation path is
showing healthy real-user evidence.

## Feedback Routing

- installation, checksum, signature or first-launch failure:
  [installation problem form](https://github.com/Vitalisimon/desktoplab/issues/new?template=installation_problem.yml);
- reproducible product defect:
  [bug report](https://github.com/Vitalisimon/desktoplab/issues/new?template=bug_report.yml);
- question or early idea:
  [GitHub Discussions Q&A](https://github.com/Vitalisimon/desktoplab/discussions/categories/q-a);
- vulnerability:
  [Private Vulnerability Reporting](https://github.com/Vitalisimon/desktoplab/security/advisories).
