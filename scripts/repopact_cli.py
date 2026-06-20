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

    p_adopt = sub.add_parser("adopt", help="Adopt RepoPact into an existing repository")
    p_adopt.add_argument("--target", type=Path, required=True)
    p_adopt.add_argument("--dry-run", action="store_true", help="Report the plan without writing files")

    p_imp = sub.add_parser("import-plan", help="Import existing plan items (todos, checklists, issues) into work/")
    p_imp.add_argument("--root", type=Path, default=Path.cwd())
    p_imp.add_argument("--dry-run", action="store_true", help="Report the plan without writing files")
    p_imp.add_argument("--issues", action="store_true", help="Also import GitHub issues (needs gh + a GitHub remote)")

    p_doc = sub.add_parser("doctor", help="Diagnose (and optionally --fix) RepoPact drift")
    p_doc.add_argument("--root", type=Path, default=Path.cwd())
    p_doc.add_argument("--fix", action="store_true", help="Apply safe, non-destructive repairs")

    p_take = sub.add_parser("takeover", help="Retire legacy planning sources RepoPact has fully imported")
    p_take.add_argument("--root", type=Path, default=Path.cwd())
    p_take.add_argument("--delete", action="store_true",
                        help="Delete instead of archiving (git-guarded; writes a decisions/ ADR; "
                             "downgrades to archive if not recoverable from git)")
    p_take.add_argument("--dry-run", action="store_true", help="Report the plan without changing files")

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

    if args.command == "adopt":
        import adopt_repo
        target = args.target.resolve()
        rep = adopt_repo.adopt(target, dry_run=args.dry_run)
        adopt_repo._print_report(rep)
        if args.dry_run:
            print("\nDry run: nothing written. Re-run without --dry-run to apply.")
            return 0
        import validate_repo
        problems = validate_repo.validate(target)
        if problems:
            for p in problems:
                print(f"ERROR {p.path.relative_to(target)}: {p.message}")
            print(f"\nAdoption produced {len(problems)} validation error(s) to resolve.")
            return 1
        print("\nAdopted repository validates as a conformant RepoPact.")
        return 0

    root = args.root.resolve()

    if args.command == "doctor":
        import doctor, validate_repo
        if args.fix:
            for a in (doctor.fix(root) or ["nothing to fix"]):
                print(f"  ~ {a}")
        findings = doctor.diagnose(root)
        problems = validate_repo.validate(root)
        errs = [f for f in findings if f.severity == "error"]
        warns = [f for f in findings if f.severity == "warn"]
        for f in errs + warns:
            print(f"{f.severity.upper():5} [{f.code}] {f.message}")
        for p in problems:
            print(f"INVALID {p.path.relative_to(root)}: {p.message}")
        if not errs and not warns and not problems:
            print("repopact doctor: healthy.")
            return 0
        return 1 if errs or problems else 0

    if args.command == "takeover":
        import takeover
        report = takeover.takeover(root, delete=args.delete, dry_run=args.dry_run)
        rc = takeover._print(report, args.dry_run)
        if args.dry_run:
            print("\nDry run: nothing changed.")
        return rc

    if args.command == "import-plan":
        import plan_import
        rep = plan_import.import_plan(root, dry_run=args.dry_run, import_issues=args.issues)
        plan_import._print(rep)
        if args.dry_run:
            print("\nDry run: nothing written.")
            return 0
        import validate_repo
        problems = validate_repo.validate(root)
        for p in problems:
            print(f"ERROR {p.path.relative_to(root)}: {p.message}")
        print("\nwork/ ledger imported; repository validates." if not problems
              else f"\nImport produced {len(problems)} validation error(s).")
        return 1 if problems else 0

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
        if not spec.is_file():
            print(
                f"No SPEC.md at {root}. `spec` regenerates the derived blocks of an "
                "existing SPEC.md (it is a maintainer command for repositories that "
                "publish a RepoPact specification); an adopter repository does not "
                "need one.",
                file=sys.stderr,
            )
            return 1
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
