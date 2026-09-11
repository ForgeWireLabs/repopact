mod adopters;
mod research;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
#[cfg(test)]
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use repopact_repository::{
    path_string, IndexedRecord, RecordIndex, Repository, RepositorySnapshot, RepositoryTopology,
    STATUSES,
};
use repopact_schema::SchemaStore;
use repopact_types::{Diagnostic, LifecycleStatus, ValidationReport, WorkItem};
use serde_json::{Map, Value};

const REQUIRED_WORK_FIELDS: [&str; 9] = [
    "id",
    "title",
    "status",
    "owner_scope",
    "affected_scopes",
    "depends_on",
    "acceptance_criteria",
    "created",
    "updated",
];
#[derive(Debug, Clone)]
struct LoadedWork {
    path: PathBuf,
    item: Option<WorkItem>,
}

pub fn validate(root: impl AsRef<Path>) -> ValidationReport {
    let repository = Repository::open(root);
    let snapshot = repository.session().snapshot();
    validate_snapshot(&snapshot)
}

/// Validate a snapshot opened by the shared repository session. The current
/// semantic validator still owns the WI053 rule implementation, but callers do
/// not need to reopen or independently crawl a repository to select the
/// validation boundary.
pub fn validate_snapshot(snapshot: &RepositorySnapshot) -> ValidationReport {
    Validator::from_snapshot(snapshot).validate()
}

/// Render the owned dashboard projection without writing it. Mutation planning
/// and validation use this same projection so there is one Rust implementation
/// of the generated artifact.
pub fn render_dashboard(root: impl AsRef<Path>) -> Result<String, String> {
    let repository = Repository::open(root);
    let snapshot = repository.session().snapshot();
    render_dashboard_snapshot(&snapshot)
}

/// Render the dashboard from an already opened immutable repository generation.
/// This keeps callers such as the compatibility engine on the same snapshot
/// boundary as validation, graph, and analysis instead of recrawling the root.
pub fn render_dashboard_snapshot(snapshot: &RepositorySnapshot) -> Result<String, String> {
    let mut validator = Validator::from_snapshot(&snapshot);
    validator.work = loaded_work(snapshot.index());
    if validator.work.iter().any(|record| record.item.is_none()) {
        return Err("unable to render dashboard from malformed work-item records".to_owned());
    }
    validator
        .generate_dashboard()
        .ok_or_else(|| "unable to render dashboard from repository source records".to_owned())
}

pub struct Validator {
    repository: Repository,
    topology: RepositoryTopology,
    index: RecordIndex,
    schemas: SchemaStore,
    diagnostics: Vec<Diagnostic>,
    work: Vec<LoadedWork>,
    evidence_ids: BTreeSet<String>,
    work_ids: BTreeSet<String>,
    /// Deterministic "today" (`YYYY-MM-DD`) for research claim-freshness
    /// expiry, overridable so tests are not wall-clock-dependent; `None`
    /// means production behavior (the real current date via `today_utc()`).
    today: Option<String>,
}

impl Validator {
    pub fn new(repository: Repository) -> Self {
        let snapshot = repository.session().snapshot();
        Self::from_snapshot(&snapshot)
    }

