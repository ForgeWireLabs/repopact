# Threats to validity

Recorded so the paper's evaluation cannot be read as stronger than it is. Updated as
threats are identified or retired.

## T1 — Reflexivity / progenitor adoption (the central threat)

RepoPact was not designed in the abstract and then tested on forgewire. It was
**distilled from forgewire's own working practices**: the tiered `AGENTS.md`
contracts, the `_audit` inventory/alignment system, todos, logs, history, and
trackers. Consequently:

- forgewire adopting cleanly (capture 004, F-007) is **confirmatory**, not
  independent — the architecture is meeting the project it was abstracted from. It
  proves the `adopt` tooling works and that the kernel re-integrates into its source
  in a structured, explainable way; it does **not** prove the model generalizes.
- The greenfield proving ground (`unitconv`) was author-built specifically to
  exercise RepoPact, so it shares the same author-as-evaluator bias.

**Mitigation (planned).** Adopt at least one repository with **no lineage to
RepoPact** — preferably an external open-source project that has its own CODEOWNERS
and CI — and report the result whether it holds or cracks. Until that datum exists,
the brownfield/generality claim is worded as "demonstrated on the progenitor."

## T2 — Single evaluator, single session

All evidence was produced by one operator/agent pair in one session. No independent
re-runner has reproduced the captures. **Mitigation:** every run links a raw capture
and the exact commands, so the procedure is reproducible by a third party.

## T3 — Scale and domain narrowness

The greenfield subject is trivial (a unit converter); the brownfield subject is a
single application. The record formats may strain on very large monorepos, polyglot
trees, or unusual `_audit` layouts. **Mitigation:** the conformance fixtures
(`tests/fixtures/`) and additional adopters broaden coverage over time.

## T4 — "Holds" findings are absence-of-failure

A finding marked *holds* means a specific adversarial case was caught; it is not a
proof that no bypass exists. **Mitigation:** treat the hypothesis list as falsifiable
and additive, not as a closed proof.
