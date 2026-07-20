# Security Policy

Status: public source published; reporting enabled; external path verified

DesktopLab is local-first software that can inspect repositories, start local runtimes, execute commands after approval and store provider credentials through the native OS vault. Security reports must be handled privately before any public disclosure.

## Supported Versions

No public release is currently supported.

| Version | Supported |
| --- | --- |
| public source snapshots | Source available; no versioned binary support commitment |
| current public beta | Latest beta only, after the beta gate is accepted |
| previous public beta | 30 days after the next beta is published |
| future stable release | Current stable release and its immediate predecessor |

## Reporting A Vulnerability

Do not open a public issue with exploit details, secrets, credentials, private repository data, local file paths or proof-of-exploit payloads.

Use [GitHub Private Vulnerability Reporting](https://github.com/Vitalisimon/desktoplab/security/advisories) for confidential reports.

GitHub Private Vulnerability Reporting is enabled on the current public repository. Do not send vulnerability details through public issues or discussions.

The reporter-to-maintainer path has been verified end to end on this repository by an authorized external non-collaborator report. The report was received, triaged and closed without public disclosure. This channel proof does not release or support any DesktopLab binary; public beta binaries remain blocked by the separate exact-source signing and certification gates.

Reports should include:

- affected DesktopLab version or commit;
- operating system and package type;
- affected area, such as local API, vault boundary, runtime install, model download, plugin loading, terminal execution or update flow;
- concise reproduction steps;
- expected impact;
- whether the report includes secrets or user data.

## Response And Disclosure

DesktopLab maintainers will not request public exploit details before a fix or mitigation exists.

For accepted private reports, the maintainer response targets are:

- acknowledgement within 3 business days;
- initial severity and scope assessment within 7 business days;
- a mitigation target within 14 days for accepted critical reports and 30 days for accepted high-severity reports;
- coordinated disclosure only after a fix or mitigation is available, normally within 90 days unless active exploitation or reporter safety requires a different schedule;
- a GitHub Security Advisory and release note when disclosure is appropriate.

These are response targets, not a guarantee that every report is a vulnerability or that every fix can ship inside the target. If a target cannot be met, the reporter should receive a private status update and revised estimate.
