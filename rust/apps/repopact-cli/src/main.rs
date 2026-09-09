use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn usage() -> &'static str {
    "usage: repopact-cli validate --root <repository>"
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        eprintln!("{}", usage());
        return ExitCode::from(2);
    };
    if command != "validate" {
        eprintln!("unsupported Rust operation '{command}'; {}", usage());
        return ExitCode::from(2);
    }
    let mut root = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--root" => {
                root = args.next().map(PathBuf::from);
            }
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
