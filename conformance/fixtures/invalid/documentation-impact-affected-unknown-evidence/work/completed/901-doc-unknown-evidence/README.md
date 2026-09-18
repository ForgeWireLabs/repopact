# 901 — Documentation impact referencing unknown evidence

A `completed` item declares `documentation_impact.state: "affected"` but
links an evidence run id that does not exist. Exercises decision 0065's
evidence-reference rule: a bare "docs updated" declaration is not evidence,
and even a structured one must resolve to a real evidence run.
