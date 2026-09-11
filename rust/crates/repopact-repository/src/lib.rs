use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use repopact_types::{
    hex_digest, PathState, ReadFact, ReadSet, RecordKind, RecordRef, RepositoryIdentity, WorkItem,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub mod git;
pub use git::{CountingGitRunner, GitError, GitInvocation, GitOutput, GitRunner, NativeGitRunner};

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
    git_runner: Arc<dyn GitRunner>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepositoryTopology {
    common_dir: Option<PathBuf>,
    linked_worktree_roots: BTreeSet<PathBuf>,
    tracked_paths: Option<BTreeSet<String>>,
    recording_commits: BTreeMap<String, (i64, String)>,
}

impl RepositoryTopology {
    fn build(repository: &Repository) -> Self {
        let common_dir = git_common_dir(&repository.root, repository.git_runner.as_ref());
        let mut linked_worktree_roots =
            registered_worktree_roots(&repository.root, repository.git_runner.as_ref());
        if let Some(worktrees_dir) = common_dir.as_ref().map(|path| path.join("worktrees")) {
            walk_for_linked_worktrees(
                &repository.root,
                &repository.root,
                &worktrees_dir,
                &mut linked_worktree_roots,
            );
        }
        let tracked_paths = repository
            .root
            .join(".git")
            .exists()
            .then(|| {
                repository
                    .git_runner
                    .run(&repository.root, &["ls-files", "--cached"], "tracked-paths")
                    .ok()
                    .filter(|output| output.status.success())
                    .map(|output| {
                        String::from_utf8_lossy(&output.stdout)
                            .lines()
                            .filter(|path| !path.is_empty())
                            .map(str::to_owned)
                            .collect()
                    })
            })
            .flatten();
        let recording_commits = repository
            .root
            .join(".git")
            .exists()
            .then(|| {
                repository
                    .git_runner
                    .run(
                        &repository.root,
                        &[
                            "log",
                            "--diff-filter=A",
                            "--format=__REPOPACT_COMMIT__%ct %H",
                            "--reverse",
                            "--name-only",
                            "--",
                            "evidence/runs",
                        ],
                        "evidence-recording-commits",
                    )
                    .ok()
                    .filter(|output| output.status.success())
                    .map(|output| parse_recording_commits(&output.stdout))
            })
            .flatten()
            .unwrap_or_default();
        Self {
            common_dir,
            linked_worktree_roots,
            tracked_paths,
            recording_commits,
        }
    }

    pub fn common_dir(&self) -> Option<&Path> {
        self.common_dir.as_deref()
    }

    pub fn linked_worktree_roots(&self) -> &BTreeSet<PathBuf> {
        &self.linked_worktree_roots
    }

    pub fn tracked_paths(&self) -> Option<&BTreeSet<String>> {
        self.tracked_paths.as_ref()
    }

    pub fn recording_commit(&self, path: &str) -> Option<&(i64, String)> {
        self.recording_commits.get(path)
    }
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

#[derive(Debug, Clone)]
pub struct IndexedRecord {
    pub reference: RecordRef,
    pub path: PathBuf,
    pub value: Result<Value, String>,
    pub text: Option<String>,
    pub front_matter: Result<BTreeMap<String, Value>, String>,
}

#[derive(Debug, Clone, Default)]
pub struct RecordIndex {
    pub work_items: Vec<IndexedRecord>,
    pub evidence: Vec<IndexedRecord>,
    pub decisions: Vec<IndexedRecord>,
    pub policies: Vec<IndexedRecord>,
    pub contracts: Vec<IndexedRecord>,
    pub invariants: Option<IndexedRecord>,
    pub frozen_surface: Option<IndexedRecord>,
    pub owners: Option<IndexedRecord>,
    pub audit_registry: Option<IndexedRecord>,
    pub audit_findings: Vec<IndexedRecord>,
    pub dashboard: Option<IndexedRecord>,
    /// `governance/adopters.json` (WI059 CVP-003): optional maintainer-only
    /// public adopter-fleet declaration. Locally referenced overlay files
    /// (vendored `mode == "overlay"` contracts) are folded into
    /// `source_paths`/`text_files` below so adopter validation stays a pure
    /// projection over this one generation.
    pub adopters: Option<IndexedRecord>,
    /// `research/metadata.json` (WI059 CVP-005): optional canonical research
    /// governance metadata. Every local document it references (freshness
    /// policy, lifecycle/benchmark/threat/trace documents) is folded into
    /// `source_paths`/`text_files` below for the same reason.
    pub research_metadata: Option<IndexedRecord>,
    pub source_paths: Vec<PathBuf>,
    pub work_directories: Vec<PathBuf>,
    pub text_files: BTreeMap<PathBuf, String>,
    /// Raw bytes of every adopter-manifest overlay path (WI059 CVP-003).
    /// Checksum verification must hash exact bytes (after CRLF normalization),
    /// not `text_files`' decoded-UTF-8 copy, so overlay content that is not
    /// valid UTF-8 still participates correctly instead of silently vanishing
    /// from the generation.
    pub adopter_overlay_bytes: BTreeMap<PathBuf, Vec<u8>>,
}

impl RecordIndex {
    pub fn build(repository: &Repository) -> Self {
        let topology = repository.topology();
        Self::build_with_topology(repository, &topology)
    }

    pub fn build_with_topology(repository: &Repository, topology: &RepositoryTopology) -> Self {
        let mut index = Self::default();
        index.work_directories = repository.discover_work_directories();
        for record in repository.discover_work_items() {
            let id = record
                .value
                .as_ref()
                .ok()
                .and_then(|value| value.get("id"))
                .and_then(Value::as_str)
                .unwrap_or_else(|| {
                    record
                        .path
                        .file_stem()
                        .and_then(|name| name.to_str())
                        .unwrap_or("unknown")
                });
            index.work_items.push(IndexedRecord::json(
                RecordRef::new(
                    RecordKind::WorkItem,
                    id,
                    repository.relative_path(&record.path),
                ),
                record.path.clone(),
                record.value,
            ));
            index.add_directory_files(repository, &record.directory, topology);
        }
        for record in repository.discover_evidence() {
            let id = record
                .value
                .as_ref()
                .ok()
                .and_then(|value| value.get("id"))
                .and_then(Value::as_str)
                .unwrap_or_else(|| {
                    record
                        .path
                        .file_stem()
                        .and_then(|name| name.to_str())
                        .unwrap_or("unknown")
                });
            index.evidence.push(IndexedRecord::json(
                RecordRef::new(
                    RecordKind::EvidenceRun,
                    id,
                    repository.relative_path(&record.path),
                ),
                record.path.clone(),
                record.value,
            ));
        }
        index.decisions = discover_markdown_records(repository, "decisions", RecordKind::Decision);
        index.policies =
            discover_markdown_records(repository, "governance/policies", RecordKind::Policy);
        index.contracts = repository
            .iter_contracts_with_topology(topology)
            .into_iter()
            .map(|path| {
                let id = repository.relative_path(&path);
                IndexedRecord::markdown(RecordRef::new(RecordKind::Contract, id.clone(), id), path)
            })
            .collect();
        index.invariants = index.json_record(
            repository,
            "governance/invariants.json",
            RecordKind::Invariant,
        );
        index.frozen_surface = index.json_record(
            repository,
            "governance/frozen-surface.json",
            RecordKind::FrozenSurface,
        );
        index.owners = index.json_record(repository, "governance/owners.json", RecordKind::Scope);
        index.audit_registry = index.json_record(
            repository,
            "audits/registry.json",
            RecordKind::AuditRegistry,
        );
        index.audit_findings =
            discover_json_records(repository, "audits/findings", RecordKind::AuditFinding);
        index.dashboard = index.json_or_text_record(
            repository,
            "audits/reports/dashboard.md",
            RecordKind::Dashboard,
        );
        index.adopters = index.json_record(
            repository,
            "governance/adopters.json",
            RecordKind::AdopterManifest,
        );
        index.research_metadata = index.json_record(
            repository,
            "research/metadata.json",
            RecordKind::ResearchMetadata,
        );

        for path in [
            "AGENTS.md",
            "repopact/AGENTS.md",
            "README.md",
            "VERSION",
            "RELEASE_LABEL",
            "scripts/REPOPACT_VERSION",
            "templates/work-item.README.md",
            "templates/work-item.json",
            "repopact/templates/work-item.README.md",
            "repopact/templates/work-item.json",
        ] {
            let candidate = repository.root.join(path);
            if candidate.is_file() {
                index.source_paths.push(candidate);
            }
        }
        index
            .source_paths
            .extend(index.work_items.iter().flat_map(|record| {
                record
                    .path
                    .parent()
                    .map(|directory| repository.files_under_with_topology(directory, topology))
                    .unwrap_or_default()
            }));
        index
            .source_paths
            .extend(index.evidence.iter().map(|record| record.path.clone()));
        index
            .source_paths
            .extend(index.decisions.iter().map(|record| record.path.clone()));
        index
            .source_paths
            .extend(index.policies.iter().map(|record| record.path.clone()));
        index
            .source_paths
            .extend(index.contracts.iter().map(|record| record.path.clone()));
        index.source_paths.extend(
            index
                .audit_findings
                .iter()
                .map(|record| record.path.clone()),
        );
        for record in [
            index.invariants.as_ref(),
            index.frozen_surface.as_ref(),
            index.owners.as_ref(),
            index.audit_registry.as_ref(),
            index.dashboard.as_ref(),
            index.adopters.as_ref(),
            index.research_metadata.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            index.source_paths.push(record.path.clone());
        }
        // WI059: the top-level research/*.md set (README included, unlike
        // discover_markdown_records) participates in freshness-coverage
        // checking, so it must be part of this same generation.
        index
            .source_paths
            .extend(top_level_markdown_files(&repository.root.join("research")));
        // WI059: fold in every local file the adopter manifest / research
        // metadata reference, so the new validators consume this one
        // generation instead of reopening the filesystem per referenced path.
        // Every reference is repository-containment-checked via
        // `resolve_within_root` BEFORE it is added to source_paths/
        // text_files/adopter_overlay_bytes; an escaping reference is simply
        // dropped here (never read) and the validator reports it separately
        // from the already-materialized generation.
        if let Some(value) = index
            .adopters
            .as_ref()
            .and_then(|record| record.value.as_ref().ok())
        {
            let overlays: Vec<PathBuf> = adopter_overlay_paths(value)
                .into_iter()
                .filter_map(|relative| resolve_within_root(&repository.root, &relative))
                .collect();
            index.adopter_overlay_bytes = overlays
                .iter()
                .filter_map(|path| Some((path.clone(), fs::read(path).ok()?)))
                .collect();
            index.source_paths.extend(overlays);
        }
        if let Some(value) = index
            .research_metadata
            .as_ref()
            .and_then(|record| record.value.as_ref().ok())
        {
            index.source_paths.extend(
                research_referenced_paths(value)
                    .into_iter()
                    .filter_map(|relative| resolve_within_root(&repository.root, &relative)),
            );
        }
        index
            .source_paths
            .sort_by(|left, right| path_string(left).cmp(&path_string(right)));
        index.source_paths.dedup();
        index.text_files = index
            .source_paths
            .iter()
            .filter_map(|path| Some((path.clone(), fs::read_to_string(path).ok()?)))
            .collect();
        for directory in &index.work_directories {
            for path in repository.files_under_with_topology(directory, topology) {
                if let Ok(text) = fs::read_to_string(&path) {
                    index.text_files.entry(path).or_insert(text);
                }
            }
        }
        index
    }

    pub fn work_item(&self, id: &str) -> Option<&IndexedRecord> {
        self.work_items
            .iter()
            .find(|record| record.reference.id == id)
    }

    pub fn typed_work_item(&self, id: &str) -> Option<WorkItem> {
        self.work_item(id)
            .and_then(|record| record.value.clone().ok())
            .and_then(|value| serde_json::from_value(value).ok())
    }

    pub fn text(&self, path: &Path) -> Option<&str> {
        self.text_files.get(path).map(String::as_str)
    }

    pub fn overlay_bytes(&self, path: &Path) -> Option<&[u8]> {
        self.adopter_overlay_bytes.get(path).map(Vec::as_slice)
    }

    pub fn read_set(&self, repository: &Repository) -> ReadSet {
        repository.read_set_for_paths(&self.source_paths)
    }

    fn json_record(
        &self,
        repository: &Repository,
        relative: &str,
        kind: RecordKind,
    ) -> Option<IndexedRecord> {
        let path = repository.root.join(relative);
        path.is_file().then(|| {
            IndexedRecord::json(
                RecordRef::new(kind, relative, relative),
                path.clone(),
                read_json(&path),
            )
        })
    }

    fn json_or_text_record(
        &self,
        repository: &Repository,
        relative: &str,
        kind: RecordKind,
    ) -> Option<IndexedRecord> {
        let path = repository.root.join(relative);
        path.is_file()
            .then(|| IndexedRecord::text(RecordRef::new(kind, relative, relative), path))
    }

    fn add_directory_files(
        &mut self,
        repository: &Repository,
        directory: &Path,
        topology: &RepositoryTopology,
    ) {
        self.source_paths
            .extend(repository.files_under_with_topology(directory, topology));
    }
}

impl IndexedRecord {
    fn json(reference: RecordRef, path: PathBuf, value: Result<Value, String>) -> Self {
        Self {
            reference,
            path,
            value,
            text: None,
            front_matter: Err("not a Markdown record".to_owned()),
        }
    }

    fn text(reference: RecordRef, path: PathBuf) -> Self {
        let text = fs::read_to_string(&path).ok();
        Self {
            reference,
            path,
            value: Err("not a JSON record".to_owned()),
            front_matter: text
                .as_deref()
                .map(parse_front_matter)
                .unwrap_or_else(|| Err("unable to read record".to_owned())),
            text,
        }
    }

    fn markdown(reference: RecordRef, path: PathBuf) -> Self {
        Self::text(reference, path)
    }
}

#[derive(Debug, Clone)]
pub struct RepositorySession {
    repository: Repository,
}

#[derive(Debug, Clone)]
pub struct RepositorySnapshot {
    repository: Repository,
    topology: RepositoryTopology,
    index: RecordIndex,
    read_set: ReadSet,
}

impl RepositorySession {
    pub fn open(root: impl AsRef<Path>) -> Self {
        Self {
            repository: Repository::open(root),
        }
    }

    pub fn with_git_runner(root: impl AsRef<Path>, git_runner: Arc<dyn GitRunner>) -> Self {
        Self {
            repository: Repository::with_git_runner(root, git_runner),
        }
    }

    pub fn repository(&self) -> &Repository {
        &self.repository
    }

    pub fn snapshot(&self) -> RepositorySnapshot {
        let topology = self.repository.topology();
        let index = RecordIndex::build_with_topology(&self.repository, &topology);
        let read_set = index.read_set(&self.repository);
        RepositorySnapshot {
            repository: self.repository.clone(),
            topology,
            index,
            read_set,
        }
    }
}

impl RepositorySnapshot {
    pub fn repository(&self) -> &Repository {
        &self.repository
    }
    pub fn index(&self) -> &RecordIndex {
        &self.index
    }
    pub fn topology(&self) -> &RepositoryTopology {
        &self.topology
    }
    pub fn read_set(&self) -> &ReadSet {
        &self.read_set
    }
    pub fn identity(&self) -> RepositoryIdentity {
        self.repository.identity_from_topology(&self.topology)
    }
    pub fn token(&self) -> String {
        self.read_set.token(&self.identity())
    }
}

impl Repository {
    pub fn open(root: impl AsRef<Path>) -> Self {
        Self::with_git_runner(root, Arc::new(NativeGitRunner::default()))
    }

    pub fn with_git_runner(root: impl AsRef<Path>, git_runner: Arc<dyn GitRunner>) -> Self {
        Self {
            root: normalize_path(root.as_ref()),
            git_runner,
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
        let topology = self.topology();
        self.identity_from_topology(&topology)
    }

    pub fn identity_from_topology(&self, topology: &RepositoryTopology) -> RepositoryIdentity {
        let common = topology.common_dir();
        let linked_root = linked_worktree_root(&self.root, common);
        RepositoryIdentity {
            root: path_string(&self.root),
            git_common_dir: common.map(path_string),
            git_worktree_root: linked_root.as_deref().map(path_string),
            linked_worktree: linked_root.is_some(),
        }
    }

    pub fn topology(&self) -> RepositoryTopology {
        RepositoryTopology::build(self)
    }

    pub fn git_runner(&self) -> &Arc<dyn GitRunner> {
        &self.git_runner
    }

    pub fn git_query(&self, args: &[&str], label: &str) -> Result<GitOutput, GitError> {
        self.git_runner.run(&self.root, args, label)
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

    pub fn discover_work_directories(&self) -> Vec<PathBuf> {
        let work = self.root.join("work");
        let Ok(children) = sorted_directories(&work) else {
            return Vec::new();
        };
        let mut directories = Vec::new();
        for child in children {
            let name = child
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            if name.starts_with('.') || name.starts_with('_') {
                continue;
            }
            if STATUSES.contains(&name) {
                directories.extend(sorted_directories(&child).unwrap_or_default());
            } else {
                directories.push(child);
            }
        }
        directories
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
        let topology = self.topology();
        self.iter_contracts_with_topology(&topology)
    }

    pub fn iter_contracts_with_topology(&self, topology: &RepositoryTopology) -> Vec<PathBuf> {
        let root = &self.root;
        let mut contracts = Vec::new();
        walk_contracts(root, root, topology.linked_worktree_roots(), &mut contracts);
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
        registered_worktree_roots(&self.root, self.git_runner.as_ref())
    }

    pub fn discover_embedded_worktree_roots(&self) -> BTreeSet<PathBuf> {
        discover_embedded_worktree_roots_with_runner(&self.root, self.git_runner.as_ref())
    }

    pub fn session(&self) -> RepositorySession {
        RepositorySession {
            repository: self.clone(),
        }
    }

    pub fn files_under(&self, path: &Path) -> Vec<PathBuf> {
        let topology = self.topology();
        self.files_under_with_topology(path, &topology)
    }

    pub fn files_under_with_topology(
        &self,
        path: &Path,
        topology: &RepositoryTopology,
    ) -> Vec<PathBuf> {
        let path = normalize_path(path);
        let mut files = Vec::new();
        walk_files_inner(
            &self.root,
            &path,
            topology.linked_worktree_roots(),
            &mut files,
        );
        files.sort_by(|left, right| path_string(left).cmp(&path_string(right)));
        files
    }

    pub fn all_files(&self) -> Vec<PathBuf> {
        self.files_under(&self.root)
    }

    pub fn path_state(&self, path: &Path) -> PathState {
        path_state(path)
    }

    pub fn read_set_for_paths(&self, paths: &[PathBuf]) -> ReadSet {
        let mut facts = paths
            .iter()
            .map(|path| ReadFact {
                path: self.relative_path(path),
                expected: self.path_state(path),
            })
            .collect::<Vec<_>>();
        facts.sort_by(|left, right| left.path.cmp(&right.path));
        facts.dedup_by(|left, right| left.path == right.path);
        ReadSet::new(facts)
    }

    pub fn read_set_for_relative_paths<I, S>(&self, paths: I) -> ReadSet
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let absolute = paths
            .into_iter()
            .map(|path| self.root.join(path.as_ref()))
            .collect::<Vec<_>>();
        self.read_set_for_paths(&absolute)
    }
}

/// Resolve `relative` against `root` and return it only if the result is
/// provably contained within `root` — never by reading the target's
/// *content*, only by resolving the path itself (which `normalize_path`
/// already does via `fs::canonicalize` when the target exists, following
/// symlinks/reparse points; a nonexistent target falls back to pure textual
/// `.`/`..` normalization). This is the single choke point every
/// metadata-directed path (adopter overlay, research-metadata reference)
/// must pass before it may be added to `source_paths`/`text_files`/
/// `adopter_overlay_bytes`, so a snapshot can never ingest content from
/// outside the repository (WI059 containment correction): `../outside`,
/// nested escape sequences, an absolute path outside the root (which
/// `Path::join` already treats as replacing `root` entirely), and a
/// symlink/reparse point whose canonical target resolves outside the root
/// are all rejected the same way, before any `fs::read`/`fs::read_to_string`
/// of that path is attempted.
pub fn resolve_within_root(root: &Path, relative: &str) -> Option<PathBuf> {
    let candidate = root.join(relative);
    let resolved = normalize_path(&candidate);
    let root_resolved = normalize_path(root);
    resolved.starts_with(&root_resolved).then_some(resolved)
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

fn parse_recording_commits(bytes: &[u8]) -> BTreeMap<String, (i64, String)> {
    let mut commits = BTreeMap::new();
    let mut current = None;
    for line in String::from_utf8_lossy(bytes).lines() {
        if let Some(value) = line.strip_prefix("__REPOPACT_COMMIT__") {
            let mut parts = value.split_whitespace();
            current = Some((
                parts
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0),
                parts.next().unwrap_or_default().to_owned(),
            ));
        } else if line.starts_with("evidence/runs/") {
            if let Some(commit) = current.clone() {
                commits.entry(line.to_owned()).or_insert(commit);
            }
        }
    }
    commits
}

fn read_json(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn discover_markdown_records(
    repository: &Repository,
    relative: &str,
    kind: RecordKind,
) -> Vec<IndexedRecord> {
    let directory = repository.root.join(relative);
    let Ok(entries) = fs::read_dir(&directory) else {
        return Vec::new();
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            (entry.file_type().ok()?.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().to_ascii_uppercase() != "README.MD"))
            .then_some(path)
        })
        .collect::<Vec<_>>();
    paths.sort_by(|left, right| path_string(left).cmp(&path_string(right)));
    paths
        .into_iter()
        .map(|path| {
            let fallback = path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown");
            let id = fs::read_to_string(&path)
                .ok()
                .and_then(|text| parse_front_matter(&text).ok())
                .and_then(|matter| matter.get("id").and_then(Value::as_str).map(str::to_owned))
                .unwrap_or_else(|| fallback.to_owned());
            IndexedRecord::markdown(
                RecordRef::new(kind, id, repository.relative_path(&path)),
                path,
            )
        })
        .collect()
}

fn discover_json_records(
    repository: &Repository,
    relative: &str,
    kind: RecordKind,
) -> Vec<IndexedRecord> {
    let directory = repository.root.join(relative);
    let Ok(entries) = fs::read_dir(&directory) else {
        return Vec::new();
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            (entry.file_type().ok()?.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("json")))
            .then_some(path)
        })
        .collect::<Vec<_>>();
    paths.sort_by(|left, right| path_string(left).cmp(&path_string(right)));
    paths
        .into_iter()
        .map(|path| {
            let fallback = path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown");
            let id = read_json(&path)
                .ok()
                .and_then(|value| value.get("id").and_then(Value::as_str).map(str::to_owned))
                .unwrap_or_else(|| fallback.to_owned());
            IndexedRecord::json(
                RecordRef::new(kind, id, repository.relative_path(&path)),
                path.clone(),
                read_json(&path),
            )
        })
        .collect()
}

