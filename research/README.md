# RepoPact Proving Ground — Research Record

This directory is the lab notebook for an adversarial evaluation of the RepoPact
architecture. RepoPact claims to be a *repository-native operating system for
durable agent work*: authority, intent, evidence, and drift recorded as versioned
files so project state survives without a prior conversation. This record exists
to test that claim — to **prove or disprove** it — rather than to assert it.

The exercise is deliberately reflexive: RepoPact is the instrument as well as the
subject. The proving ground is a throwaway project that adopts RepoPact *from the
published package* and is then driven hard across every primitive, including the
cases designed to make it fail. The evidence runs, audit findings, and validator
output produced along the way **are the data**.

## Contents

| File | Purpose |
| --- | --- |
| [`protocol.md`](protocol.md) | The experiment: research question, hypotheses, method, and falsification criteria. Write before running. |
| [`run-log.md`](run-log.md) | Chronological log of every experimental action and its result. Append-only. |
| [`findings.md`](findings.md) | Findings register: each place the architecture held or cracked, with severity and resolution. |
| [`paper-outline.md`](paper-outline.md) | Working outline for a paper on repository-native agentic operating systems. |
| [`threats-to-validity.md`](threats-to-validity.md) | Why the evidence must not be over-read — reflexivity (forgewire is the progenitor), single evaluator, scale. |
| [`release-runbook.md`](release-runbook.md) | The credential-bound release steps for v1.0.0, recorded for the operator and the paper. |
| [`captures/`](captures/) | Raw, unedited terminal transcripts referenced by the run log and findings. |

## How to read this as evidence

A finding is only as good as the artifact behind it. Every entry in
[`findings.md`](findings.md) cites a capture under [`captures/`](captures/) and,
where the finding drove a change, the work item or audit finding that resolved it.
Confidence is not evidence; the captures are.
