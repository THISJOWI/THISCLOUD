# Security Policy

## Reporting a Vulnerability

THISCLOUD is a hypervisor and cloud platform — security issues are taken seriously.

**Do not open a public issue for security vulnerabilities.** Report privately via GitHub's Private Vulnerability Reporting:

https://github.com/THISJOWI/THISCLOUD/security/advisories

Please include:

- Description of the vulnerability
- Steps to reproduce
- Affected components (daemon, go-api, web-ui, ISO)
- Impact assessment, if known

## Response Times

- **Acknowledgment:** within 48 hours
- **Status update:** within 5 business days
- **Coordinated disclosure:** we will work with you on timing before a public fix is released

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| latest release | Yes |
| older releases | Best effort |

## Scope

The following are in scope:

- `thiscloudd` (Rust daemon)
- `thiscloud-cli`
- `go-api` orchestrator
- `web-ui` dashboard
- ISO build pipeline

Third-party dependencies packaged into the ISO retain their own licenses and security reporting processes.
