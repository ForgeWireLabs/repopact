//! Linux Landlock process-tree confinement for the RepoPact reference launcher.
//!
//! This crate deliberately contains no RepoPact policy or authority logic. It
//! accepts a ceiling derived by the protected guard and either represents that
//! ceiling without broadening it or fails closed.

use std::fmt;
use std::path::{Path, PathBuf};

pub const MINIMUM_LANDLOCK_ABI: i32 = 3;

/// Filesystem mutation rights required by the RepoPact sandbox contract.
///
/// These are all filesystem rights introduced by ABI 1, except `REFER`
/// (ABI 2) and `TRUNCATE` (ABI 3). Keeping the complete mutation matrix in one
/// constant makes it difficult to accidentally advertise a partial class.
pub const LANDLOCK_MUTATION_RIGHTS: u64 = 1 << 1  // WRITE_FILE
    | 1 << 4 // REMOVE_DIR
    | 1 << 5 // REMOVE_FILE
    | 1 << 6 // MAKE_CHAR
    | 1 << 7 // MAKE_DIR
    | 1 << 8 // MAKE_REG
    | 1 << 9 // MAKE_SOCK
    | 1 << 10 // MAKE_FIFO
    | 1 << 11 // MAKE_BLOCK
    | 1 << 12 // MAKE_SYM
    | 1 << 13 // REFER (ABI 2)
    | 1 << 14; // TRUNCATE (ABI 3)