/// Every `*.md` file directly under `directory` (including `README.md`,
/// unlike `discover_markdown_records`). Used for `research/*.md`, whose
/// membership as a *set* is itself part of the WI059 freshness-coverage
/// contract, so README must participate.
fn top_level_markdown_files(directory: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            (entry.file_type().ok()?.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("md")))
            .then_some(path)
        })
        .collect();
    paths.sort_by(|left, right| path_string(left).cmp(&path_string(right)));
    paths
}

/// Repository-relative paths of every vendored `mode == "overlay"` file
/// contract in a `governance/adopters.json` value (WI059 CVP-003). Only local
/// checksum-verification inputs are collected here; fleet/network state is
/// out of scope for the canonical validator.
fn adopter_overlay_paths(adopters: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    let Some(entries) = adopters.get("adopters").and_then(Value::as_array) else {
        return paths;
    };
    for entry in entries {
        let Some(consumption) = entry.get("consumption") else {
            continue;
        };
        if consumption.get("type").and_then(Value::as_str) != Some("vendored") {
            continue;
        }
        let Some(files) = consumption.get("files").and_then(Value::as_array) else {
            continue;
        };
        for contract in files {
            if contract.get("mode").and_then(Value::as_str) != Some("overlay") {
                continue;
            }
            if let Some(overlay) = contract.get("overlay_path").and_then(Value::as_str) {
                paths.push(overlay.to_owned());
            }
        }
    }
    paths
}

