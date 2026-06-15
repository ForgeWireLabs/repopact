---
id: 0002
title: License Under Apache-2.0
status: accepted
date: 2026-06-15
supersedes: []
---

# 0002: License Under Apache-2.0

## Context

RepoPact is published publicly and intended for adoption as a reusable standard
(decision `0001`). With no license, reuse rights are undefined and adoption is
legally ambiguous (backlog item `003` B3).

## Decision

License RepoPact under **Apache License 2.0**.

## Alternatives considered

- **MIT.** Simpler and permissive, but provides no explicit patent grant.
  Rejected: a governance standard that organizations embed benefits from
  Apache-2.0's express patent license and contribution terms.
- **No license / all-rights-reserved.** Rejected: defeats the adoption goal.
- **Copyleft (GPL/MPL).** Rejected: too restrictive for a standard meant to be
  copied into arbitrary (including proprietary) repositories.

## Consequences

- A verbatim `LICENSE` file is added at the repository root.
- Contributions are inbound under the same terms (Apache-2.0 §5).
