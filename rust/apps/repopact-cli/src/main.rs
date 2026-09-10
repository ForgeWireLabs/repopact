use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn usage() -> &'static str {
    "usage: repopact-cli validate --root <repository> | plan-create --root <repository> --title <title> --date <YYYY-MM-DD> [--status <status>] | apply-create --root <repository> --title <title> --date <YYYY-MM-DD> [--status <status>] | apply-edit-title --root <repository> --id <id> --title <title> --date <YYYY-MM-DD> | apply-transition --root <repository> --id <id> --status <status>"
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        eprintln!("{}", usage());
        return ExitCode::from(2);
    };
    if !matches!(
        command.as_str(),
        "validate" | "plan-create" | "apply-create" | "apply-edit-title" | "apply-transition"
    ) {
        eprintln!("unsupported Rust operation '{command}'; {}", usage());
        return ExitCode::from(2);
    }
    let mut root = None;
    let mut title = None;
    let mut date = None;
    let mut status = None;
    let mut id = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--root" => {
                root = args.next().map(PathBuf::from);
            }
            "--title" => title = args.next(),
            "--date" => date = args.next(),
            "--status" => status = args.next(),
            "--id" => id = args.next(),
            _ => {
                eprintln!(
                    "unsupported Rust validate argument '{argument}'; {}",
                    usage()
                );
                return ExitCode::from(2);
            }
        }
    }
    let Some(root) = root else {
        eprintln!("missing --root; {}", usage());
        return ExitCode::from(2);
    };
    if command != "validate" {
        let core = repopact_core::RepoPactCore::open(&root);
        let request = match command.as_str() {
            "plan-create" | "apply-create" => {
                let Some(title) = title else {
                    eprintln!("missing --title; {}", usage());
                    return ExitCode::from(2);
                };
                let Some(date) = date else {
                    eprintln!("missing --date; {}", usage());
                    return ExitCode::from(2);
                };
                let mut request = repopact_mutation::CreateWorkItem::new(title, date);
                if let Some(status) = status {
                    request = request.with_status(status);
                }
                repopact_mutation::MutationRequest::create_work_item(request)
            }
            "apply-edit-title" => {
                let Some(id) = id else {
                    eprintln!("missing --id; {}", usage());
                    return ExitCode::from(2);
                };
                let Some(title) = title else {
                    eprintln!("missing --title; {}", usage());
                    return ExitCode::from(2);
                };
                let Some(date) = date else {
                    eprintln!("missing --date; {}", usage());
                    return ExitCode::from(2);
                };
                repopact_mutation::MutationRequest::edit_work_item(
                    repopact_mutation::EditWorkItem::new(
                        id,
                        repopact_mutation::WorkItemEdits::title(title),
                        date,
                    ),
                )
            }
            "apply-transition" => {
                let Some(id) = id else {
                    eprintln!("missing --id; {}", usage());
                    return ExitCode::from(2);
                };
                let Some(status) = status else {
                    eprintln!("missing --status; {}", usage());
                    return ExitCode::from(2);
                };
                repopact_mutation::MutationRequest::transition_work_item(
                    repopact_mutation::TransitionWorkItem::new(id, status),
                )
            }
            _ => unreachable!(),
        };
        let plan = core.plan_mutation(request);
        if command == "plan-create" {
            println!(
                "{}",
                serde_json::to_string_pretty(&plan).unwrap_or_else(|_| "{}".to_owned())
            );
            return if plan.is_applicable() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            };
        }
        let result = core.apply_mutation(&plan);
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_owned())
        );
        return if result.success {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }
    let report = repopact_core::validate(root);
    for diagnostic in &report.diagnostics {
        println!("{}", diagnostic.render());
    }
    if report.has_errors() {
        println!(
            "\nValidation failed with {} error(s).",
            report.diagnostics.len()
        );
        ExitCode::from(1)
    } else {
        println!("Repository governance validation passed.");
        ExitCode::SUCCESS
    }
}
