"""RepoPact benchmark harness.

A model-agnostic harness for the comparative studies in
``research/benchmark-protocol.md``. It loads pre-registered tasks, runs each in the
matched arms (``baseline`` = convention-file, ``repopact`` = RepoPact records), grades the
outcome, and reports a confusion matrix plus token/cost instrumentation.

The live-model runner is operator-gated (compute + API keys). A deterministic ``MockRunner``
ships so the harness plumbing is testable and self-checkable without a model; its numbers
are illustrative plumbing checks, **not** findings.
"""
