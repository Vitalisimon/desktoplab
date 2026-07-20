# Windows Code Signing Policy

Status: SignPath application submitted, acceptance pending
Date: 2026-07-17

DesktopLab is not publicly available for Windows yet. This policy describes the
trust boundary that must be accepted before a Windows release is published.

## Development Test Signing

Physical-host certification uses a short-lived self-signed certificate with a
non-exportable private key. The certificate is trusted only in the current
Windows user's certificate stores and is removed after the test campaign.

This lane proves that DesktopLab can build, sign, install, launch and uninstall
on the tested host. It does not establish public publisher identity, SmartScreen
reputation or release eligibility. Test-signed artifacts must not be distributed.

## Planned Public Signing

DesktopLab submitted its application for the free open-source signing service
provided by SignPath.io, with the certificate supplied by SignPath Foundation.
The former source repository was public and no Windows binary was published.
Application was submitted to SignPath Foundation on 2026-07-17. Acceptance has
not been granted. The clean repository must be republished and the application
details revalidated before trusted signing can proceed.

Free code signing provided by [SignPath.io](https://signpath.io/), certificate
by [SignPath Foundation](https://signpath.org/).

The release lane will require:

- an OSI-approved repository license and no proprietary code in signed output;
- an actively maintained, released and publicly documented project;
- multi-factor authentication for repository and signing access;
- declared committers/reviewers and release approvers;
- GitHub as a trusted build system with origin verification;
- release artifacts produced entirely by the reviewed repository workflow;
- manual approval for every release signing request;
- consistent product name and version metadata;
- uninstall support and disclosure of user-visible system changes;
- signature verification for the NSIS installer and installed executable.

## Signing Roles

- Committer and reviewer: [Simone Vitali (`Vitalisimon`)](https://github.com/Vitalisimon)
- Signing approver: [Simone Vitali (`Vitalisimon`)](https://github.com/Vitalisimon)

DesktopLab currently has one maintainer. External contributions require review
by the maintainer. Every release signing request requires a separate manual
approval. These roles will move to repository teams if the maintainer group
grows. No artifact is represented as SignPath-signed before Foundation
acceptance and successful trusted-build verification.

## Privacy Statement

This program will not transfer any information to other networked systems
unless specifically requested by the user or the person installing or operating
it. Networked runtimes, providers, tools and update operations are opt-in and
documented. Local diagnostics and repository content remain local unless the
user chooses an integration whose documented operation requires transfer.

## Verification

Windows release evidence must bind the source commit, dependency lockfiles,
artifact checksums and signing state. The installer and installed application
executable must both report a valid Authenticode signature. Physical-host
install, launch, local API, agent workflow and uninstall checks remain mandatory
even when the signing service accepts the artifact.

See the [SignPath Foundation conditions](https://signpath.org/terms.html) and
[SignPath trusted build system documentation](https://docs.signpath.io/trusted-build-systems/)
for the external requirements governing the planned service.
