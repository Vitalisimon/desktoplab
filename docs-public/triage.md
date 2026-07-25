# Public Issue Triage

Status: active for the public beta
Date: 2026-07-25

DesktopLab uses public issues for reproducible defects and scoped feature
requests. Questions and early ideas belong in GitHub Discussions. Security
reports must use Private Vulnerability Reporting.

## Intake Labels

| Label | Meaning |
| --- | --- |
| `triage` | The report has not yet completed maintainer review. |
| `bug` | Observable behavior differs from the supported product contract. |
| `enhancement` | A user problem may require a product or documentation change. |
| `area: installation` | Download, signature, installation or first-launch path. |
| `needs reproduction` | Maintainers need a minimal repeatable case. |
| `confirmed` | Maintainers reproduced the reported behavior. |
| `release blocker` | Installation, startup, data integrity or primary workflow is blocked for a supported beta platform. |

Labels describe evidence and impact. They do not promise a delivery date.

## Priority

1. Security issues are removed from public intake and handled privately.
2. Data loss, corruption, trust failures, startup failures and installation
   blockers on supported packages are evaluated first.
3. Reproducible primary-workflow failures follow.
4. Degraded workflows, documentation gaps and cosmetic defects follow after
   release-blocking issues.

## Lifecycle

1. Intake checks version, platform, package and privacy completeness.
2. `triage` remains until the report is reproduced, redirected or closed with
   a public explanation.
3. Confirmed defects receive the smallest evidence-backed remediation scope.
4. A fix is not considered released until it appears in a new immutable
   versioned release and passes its applicable certification gates.

Do not attach unreviewed logs. Never publish credentials, private repository
content, prompts, raw tool output or confidential local paths.
