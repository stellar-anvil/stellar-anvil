# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Scope

Stellar Anvil is a **developer testing and simulation framework** — it is not
deployed on-chain and does not run in production. Its attack surface is
therefore limited to:

- The `stellar-anvil` CLI binary processing user-supplied files (snapshot JSON,
  profile JSON).
- The library crates used inside test suites during development.

Vulnerabilities in the `soroban-sdk` testutils or `stellar-cli` dependencies
should be reported to those projects directly.

## Reporting a vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Instead, report them via GitHub's private
[Security Advisories](https://github.com/stellar-anvil/stellar-anvil/security/advisories/new)
feature (requires a GitHub account).

Include:

1. A description of the vulnerability and its potential impact.
2. Steps to reproduce or a minimal proof-of-concept.
3. The affected version(s).
4. Any suggested mitigations.

You will receive an acknowledgement within **72 hours** and a status update
within **7 days**.

## Disclosure policy

We follow coordinated disclosure. We ask that you give us a reasonable time
(typically 90 days) to fix the issue before public disclosure.
