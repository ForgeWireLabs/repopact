---
id: 0059
title: WI022 AC-3 prospective model-family amendment and provider-runtime confound
status: accepted
date: 2026-09-14
supersedes: []
---

# 0059: WI022 AC-3 prospective model-family amendment and provider-runtime confound

## Context

WI022 AC-3 has a published deterministic harness, manifest v2, and whole-program
preflight, but no registered model-dependent comparative cell has been executed. The
second prospective family in the v2 plan was `gpt-6/openai/gpt-6-astra`. Before
comparative inference begins, the programme needs a genuinely different provider/runtime
ecosystem and a lower resource burden.

## Decision

Amendment `2026-09-14.ac3-model-family-amendment.1` replaces the prospective pair

* `gpt-5.6 / openai / gpt-5.6-luna`
* `gpt-6 / openai / gpt-6-astra`

with

* `gpt-5.6 / openai / gpt-5.6-luna`
* `claude-sonnet-5 / anthropic / claude-sonnet-5`

The decision was frozen with zero registered model-dependent AC-3 cells executed, no
comparative results available, and no observed AC-3 outcome used to select Sonnet.
Historical Astra evidence is immutable and remains linked as historical evidence only.

The task sets, cases, conditions, repetitions, seeds, scorers, frozen corpus, S4/S5
methods, analysis plan, paired statistics, confidence intervals, multiplicity rules,
missing-run handling, stopping rules, expected outcomes, S2 beds, S3 topology, and S6
graders are unchanged. S5 remains one shared set of 135 deterministic observations.

## Confound and estimand

Model family is confounded with provider/agent runtime in the cross-family comparison.
Within-family baseline-vs-RepoPact treatment comparisons remain matched because each
family's arms use the same provider/runtime configuration.

No absolute Luna-vs-Sonnet difference is a pure model-family effect. The primary object
is the within-family RepoPact treatment effect and whether its direction and magnitude
generalize across provider/runtime ecosystems.

## Runtime boundary

The proving-ground harness uses a provider-neutral `EmpiricalTransport` boundary. The
published Codex app-server implementation remains the v2 transport and its public
request ledger semantics are unchanged. Claude uses the documented Claude Code headless
structured stream surface with exact model `claude-sonnet-5`, provider-native default /
adaptive thinking, `acceptEdits` permission mode, and an explicitly recorded
`Read,Write,Edit,Bash` tool set. Missing request-level usage or identity fails closed;
the raw Anthropic API is not an alternative transport.

## State

This decision authorizes only the amended prospective registration and its deterministic
validation/admission gate. It does not authorize any comparative benchmark cell. WI022
AC-3 remains pending and resource-deferred.
