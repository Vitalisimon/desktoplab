# Security

Status: public-source candidate; reporting activation and reverification pending
Date: 2026-07-20

DesktopLab is a local-first development agent environment. Security boundaries matter because the app can work with repositories, local files, terminals, runtimes, providers and secrets.

## Reporting Security Issues

Do not publish exploit details in public issues.

The public source repository is not currently published. No public binary is shared by this status.

After the new public repository is published, use [GitHub Private Vulnerability Reporting](https://github.com/Vitalisimon/desktoplab/security/advisories) for confidential reports.

GitHub Private Vulnerability Reporting is not currently available because public repository publication is pending. Do not send vulnerability details through public issues or discussions while this channel is unavailable.

Historical evidence: a historical external non-collaborator report completed the private end-to-end path on the former public repository. The authorized test report was received, triaged and closed without public disclosure. The channel must be enabled and reverified after the new repository is published. This page still does not claim released-binary support.

## Supported Versions

No public version is currently supported because DesktopLab has not reached public beta. Once accepted, the latest beta is supported and the previous beta receives a 30-day transition window. Stable support begins only after the separate stable gate is accepted.

Security reports are targeted for acknowledgement within 3 business days and initial triage within 7 business days. Accepted critical and high-severity reports target mitigation within 14 and 30 days respectively. Coordinated disclosure normally occurs after mitigation and within 90 days; exact handling remains private while a report is active.

## Secret Handling Principles

- Provider keys belong in native OS secret storage.
- Local API tokens must not appear in logs or diagnostics.
- Diagnostic bundles must redact secrets and private local paths.
- Plugins, runtimes and provider bridges must declare permissions and trust level.

## Local Security Audit

DesktopLab exposes a local read-only security audit contract at `/v1/security/audit`.

The audit is separate from general diagnostics. It reports stable check ids for local-only posture, provider egress, approval mode, protected paths, plugin provenance, backend trust level and diagnostics redaction readiness.

The audit output is bounded and redacted. It is not a vulnerability scanner, does not export prompts, raw tool output, secrets or private absolute paths, and any remediation stays routed through the doctor repair contract.

## macOS Runtime Boundary

DesktopLab enables the macOS hardened runtime. It does not enable App Sandbox because its core coding-agent behavior requires access to user-selected repositories, approved terminal subprocesses and local model runtimes. These capabilities remain governed by DesktopLab workspace, approval and audit policy rather than broad macOS entitlements.

The reviewed entitlement set is empty. Generic Keychain access, loopback networking and user-selected file access do not require extra entitlements in this non-sandboxed distribution shape. Developer ID signing and notarization are separate release gates and are not claimed by this document.
