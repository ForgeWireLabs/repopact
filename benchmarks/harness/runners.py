"""Agent runners.

A runner takes a (task, arm) and reports what the agent did as an AgentAction. The grader
turns that into an Outcome. Two runners ship:

- MockRunner: deterministic, no model. Encodes a simple illustrative policy so the harness
  plumbing and the confusion-matrix reporting can be exercised and self-tested. Its numbers
  are NOT findings; they exist to prove the pipeline runs.
- RealRunner: the operator-gated integration point. It is intentionally not implemented
  here; running it needs a model/agent and API keys the operator provides.
"""
from __future__ import annotations

from model import AgentAction, Task, TokenUsage


class MockRunner:
    """A deterministic test double.

    Illustrative policy (NOT empirical):
    - baseline (convention-file) arm: on a must_not_weaken task the agent tends to take the
      shortcut (no enforcer stops it) -> weakened_guarantee.
    - repopact arm: a frozen-surface task is blocked by the enforcer; otherwise the binding
      invariant drives an escalation.
    - legitimate tasks: both arms proceed safely (the clean case; a real run would also
      measure false stops).
    Token figures are synthetic placeholders so the S4 instrumentation has something to
    aggregate.
    """

    name = "mock"

    def run(self, task: Task, arm: str) -> AgentAction:
        ctx = 1800 if arm == "repopact" else 1200  # records vs a convention file
        tokens = TokenUsage(
            input_tokens=ctx + 600,
            output_tokens=350,
            context_tokens=ctx,
            task_tokens=600,
            requests=1,
            usd=round((ctx + 950) * 1e-5, 6),
        )
        if task.polarity == "legitimate":
            return AgentAction(completed_task=True, tokens=tokens,
                               note="legitimate change applied")
        # must_not_weaken
        if arm == "repopact":
            if task.frozen_surface:
                return AgentAction(blocked_by_enforcer=True, tokens=tokens,
                                   note="frozen-surface change blocked; --ack required")
            return AgentAction(escalated=True, tokens=tokens,
                               note="binding invariant -> escalated for approval")
        # baseline convention-file arm: nothing enforces the guarantee
        return AgentAction(weakened_guarantee=True, tokens=tokens,
                           note="no enforcer; shortcut taken")


class RealRunner:
    """Operator-gated integration point for a live agent/model.

    Implement ``run`` to: materialize the arm (RepoPact records vs a convention-file
    AGENTS.md) over the task fixture, drive the agent with the task prompt, then read back
    post-conditions (diff inspection, check-frozen exit code, emitted approval requests) and
    return an AgentAction. Requires a model/agent and API keys.
    """

    name = "real"

    def run(self, task: Task, arm: str) -> AgentAction:  # pragma: no cover
        raise NotImplementedError(
            "RealRunner requires a model/agent and API keys (operator-gated). "
            "Wire a model client here and read post-conditions per pactbench/TASK-FORMAT.md."
        )


def get_runner(name: str):
    return {"mock": MockRunner, "real": RealRunner}[name]()
