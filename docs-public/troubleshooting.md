# Troubleshooting DesktopLab

Status: active for v0.1.0-beta.11
Date: 2026-08-03

This page is the starting point for public beta troubleshooting.

## The Download Fails Verification

Download the package, `SHA256SUMS.txt` and any adjacent signature bundle again
from the same official GitHub Release. Do not use a mirror and do not disable
Gatekeeper or package-signature checks.

If verification still fails, stop and use the
[installation problem form](https://github.com/Vitalisimon/desktoplab/issues/new?template=installation_problem.yml).

## macOS Rejects The App

The official Apple Silicon DMG is Developer ID signed, notarized and stapled.
Move the app to Applications and open it normally. Do not use `xattr`,
`spctl --add` or ad-hoc signing to bypass a rejection.

## Setup Does Not Continue

Check whether the selected runtime or model is compatible with the current machine.

DesktopLab should show a clear blocked reason instead of asking the user to inspect ports, services or configuration files.

## A Local Runtime Does Not Start

Use the setup or runtime screen to retry detection and verification.

Do not manually edit DesktopLab data files unless maintainers ask for a diagnostic bundle.

If Ollama or another local runtime was already running before DesktopLab opened, DesktopLab treats it as user-owned. Closing DesktopLab should not stop that user-owned runtime.

If DesktopLab installed and started a runtime itself, it may stop only that DesktopLab-managed instance during app shutdown.

## A Linux Package Does Not Start

Confirm the host is x86_64/amd64 and use the package intended for the
distribution. The AppImage must be executable. Verify the package checksum and
Sigstore bundle before retrying. For rpm, also verify the native package
signature using the published key.

DesktopLab does not currently publish Linux arm64 packages.

## Cloud API Key Does Not Connect

Confirm that the selected mode is API-key billing.

DesktopLab stores only a native-vault credential reference. It must not echo the key in diagnostics, logs or UI state.

Live cloud-model execution is not a public claim until provider certification is complete.

## Custom Endpoint Is Blocked

OpenAI-compatible custom endpoints must use an explicit `/v1` endpoint URL.

Localhost endpoints are treated differently from remote HTTPS endpoints. Remote endpoints require an egress policy before DesktopLab can use them.

Endpoint validation does not mean model execution is supported. Health checks and model-listing must be certified before custom endpoint execution is advertised.

## External App Bridge Is Blocked

Subscription-account and local app bridge modes are not API-token billing.

DesktopLab may show these modes as future or guided, but they must not become clickable execution paths until bridge discovery, account ownership and capability evidence exist.

## Diagnostics

Diagnostic exports must redact tokens, secrets, local API auth material and private paths before sharing.

The diagnostics export is generated locally. Review the bundle before sharing it with maintainers. It should include a manifest, service states, selected route/backend facts, recent redacted local decisions and bounded size metadata; it should not include prompts, raw tool output, credentials or private absolute paths.

## Operator CLI

The smoke CLI has read-only operator commands for status, doctor lint, diagnostics export, runtime inspect, security audit and migration status. Each command can emit JSON for evidence automation or a short plain summary for humans.

Repair commands are intentionally not part of this read-only operator surface.

Security-sensitive reports should follow `security.md`.

For assistance, reproducible defects and feature requests, use the channels in
`support.md`.
