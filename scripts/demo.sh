#!/usr/bin/env bash
# Scripted RepoPact demo (see docs/demo.md). Safe to run; uses a temp directory.
set -uo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

step() { printf '\n\033[1;36m$ %s\033[0m\n' "$*"; }

step "repopact init --target $TMP/demo"
python -m repopact.cli init --target "$TMP/demo"

cd "$TMP/demo"
step "repopact validate"
python -m repopact.cli validate

step 'repopact new work-item "Demo work"'
python -m repopact.cli new work-item "Demo work"

step "repopact validate   # active item with a pending criterion is fine"
python -m repopact.cli validate

step "mark the criterion satisfied WITHOUT evidence, then validate (expect failure)"
item="$(ls -d work/active/*/)"
python - "$item/work-item.json" <<'PY'
import json, sys
p = sys.argv[1]
d = json.load(open(p))
d["acceptance_criteria"][0]["state"] = "satisfied"
json.dump(d, open(p, "w"), indent=2)
PY
python -m repopact.cli validate || printf '\n\033[1;33mValidator rejected it — completion requires proof.\033[0m\n'
