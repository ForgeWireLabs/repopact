//! Platform-neutral command shim for the Python compatibility CLI.
//!
//! Maturin's `bin` mode intentionally does not accept PEP 621 `project.scripts`
//! alongside native binaries. This launcher preserves the established
//! `repopact` command while dispatching to installed Python command modules.
//! It owns no semantic or filesystem authority.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn python_for_scripts(scripts: &PathBuf) -> Option<PathBuf> {
    let names = if cfg!(windows) {
        ["python.exe", "python"]
    } else {
        ["python", "python3"]
    };
    names
        .into_iter()
        .map(|name| scripts.join(name))
        .find(|path| path.is_file())
        .or_else(|| {
            scripts
                .parent()
                .map(|parent| {
                    parent.join(if cfg!(windows) {
                        "python.exe"
                    } else {
                        "bin/python"
                    })
                })
                .filter(|path| path.is_file())
        })
}

fn main() {
    let executable = env::current_exe().expect("repopact launcher path is available");
    let scripts = executable
        .parent()
        .expect("repopact launcher has a scripts directory")
        .to_path_buf();
    let python = python_for_scripts(&scripts).unwrap_or_else(|| {
        eprintln!("repopact launcher could not find Python beside its installed scripts");
        std::process::exit(1);
    });

    let args = env::args().skip(1).collect::<Vec<_>>();
    let (module, forwarded) = match args.first().map(String::as_str) {
        Some("verify") => ("repopact.verify_cli", args.iter().skip(1).cloned().collect::<Vec<_>>()),
        Some("release") => ("repopact.release_local", args.iter().skip(1).cloned().collect::<Vec<_>>()),
        _ => ("repopact.cli", args),
    };

    let status = Command::new(python)
        .arg("-m")
        .arg(module)
        .args(forwarded)
        .status()
        .unwrap_or_else(|error| {
            eprintln!("repopact launcher could not start the compatibility command: {error}");
            std::process::exit(1);
        });
    std::process::exit(status.code().unwrap_or(1));
}