    pub fn from_snapshot(snapshot: &RepositorySnapshot) -> Self {
        let repository = snapshot.repository().clone();
        let schemas = SchemaStore::new(repository.root());
        let evidence_ids = snapshot
            .index()
            .evidence
            .iter()
            .filter_map(|record| {
                record
                    .value
                    .as_ref()
                    .ok()?
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .collect();
        Self {
            repository,
            topology: snapshot.topology().clone(),
            index: snapshot.index().clone(),
            schemas,
            diagnostics: Vec::new(),
            work: Vec::new(),
            evidence_ids,
            work_ids: BTreeSet::new(),
            today: None,
        }
    }

    /// Override "today" for deterministic research claim-freshness testing
    /// (`YYYY-MM-DD`). Production validation should not call this; leaving it
    /// unset uses the real current date.
    pub fn with_today(mut self, today: impl Into<String>) -> Self {
        self.today = Some(today.into());
        self
    }

    pub fn validate(mut self) -> ValidationReport {
        self.report_unsupported_surfaces();
        self.validate_version();
        self.validate_release_label();
        self.validate_release_surface();
        self.validate_contracts();
        self.validate_invariants();
        self.validate_frozen_surface();
        let (owner_scopes, enforce_disjoint) = self.validate_owners();
        self.validate_work(&owner_scopes, enforce_disjoint);
        self.validate_orphan_work_dirs();
        self.validate_evidence();
        self.validate_audit_registry();
        self.validate_dashboard();
        self.validate_adopters();
        self.validate_research();
        self.diagnostics.sort_by(|left, right| {
            (left.path.as_deref(), &left.message).cmp(&(right.path.as_deref(), &right.message))
        });
        ValidationReport {
            diagnostics: self.diagnostics,
        }
    }

    fn report_unsupported_surfaces(&mut self) {
        let root = self.repository.root();
        let unsupported = [
            (
                root.join("governance/admission-policy.json"),
                "WI050 admission/enforcement records",
            ),
            (
                root.join("governance/operator-authority.json"),
                "WI050 operator authority records",
            ),
            (
                root.join("governance/repository-registration.json"),
                "WI050 repository registration records",
            ),
        ];
        for (path, surface) in unsupported {
            if path.is_file() {
                self.push(
                    Diagnostic::error(
                        "unsupported.semantic-surface",
                        format!("unsupported Rust semantics: {surface} are outside WI053"),
                    )
                    .with_path(self.rel(&path)),
                );
            }
        }
    }

    fn validate_version(&mut self) {
        let path = self.repository.root().join("VERSION");
        let Some(value) = self.index.text(&path) else {
            self.push(self.at("version.missing", "missing VERSION file", &path));
            return;
        };
        let value = value.trim();
        if !is_semver_core(value) {
            self.push(self.at(
                "version.invalid",
                format!("VERSION '{value}' must be semantic (MAJOR.MINOR.PATCH)"),
                &path,
            ));
        }
    }

    fn validate_release_label(&mut self) {
        let path = self.repository.root().join("RELEASE_LABEL");
        let Some(label) = self.index.text(&path).map(str::trim) else {
            return;
        };
        let version = self
            .index
            .text(&self.repository.root().join("VERSION"))
            .unwrap_or_default()
            .trim();
        let Some((base, prerelease)) = parse_release_label(&label) else {
            self.push(self.at(
                "release-label.invalid",
                format!(
                    "RELEASE_LABEL '{label}' must be a SemVer pre-release of VERSION (MAJOR.MINOR.PATCH-prerelease[+build], e.g. 2.3.0-rc.1)"
                ),
                &path,
            ));
            return;
        };
        let _ = prerelease;
        if base != version {
            self.push(self.at(
                "release-label.version-mismatch",
                format!("RELEASE_LABEL base '{base}' must equal VERSION '{version}'"),
                &path,
            ));
        }
    }

    fn validate_release_surface(&mut self) {
        let readme = self.repository.root().join("README.md");
        let version_path = self.repository.root().join("VERSION");
        let (Some(text), Some(version)) = (
            self.index.text(&readme).map(str::to_owned),
            self.index.text(&version_path).map(str::to_owned),
        ) else {
            return;
        };
        let Some(start) = text.find("current release **") else {
            return;
        };
        let claim_start = start + "current release **".len();
        let Some(end_offset) = text[claim_start..].find("**") else {
            return;
        };
        let claimed = &text[claim_start..claim_start + end_offset];
        let version = version.trim();
        if claimed != version {
            self.push(self.at(
                "release-surface.version-mismatch",
                format!(
                    "README advertises release '{claimed}' but VERSION is '{version}'; update the release line together with VERSION"
                ),
                &readme,
            ));
        }
        let after = &text[claim_start + end_offset + 2..];
        let Some(link_start) = after.find("](") else {
            return;
        };
        let target_start = link_start + 2;
        let Some(target_end) = after[target_start..].find(')') else {
            return;
        };
        let link = &after[target_start..target_start + target_end];
        if link.contains(":") {
            return;
        }
        let target = self
            .repository
            .root()
            .join(link.split('#').next().unwrap_or(link));
        if !target.is_file() {
            self.push(self.at(
                "release-surface.link-missing",
                format!("release changelog link does not resolve: {link}"),
                &readme,
            ));
        }
    }

    fn validate_contracts(&mut self) {
        let contracts = self
            .index
            .contracts
            .iter()
            .map(|record| record.path.clone())
            .collect::<Vec<_>>();
        let root_contract = self.repository.root().join("AGENTS.md");
        if !contracts.iter().any(|path| path == &root_contract) {
            self.push(self.at(
                "contract.root-missing",
                "missing root AGENTS.md",
                self.repository.root(),
            ));
        }
        let covered = self.registered_contract_dirs();
        for contract in contracts {
            if contract.parent() == Some(self.repository.root()) {
                continue;
            }
            let parent = contract.parent().map(repopact_repository::normalize_path);
            if parent.as_ref().is_none_or(|path| !covered.contains(path)) {
                self.push(self.at(
                    "contract.unregistered",
                    "nested contract is not registered in audits/registry.json",
                    &contract,
                ));
            }
            if let Some(parent) = parent {
                let audit = parent.join("_audit");
                if audit.is_dir() {
                    for name in ["README.md", "inventory.md", "alignment-report.md"] {
                        if !audit.join(name).is_file() {
                            self.push(self.at(
                                "contract.audit-companion-incomplete",
                                format!("incomplete _audit companion, missing _audit/{name}"),
                                &contract,
                            ));
                        }
                    }
                }
            }
        }
    }

    fn registered_contract_dirs(&self) -> BTreeSet<PathBuf> {
        let Some(record) = self.index.audit_registry.as_ref() else {
            return BTreeSet::new();
        };
        let Ok(value) = record.value.as_ref() else {
            return BTreeSet::new();
        };
        object_array(&value, "scopes")
            .filter_map(|entry| entry.get("contract").and_then(Value::as_str))
            .map(|contract| {
                repopact_repository::normalize_path(&self.repository.root().join(contract))
                    .parent()
                    .unwrap_or(Path::new("."))
                    .to_path_buf()
            })
            .collect()
    }

    fn validate_invariants(&mut self) {
        let path = self.repository.root().join("governance/invariants.json");
        let Some(value) = self
            .index
            .invariants
            .as_ref()
            .and_then(|record| record.value.clone().ok())
        else {
            self.push(self.at(
                "invariants.unreadable",
                "governance/invariants.json is not valid JSON",
                &path,
            ));
            return;
        };
        self.extend_schema(&value, "invariants.schema.json", &path);
        let mut seen = HashSet::new();
        for entry in object_array(&value, "invariants") {
            let id = entry.get("id").and_then(Value::as_str).unwrap_or("");
            if !seen.insert(id.to_owned()) {
                self.push(self.at(
                    "invariants.duplicate-id",
                    format!("duplicate invariant id '{id}'"),
                    &path,
                ));
            }
            for field in ["statement", "rationale", "escalation"] {
                if entry
                    .get(field)
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .is_empty()
                {
                    self.push(self.at(
                        "invariants.field-missing",
                        format!("invariant {id} is missing {field}"),
                        &path,
                    ));
                }
            }
        }
    }

    fn validate_frozen_surface(&mut self) {
        let path = self
            .repository
            .root()
            .join("governance/frozen-surface.json");
        let Some(value) = self
            .index
            .frozen_surface
            .as_ref()
            .and_then(|record| record.value.clone().ok())
        else {
            self.push(self.at(
                "frozen-surface.unreadable",
                "governance/frozen-surface.json is not valid JSON",
                &path,
            ));
            return;
        };
        self.extend_schema(&value, "frozen-surface.schema.json", &path);
        for entry in object_array(&value, "protected") {
            if entry
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .is_empty()
            {
                self.push(self.at(
                    "frozen-surface.reason-missing",
                    format!(
                        "protected entry '{}' needs a reason",
                        entry.get("glob").and_then(Value::as_str).unwrap_or("")
                    ),
                    &path,
                ));
            }
        }
    }

    fn validate_owners(&mut self) -> (BTreeSet<String>, bool) {
        let path = self.repository.root().join("governance/owners.json");
        let Some(value) = self
            .index
            .owners
            .as_ref()
            .and_then(|record| record.value.clone().ok())
        else {
            self.push(self.at(
                "owners.unreadable",
                "governance/owners.json is not valid JSON",
                &path,
            ));
            return (BTreeSet::new(), false);
        };
        let scopes = object_array(&value, "scopes").collect::<Vec<_>>();
        let mut scope_ids = BTreeSet::new();
        for scope in &scopes {
            let id = scope
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            if !scope_ids.insert(id.clone()) {
                self.push(self.at("owners.duplicate-scope", "scope IDs must be unique", &path));
            }
            if map_string_array(scope, "paths").next().is_none() {
                self.push(self.at(
                    "owners.paths-missing",
                    format!("scope '{id}' must declare non-empty path patterns"),
                    &path,
                ));
            }
        }
        for role in object_array(&value, "roles") {
            let role_id = role.get("id").and_then(Value::as_str).unwrap_or("");
            for scope in map_string_array(role, "scopes") {
                if !scope_ids.contains(scope) {
                    self.push(self.at(
                        "owners.unknown-scope",
                        format!("role '{role_id}' references unknown scope '{scope}'"),
                        &path,
                    ));
                }
            }
        }
        let enforce = value
            .get("enforce_tracked_path_ownership")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if enforce {
            self.validate_tracked_path_ownership(&scopes, &path);
        }
        let disjoint = value
            .get("concurrency")
            .and_then(Value::as_object)
            .and_then(|object| object.get("enforce_disjoint_active_scopes"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        (scope_ids, disjoint)
    }

    fn validate_tracked_path_ownership(
        &mut self,
        scopes: &[&Map<String, Value>],
        owners_path: &Path,
    ) {
        let Some(tracked_paths) = self.topology.tracked_paths().cloned() else {
            return;
        };
        let patterns = scopes
            .iter()
            .flat_map(|scope| {
                let id = scope.get("id").and_then(Value::as_str).unwrap_or("");
                map_string_array(scope, "paths").map(move |pattern| (id, pattern))
            })
            .collect::<Vec<_>>();
        for relative in tracked_paths {
            let matches = patterns
                .iter()
                .filter(|(_, pattern)| wildcard_match(&relative, pattern))
                .map(|(id, _)| *id)
                .collect::<BTreeSet<_>>();
            if matches.is_empty() {
                self.push(self.at(
                    "owners.unowned-tracked-path",
                    format!("tracked path '{relative}' has no owner scope"),
                    owners_path,
                ));
            } else if matches.len() > 1 {
                self.push(self.at(
                    "owners.overlapping-tracked-path",
                    format!(
                        "tracked path '{relative}' has multiple owner scopes: {}",
                        matches.into_iter().collect::<Vec<_>>().join(", ")
                    ),
                    owners_path,
                ));
            }
        }
    }

    fn validate_work(&mut self, owner_scopes: &BTreeSet<String>, enforce_disjoint: bool) {
        let preflight = self.preflight_config();
        let evidence_provenance = self.evidence_provenance();
        let mut seen = BTreeMap::new();
        for record in self.index.work_items.clone() {
            let directory = record_directory(&record);
            let data = match record.value.clone() {
                Ok(value) => value,
                Err(error) => {
                    self.push(self.at("work.json-invalid", error, &record.path));
                    continue;
                }
            };
            let Some(object) = data.as_object() else {
                self.push(self.at("work.json-invalid", "expected a JSON object", &record.path));
                continue;
            };
            let missing = REQUIRED_WORK_FIELDS
                .iter()
                .filter(|field| !object.contains_key(**field))
                .copied()
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                self.push(self.at(
                    "work.fields-missing",
                    format!("missing fields: {}", missing.join(", ")),
                    &record.path,
                ));
                continue;
            }
            self.extend_schema(&data, "work-item.schema.json", &record.path);
            let item: WorkItem = match serde_json::from_value(data.clone()) {
                Ok(item) => item,
                Err(error) => {
                    self.push(self.at("work.record-unreadable", error.to_string(), &record.path));
                    continue;
                }
            };
            let expected_status = directory
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if item.status != expected_status || LifecycleStatus::parse(&item.status).is_none() {
                self.push(self.at(
                    "work.status-directory-mismatch",
                    format!(
                        "status '{}' does not match directory '{}'",
                        item.status, expected_status
                    ),
                    &record.path,
                ));
            }
            if !is_digits_at_least(&item.id, 3) {
                self.push(self.at(
                    "work.id-invalid",
                    "id must contain at least three digits",
                    &record.path,
                ));
            } else if let Some(previous) = seen.insert(item.id.clone(), record.path.clone()) {
                self.push(self.at(
                    "work.duplicate-id",
                    format!("duplicate id also used by {}", self.rel(&previous)),
                    &record.path,
                ));
            } else {
                self.work_ids.insert(item.id.clone());
            }
            let directory_name = directory
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if directory_name.split('-').next().unwrap_or("") != item.id {
                self.push(self.at(
                    "work.directory-id-mismatch",
                    "directory prefix must match work-item id",
                    &record.path,
                ));
            }
            if self.index.text(&directory.join("README.md")).is_none() {
                self.push(self.at(
                    "work.readme-missing",
                    "missing README.md narrative",
                    directory,
                ));
            }
            if !owner_scopes.contains(&item.owner_scope) {
                self.push(self.at(
                    "work.unknown-owner-scope",
                    format!("unknown owner_scope '{}'", item.owner_scope),
                    &record.path,
                ));
            }
            for scope in &item.affected_scopes {
                if !owner_scopes.contains(scope) {
                    self.push(self.at(
                        "work.unknown-affected-scope",
                        format!("unknown affected_scope '{scope}'"),
                        &record.path,
                    ));
                }
            }
            if !is_iso_date(&item.created) {
                self.push(self.at(
                    "work.date-invalid",
                    "created must be an ISO date",
                    &record.path,
                ));
            }
            if !is_iso_date(&item.updated) {
                self.push(self.at(
                    "work.date-invalid",
                    "updated must be an ISO date",
                    &record.path,
                ));
            }
            let mut criterion_ids = HashSet::new();
            for criterion in &item.acceptance_criteria {
                if criterion.id.is_empty() || !criterion_ids.insert(criterion.id.clone()) {
                    self.push(self.at(
                        "work.criterion-id-invalid",
                        "acceptance criterion IDs must be present and unique",
                        &record.path,
                    ));
                }
                if criterion.state == "satisfied" && criterion.evidence.is_empty() {
                    self.push(self.at(
                        "work.satisfied-without-evidence",
                        format!("criterion {} is satisfied without evidence", criterion.id),
                        &record.path,
                    ));
                }
                for evidence in &criterion.evidence {
                    if !self.evidence_ids.contains(evidence) {
                        self.push(self.at(
                            "work.unknown-evidence",
                            format!(
                                "criterion {} references unknown evidence '{evidence}'",
                                criterion.id
                            ),
                            &record.path,
                        ));
                    }
                }
                if item.status == "completed" && criterion.state == "pending" {
                    self.push(self.at(
                        "work.completed-pending-criterion",
                        format!("completed item has pending criterion {}", criterion.id),
                        &record.path,
                    ));
                }
            }
            if requires_preflight(&item, &preflight) && item.preflight.is_none() {
                self.push(self.at(
                    "work.preflight-missing",
                    "work item requires a preflight marker (enabled via governance/owners.json preflight.enabled)",
                    &record.path,
                ));
            }
            let mut rests_on_nonconcrete = false;
            for criterion in &item.acceptance_criteria {
                if criterion.state != "satisfied" {
                    continue;
                }
                let criterion_nonconcrete = criterion.provenance != "concrete"
                    || criterion.evidence.iter().any(|evidence| {
                        evidence_provenance
                            .get(evidence)
                            .map(String::as_str)
                            .unwrap_or("concrete")
                            != "concrete"
                    });
                if criterion_nonconcrete {
                    rests_on_nonconcrete = true;
                    if item.status == "completed" {
                        self.push(self.at(
                            "work.completed-nonconcrete-evidence",
                            format!(
                                "completed item criterion {} rests on non-concrete evidence; ratchet it to concrete before completing (P2)",
                                criterion.id
                            ),
                            &record.path,
                        ));
                    }
                }
            }
            if item.status == "completed" && item.provenance != "concrete" {
                self.push(self.at(
                    "work.completed-provisional",
                    format!(
                        "item provenance '{}' cannot be completed; ratchet to concrete first (P2)",
                        item.provenance
                    ),
                    &record.path,
                ));
            }
            if item.provenance == "concrete" && rests_on_nonconcrete && item.status != "completed" {
                self.push(self.at(
                    "work.concrete-nonconcrete-evidence",
                    "concrete item rests on non-concrete evidence; mark it provisional/inferred or ratchet the evidence (P3)",
                    &record.path,
                ));
            }
            self.validate_readme_checkbox_parity(directory, &item, &record.path);
            self.work.push(LoadedWork {
                path: record.path,
                item: Some(item),
            });
        }
        let status_by_id = self
            .work
            .iter()
            .filter_map(|record| {
                record
                    .item
                    .as_ref()
                    .map(|item| (item.id.clone(), item.status.clone()))
            })
            .collect::<HashMap<_, _>>();
        for record in self.work.clone() {
            let Some(item) = record.item.as_ref() else {
                continue;
            };
            for dependency in &item.depends_on {
                if !self.work_ids.contains(dependency) {
                    self.push(self.at(
                        "work.unknown-dependency",
                        format!("unknown dependency '{dependency}'"),
                        &record.path,
                    ));
                } else if matches!(item.status.as_str(), "active" | "completed")
                    && status_by_id.get(dependency).map(String::as_str) == Some("proposed")
                {
                    self.push(self.at(
                        "work.proposed-dependency",
                        format!(
                            "{} work item depends on proposed work item '{}'; proposed work is not accepted implementation authority",
                            item.status, dependency
                        ),
                        &record.path,
                    ));
                }
            }
        }
        self.detect_dependency_cycles();
        if enforce_disjoint {
            self.validate_disjoint_scopes();
        }
    }

    fn preflight_config(&self) -> Value {
        self.index
            .owners
            .as_ref()
            .and_then(|record| record.value.clone().ok())
            .and_then(|value| value.get("preflight").cloned())
            .unwrap_or_else(|| Value::Object(Map::new()))
    }

    fn evidence_provenance(&self) -> HashMap<String, String> {
        self.index
            .evidence
            .iter()
            .filter_map(|record| {
                let Value::Object(value) = record.value.as_ref().ok()? else {
                    return None;
                };
                Some((
                    value.get("id").and_then(Value::as_str)?.to_owned(),
                    value
                        .get("provenance")
                        .and_then(Value::as_str)
                        .unwrap_or("concrete")
                        .to_owned(),
                ))
            })
            .collect()
    }

    fn validate_readme_checkbox_parity(&mut self, directory: &Path, item: &WorkItem, path: &Path) {
        let Some(text) = self.index.text(&directory.join("README.md")) else {
            return;
        };
        let mut boxes = HashMap::new();
        for line in text.lines() {
            let Some(marker) = line.find("**") else {
                continue;
            };
            let before = &line[..marker];
            let Some(open) = before.find('[') else {
                continue;
            };
            let Some(close) = before[open + 1..].find(']') else {
                continue;
            };
            let state = before[open + 1..open + 1 + close]
                .trim()
                .to_ascii_lowercase();
            if state != "x" && !state.is_empty() {
                continue;
            }
            let rest = &line[marker + 2..];
            let Some(end) = rest.find("**") else { continue };
            let label = &rest[..end];
            if let Some(id) = decision_0014_criterion_id(label) {
                boxes.insert(id.to_owned(), state);
            }
        }
        if boxes.is_empty() {
            return;
        }
        for criterion in &item.acceptance_criteria {
            let Some(state) = boxes.get(&criterion.id) else {
                self.push(self.at(
                    "work.readme-checkbox-missing",
                    format!("criterion {} has no checkbox in README", criterion.id),
                    path,
                ));
                continue;
            };
            if criterion.state == "satisfied" && state != "x" {
                self.push(self.at(
                    "work.readme-checkbox-mismatch",
                    format!(
                        "criterion {} is satisfied but its README checkbox is unchecked",
                        criterion.id
                    ),
                    path,
                ));
            } else if criterion.state == "pending" && state == "x" {
                self.push(self.at(
                    "work.readme-checkbox-mismatch",
                    format!(
                        "criterion {} is pending but its README checkbox is checked",
                        criterion.id
                    ),
                    path,
                ));
            }
        }
    }

    fn detect_dependency_cycles(&mut self) {
        let mut graph = BTreeMap::new();
        for record in &self.work {
            if let Some(item) = record.item.as_ref() {
                graph.insert(item.id.clone(), item.depends_on.clone());
            }
        }
        let mut color = HashMap::new();
        let mut reported = HashSet::new();
        let mut stack = Vec::new();
        let ids = graph.keys().cloned().collect::<Vec<_>>();
        for id in ids {
            if color.get(&id).copied().unwrap_or(0) == 0 {
                self.visit_cycle(&id, &graph, &mut color, &mut stack, &mut reported);
            }
        }
    }

    fn visit_cycle(
        &mut self,
        node: &str,
        graph: &BTreeMap<String, Vec<String>>,
        color: &mut HashMap<String, u8>,
        stack: &mut Vec<String>,
        reported: &mut HashSet<String>,
    ) {
        color.insert(node.to_owned(), 1);
        stack.push(node.to_owned());
        for next in graph.get(node).into_iter().flatten() {
            if !graph.contains_key(next) {
                continue;
            }
            if color.get(next).copied().unwrap_or(0) == 1 {
                let start = stack.iter().position(|value| value == next).unwrap_or(0);
                let cycle = stack[start..].to_vec();
                let key = cycle
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(",");
                if reported.insert(key) {
                    let location = self
                        .work
                        .iter()
                        .find(|record| record.item.as_ref().is_some_and(|item| item.id == node))
                        .map(|record| record.path.clone());
                    if let Some(path) = location {
                        self.push(self.at(
                            "work.dependency-cycle",
                            format!(
                                    "dependency cycle: {}",
                                    cycle
                                        .into_iter()
                                        .chain(std::iter::once(next.clone()))
                                        .collect::<Vec<_>>()
                                        .join(" -> ")
                                ),
                            &path,
                        ));
                    }
                }
            } else if color.get(next).copied().unwrap_or(0) == 0 {
                self.visit_cycle(next, graph, color, stack, reported);
            }
        }
        stack.pop();
        color.insert(node.to_owned(), 2);
    }

    fn validate_disjoint_scopes(&mut self) {
        let active = self
            .work
            .iter()
            .filter_map(|record| {
                let item = record.item.as_ref()?;
                matches!(item.status.as_str(), "active" | "blocked")
                    .then_some((item.clone(), record.path.clone()))
            })
            .collect::<Vec<_>>();
        for (index, (left, left_path)) in active.iter().enumerate() {
            let left_scopes = std::iter::once(left.owner_scope.as_str())
                .chain(left.affected_scopes.iter().map(String::as_str))
                .collect::<BTreeSet<_>>();
            for (right, _) in active.iter().skip(index + 1) {
                let right_scopes = std::iter::once(right.owner_scope.as_str())
                    .chain(right.affected_scopes.iter().map(String::as_str))
                    .collect::<BTreeSet<_>>();
                let overlap = left_scopes
                    .intersection(&right_scopes)
                    .copied()
                    .collect::<Vec<_>>();
                if !overlap.is_empty() {
                    self.push(self.at(
                        "work.active-scope-conflict",
                        format!(
                            "active scope conflict with {} on {}",
                            right.id,
                            overlap.join(", ")
                        ),
                        left_path,
                    ));
                }
            }
        }
    }

    fn validate_orphan_work_dirs(&mut self) {
        let mut candidates = Vec::new();
        for child in &self.index.work_directories {
            let name = child
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            if name.starts_with('.') || name.starts_with('_') {
                continue;
            }
            if STATUSES.contains(&name) {
                candidates.push(child.clone());
            } else {
                candidates.push(child.clone());
            }
        }
        for directory in candidates {
            if self
                .index
                .work_items
                .iter()
                .any(|record| record.path.parent() == Some(directory.as_path()))
            {
                continue;
            }
            let has_planning = self.index.text(&directory.join("README.md")).is_some()
                || self.index.text(&directory.join("AGENTS.md")).is_some()
                || self.index.text_files.keys().any(|path| {
                    path.parent()
                        .is_some_and(|parent| parent == directory.join("_audit"))
                });
            if has_planning {
                self.push(self.at(
                    "work.orphan-directory",
                    "work directory holds planning content (README/AGENTS/_audit) but no work-item.json; it is invisible to the ledger, validator, and dashboard (record it with `repopact new work-item` or `repopact import-plan`, or move it out of work/)",
                    &directory,
                ));
            }
        }
    }

    fn validate_evidence(&mut self) {
        let mut seen = HashMap::new();
        for record in self.index.evidence.clone() {
            let data = match record.value {
                Ok(value) => value,
                Err(error) => {
                    self.push(self.at("evidence.json-invalid", error, &record.path));
                    continue;
                }
            };
            let Some(object) = data.as_object() else {
                self.push(self.at(
                    "evidence.json-invalid",
                    "expected a JSON object",
                    &record.path,
                ));
                continue;
            };
            let required = [
                "id",
                "timestamp",
                "work_item",
                "result",
                "commands",
                "artifacts",
                "environment",
            ];
            let missing = required
                .iter()
                .filter(|field| !object.contains_key(**field))
                .copied()
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                self.push(self.at(
                    "evidence.fields-missing",
                    format!("missing evidence fields: {}", missing.join(", ")),
                    &record.path,
                ));
                continue;
            }
            self.extend_schema(&data, "evidence-run.schema.json", &record.path);
            let id = object
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            if id
                != record
                    .path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("")
            {
                self.push(self.at(
                    "evidence.id-filename-mismatch",
                    "evidence id must match filename",
                    &record.path,
                ));
            }
            if let Some(previous) = seen.insert(id.clone(), record.path.clone()) {
                self.push(self.at(
                    "evidence.duplicate-id",
                    format!("duplicate evidence id also used by {}", self.rel(&previous)),
                    &record.path,
                ));
            }
            let timestamp = object
                .get("timestamp")
                .and_then(Value::as_str)
                .unwrap_or("");
            let parsed = parse_timestamp(timestamp);
            if parsed.is_none() {
                self.push(self.at(
                    "evidence.timestamp-invalid",
                    "timestamp must be ISO 8601",
                    &record.path,
                ));
            }
            if let Some(basis) = object.get("timestamp_basis").and_then(Value::as_str) {
                if basis != "git-recording" {
                    self.push(self.at(
                        "evidence.timestamp-basis-invalid",
                        "timestamp_basis must be 'git-recording' when present",
                        &record.path,
                    ));
                } else if let Some(timestamp) = parsed {
                    if let Some((commit_time, sha)) = self
                        .topology
                        .recording_commit(&self.repository.relative_path(&record.path))
                    {
                        if timestamp > commit_time + 300 {
                            self.push(self.at(
                                "evidence.timestamp-after-recording",
                                format!("timestamp {timestamp} is later than its recording commit {sha} by more than 5 minutes; evidence timestamps must describe execution no later than the recording commit plus the allowed clock-skew tolerance"),
                                &record.path,
                            ));
                        }
                    }
                }
            }
            let work_item = object
                .get("work_item")
                .and_then(Value::as_str)
                .unwrap_or("");
            if !self.work_ids.contains(work_item) {
                self.push(self.at(
                    "evidence.unknown-work-item",
                    format!("unknown work_item '{work_item}'"),
                    &record.path,
                ));
            }
        }
    }

    fn validate_audit_registry(&mut self) {
        let path = self.repository.root().join("audits/registry.json");
        let Some(record) = self.index.audit_registry.as_ref() else {
            self.push(self.at(
                "audit-registry.unreadable",
                "audits/registry.json is not valid JSON",
                &path,
            ));
            return;
        };
        let Ok(value) = record.value.clone() else {
            self.push(self.at(
                "audit-registry.unreadable",
                "audits/registry.json is not valid JSON",
                &path,
            ));
            return;
        };
        let today = today_utc();
        for entry in object_array(&value, "scopes") {
            let scope_path = entry.get("path").and_then(Value::as_str).unwrap_or("");
            let target = if scope_path == "." {
                self.repository.root().to_path_buf()
            } else {
                self.repository.root().join(scope_path)
            };
            if !target.exists() {
                self.push(self.at(
                    "audit-registry.scope-missing",
                    format!("audit scope does not exist: {scope_path}"),
                    &path,
                ));
            }
            let last = entry
                .get("last_reviewed")
                .and_then(Value::as_str)
                .unwrap_or("");
            let next = entry
                .get("next_review")
                .and_then(Value::as_str)
                .unwrap_or("");
            if !is_iso_date(last) {
                self.push(self.at(
                    "audit-registry.date-invalid",
                    "last_reviewed must be an ISO date",
                    &path,
                ));
            }
            if !is_iso_date(next) {
                self.push(self.at(
                    "audit-registry.date-invalid",
                    "next_review must be an ISO date",
                    &path,
                ));
            }
            if is_iso_date(last) && is_iso_date(next) {
                if next < last {
                    self.push(self.at(
                        "audit-registry.date-order",
                        format!(
                            "audit scope '{scope_path}' review deadline precedes its last review"
                        ),
                        &path,
                    ));
                }
                if next < today.as_str() {
                    self.push(self.at("audit-registry.freshness-expired", format!("audit scope '{scope_path}' freshness expired on {next}; re-review the scope and advance the registry dates"), &path));
                }
            }
        }
    }

    fn validate_dashboard(&mut self) {
        let path = self.repository.root().join("audits/reports/dashboard.md");
        let Some(actual) = self.index.text(&path) else {
            self.push(self.at(
                "dashboard.missing",
                "missing generated dashboard; run `repopact dashboard --root .`",
                &path,
            ));
            return;
        };
        let Some(expected) = self.generate_dashboard() else {
            return;
        };
        if actual.replace("\r\n", "\n") != expected {
            self.push(self.at("dashboard.stale", "generated dashboard is stale; run `repopact dashboard --root .` and commit the result", &path));
        }
    }

    fn generate_dashboard(&self) -> Option<String> {
        let registry = self.index.audit_registry.as_ref()?.value.as_ref().ok()?;
        let invariants = self.index.invariants.as_ref()?.value.as_ref().ok()?;
        let frozen = self.index.frozen_surface.as_ref()?.value.as_ref().ok()?;
        let mut counts = HashMap::new();
        for record in &self.work {
            let item = record.item.as_ref()?;
            *counts.entry(item.status.as_str()).or_insert(0usize) += 1;
        }
        let contracts = self.index.contracts.len();
        let evidence_count = self.index.evidence.len();
        let audit_entries = object_array(&registry, "scopes").count();
        let invariant_count = object_array(&invariants, "invariants").count();
        let frozen_count = object_array(&frozen, "protected").count();
        let decision_count = self.index.decisions.len();
        let policy_count = self.index.policies.len();
        let finding_count = self.index.audit_findings.len();
        let spec_version = [
            self.repository.root().join("scripts/REPOPACT_VERSION"),
            self.repository.root().join("VERSION"),
        ]
        .into_iter()
        .find_map(|path| self.index.text(&path).map(str::to_owned))
        .map(|value| value.trim().to_owned())
        .unwrap_or_else(|| "unknown".to_owned());
        let today = today_utc();
        let overdue = object_array(&registry, "scopes")
            .filter_map(|entry| {
                let due = entry.get("next_review").and_then(Value::as_str)?;
                (is_iso_date(due) && due < today.as_str())
                    .then_some((entry.get("path").and_then(Value::as_str).unwrap_or(""), due))
            })
            .collect::<Vec<_>>();
        let mut lines = vec![
            "# Repository Dashboard".to_owned(),
            String::new(),
            "> Canonically generated from source records. Do not edit manually.".to_owned(),
            "> Validation fails when this file differs from `repopact dashboard` output."
                .to_owned(),
            format!("> RepoPact spec version: {spec_version}"),
            String::new(),
            "## Health".to_owned(),
            String::new(),
            "| Metric | Count |".to_owned(),
            "| --- | ---: |".to_owned(),
            format!("| Invariants | {invariant_count} |"),
            format!("| Frozen-surface entries | {frozen_count} |"),
            format!("| Scope contracts | {contracts} |"),
            format!("| Audit registry entries | {audit_entries} |"),
            format!("| Audit findings | {finding_count} |"),
            format!("| Decision records | {decision_count} |"),
            format!("| Policy records | {policy_count} |"),
            format!("| Evidence runs | {evidence_count} |"),
            String::new(),
            "## Work".to_owned(),
            String::new(),
            "| Status | Count |".to_owned(),
            "| --- | ---: |".to_owned(),
        ];
        for status in STATUSES {
            lines.push(format!(
                "| {status} | {} |",
                counts.get(status).copied().unwrap_or(0)
            ));
        }
        lines.extend([
            String::new(),
            "## Audit freshness".to_owned(),
            String::new(),
        ]);
        if overdue.is_empty() {
            lines.push("All audit scopes are within their review cadence.".to_owned());
        } else {
            lines.push("| Scope | Review was due |".to_owned());
            lines.push("| --- | --- |".to_owned());
            lines.extend(
                overdue
                    .into_iter()
                    .map(|(scope, due)| format!("| {scope} | {due} |")),
            );
        }
        lines.extend([String::new(), "## Active items".to_owned(), String::new()]);
        let active = self
            .work
            .iter()
            .filter_map(|record| {
                let item = record.item.as_ref()?;
                matches!(item.status.as_str(), "active" | "blocked").then_some(item)
            })
            .collect::<Vec<_>>();
        if active.is_empty() {
            lines.push("No active or blocked work.".to_owned());
        } else {
            lines.extend(
                active
                    .into_iter()
                    .map(|item| format!("- {}: {} ({})", item.id, item.title, item.status)),
            );
        }
        Some(lines.join("\n") + "\n")
    }

    fn extend_schema(&mut self, value: &Value, schema_name: &str, path: &Path) {
        let root = self.repository.root().to_path_buf();
        self.diagnostics.extend(
            self.schemas
                .validate(value, schema_name, path, |candidate| {
                    match repopact_repository::normalize_path(candidate).strip_prefix(&root) {
                        Ok(relative) if relative.as_os_str().is_empty() => "<root>".to_owned(),
                        Ok(relative) => path_string(relative),
                        Err(_) => path_string(candidate),
                    }
                }),
        );
    }

    fn at(&self, code: impl Into<String>, message: impl Into<String>, path: &Path) -> Diagnostic {
        Diagnostic::error(code, message).with_path(self.rel(path))
    }

    fn rel(&self, path: &Path) -> String {
        self.repository.relative_path(path)
    }

    fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }
}

fn loaded_work(index: &RecordIndex) -> Vec<LoadedWork> {
    index
        .work_items
        .iter()
        .map(|record| LoadedWork {
            path: record.path.clone(),
            item: record
                .value
                .clone()
                .ok()
                .and_then(|value| serde_json::from_value::<WorkItem>(value).ok()),
        })
        .collect()
}

fn record_directory(record: &IndexedRecord) -> &Path {
    record.path.parent().unwrap_or(&record.path)
}

fn object_array<'a>(value: &'a Value, key: &str) -> impl Iterator<Item = &'a Map<String, Value>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
}

