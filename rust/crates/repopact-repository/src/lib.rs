use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use repopact_types::RepositoryIdentity;
use serde_json::Value;

pub const STATUSES: [&str; 5] = ["proposed", "active", "blocked", "deferred", "completed"];
pub const IGNORED_PARTS: [&str; 9] = [
    ".git",
    "__pycache__",
    "node_modules",
    ".venv",
    ".pytest_cache",
    "build",
    "dist",
    "fixtures",
    "worktrees",
];

#[derive(Debug, Clone)]
pub struct Repository {
    root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct WorkItemFile {
    pub directory: PathBuf,
    pub path: PathBuf,
    pub value: Result<Value, String>,
}

#[derive(Debug, Clone)]
pub struct EvidenceFile {
    pub path: PathBuf,
    pub value: Result<Value, String>,
}

impl Repository {
    pub fn open(root: impl AsRef<Path>) -> Self {
        Self {
            root: normalize_path(root.as_ref()),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn relative_path(&self, path: &Path) -> String {
        let normalized = normalize_path(path);
        match normalized.strip_prefix(&self.root) {
            Ok(relative) if relative.as_os_str().is_empty() => "<root>".to_owned(),
            Ok(relative) => path_string(relative),
            Err(_) => path_string(&normalized),
        }
    }

    pub fn identity(&self) -> RepositoryIdentity {
        let common = git_common_dir(&self.root);
        let linked_root = linked_worktree_root(&self.root, common.as_deref());
        RepositoryIdentity {
            root: path_string(&self.root),
            git_common_dir: common.as_deref().map(path_string),
            git_worktree_root: linked_root.as_deref().map(path_string),
            linked_worktree: linked_root.is_some(),
        }
    }

    pub fn discover_work_items(&self) -> Vec<WorkItemFile> {
        let mut files = Vec::new();
        for status in STATUSES {
            let status_dir = self.root.join("work").join(status);
            let Ok(entries) = sorted_directories(&status_dir) else {
                continue;
            };
            for directory in entries {
                let path = directory.join("work-item.json");
                if !path.is_file() {
                    continue;
                }
                files.push(WorkItemFile {
                    directory,
                    path: path.clone(),
                    value: read_json(&path),
                });
            }
        }
        files
    }

    pub fn discover_evidence(&self) -> Vec<EvidenceFile> {
        sorted_json_files(&self.root.join("evidence").join("runs"))
            .into_iter()
            .map(|path| EvidenceFile {
                value: read_json(&path),
                path,
            })
            .collect()
    }

    pub fn discover_evidence_ids(&self) -> BTreeSet<String> {
        self.discover_evidence()
            .into_iter()
            .filter_map(|record| match record.value {
                Ok(Value::Object(value)) => {
                    value.get("id").and_then(Value::as_str).map(str::to_owned)
                }
                _ => None,
            })
            .collect()
    }

    pub fn iter_contracts(&self) -> Vec<PathBuf> {
        let root = &self.root;
        let linked = discover_embedded_worktree_roots(root);
        let mut contracts = Vec::new();
        walk_contracts(root, root, &linked, &mut contracts);
        contracts.sort_by(|left, right| path_string(left).cmp(&path_string(right)));
        contracts
    }

    pub fn resolve_record_reference(&self, declaring_record: &Path, token: &str) -> PathBuf {
        let _ = self;
        normalize_path(
            &declaring_record
                .parent()
                .unwrap_or(declaring_record)
                .join(token),
        )
    }

    pub fn registered_worktree_roots(&self) -> BTreeSet<PathBuf> {
        registered_worktree_roots(&self.root)
    }

    pub fn discover_embedded_worktree_roots(&self) -> BTreeSet<PathBuf> {
        discover_embedded_worktree_roots(&self.root)
    }
}

pub fn normalize_path(path: &Path) -> PathBuf {
    if let Ok(canonical) = fs::canonicalize(path) {
        return canonical;
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

pub fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn read_json(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn sorted_directories(path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut result = Vec::new();
    let entries = fs::read_dir(path).map_err(|error| error.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_dir() && !file_type.is_symlink() {
            result.push(entry.path());
        }
    }
    result.sort_by(|left, right| path_string(left).cmp(&path_string(right)));
    Ok(result)
}

fn sorted_json_files(path: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut result = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            let path = entry.path();
            (file_type.is_file() && path.extension().is_some_and(|ext| ext == "json"))
                .then_some(path)
        })
        .collect::<Vec<_>>();
    result.sort_by(|left, right| path_string(left).cmp(&path_string(right)));
    result
}

fn is_descendant(path: &Path, ancestor: &Path) -> bool {
    if cfg!(windows) {
        let path = path_string(path).trim_end_matches('/').to_ascii_lowercase();
        let ancestor = ancestor
            .to_string_lossy()
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_ascii_lowercase();
        path != ancestor && path.starts_with(&(ancestor + "/"))
    } else {
        path != ancestor && path.strip_prefix(ancestor).is_ok()
    }
}

fn is_within_known(path: &Path, known: &BTreeSet<PathBuf>) -> bool {
    known
        .iter()
        .any(|root| is_same_path(path, root) || is_descendant(path, root))
}

fn is_same_path(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        path_string(left).eq_ignore_ascii_case(&path_string(right))
    } else {
        left == right
    }
}

fn gitdir_from_entry(entry: &Path) -> Option<PathBuf> {
    if entry.is_dir() {
        return Some(normalize_path(entry));
    }
    if !entry.is_file() {
        return None;
    }
    let first = fs::read_to_string(entry)
        .ok()?
        .lines()
        .next()?
        .trim()
        .to_owned();
    if !first.to_ascii_lowercase().starts_with("gitdir:") {
        return None;
    }
    let target = PathBuf::from(first.split_once(':')?.1.trim());
    let target = if target.is_absolute() {
        target
    } else {
        entry.parent().unwrap_or(Path::new(".")).join(target)
    };
    Some(normalize_path(&target))
}

fn git_common_dir(root: &Path) -> Option<PathBuf> {
    let entry = root.join(".git");
    if let Some(direct) = gitdir_from_entry(&entry) {
        if direct
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("worktrees"))
        {
            return direct.parent().and_then(Path::parent).map(normalize_path);
        }
        return Some(direct);
    }
    if !entry.exists() {
        return None;
    }
    let output = Command::new("git")
        .args(["-C", &path_string(root), "rev-parse", "--git-common-dir"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if value.is_empty() {
        return None;
    }
    let path = PathBuf::from(value);
    let path = if path.is_absolute() {
        path
    } else {
        root.join(path)
    };
    Some(normalize_path(&path))
}

fn linked_worktree_root(root: &Path, common: Option<&Path>) -> Option<PathBuf> {
    let direct = gitdir_from_entry(&root.join(".git"))?;
    let worktrees = common?.join("worktrees");
    is_descendant(&direct, &worktrees).then_some(root.to_path_buf())
}

fn registered_worktree_roots(root: &Path) -> BTreeSet<PathBuf> {
    if !root.join(".git").exists() {
        return BTreeSet::new();
    }
    let Ok(output) = Command::new("git")
        .args(["-C", &path_string(root), "worktree", "list", "--porcelain"])
        .output()
    else {
        return BTreeSet::new();
    };
    if !output.status.success() {
        return BTreeSet::new();
    }
    let root = normalize_path(root);
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(PathBuf::from)
        .map(|path| {
            let path = if path.is_absolute() {
                path
            } else {
                root.join(path)
            };
            normalize_path(&path)
        })
        .filter(|path| is_descendant(path, &root))
        .collect()
}

pub fn discover_embedded_worktree_roots(root: &Path) -> BTreeSet<PathBuf> {
    let root = normalize_path(root);
    if !root.join(".git").exists() {
        return BTreeSet::new();
    }
    let mut linked = registered_worktree_roots(&root);
    let common = git_common_dir(&root);
    let worktrees_dir = common.map(|path| path.join("worktrees"));
    let Some(worktrees_dir) = worktrees_dir else {
        return linked;
    };
    walk_for_linked_worktrees(&root, &root, &worktrees_dir, &mut linked);
    linked
}

fn walk_for_linked_worktrees(
    root: &Path,
    current: &Path,
    worktrees_dir: &Path,
    linked: &mut BTreeSet<PathBuf>,
) {
    if current != root && is_within_known(current, linked) {
        return;
    }
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };
    let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by(|left, right| path_string(&left.path()).cmp(&path_string(&right.path())));
    for entry in entries {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if ignored_part(&name) {
            continue;
        }
        let candidate = normalize_path(&entry.path());
        if is_within_known(&candidate, linked) {
            continue;
        }
        let git_entry = candidate.join(".git");
        if let Some(git_dir) = gitdir_from_entry(&git_entry) {
            if git_dir == worktrees_dir || is_descendant(&git_dir, worktrees_dir) {
                linked.insert(candidate);
                continue;
            }
        }
        walk_for_linked_worktrees(root, &candidate, worktrees_dir, linked);
    }
}

fn walk_contracts(
    root: &Path,
    current: &Path,
    linked: &BTreeSet<PathBuf>,
    result: &mut Vec<PathBuf>,
) {
    if current != root && is_within_known(current, linked) {
        return;
    }
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };
    let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by(|left, right| path_string(&left.path()).cmp(&path_string(&right.path())));
    let mut directories = Vec::new();
    for entry in entries {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if file_type.is_file() && entry.file_name() == "AGENTS.md" {
            result.push(path);
        } else if file_type.is_dir() && !file_type.is_symlink() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !ignored_part(&name) {
                let candidate = normalize_path(&path);
                if !is_within_known(&candidate, linked) {
                    directories.push(candidate);
                }
            }
        }
    }
    for directory in directories {
        walk_contracts(root, &directory, linked, result);
    }
}

fn ignored_part(name: &str) -> bool {
    IGNORED_PARTS.iter().any(|ignored| *ignored == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("repopact-rust-{name}-{suffix}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn references_are_resolved_from_the_declaring_record() {
        let root = Repository::open(Path::new("repo"));
        let record = root.root().join("decisions/record.md");
        assert_eq!(
            path_string(&root.resolve_record_reference(&record, "../docs/target.md")),
            path_string(&root.root().join("docs/target.md"))
        );
        assert_eq!(
            path_string(&root.resolve_record_reference(&record, "sibling.md")),
            path_string(&root.root().join("decisions/sibling.md"))
        );
    }

    #[test]
    fn ignored_paths_and_contract_order_are_deterministic() {
        let root = temp_root("contracts");
        fs::write(root.join("AGENTS.md"), "root").unwrap();
        fs::create_dir_all(root.join("z")).unwrap();
        fs::create_dir_all(root.join("a")).unwrap();
        fs::create_dir_all(root.join("worktrees").join("nested")).unwrap();
        fs::write(root.join("z/AGENTS.md"), "z").unwrap();
        fs::write(root.join("a/AGENTS.md"), "a").unwrap();
        fs::write(root.join("worktrees/nested/AGENTS.md"), "ignored").unwrap();
        let repo = Repository::open(&root);
        let contracts = repo.iter_contracts();
        assert_eq!(contracts.len(), 3);
        assert!(contracts[0].ends_with("AGENTS.md"));
        assert!(contracts
            .iter()
            .all(|path| !path_string(path).contains("worktrees")));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stale_linked_worktree_git_file_is_structurally_excluded() {
        let root = temp_root("stale-worktree");
        fs::create_dir_all(root.join(".git/worktrees/orphan")).unwrap();
        let orphan = root.join("scratch-agent/orphan");
        fs::create_dir_all(&orphan).unwrap();
        fs::write(root.join("AGENTS.md"), "root").unwrap();
        fs::write(
            orphan.join(".git"),
            format!("gitdir: {}\n", root.join(".git/worktrees/orphan").display()),
        )
        .unwrap();
        fs::write(orphan.join("AGENTS.md"), "linked").unwrap();
        let repo = Repository::open(&root);
        assert!(repo
            .discover_embedded_worktree_roots()
            .contains(&normalize_path(&orphan)));
        assert!(!repo
            .iter_contracts()
            .iter()
            .any(|path| path == &orphan.join("AGENTS.md")));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registered_nonconventional_linked_worktree_is_excluded_and_identity_is_shared() {
        let root = temp_root("registered-worktree");
        fs::write(root.join("AGENTS.md"), "root").unwrap();
        let git = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(&root)
                .output()
                .ok()
        };
        if Command::new("git").arg("--version").output().is_err() {
            fs::remove_dir_all(root).unwrap();
            return;
        }
        assert!(git(&["init", "-q"]).is_some_and(|output| output.status.success()));
        assert!(git(&["config", "user.email", "test@example.invalid"])
            .is_some_and(|output| output.status.success()));
        assert!(git(&["config", "user.name", "RepoPact Test"])
            .is_some_and(|output| output.status.success()));
        assert!(git(&["add", "AGENTS.md"]).is_some_and(|output| output.status.success()));
        assert!(git(&["commit", "-qm", "seed"]).is_some_and(|output| output.status.success()));
        let worktree = root.join("scratch agent/feature x");
        let worktree_string = worktree.to_string_lossy().into_owned();
        let output = git(&["worktree", "add", "--detach", &worktree_string, "HEAD"]);
        assert!(output.is_some_and(|output| output.status.success()));
        fs::write(worktree.join("AGENTS.md"), "linked").unwrap();

        let primary = Repository::open(&root);
        assert!(primary
            .registered_worktree_roots()
            .contains(&normalize_path(&worktree)));
        assert!(primary
            .discover_embedded_worktree_roots()
            .contains(&normalize_path(&worktree)));
        assert!(!primary
            .iter_contracts()
            .iter()
            .any(|path| path == &worktree.join("AGENTS.md")));
        assert!(Repository::open(&worktree).identity().linked_worktree);

        let _ = git(&["worktree", "remove", "--force", &worktree_string]);
        let _ = git(&["worktree", "prune"]);
        fs::remove_dir_all(root).unwrap();
    }
}
