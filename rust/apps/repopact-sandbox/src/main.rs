#[cfg(target_os = "linux")]
mod unix {
    use repopact_confinement::{
        apply, compile_ceiling, probe, sanitize_inherited_fds, CompiledCeiling, ConfinementError,
        MINIMUM_LANDLOCK_ABI,
    };
    use serde_json::{json, Map, Value};
    use std::env;
    use std::fs;
    use std::io::{Read, Write};
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    use std::os::unix::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, ExitStatus, Stdio};
    use std::thread;
    use std::time::Duration;

    const DEFAULT_ENDPOINT: &str = "/run/repopact/guard.sock";
    const LAUNCH_FAILURE_EXIT: i32 = 125;

    pub fn entry() {
        let args: Vec<String> = env::args().skip(1).collect();
        let result = if args.iter().any(|arg| arg == "--probe") {
            run_probe()
        } else if args.iter().any(|arg| arg == "--capabilities") {
            run_capabilities()
        } else {
            run_launch(&args)
        };
        match result {
            Ok(code) => std::process::exit(code),
            Err(error) => {
                let _ = writeln!(
                    std::io::stderr(),
                    "{}",
                    json!({"allowed": false, "code": error.kind, "reason": error.message})
                );
                std::process::exit(LAUNCH_FAILURE_EXIT);
            }
        }
    }

    fn run_capabilities() -> Result<i32, ConfinementError> {
        let (abi, _) = probe()?;
        println!(
            "{}",
            json!({
                "backend": "linux-landlock",
                "abi": abi,
                "minimum_abi": MINIMUM_LANDLOCK_ABI,
                "minimum_supported": abi >= MINIMUM_LANDLOCK_ABI,
                "mutation_rights": repopact_confinement::LANDLOCK_MUTATION_RIGHTS,
            })
        );
        Ok(0)
    }

    fn run_probe() -> Result<i32, ConfinementError> {
        let root = env::temp_dir().join(format!("repopact-landlock-probe-{}", std::process::id()));
        let allowed = root.join("allowed");
        let denied = root.join("denied");
        fs::remove_dir_all(&root).ok();
        fs::create_dir_all(&allowed)
            .map_err(|error| io_error("probe fixture creation failed", error))?;
        fs::create_dir_all(&denied)
            .map_err(|error| io_error("probe fixture creation failed", error))?;
        fs::write(denied.join("sentinel"), b"unchanged")
            .map_err(|error| io_error("probe sentinel creation failed", error))?;

        let executable = env::current_exe()
            .map_err(|error| io_error("probe executable lookup failed", error))?;
        let child = Command::new(executable)
            .arg("--probe-child")
            .arg(&allowed)
            .arg(&denied)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| io_error("probe child start failed", error))?;
        let output = child
            .wait_with_output()
            .map_err(|error| io_error("probe child wait failed", error))?;
        let sentinel = fs::read(denied.join("sentinel")).unwrap_or_default();
        fs::remove_dir_all(&root)
            .map_err(|error| io_error("probe fixture cleanup failed", error))?;
        if !output.status.success() || sentinel != b"unchanged" {
            return Err(ConfinementError::new(
                "LANDLOCK_PROBE_FAILED",
                format!(
                    "probe child failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }
        let abi = probe()?.0;
        println!(
            "{}",
            json!({"backend":"linux-landlock", "abi":abi, "minimum_abi":MINIMUM_LANDLOCK_ABI, "probe":"passed"})
        );
        Ok(0)
    }

    fn run_probe_child(args: &[String]) -> Result<i32, ConfinementError> {
        if args.len() != 3 {
            return Err(ConfinementError::new(
                "LANDLOCK_PROBE_FAILED",
                "probe child arguments are malformed",
            ));
        }
        let allowed = PathBuf::from(&args[1]);
        let denied = PathBuf::from(&args[2]);
        let ceiling = CompiledCeiling {
            repository_root: allowed.parent().unwrap_or(&allowed).to_path_buf(),
            writable_roots: vec![fs::canonicalize(&allowed)
                .map_err(|error| io_error("probe allowed root lookup failed", error))?],
            source_patterns: vec!["probe".to_owned()],
            frozen_approval: false,
        };
        apply(&ceiling)?;
        fs::write(allowed.join("created"), b"allowed")
            .map_err(|error| io_error("probe allowed write failed", error))?;
        if fs::write(denied.join("sentinel"), b"changed").is_ok() {
            return Err(ConfinementError::new(
                "LANDLOCK_PROBE_FAILED",
                "denied probe write unexpectedly succeeded",
            ));
        }
        Ok(0)
    }

    fn run_launch(args: &[String]) -> Result<i32, ConfinementError> {
        if args.first().is_some_and(|arg| arg == "--probe-child") {
            return run_probe_child(args);
        }
        let separator = args.iter().position(|arg| arg == "--").ok_or_else(|| {
            ConfinementError::new(
                "INVALID_LAUNCH",
                "target argv must follow an explicit -- separator",
            )
        })?;
        let (options, command_args) = args.split_at(separator);
        let command_args = &command_args[1..];
        if command_args.is_empty() {
            return Err(ConfinementError::new(
                "INVALID_LAUNCH",
                "target argv is empty",
            ));
        }
        let root = option_path(options, "--root")?
            .ok_or_else(|| ConfinementError::new("INVALID_LAUNCH", "--root is required"))?;
        let endpoint =
            option_path(options, "--endpoint")?.unwrap_or_else(|| PathBuf::from(DEFAULT_ENDPOINT));
        let request = option_json(options, "--request-json")?
            .ok_or_else(|| ConfinementError::new("INVALID_LAUNCH", "--request-json is required"))?;
        let receipt = option_json(options, "--receipt-json")?
            .ok_or_else(|| ConfinementError::new("INVALID_LAUNCH", "--receipt-json is required"))?;
        let cwd = option_path(options, "--cwd")?;
        let repository_root = fs::canonicalize(&root)
            .map_err(|error| io_error("repository root lookup failed", error))?;

        let authorized = guard_authorize(&endpoint, &repository_root, &request, &receipt)?;
        let metadata = authorized
            .get("lease_metadata")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                ConfinementError::new(
                    "GUARD_UNHEALTHY",
                    "protected guard did not return lease metadata",
                )
            })?;
        let token = authorized
            .get("lease_token")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ConfinementError::new(
                    "GUARD_UNHEALTHY",
                    "protected guard did not return an opaque lease token",
                )
            })?;
        validate_metadata(&repository_root, &request, metadata)?;
        let patterns = metadata
            .get("paths")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ConfinementError::new(
                    "UNREPRESENTABLE_POLICY",
                    "guard metadata has no path ceiling",
                )
            })?
            .iter()
            .map(|value| {
                value.as_str().map(str::to_owned).ok_or_else(|| {
                    ConfinementError::new(
                        "UNREPRESENTABLE_POLICY",
                        "guard path ceiling contains a non-string",
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let frozen_approval =
            metadata.get("approval_class").and_then(Value::as_str) == Some("frozen");
        let ceiling = compile_ceiling(&repository_root, &patterns, frozen_approval)?;
        sanitize_inherited_fds(&ceiling.writable_roots)?;
        let applied = apply(&ceiling)?;

        let action = process_revalidation_action(&request);
        let initial_check = guard_check(&endpoint, &repository_root, &action, token)?;
        if !initial_check
            .get("allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return Err(response_error(
                "pre-execution guard check denied target start",
                &initial_check,
            ));
        }

        let mut command = Command::new(&command_args[0]);
        command.args(&command_args[1..]);
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        command
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        // The launcher has already entered the Landlock domain. A fresh session
        // gives the supervisor a bounded process-group handle while Landlock
        // independently covers descendants that detach from the group.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::prctl(36, libc::SIGKILL, 0, 0, 0) != 0 {
                    // PR_SET_PDEATHSIG
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command
            .spawn()
            .map_err(|error| io_error("target was not started after confinement", error))?;
        supervise(
            &mut child,
            &endpoint,
            &repository_root,
            &action,
            token,
            applied,
        )
    }

    fn supervise(
        child: &mut Child,
        endpoint: &Path,
        root: &Path,
        action: &Value,
        token: &str,
        applied: repopact_confinement::LandlockApplication,
    ) -> Result<i32, ConfinementError> {
        let _applied = applied;
        loop {
            if let Some(status) = child
                .try_wait()
                .map_err(|error| io_error("target wait failed", error))?
            {
                return Ok(exit_code(status));
            }
            thread::sleep(Duration::from_millis(250));
            let response = match guard_check(endpoint, root, action, token) {
                Ok(response) => response,
                Err(error) => {
                    terminate_process_group(child);
                    return Err(ConfinementError::new(
                        "GUARD_UNHEALTHY",
                        format!(
                            "guard revalidation failed; target terminated: {}",
                            error.message
                        ),
                    ));
                }
            };
            if !response
                .get("allowed")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                terminate_process_group(child);
                return Err(response_error(
                    "lease became invalid; target terminated",
                    &response,
                ));
            }
        }
    }

    fn terminate_process_group(child: &mut Child) {
        unsafe {
            // The child created a new session, so its pid is also the process-group
            // id. Failure is intentionally ignored; Landlock remains in force for
            // any descendant that survives this best-effort termination.
            let _ = libc::kill(-(child.id() as libc::pid_t), libc::SIGTERM);
        }
        thread::sleep(Duration::from_millis(100));
        if child.try_wait().ok().flatten().is_none() {
            unsafe {
                let _ = libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
            }
        }
        let _ = child.wait();
    }

    fn validate_metadata(
        root: &Path,
        request: &Value,
        metadata: &Map<String, Value>,
    ) -> Result<(), ConfinementError> {
        let metadata_root = metadata
            .get("repopact_root")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ConfinementError::new("WRONG_REPOSITORY", "guard metadata has no repository root")
            })?;
        let canonical_metadata_root = fs::canonicalize(metadata_root)
            .map_err(|error| io_error("guard metadata root is unavailable", error))?;
        if canonical_metadata_root != root {
            return Err(ConfinementError::new(
                "WRONG_REPOSITORY",
                "guard-derived root does not match launcher root",
            ));
        }
        let request_identity = request
            .get("repository_identity")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ConfinementError::new(
                    "WRONG_REPOSITORY",
                    "authorization request has no canonical repository identity",
                )
            })?;
        if metadata.get("repository_identity").and_then(Value::as_str) != Some(request_identity) {
            return Err(ConfinementError::new(
                "WRONG_REPOSITORY",
                "guard metadata identity does not match the signed request",
            ));
        }
        let capabilities = metadata
            .get("capabilities")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ConfinementError::new("PROFILE_ESCALATION", "guard metadata has no capabilities")
            })?;
        if !capabilities
            .iter()
            .any(|value| value.as_str() == Some("process"))
        {
            return Err(ConfinementError::new(
                "PROFILE_ESCALATION",
                "process execution is not in the guard-authorized capability ceiling",
            ));
        }
        Ok(())
    }

    fn process_revalidation_action(request: &Value) -> Value {
        let mut action = request.as_object().cloned().unwrap_or_default();
        // This is a non-mutating lease revalidation. The target is only started
        // after this launcher has independently applied Landlock, so this field
        // does not authorize an unsandboxed process.
        action.insert("kind".to_owned(), Value::String("read/orient".to_owned()));
        if let Some(session) = action.get("adapter_session").cloned() {
            action.insert("session_id".to_owned(), session);
        }
        Value::Object(action)
    }

    fn guard_authorize(
        endpoint: &Path,
        root: &Path,
        request: &Value,
        receipt: &Value,
    ) -> Result<Value, ConfinementError> {
        guard_call(
            json!({"protocol_version":"1","op":"authorize","payload":{"root":root.to_string_lossy(),"request":request,"receipt":receipt}}),
            endpoint,
        )
    }

    fn guard_check(
        endpoint: &Path,
        root: &Path,
        action: &Value,
        token: &str,
    ) -> Result<Value, ConfinementError> {
        let identity = action.get("repository_identity").cloned().ok_or_else(|| {
            ConfinementError::new(
                "WRONG_REPOSITORY",
                "revalidation action has no repository identity",
            )
        })?;
        guard_call(
            json!({"protocol_version":"1","op":"check","payload":{"root":root.to_string_lossy(),"repository_identity":identity,"action":action,"lease_token":token}}),
            endpoint,
        )
    }

    fn guard_call(message: Value, endpoint: &Path) -> Result<Value, ConfinementError> {
        verify_endpoint(endpoint)?;
        let mut stream = std::os::unix::net::UnixStream::connect(endpoint)
            .map_err(|error| io_error("protected guard connection failed", error))?;
        let bytes = serde_json::to_vec(&message).map_err(|error| {
            ConfinementError::new(
                "GUARD_FAILURE",
                format!("guard request encoding failed: {error}"),
            )
        })?;
        stream
            .write_all(&[&bytes[..], b"\n"[..]].concat())
            .map_err(|error| io_error("guard request write failed", error))?;
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .map_err(|error| io_error("guard response read failed", error))?;
        let line = response
            .split(|byte| *byte == b'\n')
            .next()
            .unwrap_or_default();
        serde_json::from_slice(line).map_err(|error| {
            ConfinementError::new(
                "GUARD_FAILURE",
                format!("guard response was invalid JSON: {error}"),
            )
        })
    }

    fn verify_endpoint(endpoint: &Path) -> Result<(), ConfinementError> {
        let metadata = fs::symlink_metadata(endpoint)
            .map_err(|error| io_error("protected guard endpoint is unavailable", error))?;
        if metadata.file_type().is_symlink()
            || !metadata.file_type().is_socket()
            || metadata.uid() != 0
            || metadata.mode() & 0o002 != 0
        {
            return Err(ConfinementError::new(
                "GUARD_UNHEALTHY",
                "guard endpoint is not a protected root-owned Unix socket",
            ));
        }
        let mut current = endpoint
            .parent()
            .ok_or_else(|| {
                ConfinementError::new("GUARD_UNHEALTHY", "guard endpoint has no parent")
            })?
            .to_path_buf();
        loop {
            let metadata = fs::symlink_metadata(&current)
                .map_err(|error| io_error("guard endpoint parent is unavailable", error))?;
            if metadata.file_type().is_symlink()
                || metadata.uid() != 0
                || metadata.mode() & 0o022 != 0
            {
                return Err(ConfinementError::new(
                    "GUARD_UNHEALTHY",
                    format!(
                        "guard endpoint parent is not protected: {}",
                        current.display()
                    ),
                ));
            }
            if current.parent() == Some(current.as_path()) {
                break;
            }
            current = current.parent().unwrap().to_path_buf();
        }
        Ok(())
    }

    fn response_error(prefix: &str, response: &Value) -> ConfinementError {
        let code = response
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or("GUARD_UNHEALTHY");
        let reason = response
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("protected guard denied the request");
        ConfinementError::new(code, format!("{prefix}: {reason}"))
    }

    fn option_path(options: &[String], name: &str) -> Result<Option<PathBuf>, ConfinementError> {
        option_value(options, name).map(|value| value.map(PathBuf::from))
    }

    fn option_json(options: &[String], name: &str) -> Result<Option<Value>, ConfinementError> {
        option_value(options, name)?
            .map(|value| {
                serde_json::from_str(&value).map_err(|error| {
                    ConfinementError::new(
                        "INVALID_LAUNCH",
                        format!("{name} is not valid JSON: {error}"),
                    )
                })
            })
            .transpose()
    }

    fn option_value(options: &[String], name: &str) -> Result<Option<String>, ConfinementError> {
        match options.iter().position(|value| value == name) {
            None => Ok(None),
            Some(index) if index + 1 >= options.len() => Err(ConfinementError::new(
                "INVALID_LAUNCH",
                format!("{name} requires a value"),
            )),
            Some(index) => Ok(Some(options[index + 1].clone())),
        }
    }

    fn exit_code(status: ExitStatus) -> i32 {
        status.code().unwrap_or(129)
    }

    fn io_error(prefix: &str, error: impl std::fmt::Display) -> ConfinementError {
        ConfinementError::new("LAUNCH_FAILURE", format!("{prefix}: {error}"))
    }
}

#[cfg(target_os = "linux")]
fn main() {
    unix::entry();
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("{{\"allowed\":false,\"code\":\"LANDLOCK_UNAVAILABLE\",\"reason\":\"repopact-sandbox requires Linux\"}}");
    std::process::exit(125);
}