fn map_string_array<'a>(value: &'a Map<String, Value>, key: &str) -> impl Iterator<Item = &'a str> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
}

fn wildcard_match(value: &str, pattern: &str) -> bool {
    let value = value.as_bytes();
    let pattern = pattern.as_bytes();
    let mut previous = vec![false; value.len() + 1];
    previous[0] = true;
    for character in pattern {
        let mut current = vec![false; value.len() + 1];
        match character {
            b'*' => {
                current[0] = previous[0];
                for index in 1..=value.len() {
                    current[index] = previous[index] || current[index - 1];
                }
            }
            b'?' => {
                for index in 1..=value.len() {
                    current[index] = previous[index - 1];
                }
            }
            literal => {
                for index in 1..=value.len() {
                    current[index] = previous[index - 1] && value[index - 1] == *literal;
                }
            }
        }
        previous = current;
    }
    previous[value.len()]
}

fn is_digits_at_least(value: &str, length: usize) -> bool {
    value.len() >= length && value.chars().all(|char| char.is_ascii_digit())
}

fn is_semver_core(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() == 3 && parts.iter().all(|part| is_digits_at_least(part, 1))
}

fn parse_release_label(value: &str) -> Option<(String, String)> {
    let (base, suffix) = value.split_once('-')?;
    let (prerelease, build) = suffix
        .split_once('+')
        .map_or((suffix, None), |(pre, build)| (pre, Some(build)));
    if !is_semver_core(base) || prerelease.is_empty() || build.is_some_and(str::is_empty) {
        return None;
    }
    for segment in prerelease.split('.') {
        if segment.is_empty()
            || !segment
                .chars()
                .all(|char| char.is_ascii_alphanumeric() || char == '-')
            || (segment.chars().all(|char| char.is_ascii_digit())
                && segment.starts_with('0')
                && segment.len() > 1)
        {
            return None;
        }
    }
    if let Some(build) = build {
        for segment in build.split('.') {
            if segment.is_empty()
                || !segment
                    .chars()
                    .all(|char| char.is_ascii_alphanumeric() || char == '-')
            {
                return None;
            }
        }
    }
    Some((base.to_owned(), prerelease.to_owned()))
}