const FROZEN_RELATIVE_PATHS: &[&str] = &["governance", "repopact/schemas", ".github/workflows"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfinementError {
    pub kind: &'static str,
    pub message: String,
}

impl ConfinementError {
    fn new(kind: &'static str, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfinementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for ConfinementError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledCeiling {
    pub repository_root: PathBuf,
    pub writable_roots: Vec<PathBuf>,
    pub source_patterns: Vec<String>,
    pub frozen_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LandlockApplication {
    pub abi: i32,
    pub handled_access_fs: u64,
    pub no_new_privs: bool,
    pub writable_roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FdSanitization {
    pub closed_descriptors: Vec<i32>,
    pub checked_standard_descriptors: Vec<i32>,
}

/// Compile guard-derived relative path patterns into safe kernel rule roots.
///
/// Only an exact existing path or a trailing `/**` directory pattern is
/// representable. In particular, this function never turns a missing exact
/// file into its parent directory, and never grants a broad ancestor that
/// contains a frozen/protected subtree without matching approval.
pub fn compile_ceiling(
    repository_root: &Path,
    patterns: &[String],
    frozen_approval: bool,
) -> Result<CompiledCeiling, ConfinementError> {
    let root = std::fs::canonicalize(repository_root).map_err(|error| {
        ConfinementError::new(
            "UNREPRESENTABLE_POLICY",
            format!("repository root is not canonicalizable: {error}"),
        )
    })?;
    if !root.is_dir() {
        return Err(ConfinementError::new(
            "UNREPRESENTABLE_POLICY",
            "repository root is not a directory",
        ));
    }
    if patterns.is_empty() {
        return Err(ConfinementError::new(
            "UNREPRESENTABLE_POLICY",
            "guard lease contains no writable path ceiling",
        ));
    }

    let mut roots = Vec::new();
    for raw in patterns {
        let raw = raw.replace('\\', "/");
        if raw.is_empty() || raw.starts_with('/') || raw.contains('\0') {
            return Err(ConfinementError::new(
                "UNREPRESENTABLE_POLICY",
                format!("invalid relative path pattern: {raw:?}"),
            ));
        }
        let is_recursive = raw == "**" || raw.ends_with("/**");
        let relative = if raw == "**" {
            ".".to_owned()
        } else if is_recursive {
            raw.trim_end_matches("/**").trim_end_matches('/').to_owned()
        } else {
            raw.clone()
        };
        if relative.is_empty() {
            return Err(ConfinementError::new(
                "UNREPRESENTABLE_POLICY",
                format!("invalid path pattern: {raw:?}"),
            ));
        }
        if relative
            .split('/')
            .any(|component| component == ".." || component.is_empty())
        {
            return Err(ConfinementError::new(
                "UNREPRESENTABLE_POLICY",
                format!("path traversal or empty component in {raw:?}"),
            ));
        }
        if relative.contains('*')
            || relative.contains('?')
            || relative.contains('[')
            || relative.contains(']')
        {
            return Err(ConfinementError::new(
                "UNREPRESENTABLE_POLICY",
                format!("only a trailing /** wildcard is representable: {raw:?}"),
            ));
        }

        let lexical = if relative == "." {
            root.clone()
        } else {
            root.join(&relative)
        };
        reject_symlink_components(&root, &lexical)?;
        let canonical = std::fs::canonicalize(&lexical).map_err(|error| {
            ConfinementError::new(
                "UNREPRESENTABLE_POLICY",
                format!("authorized path does not exist: {raw:?}: {error}"),
            )
        })?;
        if !is_within(&canonical, &root) {
            return Err(ConfinementError::new(
                "UNREPRESENTABLE_POLICY",
                format!(
                    "authorized path resolves outside repository: {raw:?} -> {} (root {})",
                    canonical.display(),
                    root.display()
                ),
            ));
        }
        if is_recursive && !canonical.is_dir() {
            return Err(ConfinementError::new(
                "UNREPRESENTABLE_POLICY",
                format!(
                    "recursive ceiling is not a directory: {raw:?} -> {}",
                    canonical.display()
                ),
            ));
        }
        if !frozen_approval && contains_frozen_descendant(&root, &canonical) {
            return Err(ConfinementError::new(
                "UNREPRESENTABLE_POLICY",
                format!("broad writable root would include a protected/frozen descendant: {raw:?}"),
            ));
        }
        roots.push(canonical);
    }

    roots.sort();
    roots.dedup();
    // An ancestor rule subsumes a descendant rule. Removing the descendant
    // does not broaden the union and keeps generated rules deterministic.
    let roots_snapshot = roots.clone();
    roots.retain(|candidate| {
        !roots_snapshot
            .iter()
            .any(|ancestor| ancestor != candidate && is_within(candidate, ancestor))
    });
    if roots.is_empty() {
        return Err(ConfinementError::new(
            "UNREPRESENTABLE_POLICY",
            "writable ceiling compiled to no roots",
        ));
    }
    Ok(CompiledCeiling {
        repository_root: root,
        writable_roots: roots,
        source_patterns: patterns.to_vec(),
        frozen_approval,
    })
}

fn is_within(candidate: &Path, ancestor: &Path) -> bool {
    candidate == ancestor || candidate.starts_with(ancestor)
}

fn contains_frozen_descendant(root: &Path, candidate: &Path) -> bool {
    FROZEN_RELATIVE_PATHS
        .iter()
        .any(|relative| is_within(&root.join(relative), candidate))
}

fn reject_symlink_components(root: &Path, candidate: &Path) -> Result<(), ConfinementError> {
    let relative = candidate.strip_prefix(root).map_err(|_| {
        ConfinementError::new(
            "UNREPRESENTABLE_POLICY",
            "path is outside canonical repository root",
        )
    })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(ConfinementError::new(
                    "UNREPRESENTABLE_POLICY",
                    format!(
                        "authorized path contains a symlink component: {}",
                        current.display()
                    ),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => {
                return Err(ConfinementError::new(
                    "UNREPRESENTABLE_POLICY",
                    format!(
                        "cannot inspect authorized path component {}: {error}",
                        current.display()
                    ),
                ));
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::ffi::CString;
    use std::os::fd::RawFd;
    use std::os::unix::fs::MetadataExt;

    const LANDLOCK_CREATE_RULESET_VERSION: libc::c_int = 1;
    const LANDLOCK_RULE_TYPE_PATH_BENEATH: libc::c_int = 1;
    const O_PATH: libc::c_int = 0o10000000;
    const O_CLOEXEC: libc::c_int = 0o2000000;
    const PR_SET_NO_NEW_PRIVS: libc::c_int = 38;

    #[repr(C)]
    struct LandlockRulesetAttr {
        handled_access_fs: u64,
        handled_access_net: u64,
        scoped: u64,
    }

    #[repr(C)]
    struct LandlockPathBeneathAttr {
        allowed_access: u64,
        parent_fd: libc::c_int,
    }

    fn errno_message(prefix: &str) -> String {
        format!("{prefix}: {}", std::io::Error::last_os_error())
    }

    fn syscall_failed(value: libc::c_long, prefix: &str) -> Result<libc::c_long, ConfinementError> {
        if value == -1 {
            Err(ConfinementError::new(
                "LANDLOCK_FAILURE",
                errno_message(prefix),
            ))
        } else {
            Ok(value)
        }
    }

    pub fn query_abi() -> Result<i32, ConfinementError> {
        let value = unsafe {
            libc::syscall(
                libc::SYS_landlock_create_ruleset,
                std::ptr::null::<LandlockRulesetAttr>(),
                0usize,
                LANDLOCK_CREATE_RULESET_VERSION,
            )
        };
        if value == -1 {
            let error = std::io::Error::last_os_error();
            let kind = match error.raw_os_error() {
                Some(libc::ENOSYS) => "LANDLOCK_UNAVAILABLE",
                Some(libc::EOPNOTSUPP) => "LANDLOCK_DISABLED",
                _ => "LANDLOCK_FAILURE",
            };
            return Err(ConfinementError::new(
                kind,
                format!("Landlock ABI query failed: {error}"),
            ));
        }
        Ok(value as i32)
    }

    pub fn apply(ceiling: &CompiledCeiling) -> Result<LandlockApplication, ConfinementError> {
        let abi = query_abi()?;
        if abi < MINIMUM_LANDLOCK_ABI {
            return Err(ConfinementError::new(
                "LANDLOCK_ABI_TOO_OLD",
                format!("Landlock ABI {abi} is below required ABI {MINIMUM_LANDLOCK_ABI}"),
            ));
        }
        let attr = LandlockRulesetAttr {
            handled_access_fs: LANDLOCK_MUTATION_RIGHTS,
            handled_access_net: 0,
            scoped: 0,
        };
        // Passing only the filesystem field keeps the request compatible with
        // ABI 3 while deliberately handling no network rights.
        let ruleset_fd = unsafe {
            libc::syscall(
                libc::SYS_landlock_create_ruleset,
                &attr,
                std::mem::size_of::<u64>(),
                0usize,
            )
        };
        let ruleset_fd = syscall_failed(ruleset_fd, "Landlock ruleset creation failed")? as RawFd;

        for root in &ceiling.writable_roots {
            let path = CString::new(root.as_os_str().as_encoded_bytes()).map_err(|_| {
                ConfinementError::new(
                    "UNREPRESENTABLE_POLICY",
                    format!("path contains NUL: {}", root.display()),
                )
            })?;
            let parent_fd = unsafe { libc::open(path.as_ptr(), O_PATH | O_CLOEXEC) };
            if parent_fd < 0 {
                unsafe { libc::close(ruleset_fd) };
                return Err(ConfinementError::new(
                    "LANDLOCK_FAILURE",
                    errno_message("opening writable rule root failed"),
                ));
            }
            let rule = LandlockPathBeneathAttr {
                allowed_access: LANDLOCK_MUTATION_RIGHTS,
                parent_fd,
            };
            let result = unsafe {
                libc::syscall(
                    libc::SYS_landlock_add_rule,
                    ruleset_fd,
                    LANDLOCK_RULE_TYPE_PATH_BENEATH,
                    &rule,
                    0usize,
                )
            };
            unsafe { libc::close(parent_fd) };
            if let Err(error) = syscall_failed(result, "Landlock rule addition failed") {
                unsafe { libc::close(ruleset_fd) };
                return Err(error);
            }
        }

        let no_new_privs = unsafe { libc::prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } == 0;
        if !no_new_privs {
            unsafe { libc::close(ruleset_fd) };
            return Err(ConfinementError::new(
                "NO_NEW_PRIVS_FAILURE",
                errno_message("setting no_new_privs failed"),
            ));
        }
        let result = unsafe { libc::syscall(libc::SYS_landlock_restrict_self, ruleset_fd, 0u32) };
        unsafe { libc::close(ruleset_fd) };
        syscall_failed(result, "Landlock restriction failed")?;
        Ok(LandlockApplication {
            abi,
            handled_access_fs: LANDLOCK_MUTATION_RIGHTS,
            no_new_privs,
            writable_roots: ceiling.writable_roots.clone(),
        })
    }

    fn fd_path(fd: RawFd) -> Option<PathBuf> {
        std::fs::read_link(format!("/proc/self/fd/{fd}")).ok()
    }

    fn path_allowed(path: &Path, roots: &[PathBuf]) -> bool {
        let canonical = std::fs::canonicalize(path).ok();
        canonical
            .as_ref()
            .is_some_and(|value| roots.iter().any(|root| is_within(value, root)))
    }

    fn inspect_standard_fd(fd: RawFd, roots: &[PathBuf]) -> Result<(), ConfinementError> {
        let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
        if unsafe { libc::fstat(fd, &mut stat) } != 0 {
            return Err(ConfinementError::new(
                "INHERITED_FD_FAILURE",
                errno_message("cannot inspect standard descriptor"),
            ));
        }
        let mode = stat.st_mode as libc::mode_t;
        let file_type = mode & libc::S_IFMT;
        let is_regular = file_type == libc::S_IFREG;
        let is_directory = file_type == libc::S_IFDIR;
        if is_directory {
            return Err(ConfinementError::new(
                "INHERITED_FD_ESCAPE",
                format!("standard descriptor {fd} is a directory capability"),
            ));
        }
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 {
            return Err(ConfinementError::new(
                "INHERITED_FD_FAILURE",
                errno_message("cannot inspect standard descriptor flags"),
            ));
        }
        let writable = (flags & libc::O_WRONLY) != 0 || (flags & libc::O_RDWR) != 0;
        if is_regular && writable {
            let path = fd_path(fd).ok_or_else(|| {
                ConfinementError::new(
                    "INHERITED_FD_ESCAPE",
                    format!("standard descriptor {fd} has no inspectable target"),
                )
            })?;
            if !path_allowed(&path, roots) {
                return Err(ConfinementError::new(
                    "INHERITED_FD_ESCAPE",
                    format!("writable standard descriptor {fd} targets outside the confinement ceiling: {}", path.display()),
                ));
            }
        } else if file_type == libc::S_IFCHR {
            // Terminals and the null device are normal standard streams; an
            // arbitrary writable device is outside this pathname contract.
            let path = fd_path(fd).unwrap_or_default();
            let terminal = unsafe { libc::isatty(fd) == 1 };
            let null_device = path == Path::new("/dev/null")
                || path == Path::new("/dev/tty")
                || path.starts_with("/dev/pts/");
            if writable && !terminal && !null_device {
                return Err(ConfinementError::new(
                    "INHERITED_FD_ESCAPE",
                    format!("standard descriptor {fd} targets an unapproved device"),
                ));
            }
        }
        Ok(())
    }

    /// Close every descriptor above stderr and validate writable standard
    /// streams. There is intentionally no pass-fd feature in the reference
    /// launcher: retained descriptors are limited to standard streams whose
    /// capability is checked before Landlock is entered.
    pub fn sanitize_inherited_fds(roots: &[PathBuf]) -> Result<FdSanitization, ConfinementError> {
        for fd in 0..=2 {
            inspect_standard_fd(fd, roots)?;
        }
        let mut descriptors = Vec::new();
        let entries = std::fs::read_dir("/proc/self/fd").map_err(|error| {
            ConfinementError::new(
                "INHERITED_FD_FAILURE",
                format!("cannot enumerate inherited descriptors: {error}"),
            )
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                ConfinementError::new(
                    "INHERITED_FD_FAILURE",
                    format!("cannot enumerate inherited descriptors: {error}"),
                )
            })?;
            if let Ok(fd) = entry.file_name().to_string_lossy().parse::<i32>() {
                if fd > 2 {
                    descriptors.push(fd);
                }
            }
        }
        descriptors.sort_unstable();
        descriptors.dedup();
        let mut closed = Vec::new();
        for fd in &descriptors {
            if unsafe { libc::close(*fd) } == 0 {
                closed.push(*fd);
            }
        }
        Ok(FdSanitization {
            closed_descriptors: closed,
            checked_standard_descriptors: vec![0, 1, 2],
        })
    }

    pub fn probe() -> Result<(i32, bool), ConfinementError> {
        let abi = query_abi()?;
        if abi < MINIMUM_LANDLOCK_ABI {
            return Err(ConfinementError::new(
                "LANDLOCK_ABI_TOO_OLD",
                format!("Landlock ABI {abi} is below required ABI {MINIMUM_LANDLOCK_ABI}"),
            ));
        }
        Ok((abi, true))
    }
}

#[cfg(target_os = "linux")]
pub use linux::{apply, probe, query_abi, sanitize_inherited_fds};

#[cfg(not(target_os = "linux"))]
pub fn query_abi() -> Result<i32, ConfinementError> {
    Err(ConfinementError::new(
        "LANDLOCK_UNAVAILABLE",
        "Landlock is Linux-specific",
    ))
}

#[cfg(not(target_os = "linux"))]
pub fn probe() -> Result<(i32, bool), ConfinementError> {
    Err(ConfinementError::new(
        "LANDLOCK_UNAVAILABLE",
        "Landlock is Linux-specific",
    ))
}

#[cfg(not(target_os = "linux"))]
pub fn apply(_: &CompiledCeiling) -> Result<LandlockApplication, ConfinementError> {
    Err(ConfinementError::new(
        "LANDLOCK_UNAVAILABLE",
        "Landlock is Linux-specific",
    ))
}

#[cfg(not(target_os = "linux"))]
pub fn sanitize_inherited_fds(_: &[PathBuf]) -> Result<FdSanitization, ConfinementError> {
    Err(ConfinementError::new(
        "LANDLOCK_UNAVAILABLE",
        "Landlock is Linux-specific",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn rejects_broad_root_containing_frozen_subtree_without_approval() {
        let root = tempfile_root();
        fs::create_dir_all(root.join("governance")).unwrap();
        let error = compile_ceiling(&root, &["**".to_owned()], false).unwrap_err();
        assert_eq!(error.kind, "UNREPRESENTABLE_POLICY");
        assert!(error.message.contains("protected/frozen"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_disjoint_leaf_roots_and_deduplicates_descendants() {
        let root = tempfile_root();
        fs::create_dir_all(root.join("src/nested")).unwrap();
        let compiled = compile_ceiling(
            &root,
            &["src/**".to_owned(), "src/nested/**".to_owned()],
            false,
        )
        .unwrap();
        assert_eq!(
            compiled.writable_roots,
            vec![fs::canonicalize(root.join("src")).unwrap()]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_symlinked_authorized_component() {
        let root = tempfile_root();
        fs::create_dir_all(root.join("src")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/tmp", root.join("src/outside")).unwrap();
        #[cfg(unix)]
        {
            let error = compile_ceiling(&root, &["src/outside/**".to_owned()], false).unwrap_err();
            assert_eq!(error.kind, "UNREPRESENTABLE_POLICY");
        }
        let _ = fs::remove_dir_all(root);
    }

    fn tempfile_root() -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "repopact-confinement-test-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }
}
