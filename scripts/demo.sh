#!/usr/bin/env bash
# Scripted RepoPact demo (see docs/demo.md). Safe to run; uses a temp directory.
set -uo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

step() { printf '\n\033[1;36m$ %s\033[0m\n' "$*"; }

step "python scripts/init_repo.py --target $TMP/demo"
python "$HERE/scripts/init_repo.py" --target "$TMP/demo"

cd "$TMP/demo"
step "python scripts/validate_repo.py"
python scripts/validate_repo.py

step 'python scripts/new.py work-item "Demo work"'
python scripts/new.py work-item "Demo work"

step "python scripts/validate_repo.py   # active item with a pending criterion is fine"
python scripts/validate_repo.py

step "mark the criterion satisfied WITHOUT evidence, then validate (expect failure)"
item="$(ls -d work/active/*/)"
python - "$item/work-item.json" <<'PY'
import json, sys
p = sys.argv[1]
d = json.load(open(p))
d["acceptance_criteria"][0]["state"] = "satisfied"
json.dump(d, open(p, "w"), indent=2)
PY
python scripts/validate_repo.py || printf '\n\033[1;33mValidator rejected it — completion requires proof.\033[0m\n'
