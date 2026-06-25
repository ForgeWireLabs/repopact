"""Core types for the benchmark harness."""
from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum


class Outcome(str, Enum):
    """The PactBench confusion-matrix cells (see pactbench/TASK-FORMAT.md)."""
    VIOLATED_SILENTLY = "violated_silently"
    BLOCKED = "blocked"
    ESCALATED = "escalated"
    PROCEEDED_SAFELY = "proceeded_safely"
    FALSE_STOPPED = "false_stopped"
    ERRORED = "errored"


# Outcomes that count as a "catch" on a must_not_weaken task.
CAUGHT = {Outcome.BLOCKED, Outcome.ESCALATED, Outcome.PROCEEDED_SAFELY}


@dataclass
class TokenUsage:
    """Per-request/aggregate token + cost instrumentation (S4 / H11)."""
    input_tokens: int = 0
    output_tokens: int = 0
    context_tokens: int = 0   # the slice spent orienting (vs the task itself)
    task_tokens: int = 0
    requests: int = 0
    usd: float = 0.0

    def __add__(self, other: "TokenUsage") -> "TokenUsage":
        return TokenUsage(
            self.input_tokens + other.input_tokens,
            self.output_tokens + other.output_tokens,
            self.context_tokens + other.context_tokens,
            self.task_tokens + other.task_tokens,
            self.requests + other.requests,
            round(self.usd + other.usd, 6),
        )


@dataclass
class Task:
    """A loaded PactBench task (the fields the harness needs to run + grade)."""
    id: str
    title: str
    category: str            # "correctness" | "security"
    polarity: str            # "must_not_weaken" | "legitimate"
    frozen_surface: bool
    arms: list[str]
    security_class: str | None = None
    fixture: str | None = None


@dataclass
class AgentAction:
    """What a runner reports the agent did on one (task, arm) run.

    The grader maps this to an Outcome; the runner never assigns the Outcome itself.
    """
    weakened_guarantee: bool = False   # took the shortcut and committed it
    escalated: bool = False            # asked for approval / used the escalation path
    blocked_by_enforcer: bool = False  # an enforcer prevented the change
    completed_task: bool = False       # resolved the task without weakening anything
    errored: bool = False
    tokens: TokenUsage = field(default_factory=TokenUsage)
    note: str = ""


@dataclass
class RunResult:
    task_id: str
    arm: str
    outcome: Outcome
    polarity: str
    tokens: TokenUsage
    note: str = ""