/// Decision 0014's canonical checklist criterion-identifier grammar:
/// `[A-Za-z][A-Za-z0-9]*-[0-9]+` with a word boundary after the numeric run.
/// Mirrors Python's `CHECKBOX_LINE` regex group so both implementations
/// recognize `AC-7` inside `**AC-7 (waived by decision 0029)**` but reject
/// `AC-7foo`, instead of requiring the whole bold label to be alphanumeric.
fn decision_0014_criterion_id(label: &str) -> Option<&str> {
    let mut chars = label.char_indices().peekable();
    let (_, first) = chars.next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    let mut end = first.len_utf8();
    let mut saw_dash = false;
    while let Some(&(pos, ch)) = chars.peek() {
        if ch == '-' {
            end = pos + ch.len_utf8();
            saw_dash = true;
            chars.next();
            break;
        } else if ch.is_ascii_alphanumeric() {
            end = pos + ch.len_utf8();
            chars.next();
        } else {
            return None;
        }
    }
    if !saw_dash {
        return None;
    }
    let digits_start = end;
    while let Some(&(pos, ch)) = chars.peek() {
        if ch.is_ascii_digit() {
            end = pos + ch.len_utf8();
            chars.next();
        } else {
            break;
        }
    }
    if end == digits_start {
        return None;
    }
    if let Some(&(_, next)) = chars.peek() {
        if next.is_ascii_alphanumeric() || next == '_' {
            return None;
        }
    }
    Some(&label[..end])
}

fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
        && value[5..7]
            .parse::<u32>()
            .is_ok_and(|month| (1..=12).contains(&month))
        && value[8..10]
            .parse::<u32>()
            .is_ok_and(|day| (1..=31).contains(&day))
}

fn parse_timestamp(value: &str) -> Option<i64> {
    let (date, time) = value.split_once('T')?;
    if !is_iso_date(date) {
        return None;
    }
    let year = date[..4].parse::<i64>().ok()?;
    let month = date[5..7].parse::<i64>().ok()?;
    let day = date[8..10].parse::<i64>().ok()?;
    let (clock, offset) = if let Some(clock) = time.strip_suffix('Z') {
        (clock, 0)
    } else {
        let position = time.rfind(|character| character == '+' || character == '-')?;
        let (clock, raw) = time.split_at(position);
        let sign = if raw.starts_with('-') { -1 } else { 1 };
        let hours = raw[1..3].parse::<i64>().ok()?;
        let minutes = raw[4..6].parse::<i64>().ok()?;
        (clock, sign * (hours * 3600 + minutes * 60))
    };
    let clock = clock.split('.').next().unwrap_or(clock);
    let parts = clock.split(':').collect::<Vec<_>>();
    if parts.len() != 3 {
        return None;
    }
    let hour = parts[0].parse::<i64>().ok()?;
    let minute = parts[1].parse::<i64>().ok()?;
    let second = parts[2].parse::<i64>().ok()?;
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second - offset)
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = (if year >= 0 { year } else { year - 399 }) / 400;
    let year_of_era = year - era * 400;
    let month_adjusted = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_adjusted + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146097 + day_of_era - 719468
}

