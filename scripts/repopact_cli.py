"""The `repopact` console entry point (work item 005, issue #2).

A thin dispatcher over the existing tooling so adopters can run `repopact init`
instead of `python scripts/init_repo.py`. Every subcommand except `init` operates
on the current working directory (or `--root`), so the installed command works
against the user's repository rather than the install location.
"""

from __future__ import annotations

import argparse
import sys
from datetime import date
from pathlib import Path


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="repopact", description="RepoPact: durable agent work, governed in the repo.")
    sub = parser.add_subparsers(dest="command", required=True)

    p_init = sub.add_parser("init", help="Bootstrap a new RepoPact repository")
    p_init.add_argument("--target", type=Path, required=True)

    p_val = sub.add_parser("validate", help="Validate the repository")
    p_val.add_argument("--root", type=Path, default=Path.cwd())

    p_dash = sub.add_parser("dashboard", help="Regenerate the dashboard")
    p_dash.add_argument("--root", type=Path, default=Path.cwd())

    p_spec = sub.add_parser("spec", help="Regenerate SPEC.md derived blocks")
    p_spec.add_argument("--root", type=Path, default=Path.cwd())

    p_new = sub.add_parser("new", help="Stamp a new record from a template")
    p_new.add_argument("kind", choices=["work-item", "decision", "policy"])
    p_new.add_argument("title")
    p_new.add_argument("--root", type=Path, default=Path.cwd())

    p_frz = sub.add_parser("check-frozen", help="Report frozen-surface changes in a diff range")
    p_frz.add_argument("--root", type=Path, default=Path.cwd())
    p_frz.add_argument("--base", default="origin/main")
    p_frz.add_argument("--ack", action="store_true")

    args = parser.parse_args(argv)

    if args.command == "init":
        import init_repo
        target = args.target.resolve()
        init_repo.bootstrap(target)
        import validate_repo
        problems = validate_repo.validate(target)
        if problems:
            for p in problems:
                print(f"ERROR {p.path.relative_to(target)}: {p.message}")
            print(f"\nBootstrap produced an invalid repository: {len(problems)} error(s).")
            return 1
        print(f"Bootstrapped a valid RepoPact at {target}")
        return 0

    root = args.root.resolve()

    if args.command == "validate":
        import validate_repo
        problems = validate_repo.validate(root)
        for p in problems:
            print(f"ERROR {p.path.relative_to(root)}: {p.message}")
        print("Repository governance validation passed." if not problems
              else f"\nValidation failed with {len(problems)} error(s).")
        return 1 if problems else 0

    if args.command == "dashboard":
        import generate_dashboard
        out = root / "audits" / "reports" / "dashboard.md"
        out.write_text(generate_dashboard.generate(root), encoding="utf-8")
        print(f"Generated {out.relative_to(root)}")
        return 0

    if args.command == "spec":
        import generate_spec
        spec = root / "SPEC.md"
        spec.write_text(generate_spec.render(spec.read_text(encoding="utf-8"), root), encoding="utf-8")
        print("Generated SPEC.md derived blocks")
        return 0

    if args.command == "new":
        import new
        if args.kind == "work-item":
            path = new.new_work_item(args.title, date.today(), root)
        else:
            path = new.new_markdown(args.kind, args.title, date.today(), root)
        print(f"Created {path.relative_to(root)}")
        return 0

    if args.command == "check-frozen":
        import check_frozen_surface
        hits = check_frozen_surface.violations(root, args.base)
        if not hits:
            print("No frozen-surface changes detected.")
            return 0
        print("Frozen-surface changes detected (INV-6 requires operator approval):")
        for name, reason in hits:
            print(f"  {name}: {reason}")
        return 0 if args.ack else 1

    return 2


if __name__ == "__main__":
    sys.exit(main())
