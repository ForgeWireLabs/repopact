# Security Policy

RepoPact is a documentation-and-tooling standard: a set of Markdown/JSON records
and small Python validators with one dependency (`jsonschema`). It executes no
network calls and runs no untrusted input by design.

## Reporting a vulnerability

If you find a security issue — for example a way the validator could be made to
execute arbitrary code, or a frozen-surface bypass that defeats INV-6 — please
report it privately:

- Use GitHub's **"Report a vulnerability"** (Security Advisories) on this
  repository, or
- email the maintainers at the address on the organization profile.

Please do not open a public issue for a suspected vulnerability. We aim to
acknowledge reports within a few days and will credit reporters who wish it.

## Scope

In scope: `scripts/`, the schemas, and the CI workflow. Out of scope: the content
of records in a *downstream* repository that has adopted RepoPact — that is the
adopter's responsibility.

## Supported versions

RepoPact is pre-1.0 (alpha). Only the latest release on `main` is supported.
