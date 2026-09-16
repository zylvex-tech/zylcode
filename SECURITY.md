# Security Policy

## Supported Versions

ZylCode is pre-1.0. Security fixes are applied to the latest `main` only. There are no back-ported
releases at this time.

## Reporting a Vulnerability

**Do not file public GitHub issues for suspected vulnerabilities.**

Please report privately:

1. **GitHub Security Advisories (preferred).** Use the repository's *Security → Report a
   vulnerability* flow. This is GitHub-native, keeps the report private, and requires no shared email
   address.
2. **Email (proposed — verify before activation).** `security@zylforge.com` is the *intended*
   disclosure address. **It is not verified as reachable in this document.** Before relying on it,
   confirm the mailbox exists and is monitored. Until then, use the GitHub Advisory channel above.

## Response Expectations (read carefully)

We will acknowledge receipt where the reporting channel permits it. **This policy makes no promise of a
specific response time or resolution SLA.** Any statement about response speed would be a claim we
cannot currently meet, and the project's own governance forbids publishing unsupported claims.

What we can commit to:
- Treating every private report as confidential.
- Not weaponizing a report against the reporter.
- Coordinating disclosure with the reporter where feasible.

## Scope Notes

- **Secrets.** Never commit API keys, MCP tokens, cloud credentials, or signing keys. See the Agent
  Operating Protocol. If you find a leaked secret, report it through the channel above — do not open a
  public issue.
- **Local-first.** ZylCode is designed to keep code and artifacts local. Report any behaviour that
  exfiltrates data without explicit, logged user action.
- **Out-of-scope claims.** This policy covers the ZylCode codebase and its published artifacts. It does
  not cover third-party services (Penpot, provider APIs) except where ZylCode's integration introduces
  the vulnerability.