/// Repository-relative paths of every local document a `research/metadata.json`
/// value references (WI059 CVP-005): freshness policy, lifecycle set/figure
/// documents, benchmark source/documents, threat documents, and the
/// proposed-state trace targets. Research execution/orchestration inputs are
/// deliberately not collected here.
fn research_referenced_paths(metadata: &Value) -> Vec<String> {
    fn push_str(paths: &mut Vec<String>, value: Option<&Value>) {
        if let Some(text) = value.and_then(Value::as_str) {
            paths.push(text.to_owned());
        }
    }
    fn push_entries_path(paths: &mut Vec<String>, list: Option<&Value>) {
        for entry in list.and_then(Value::as_array).into_iter().flatten() {
            if let Some(text) = entry.get("path").and_then(Value::as_str) {
                paths.push(text.to_owned());
            }
        }
    }
    fn push_string_list(paths: &mut Vec<String>, list: Option<&Value>) {
        for entry in list.and_then(Value::as_array).into_iter().flatten() {
            if let Some(text) = entry.as_str() {
                paths.push(text.to_owned());
            }
        }
    }

    let mut paths = Vec::new();
    push_str(
        &mut paths,
        metadata
            .get("claim_freshness")
            .and_then(|v| v.get("policy")),
    );

    if let Some(lifecycle) = metadata.get("lifecycle") {
        push_entries_path(&mut paths, lifecycle.get("set_documents"));
        push_entries_path(&mut paths, lifecycle.get("figure_documents"));
    }
    if let Some(benchmark) = metadata.get("benchmark") {
        if let Some(pactbench) = benchmark.get("pactbench") {
            push_str(&mut paths, pactbench.get("source"));
            push_entries_path(&mut paths, pactbench.get("documents"));
        }
        push_entries_path(&mut paths, benchmark.get("range_documents"));
        push_string_list(&mut paths, benchmark.get("mapping_documents"));
    }
    if let Some(threats) = metadata.get("threats") {
        push_string_list(&mut paths, threats.get("documents"));
    }
    if let Some(trace) = metadata.get("proposed_state_trace") {
        push_str(&mut paths, trace.get("capture"));
        push_string_list(&mut paths, trace.get("decisions"));
        push_str(&mut paths, trace.get("work_item"));
        push_str(&mut paths, trace.get("implementation_evidence"));
        push_str(&mut paths, trace.get("rollout_evidence"));
    }
    paths
}

