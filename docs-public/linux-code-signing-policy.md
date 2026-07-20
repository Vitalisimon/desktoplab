# Linux Code Signing Policy

Status: signing credentials provisioned, signed candidate pending
Date: 2026-07-17

DesktopLab is not publicly available for Linux yet. Current package smoke
evidence covers unsigned development artifacts only. This policy defines the
trust boundary that must pass before a Linux package is published.

## Release Trust Stack

Every public AppImage, deb and rpm candidate must be signed as a file with
Sigstore Cosign keyless signing. The detached Sigstore bundle carries the
signature, short-lived certificate and transparency-log evidence needed for
identity-bound verification.

The rpm package must also carry a native OpenPGP package signature produced by
`rpmsign`. Its public signing key and full fingerprint are included in the
signed release candidate.

The AppImage currently uses the detached Sigstore bundle. DesktopLab does not
claim that this lane embeds an AppImage-native OpenPGP signature. The standalone
deb also uses the detached Sigstore bundle; if DesktopLab later operates an APT
repository, the repository must additionally publish authenticated
`InRelease` or `Release` plus `Release.gpg` metadata.

## Trusted Build Boundary

Public signing can run only when all of these conditions are true:

- the historyless DesktopLab repository is public;
- the source is an explicit immutable version tag;
- the GitHub Actions workflow has OIDC identity-token permission;
- the `linux-release-signing` environment has required reviewers;
- the release channel is explicitly `beta` or `stable`;
- exact-source provenance is clean and matches the checked-out commit;
- AppImage, deb and rpm source artifacts all pass package verification;
- the dedicated RPM signing subkey is available through protected secrets;
- every signature is verified before the candidate is uploaded.

The workflow uploads a short-lived signed candidate artifact. It does not create
or publish a GitHub release. Publication remains a separate reviewed decision.

## OpenPGP Key Management

The RPM primary key must be generated and retained offline. CI receives only a
dedicated, expiring signing subkey. The full subkey fingerprint is configured as
`LINUX_RPM_OPENPGP_KEY_ID`; the armored private subkey is stored as base64 in
`LINUX_RPM_OPENPGP_PRIVATE_KEY_B64` inside the protected GitHub environment.
The workflow rejects a primary-key fingerprint and a subkey without signing
capability.

The public key is published at
[`docs-public/desktoplab-rpm-signing-key.asc`](desktoplab-rpm-signing-key.asc),
independent of an individual release artifact. Its primary fingerprint is
`EFEDA38FB0C5541C5639F7B41E6FC3BFC5B5A6E0`; the dedicated signing subkey
fingerprint is `26088E36AD93318EDC39B9BE8A9ACBFA0830FF3F`. Rotation requires an
overlap period in which old and new public keys are available. Compromise or
loss requires immediate environment-secret removal, key revocation, publication
of the revocation certificate and a new release signed by a replacement key.

Only the public RPM key exists in the repository. No private key, backup
password or revocation certificate is tracked, and no public DesktopLab package
is currently represented as signed by this policy.

## Verification

After installing Cosign, verify each file against its adjacent bundle and the
DesktopLab GitHub workflow identity:

```bash
cosign verify-blob DesktopLab.AppImage \
  --bundle DesktopLab.AppImage.sigstore.json \
  --certificate-identity-regexp '^https://github.com/Vitalisimon/desktoplab/.github/workflows/linux-release-signing.yml@refs/tags/v.*$' \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com'
```

Use the same command shape for the deb, rpm and signed artifact manifest. For
the native RPM signature, first compare the distributed public-key checksum and
fingerprint with the signed manifest, then import it and verify the package:

```bash
sudo rpm --import desktoplab-rpm-signing-key.asc
rpm --checksig --verbose DesktopLab.rpm
```

## Activation Gate

This policy is prepared, not accepted. Activation requires a clean public
repository, protected-environment review, creation and offline backup of the
OpenPGP primary key and revocation certificate, installation of the dedicated
signing subkey, a signed tag candidate, independent signature verification and
physical-host install/launch smoke on the exact signed packages.

External format references:

- [Sigstore blob signing](https://docs.sigstore.dev/cosign/signing/signing_with_blobs/)
- [Sigstore blob verification](https://docs.sigstore.dev/cosign/verifying/verify/)
- [RPM package signing](https://rpm.org/docs/6.1.x/man/rpmsign.1)
- [APT archive authentication](https://manpages.debian.org/apt/apt-secure.8.en.html)
- [AppImage signing](https://docs.appimage.org/packaging-guide/optional/signatures.html)
