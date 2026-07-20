# Supply Chain

Status: public source published; binary publication evidence pending
Date: 2026-07-20

DesktopLab source is published through the audited historyless repository. No
DesktopLab binary is publicly released.

This page records the public-facing dependency, license and artifact hygiene boundary for beta candidates.

## Project License

DesktopLab source is prepared under `Apache-2.0`.

The root `LICENSE` file is present.

## Exact-Source Dependency Review

`npm run security:supply-chain` generates ignored release evidence for the exact
Git commit and all three lockfile hashes. The evidence includes:

- Rust and npm advisory reports with tool and advisory-database versions;
- complete Cargo and npm dependency inventories;
- SPDX expression evaluation with unknown and restricted licenses failing closed;
- a CycloneDX 1.5 SBOM bound to the source commit and lock hashes;
- scans of the historyless public export, packaged app and real diagnostics export;
- artifact provenance comparison against the exact source commit.

The command exits non-zero for any local evidence failure. GitHub Private
Vulnerability Reporting is enabled and the authenticated channel audit passes
on the current repository. An authorized external non-collaborator report
completed the reporter-to-maintainer path: it was received, triaged and closed
without public disclosure. No environment flag can replace that external
evidence, and binary release remains blocked whenever reporting or exact-source
artifact provenance is stale or missing.

## Artifact Hygiene

Generated packages, checksums, SBOMs, dependency trees, local build output and diagnostics must not be committed.

Ignored generated paths include:

- `dist/`
- `target/`
- `dist/desktoplab-packaging/`
- `dist/release/supply-chain/`
- platform packages such as `.dmg`, `.msi`, `.AppImage`, `.deb` and `.rpm`

Local environment files such as `.env` are ignored and must not be committed.

## Secret Hygiene

Release workflows and documentation may name secret variables, but must not contain secret values.

Examples of allowed names:

- `DESKTOPLAB_MACOS_SIGNING_IDENTITY`
- `APPLE_KEYCHAIN_PROFILE`
- `WINDOWS_SIGNING_CERTIFICATE_PATH`
- `WINDOWS_SIGNING_CERTIFICATE_PASSWORD`
- `WINDOWS_SIGNING_TIMESTAMP_URL`
- `LINUX_RPM_OPENPGP_KEY_ID`
- `LINUX_RPM_OPENPGP_PRIVATE_KEY_B64`

Public beta remains blocked if release evidence requires a secret value that has not been configured in a safe store.

Rust builds remap workspace and Cargo-home source paths before packaging. An app
bundle containing the local home or repository path fails the supply-chain gate.
Internal QA evidence may contain local runner paths, but it is excluded from the
public export and is never a public diagnostic bundle.

Before a binary beta candidate is accepted, dependency, license, SBOM, privacy
and artifact-provenance checks must be rerun against the exact source and
release commit.
