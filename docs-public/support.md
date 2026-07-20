# Support

Status: public-source candidate; channels activate after publication

DesktopLab separates assistance, defects, feature proposals and security reports so each channel can be triaged with the correct privacy boundary.

## Assistance And Questions

After the repository is published, use [GitHub Discussions Q&A](https://github.com/Vitalisimon/desktoplab/discussions/categories/q-a) for setup, hardware selection, local runtimes, models, provider configuration and usage questions.

Search existing discussions and follow the [troubleshooting guide](troubleshooting.md) first. Include the DesktopLab version, operating system, package type and runtime/model combination when they affect the question.

## Bug Reports

After the repository is published, use the [GitHub issue chooser](https://github.com/Vitalisimon/desktoplab/issues/new/choose) for a reproducible product defect. One issue should describe one observable problem.

The report should contain:

- exact DesktopLab version or source commit;
- operating system and package type;
- minimal reproduction steps;
- expected and actual behavior;
- a reviewed, redacted diagnostic bundle only when needed.

Do not attach secrets, credentials, private paths, repository content, prompts or raw tool output.

## Feature Requests

Use the feature request form for a concrete user problem and desired outcome. Use Discussions for early ideas that still need scope or alternatives explored.

## Security Reports

Never disclose a vulnerability in an issue or discussion. Follow [SECURITY.md](../SECURITY.md). GitHub Private Vulnerability Reporting must be enabled and reverified when the repository is published before confidential reports are accepted there.

## Maintainer Auditability

Maintainers use an authenticated, local-only collector to retrieve complete issue bodies and comments, discussion bodies, comments and replies, and all security-advisory fields exposed by GitHub. The resulting snapshot is stored under ignored `dist/support-audit/` with owner-only permissions and must never be committed or uploaded as a workflow artifact.

This allows maintainers and coding agents with explicit repository authorization to audit reports and prepare code or documentation patches without making private advisory content public.

The public-source gate requires every support and security channel:

```bash
npm run support:audit:github -- --require issues,discussions,advisories,pvr
```

The command fails closed when a required feature is disabled, an API surface is
not readable or pagination cannot be completed. It prints metadata and counts
only; full user-authored content remains in the protected local snapshot.
