# Security Policy

## Supported Status

ROM is currently experimental software.
Security hardening is improving, but no release should be treated as hardened for production-sensitive workloads.

## Reporting a Vulnerability

Please do not open a public GitHub issue for a suspected security vulnerability.

Instead, report it privately through one of these paths:

- GitHub Security Advisories for this repository, if enabled
- direct contact with the maintainer through the repository profile

When reporting, include:

- a clear description of the issue
- impact assessment
- reproduction steps or proof of concept
- affected commit or branch
- any proposed mitigation, if available

## Response Goals

Best effort goals:

- acknowledge receipt within 7 days
- validate severity and scope
- prepare a fix or mitigation path
- disclose publicly after a fix is available, when appropriate

## Scope

Security reports are especially useful for:

- sandbox escapes
- unsafe host capability exposure
- cross-context data leaks
- cryptographic misuse or key-handling flaws
- request, cookie, or origin isolation failures

Non-security bugs should go through regular GitHub issues.