fn walk_files(path: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let mut stack = vec![path.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() && !kind.is_symlink() {
                stack.push(path);
            } else if kind.is_file() {
                result.push(path);
            }
        }
    }
    result.sort_by(|left, right| path_string(left).cmp(&path_string(right)));
    result
}

fn walk_files_inner(
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
    for entry in entries {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_dir() && !kind.is_symlink() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !ignored_part(&name) {
                walk_files_inner(root, &normalize_path(&path), linked, result);
            }
        } else if kind.is_file() {
            result.push(normalize_path(&path));
        }
    }
}

fn path_state(path: &Path) -> PathState {
    let Ok(metadata) = fs::metadata(path) else {
        return PathState::Absent;
    };
    if metadata.is_file() {
        let Ok(bytes) = fs::read(path) else {
            return PathState::Present {
                digest: "unreadable".to_owned(),
                kind: "file".to_owned(),
            };
        };
        return PathState::Present {
            digest: hex_digest(Sha256::digest(bytes)),
            kind: "file".to_owned(),
        };
    }
    if metadata.is_dir() {
        let mut entries = walk_files(path);
        entries.sort_by(|left, right| path_string(left).cmp(&path_string(right)));
        let mut hasher = Sha256::new();
        for entry in entries {
            hasher.update(path_string(&entry).as_bytes());
            if let Ok(bytes) = fs::read(&entry) {
                hasher.update(Sha256::digest(bytes));
            }
        }
        return PathState::Present {
            digest: hex_digest(hasher.finalize()),
            kind: "directory".to_owned(),
        };
    }
    PathState::Present {
        digest: "special".to_owned(),
        kind: "other".to_owned(),
    }
}

