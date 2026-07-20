# Public Export Gate

Status: historyless export candidate audited; repository publication pending
Date: 2026-07-20

This gate defines how DesktopLab can become public without publishing the internal planning method.

The goal is one public repository, not a permanent split into separate public and private product repos.

## Core Rule

The published repository's tracked surface and Git history must remain safe to expose on every commit.

Deleting private files in a final commit is not enough, because public Git history can still expose removed documents.

## Public Surface

Allowed public material:

- source code intended for open-source distribution;
- license and security policy;
- public README;
- user installation docs;
- platform support docs;
- runtime/provider support claims;
- public troubleshooting docs;
- contribution and governance docs written for outside contributors.

Private by default:

- internal plans;
- task waves;
- freeze reports;
- audit notes;
- competitor research;
- strategic open-core notes;
- release operation notes with host-specific context;
- prompts, methodology and process documents.

Local private documentation lives in `docs/` and is ignored by Git, like local environment files. Public documentation lives in `docs-public/`.

## Export Path

The current private working repository must not be made public by changing repository visibility.

Reason: the current Git history contains private planning documents, even when those files are no longer present in the current public tree.

Accepted implementation:

1. define the public export allowlist;
2. generate a public export candidate from the current repository tree;
3. initialize the public repository from that candidate with fresh history;
4. verify the public candidate contains no private docs, secrets or host-specific material;
5. switch only the clean public repository to public visibility after the gate is accepted.

This keeps the product in one public repository after publication while preserving the private working history locally.

## Local Commands

Generate the public tree candidate:

```bash
npm run product:public-export
```

Audit the current source tree and generated public candidate:

```bash
npm run product:public-export:audit
```

The audit is expected to report `directPublicVisibility: "blocked_use_historyless_export"` for the private working repository as long as private documents exist in the current tree or Git history. That is correct and does not fail the export audit by itself. The public candidate must exist and have no private findings.

To intentionally test whether the private working repository itself can be made public, run:

```bash
npm run product:public-export:audit -- --direct-source
```

That mode must fail while private docs or private history are present.

## Required Checks

- `docs-public/` contains complete public docs for the claimed release.
- `docs-public/release-claims.json` matches the human-readable platform and capability claims.
- root `README.md` points only to public docs.
- root `SECURITY.md` contains a real private contact channel.
- GitHub Issues use validated bug and feature forms with blank issues disabled.
- GitHub Discussions is enabled with a Q&A category for assistance.
- GitHub Private Vulnerability Reporting is enabled and has received, triaged
  and privately closed an authorized end-to-end report from a non-collaborator.
- `npm run support:audit:github -- --require issues,discussions,advisories,pvr`
  completes without findings under an authorized maintainer account.
- private docs are absent from the public export candidate.
- `AGENTS.md` and internal agent operating instructions are absent from the public candidate.
- generated artifacts and local `.env` files are absent.
- no secrets, tokens, passwords, local hostnames or private IPs are exposed.
- public claims match collected evidence.

## Current Gate Result

The historyless export candidate passes with zero private findings. The public
repository has not been recreated or published. The private source tree and its
history remain deliberately blocked from direct publication. The export
manifest records the exact source commit, clean-tree state and lockfile hashes.
Publication still requires a fresh one-commit repository, a second tree and
history audit, green public CI, activation and reverification of support and
Private Vulnerability Reporting channels, and explicit push authorization.
This gate does not accept a public beta binary; signing, notarization and
exact-candidate product evidence remain separate blockers.