fn today_utc() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let days = seconds.div_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let day_of_era = z - era * 146097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    (year, month, day)
}

fn requires_preflight(item: &WorkItem, config: &Value) -> bool {
    if config.get("enabled").and_then(Value::as_bool) == Some(false) {
        return false;
    }
    let from_id = config.get("required_from_id").and_then(Value::as_i64);
    let from_date = config.get("required_from_date").and_then(Value::as_str);
    if from_id.is_none() && from_date.is_none() {
        return true;
    }
    if from_id.is_some_and(|id| {
        item.id
            .parse::<i64>()
            .ok()
            .is_some_and(|item_id| item_id >= id)
    }) {
        return true;
    }
    from_date.is_some_and(|date| item.created.as_str() > date)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("repopact-rust-validation-{name}-{suffix}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn release_labels_are_pinned_to_the_core_version() {
        assert_eq!(parse_release_label("3.0.2-rc.1").unwrap().0, "3.0.2");
        assert!(parse_release_label("3.0.2-01").is_none());
        assert!(parse_release_label("3.0.2").is_none());
    }

    #[test]
    fn timestamp_parser_normalizes_offsets() {
        assert_eq!(
            parse_timestamp("2026-01-01T02:00:00+02:00"),
            parse_timestamp("2026-01-01T00:00:00Z")
        );
    }

    #[test]
    fn validate_snapshot_reuses_supplied_git_facts_without_new_queries() {
        let root = temp_root("snapshot-query-free");
        fs::create_dir_all(root.join(".git")).unwrap();
        let runner = repopact_repository::CountingGitRunner::native();
        let repository = Repository::with_git_runner(&root, runner.clone());
        let snapshot = repository.session().snapshot();
        let before = runner.count();
        let _ = validate_snapshot(&snapshot);
        assert_eq!(before, runner.count());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dashboard_validation_is_read_only() {
        let root = temp_root("no-write");
        fs::create_dir_all(root.join("governance")).unwrap();
        fs::create_dir_all(root.join("audits/reports")).unwrap();
        fs::write(root.join("VERSION"), "0.1.0\n").unwrap();
        fs::write(root.join("AGENTS.md"), "# root\n").unwrap();
        fs::write(
            root.join("governance/invariants.json"),
            r#"{"version":1,"invariants":[]}"#,
        )
        .unwrap();
        fs::write(
            root.join("governance/frozen-surface.json"),
            r#"{"version":1,"protected":[]}"#,
        )
        .unwrap();
        fs::write(
            root.join("governance/owners.json"),
            r#"{"scopes":[],"roles":[]}"#,
        )
        .unwrap();
        fs::write(root.join("audits/registry.json"), r#"{"scopes":[]}"#).unwrap();
        let before = snapshot(&root);
        let _ = validate(&root);
        assert_eq!(before, snapshot(&root));
        fs::remove_dir_all(root).unwrap();
    }

    fn snapshot(root: &Path) -> Vec<(String, u64)> {
        let mut output = Vec::new();
        fn visit(root: &Path, current: &Path, output: &mut Vec<(String, u64)>) {
            let Ok(entries) = fs::read_dir(current) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.file_name().is_some_and(|name| name == "target") {
                    continue;
                }
                if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                    visit(root, &path, output);
                } else if let Ok(bytes) = fs::read(&path) {
                    let sum = bytes
                        .iter()
                        .fold(0u64, |sum, byte| sum.wrapping_add(u64::from(*byte)));
                    output.push((
                        path.strip_prefix(root)
                            .unwrap()
                            .to_string_lossy()
                            .into_owned(),
                        sum,
                    ));
                }
            }
        }
        visit(root, root, &mut output);
        output.sort();
        output
    }

    #[test]
    fn decision_0014_id_recognizes_plain_canonical_id() {
        assert_eq!(decision_0014_criterion_id("AC-7"), Some("AC-7"));
    }

    #[test]
    fn decision_0014_id_recognizes_explanatory_suffix() {
        assert_eq!(
            decision_0014_criterion_id("AC-7 (waived by decision 0029)"),
            Some("AC-7")
        );
        assert_eq!(
            decision_0014_criterion_id("AC-7 (waived by decision `0029`)"),
            Some("AC-7")
        );
    }

    #[test]
    fn decision_0014_id_recognizes_wi036_exact_shape() {
        assert_eq!(
            decision_0014_criterion_id("AC-7 (waived by decision `0029`)"),
            Some("AC-7")
        );
    }

    #[test]
    fn decision_0014_id_rejects_malformed_attached_suffix() {
        assert_eq!(decision_0014_criterion_id("AC-7foo"), None);
        assert_eq!(decision_0014_criterion_id("AC-70abc"), None);
    }

    #[test]
    fn decision_0014_id_rejects_prose_without_convention() {
        assert_eq!(decision_0014_criterion_id("Acceptance criteria"), None);
        assert_eq!(decision_0014_criterion_id("just some prose"), None);
        assert_eq!(decision_0014_criterion_id("-7"), None);
    }

    #[test]
    fn decision_0014_id_accepts_multi_letter_prefix() {
        assert_eq!(decision_0014_criterion_id("CVP-017"), Some("CVP-017"));
        assert_eq!(decision_0014_criterion_id("UIT-001 done"), Some("UIT-001"));
    }

    fn work_item_with_readme(readme_body: &str, criteria: Vec<(&str, &str)>) -> PathBuf {
        let root = temp_root("checkbox");
        let directory = root.join("work/active/001-example");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("README.md"), readme_body).unwrap();
        let acceptance_criteria: Vec<Value> = criteria
            .iter()
            .map(|(id, state)| serde_json::json!({"id": id, "text": "criterion", "state": state, "evidence": []}))
            .collect();
        fs::write(
            directory.join("work-item.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "id": "001",
                "title": "Example",
                "status": "active",
                "owner_scope": "tooling",
                "affected_scopes": [],
                "depends_on": [],
                "acceptance_criteria": acceptance_criteria,
                "created": "2026-01-01",
                "updated": "2026-01-01",
            }))
            .unwrap(),
        )
        .unwrap();
        root
    }

    fn checkbox_codes(root: &Path) -> Vec<String> {
        let report = validate(root);
        let codes = report
            .diagnostics
            .into_iter()
            .filter(|diagnostic| diagnostic.code.starts_with("work.readme-checkbox"))
            .map(|diagnostic| diagnostic.code)
            .collect();
        fs::remove_dir_all(root).unwrap();
        codes
    }

    #[test]
    fn checkbox_checked_satisfied_matches() {
        let root = work_item_with_readme(
            "- [x] **AC-1** done\n",
            vec![("AC-1", "satisfied")],
        );
        assert!(checkbox_codes(&root).is_empty());
    }

    #[test]
    fn checkbox_unchecked_satisfied_mismatches() {
        let root = work_item_with_readme(
            "- [ ] **AC-1** done\n",
            vec![("AC-1", "satisfied")],
        );
        assert_eq!(checkbox_codes(&root), vec!["work.readme-checkbox-mismatch"]);
    }

    #[test]
    fn checkbox_checked_pending_mismatches() {
        let root = work_item_with_readme(
            "- [x] **AC-1** done\n",
            vec![("AC-1", "pending")],
        );
        assert_eq!(checkbox_codes(&root), vec!["work.readme-checkbox-mismatch"]);
    }

    #[test]
    fn checkbox_unchecked_pending_matches() {
        let root = work_item_with_readme(
            "- [ ] **AC-1** done\n",
            vec![("AC-1", "pending")],
        );
        assert!(checkbox_codes(&root).is_empty());
    }

    #[test]
    fn checkbox_waived_matches_either_state() {
        let root = work_item_with_readme(
            "- [x] **AC-1** done\n",
            vec![("AC-1", "waived")],
        );
        assert!(checkbox_codes(&root).is_empty());
        let root = work_item_with_readme(
            "- [ ] **AC-1** done\n",
            vec![("AC-1", "waived")],
        );
        assert!(checkbox_codes(&root).is_empty());
    }

    #[test]
    fn checkbox_explanatory_suffix_still_activates_convention() {
        let root = work_item_with_readme(
            "- [x] **AC-7 (waived by decision `0029`)** superseded\n",
            vec![("AC-7", "waived")],
        );
        assert!(checkbox_codes(&root).is_empty());
    }

    #[test]
    fn checkbox_prose_readme_does_not_activate_convention() {
        let root = work_item_with_readme(
            "This work item describes AC-1 in prose, with no checklist.\n",
            vec![("AC-1", "satisfied")],
        );
        assert!(checkbox_codes(&root).is_empty());
    }
}
