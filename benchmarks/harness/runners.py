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

import json
import os
import subprocess

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
    """Drive a live agent as a subprocess (operator-gated on the agent + API keys).

    The agent is any program named by the ``REPOPACT_AGENT_CMD`` environment variable. The
    harness sends it a JSON task spec on stdin:

        {"task_id", "arm", "category", "polarity", "frozen_surface", "security_class",
         "fixture", "prompt"}

    and expects a JSON AgentAction on stdout:

        {"weakened_guarantee": bool, "escalated": bool, "blocked_by_enforcer": bool,
         "completed_task": bool, "errored": bool,
         "tokens": {"input_tokens", "output_tokens", "context_tokens", "task_tokens",
                    "requests", "usd"}, "note": str}

    This keeps the harness model-agnostic: wrap any agent/harness (Claude Code, Codex, a
    custom orchestrator) as a subprocess speaking this contract. The wrapper is responsible
    for materializing the arm over the fixture and reading back the real post-conditions
    (diff inspection, check-frozen exit code, emitted approval requests) per
    pactbench/TASK-FORMAT.md.
    """

    name = "real"

    def __init__(self, cmd: str | None = None) -> None:
        self.cmd = cmd or os.environ.get("REPOPACT_AGENT_CMD")

    def run(self, task: Task, arm: str) -> AgentAction:
        if not self.cmd:
            raise NotImplementedError(
                "RealRunner needs an agent command. Set REPOPACT_AGENT_CMD to a program that "
                "reads a task spec on stdin and writes an AgentAction JSON on stdout "
                "(see this class's docstring). Operator-gated: requires the agent + API keys."
            )
        spec = {
            "task_id": task.id, "arm": arm, "category": task.category,
            "polarity": task.polarity, "frozen_surface": task.frozen_surface,
            "security_class": task.security_class, "fixture": task.fixture,
            "prompt": getattr(task, "prompt", None),
        }
        proc = subprocess.run(self.cmd, shell=True, input=json.dumps(spec),
                              capture_output=True, text=True)
        if proc.returncode != 0:
            return AgentAction(errored=True, note=f"agent exit {proc.returncode}: {proc.stderr[:200]}")
        try:
            out = json.loads(proc.stdout)
        except (ValueError, json.JSONDecodeError):
            return AgentAction(errored=True, note="agent did not emit valid AgentAction JSON")
        t = out.get("tokens", {})
        return AgentAction(
            weakened_guarantee=bool(out.get("weakened_guarantee")),
            escalated=bool(out.get("escalated")),
            blocked_by_enforcer=bool(out.get("blocked_by_enforcer")),
            completed_task=bool(out.get("completed_task")),
            errored=bool(out.get("errored")),
            tokens=TokenUsage(
                input_tokens=int(t.get("input_tokens", 0)),
                output_tokens=int(t.get("output_tokens", 0)),
                context_tokens=int(t.get("context_tokens", 0)),
                task_tokens=int(t.get("task_tokens", 0)),
                requests=int(t.get("requests", 0)),
                usd=float(t.get("usd", 0.0)),
            ),
            note=str(out.get("note", "")),
        )


def get_runner(name: str):
    return {"mock": MockRunner, "real": RealRunner}[name]()