pub fn parse_front_matter(text: &str) -> Result<BTreeMap<String, Value>, String> {
    let lines = text.lines().collect::<Vec<_>>();
    if lines.first().map(|line| line.trim()) != Some("---") {
        return Err("missing leading '---' front-matter fence".to_owned());
    }
    let mut fields = BTreeMap::new();
    for line in lines.into_iter().skip(1) {
        if line.trim() == "---" {
            return Ok(fields);
        }
        if line.trim().is_empty() {
            continue;
        }
        let Some((key, raw)) = line.split_once(':') else {
            return Err(format!("front-matter line is not 'key: value': {line:?}"));
        };
        fields.insert(key.trim().to_owned(), front_matter_value(raw.trim()));
    }
    Err("missing closing '---' front-matter fence".to_owned())
}

fn front_matter_value(raw: &str) -> Value {
    if raw.starts_with('[') && raw.ends_with(']') {
        let inner = raw[1..raw.len() - 1].trim();
        if inner.is_empty() {
            return Value::Array(Vec::new());
        }
        return Value::Array(
            inner
                .split(',')
                .map(|item| Value::String(item.trim().trim_matches(['\'', '"']).to_owned()))
                .collect(),
        );
    }
    Value::String(raw.trim_matches(['\'', '"']).to_owned())
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

fn git_common_dir(root: &Path, runner: &dyn GitRunner) -> Option<PathBuf> {
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
    let output = runner
        .run(root, &["rev-parse", "--git-common-dir"], "git-common-dir")
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

fn registered_worktree_roots(root: &Path, runner: &dyn GitRunner) -> BTreeSet<PathBuf> {
    if !root.join(".git").exists() {
        return BTreeSet::new();
    }
    let Ok(output) = runner.run(root, &["worktree", "list", "--porcelain"], "worktree-list") else {
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
    let repository = Repository::open(root);
    discover_embedded_worktree_roots_with_runner(&repository.root, repository.git_runner.as_ref())
}

fn discover_embedded_worktree_roots_with_runner(
    root: &Path,
    runner: &dyn GitRunner,
) -> BTreeSet<PathBuf> {
    let root = normalize_path(root);
    if !root.join(".git").exists() {
        return BTreeSet::new();
    }
    let mut linked = registered_worktree_roots(&root, runner);
    let common = git_common_dir(&root, runner);
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
    use std::process::Command;
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
    fn snapshot_git_invocation_count_is_bounded_independent_of_work_items() {
        let root = temp_root("git-fanout-baseline");
        let git = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(&root)
                .output()
                .expect("git is required for the deterministic count fixture")
        };
        assert!(git(&["init", "-q"]).status.success());
        fs::create_dir_all(root.join("work/active")).unwrap();
        for count in [1usize, 10, 50, 100] {
            for entry in fs::read_dir(root.join("work/active")).unwrap().flatten() {
                fs::remove_dir_all(entry.path()).unwrap();
            }
            for number in 0..count {
                let directory = root.join(format!("work/active/{number:03}-item"));
                fs::create_dir_all(&directory).unwrap();
                fs::write(directory.join("work-item.json"), "{}").unwrap();
            }
            let runner = CountingGitRunner::native();
            let repository = Repository::with_git_runner(&root, runner.clone());
            let _ = repository.session().snapshot();
            eprintln!(
                "optimized work_items={count} git_invocations={}",
                runner.count()
            );
            assert!(
                runner.count() <= 4,
                "snapshot Git count grew for {count} work items"
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn production_git_spawning_is_centralized_in_the_approved_helper() {
        let source = include_str!("lib.rs");
        let production = source
            .split("#[cfg(test)]\nmod tests")
            .next()
            .expect("repository test module marker should be present");
        assert!(!production.contains("Command::new(\"git\")"));
        let validation = include_str!("../../repopact-validation/src/lib.rs");
        assert!(!validation.contains("Command::new(\"git\")"));
        let helper = include_str!("git.rs");
        assert!(helper.contains("GIT_TERMINAL_PROMPT"));
        assert!(helper.contains("wait_bounded"));
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

    // WI059 containment correction: a snapshot must never read a metadata-
    // directed path outside Repository::root(), whether or not validation
    // later reports it as invalid. These tests inspect RecordIndex directly
    // (not through Validator) so a regression here can never be masked by a
    // diagnostic-message check alone.

    const SENTINEL_CONTENT: &str = "SENTINEL-DO-NOT-INGEST-OUTSIDE-REPOSITORY-CONTENT";

    fn write_json(path: &Path, value: &Value) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    }

    fn vendored_adopters_manifest(overlay_path: &str) -> Value {
        serde_json::json!({
            "version": 1,
            "upstream": {"repository": "org/repo", "version_file": "VERSION"},
            "adopters": [{
                "id": "adopter-one",
                "repository": "org/repo-one",
                "default_branch": "main",
                "consumption": {
                    "type": "vendored",
                    "version_file": "VERSION",
                    "upstream_version": "1.0.0",
                    "upstream_revision": "0".repeat(40),
                    "files": [{
                        "upstream_path": "repopact/cli.py",
                        "adopter_path": "vendor/cli.py",
                        "mode": "overlay",
                        "upstream_sha256": "0".repeat(64),
                        "adopter_sha256": "0".repeat(64),
                        "overlay_path": overlay_path,
                        "overlay_sha256": "0".repeat(64),
                    }],
                },
                "validation_commands": ["repopact validate"],
            }],
        })
    }

    fn research_metadata_referencing(policy: &str) -> Value {
        serde_json::json!({
            "version": 1,
            "claim_freshness": {
                "policy": policy,
                "verified_on": "2026-01-01",
                "review_by": "2026-01-15",
                "documents": [],
            },
        })
    }

    fn build_index(root: &Path) -> RecordIndex {
        let repository = Repository::open(root);
        RecordIndex::build(&repository)
    }

    fn assert_not_ingested(index: &RecordIndex, repository: &Repository, outside: &Path) {
        let normalized = normalize_path(outside);
        assert!(
            !index.source_paths.contains(&normalized),
            "source_paths must not contain an out-of-repository path"
        );
        assert!(
            index.text(&normalized).is_none(),
            "text_files must not contain out-of-repository content"
        );
        assert!(
            index.overlay_bytes(&normalized).is_none(),
            "adopter_overlay_bytes must not contain out-of-repository content"
        );
        let read_set = index.read_set(repository);
        assert!(
            !format!("{read_set:?}").contains(SENTINEL_CONTENT),
            "read set must carry no trace of out-of-repository content"
        );
    }

    #[test]
    fn valid_relative_adopter_overlay_is_ingested() {
        let root = temp_root("containment-adopter-valid");
        fs::write(root.join("VERSION"), "0.1.0\n").unwrap();
        fs::write(root.join("vendor-overlay.py"), "print('inside')\n").unwrap();
        write_json(
            &root.join("governance/adopters.json"),
            &vendored_adopters_manifest("vendor-overlay.py"),
        );
        let index = build_index(&root);
        let expected = normalize_path(&root.join("vendor-overlay.py"));
        assert!(index.source_paths.contains(&expected));
        assert!(index.overlay_bytes(&expected).is_some());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn adopter_overlay_escaping_via_relative_traversal_is_not_ingested() {
        let root = temp_root("containment-adopter-relative-escape");
        fs::write(root.join("VERSION"), "0.1.0\n").unwrap();
        let outside_dir = temp_root("containment-adopter-outside");
        let outside_file = outside_dir.join("secret.py");
        fs::write(&outside_file, SENTINEL_CONTENT).unwrap();
        let outside_name = outside_dir.file_name().unwrap().to_str().unwrap();
        write_json(
            &root.join("governance/adopters.json"),
            &vendored_adopters_manifest(&format!("../{outside_name}/secret.py")),
        );
        let repository = Repository::open(&root);
        let index = RecordIndex::build(&repository);
        assert_not_ingested(&index, &repository, &outside_file);
        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&outside_dir).unwrap();
    }

    #[test]
    fn adopter_overlay_escaping_via_absolute_path_is_not_ingested() {
        let root = temp_root("containment-adopter-absolute-escape");
        fs::write(root.join("VERSION"), "0.1.0\n").unwrap();
        let outside_dir = temp_root("containment-adopter-absolute-outside");
        let outside_file = outside_dir.join("secret.py");
        fs::write(&outside_file, SENTINEL_CONTENT).unwrap();
        write_json(
            &root.join("governance/adopters.json"),
            &vendored_adopters_manifest(&outside_file.to_string_lossy()),
        );
        let repository = Repository::open(&root);
        let index = RecordIndex::build(&repository);
        assert_not_ingested(&index, &repository, &outside_file);
        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&outside_dir).unwrap();
    }

    #[test]
    fn valid_relative_research_reference_is_ingested() {
        let root = temp_root("containment-research-valid");
        fs::create_dir_all(root.join("governance/policies")).unwrap();
        fs::write(root.join("governance/policies/freshness.md"), "# Policy\n").unwrap();
        write_json(
            &root.join("research/metadata.json"),
            &research_metadata_referencing("governance/policies/freshness.md"),
        );
        let index = build_index(&root);
        let expected = normalize_path(&root.join("governance/policies/freshness.md"));
        assert!(index.source_paths.contains(&expected));
        assert!(index.text(&expected).is_some());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn research_reference_escaping_via_relative_traversal_is_not_ingested() {
        let root = temp_root("containment-research-relative-escape");
        let outside_dir = temp_root("containment-research-outside");
        let outside_file = outside_dir.join("secret-policy.md");
        fs::write(&outside_file, SENTINEL_CONTENT).unwrap();
        let outside_name = outside_dir.file_name().unwrap().to_str().unwrap();
        write_json(
            &root.join("research/metadata.json"),
            &research_metadata_referencing(&format!("../{outside_name}/secret-policy.md")),
        );
        let repository = Repository::open(&root);
        let index = RecordIndex::build(&repository);
        assert_not_ingested(&index, &repository, &outside_file);
        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&outside_dir).unwrap();
    }

    #[test]
    fn research_reference_escaping_via_absolute_path_is_not_ingested() {
        let root = temp_root("containment-research-absolute-escape");
        let outside_dir = temp_root("containment-research-absolute-outside");
        let outside_file = outside_dir.join("secret-policy.md");
        fs::write(&outside_file, SENTINEL_CONTENT).unwrap();
        write_json(
            &root.join("research/metadata.json"),
            &research_metadata_referencing(&outside_file.to_string_lossy()),
        );
        let repository = Repository::open(&root);
        let index = RecordIndex::build(&repository);
        assert_not_ingested(&index, &repository, &outside_file);
        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&outside_dir).unwrap();
    }

    #[test]
    fn adopter_overlay_escaping_via_symlink_is_not_ingested() {
        let root = temp_root("containment-adopter-symlink-escape");
        fs::write(root.join("VERSION"), "0.1.0\n").unwrap();
        let outside_dir = temp_root("containment-adopter-symlink-outside");
        let outside_file = outside_dir.join("secret.py");
        fs::write(&outside_file, SENTINEL_CONTENT).unwrap();
        let link = root.join("link-overlay.py");
        let linked = symlink_file(&outside_file, &link);
        if !linked {
            eprintln!("skipping symlink containment test: platform/permissions do not allow creating a file symlink");
            fs::remove_dir_all(&root).unwrap();
            fs::remove_dir_all(&outside_dir).unwrap();
            return;
        }
        write_json(
            &root.join("governance/adopters.json"),
            &vendored_adopters_manifest("link-overlay.py"),
        );
        let repository = Repository::open(&root);
        let index = RecordIndex::build(&repository);
        assert_not_ingested(&index, &repository, &outside_file);
        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&outside_dir).unwrap();
    }

    #[cfg(windows)]
    fn symlink_file(target: &Path, link: &Path) -> bool {
        std::os::windows::fs::symlink_file(target, link).is_ok()
    }

    #[cfg(not(windows))]
    fn symlink_file(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
}
