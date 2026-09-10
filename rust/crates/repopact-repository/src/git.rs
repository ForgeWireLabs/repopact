use std::fmt;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Captured output from one bounded, non-interactive Git query.
#[derive(Debug)]
pub struct GitOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitInvocation {
    pub root: PathBuf,
    pub args: Vec<String>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitError {
    pub label: String,
    pub message: String,
    pub timed_out: bool,
}

impl GitError {
    fn new(label: &str, message: impl Into<String>) -> Self {
        Self {
            label: label.to_owned(),
            message: message.into(),
            timed_out: false,
        }
    }

    fn timeout(label: &str, timeout: Duration) -> Self {
        Self {
            label: label.to_owned(),
            message: format!("Git query exceeded {}ms", timeout.as_millis()),
            timed_out: true,
        }
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.label, self.message)
    }
}

impl std::error::Error for GitError {}

/// The only production boundary for native Git execution.
pub trait GitRunner: Send + Sync + fmt::Debug {
    fn run(&self, root: &Path, args: &[&str], label: &str) -> Result<GitOutput, GitError>;
}

#[derive(Debug, Clone)]
pub struct NativeGitRunner {
    timeout: Duration,
}

impl Default for NativeGitRunner {
    fn default() -> Self {
        Self::new(DEFAULT_TIMEOUT)
    }
}

impl NativeGitRunner {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    fn command(&self, program: &str, root: &Path, args: &[&str]) -> Command {
        let mut command = Command::new(program);
        command
            .args(args)
            .current_dir(root)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NO_WINDOW: GUI-origin Git queries must not allocate a
            // console or flash a native terminal window.
            command.creation_flags(0x08000000);
        }
        command
    }

    fn wait_bounded(&self, child: &mut Child, label: &str) -> Result<GitOutput, GitError> {
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| GitError::new(label, "Git stdout pipe was unavailable"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| GitError::new(label, "Git stderr pipe was unavailable"))?;
        let stdout_reader = thread::spawn(move || read_pipe(stdout));
        let stderr_reader = thread::spawn(move || read_pipe(stderr));
        let deadline = Instant::now() + self.timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() >= deadline => {
                    terminate_child_tree(child);
                    let _ = stdout_reader.join();
                    let _ = stderr_reader.join();
                    return Err(GitError::timeout(label, self.timeout));
                }
                Ok(None) => thread::sleep(Duration::from_millis(10)),
                Err(error) => {
                    terminate_child_tree(child);
                    let _ = stdout_reader.join();
                    let _ = stderr_reader.join();
                    return Err(GitError::new(label, error.to_string()));
                }
            }
        };
        let stdout = stdout_reader
            .join()
            .map_err(|_| GitError::new(label, "Git stdout reader failed"))?
            .map_err(|error| GitError::new(label, error.to_string()))?;
        let stderr = stderr_reader
            .join()
            .map_err(|_| GitError::new(label, "Git stderr reader failed"))?
            .map_err(|error| GitError::new(label, error.to_string()))?;
        Ok(GitOutput {
            status,
            stdout,
            stderr,
        })
    }
}

fn terminate_child_tree(child: &mut Child) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .creation_flags(0x08000000)
            .output();
    }
    let _ = child.kill();
    let _ = child.wait();
}

impl GitRunner for NativeGitRunner {
    fn run(&self, root: &Path, args: &[&str], label: &str) -> Result<GitOutput, GitError> {
        let mut command = self.command("git", root, args);
        let mut child = command
            .spawn()
            .map_err(|error| GitError::new(label, format!("unable to start Git: {error}")))?;
        self.wait_bounded(&mut child, label)
    }
}

#[cfg(test)]
impl NativeGitRunner {
    fn run_program_for_test(
        &self,
        program: &str,
        root: &Path,
        args: &[&str],
        label: &str,
    ) -> Result<GitOutput, GitError> {
        let mut child = self
            .command(program, root, args)
            .spawn()
            .map_err(|error| GitError::new(label, error.to_string()))?;
        self.wait_bounded(&mut child, label)
    }
}

fn read_pipe<R: Read>(mut reader: R) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    reader.read_to_end(&mut output)?;
    Ok(output)
}

/// A deterministic test/audit runner that delegates to a real runner while
/// recording call count, labels, and peak child-query concurrency.
#[derive(Debug)]
pub struct CountingGitRunner {
    delegate: Arc<dyn GitRunner>,
    calls: Mutex<Vec<GitInvocation>>,
    active: AtomicUsize,
    max_active: AtomicUsize,
}

impl CountingGitRunner {
    pub fn new(delegate: Arc<dyn GitRunner>) -> Arc<Self> {
        Arc::new(Self {
            delegate,
            calls: Mutex::new(Vec::new()),
            active: AtomicUsize::new(0),
            max_active: AtomicUsize::new(0),
        })
    }

    pub fn native() -> Arc<Self> {
        Self::new(Arc::new(NativeGitRunner::default()))
    }

    pub fn count(&self) -> usize {
        self.calls
            .lock()
            .map(|calls| calls.len())
            .unwrap_or_default()
    }

    pub fn max_concurrency(&self) -> usize {
        self.max_active.load(Ordering::SeqCst)
    }

    pub fn invocations(&self) -> Vec<GitInvocation> {
        self.calls
            .lock()
            .map(|calls| calls.clone())
            .unwrap_or_default()
    }
}

impl GitRunner for CountingGitRunner {
    fn run(&self, root: &Path, args: &[&str], label: &str) -> Result<GitOutput, GitError> {
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_active.fetch_max(active, Ordering::SeqCst);
        if let Ok(mut calls) = self.calls.lock() {
            calls.push(GitInvocation {
                root: root.to_path_buf(),
                args: args.iter().map(|arg| (*arg).to_owned()).collect(),
                label: label.to_owned(),
            });
        }
        let result = self.delegate.run(root, args, label);
        self.active.fetch_sub(1, Ordering::SeqCst);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::Instant;

    #[test]
    fn native_runner_terminates_and_reaps_a_timed_out_child() {
        let runner = NativeGitRunner::new(Duration::from_millis(50));
        let started = Instant::now();
        let result = if cfg!(windows) {
            runner.run_program_for_test(
                "cmd.exe",
                Path::new("."),
                &["/C", "ping", "-n", "20", "127.0.0.1"],
                "timeout-test",
            )
        } else {
            runner.run_program_for_test("sh", Path::new("."), &["-c", "sleep 1"], "timeout-test")
        };
        let error = result.expect_err("the test child should exceed the finite timeout");
        assert!(error.timed_out);
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
