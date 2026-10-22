# Security policy

ALTAIS protects personal archives with end-to-end encryption, so we take
security reports seriously.

## Reporting a vulnerability

**Do not open a public issue.** Report privately through GitHub:

1. Open the repository's **Security** tab.
2. Choose **Report a vulnerability** and describe the problem.

Please include the affected version or commit, steps to reproduce, the impact
you expect, and any proof of concept. We will acknowledge the report, keep you
informed, and credit you in the advisory unless you prefer otherwise.

## Scope

Especially relevant:

- cryptography, key handling and recovery;
- the sync protocol and the ALTAIS Relay server;
- data loss or corruption in import, sync, backup or migration;
- parsing of untrusted input (ENEX, HTML, PDF, images);
- exposure of content, titles or keys through logs, indexes or temporary files.

## Supported versions

ALTAIS has not been released yet. Until release 1.0, only the `main` branch is
supported. The project has no bug bounty programme.

## Status of the cryptography

The cryptographic design has not been independently audited yet. An external
audit is a requirement for release 1.0. Until then, do not rely on ALTAIS to
protect sensitive data.
