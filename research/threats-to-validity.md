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

**Mitigation (in progress).** Adopt repositories with **no lineage to RepoPact** and
report the result whether it holds or cracks.
- *Done:* pallets/flask (F-009, capture 006) — an unrelated OSS project reached a
  conformant RepoPact via `adopt`. This independently exercises the workflows + sparse
  path.
- *Done:* SkillForge Academy (F-012, capture 011) — an independent, different-domain
  real app (Tauri cert-learning) ran the full adopt+import-plan+doctor lifecycle to
  conformant, non-destructively. Adds an independent datum for flat-todo + sectioned-
  roadmap import and for coexistence with a homegrown audit/tracking system.
- *Still open:* an independent repo that also has CODEOWNERS and nested contracts, so
  those mappings are shown to generalize beyond the progenitor (forgewire). SkillForge
  has neither, so the CODEOWNERS/nested-contract generality still rests on forgewire.

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

## T5 — Benchmark selection and task curation (for H8–H10)

The comparative studies (`benchmark-protocol.md`) are only as honest as their task sets.
Hand-curating PactBench (S1) toward cases RepoPact handles well, or selecting SWE-bench /
SWE-EVO slices that favour the governed arm, would manufacture the result.
**Mitigation:** the PactBench task set and each study's "correct outcome" are
pre-registered and committed before runs (no post-hoc curation); task-set versions are
recorded per run; results are reported across at least two model families so a finding is
not a single-model artefact.

## T6 — Baseline fairness / construct validity (for H8–H10)

A comparative win is meaningless if the baseline is a strawman. Comparing a
RepoPact-governed repo against an *empty* repo would measure "having any context" rather
than "having RepoPact." **Mitigation:** the baseline arm carries a genuine, reasonable
`AGENTS.md` and README over identical source; only the governance layer differs; the
ceremony cost of RepoPact is counted *against* it (an efficiency gain that is cancelled
by governance overhead is reported as such, per ¬H9).

## T7 — Token-measurement fairness (for H11 / S4)

The S4 token-economy comparison is sensitive to artefacts that have nothing to do with
the delivery mechanism: tokenizer and model choice, **prompt caching**, and provider
pricing. Stable context (convention files, repo records) is cache-friendly and cheap on a
second request; per-request RAG injections vary and bust the cache — so raw token counts
can mis-state real cost in either direction, and a regime can look cheap on tokens while
being expensive in dollars (or vice versa). Comparing regimes that also carry *different
corpus content* would measure the content, not the mechanism. **Mitigation:** fix
tokenizer + model per run; hold corpus content constant across regimes; fix top-k for
retrieval; and report three numbers side by side — raw tokens, cache-adjusted tokens, and
USD at stated rates — so the comparison cannot hide behind any single one. The result is
read off the cost-vs-success frontier (S4), never off raw tokens alone.
