use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LifecycleStatus {
    Proposed,
    Active,
    Blocked,
    Deferred,
    Completed,
}

impl LifecycleStatus {
    pub const ALL: [&'static str; 5] = ["proposed", "active", "blocked", "deferred", "completed"];

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "proposed" => Some(Self::Proposed),
            "active" => Some(Self::Active),
            "blocked" => Some(Self::Blocked),
            "deferred" => Some(Self::Deferred),
            "completed" => Some(Self::Completed),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Active => "active",
            Self::Blocked => "blocked",
            Self::Deferred => "deferred",
            Self::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provenance {
    Concrete,
    Provisional,
    Inferred,
}

impl Provenance {
    pub fn parse_or_concrete(value: Option<&str>) -> Self {
        match value.unwrap_or("concrete") {
            "provisional" => Self::Provisional,
            "inferred" => Self::Inferred,
            _ => Self::Concrete,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Concrete => "concrete",
            Self::Provisional => "provisional",
            Self::Inferred => "inferred",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptanceCriterion {
    pub id: String,
    pub text: String,
    pub state: String,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default = "default_concrete")]
    pub provenance: String,
}

fn default_concrete() -> String {
    "concrete".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreflightMarker {
    pub created_before_work_started: bool,
    pub created_at: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItem {
    pub id: String,
    pub title: String,
    pub status: String,
    pub owner_scope: String,
    #[serde(default)]
    pub affected_scopes: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default = "default_concrete")]
    pub provenance: String,
    #[serde(default)]
    pub preflight: Option<PreflightMarker>,
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    pub created: String,
    pub updated: String,
}

impl WorkItem {
    pub fn lifecycle_status(&self) -> Option<LifecycleStatus> {
        LifecycleStatus::parse(&self.status)
    }

    pub fn provenance_level(&self) -> Provenance {
        Provenance::parse_or_concrete(Some(&self.provenance))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceReference {
    pub id: String,
    pub work_item: String,
    pub result: String,
    #[serde(default = "default_concrete")]
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryIdentity {
    pub root: String,
    #[serde(default)]
    pub git_common_dir: Option<String>,
    #[serde(default)]
    pub git_worktree_root: Option<String>,
    pub linked_worktree: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordKind {
    Repository,
    WorkItem,
    AcceptanceCriterion,
    EvidenceRun,
    Scope,
    Role,
    Decision,
    Policy,
    Contract,
    Invariant,
    FrozenSurface,
    AuditFinding,
    AuditRegistry,
    Dashboard,
    Template,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RecordRef {
    pub kind: RecordKind,
    pub id: String,
    pub path: String,
}

pub type SourceRef = RecordRef;

impl RecordRef {
    pub fn new(kind: RecordKind, id: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            kind,
            id: id.into(),
            path: path.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PathState {
    Absent,
    Present { digest: String, kind: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadFact {
    pub path: String,
    pub expected: PathState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadSet {
    pub facts: Vec<ReadFact>,
}

impl ReadSet {
    pub fn new(mut facts: Vec<ReadFact>) -> Self {
        facts.sort_by(|left, right| left.path.cmp(&right.path));
        facts.dedup_by(|left, right| left.path == right.path);
        Self { facts }
    }

    pub fn with_absent(mut self, path: impl Into<String>) -> Self {
        self.facts.push(ReadFact {
            path: path.into(),
            expected: PathState::Absent,
        });
        Self::new(self.facts)
    }

    pub fn token(&self, identity: &RepositoryIdentity) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"repopact-plan-token-v1\0");
        update_len_prefixed(&mut hasher, identity.root.as_bytes());
        update_len_prefixed(
            &mut hasher,
            identity.git_common_dir.as_deref().unwrap_or("").as_bytes(),
        );
        update_len_prefixed(
            &mut hasher,
            identity
                .git_worktree_root
                .as_deref()
                .unwrap_or("")
                .as_bytes(),
        );
        hasher.update([u8::from(identity.linked_worktree)]);
        for fact in &self.facts {
            update_len_prefixed(&mut hasher, fact.path.as_bytes());
            match &fact.expected {
                PathState::Absent => hasher.update(b"absent"),
                PathState::Present { digest, kind } => {
                    hasher.update(b"present");
                    update_len_prefixed(&mut hasher, digest.as_bytes());
                    update_len_prefixed(&mut hasher, kind.as_bytes());
                }
            }
        }
        hex_digest(hasher.finalize())
    }
}

fn update_len_prefixed(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

pub fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_records: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_actions: Option<Vec<String>>,
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            message: message.into(),
            path: None,
            record: None,
            field: None,
            related_records: None,
            suggested_actions: None,
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_record(mut self, record: impl Into<String>) -> Self {
        self.record = Some(record.into());
        self
    }

    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    pub fn render(&self) -> String {
        let location = self
            .path
            .as_deref()
            .or(self.record.as_deref())
            .unwrap_or("<repository>");
        format!("ERROR [{}] {}: {}", self.code, location, self.message)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}
