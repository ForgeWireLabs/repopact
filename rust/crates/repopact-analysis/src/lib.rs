use repopact_graph::RepositoryGraph;
use repopact_repository::{RecordIndex, RepositorySnapshot};
use repopact_types::{RecordKind, RecordRef, SourceRef, WorkItem};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingClassification {
    Fact,
    Constraint,
    Suggestion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisKind {
    NextWorkId,
    Scope,
    Dependency,
    Evidence,
    Finding,
    Contract,
    FrozenSurface,
    Provenance,
    RelatedWork,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisFinding {
    pub kind: AnalysisKind,
    pub classification: FindingClassification,
    pub code: String,
    pub message: String,
    pub basis: Vec<SourceRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_records: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub findings: Vec<AnalysisFinding>,
}

impl AnalysisReport {
    pub fn constraints(&self) -> impl Iterator<Item = &AnalysisFinding> {
        self.findings
            .iter()
            .filter(|finding| finding.classification == FindingClassification::Constraint)
    }

    pub fn facts(&self) -> impl Iterator<Item = &AnalysisFinding> {
        self.findings
            .iter()
            .filter(|finding| finding.classification == FindingClassification::Fact)
    }

    pub fn suggestions(&self) -> impl Iterator<Item = &AnalysisFinding> {
        self.findings
            .iter()
            .filter(|finding| finding.classification == FindingClassification::Suggestion)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisQuery {
    pub candidate_id: Option<String>,
    pub title: Option<String>,
    pub owner_scope: Option<String>,
    #[serde(default)]
    pub affected_scopes: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub status: Option<String>,
    pub provenance: Option<String>,
    #[serde(default)]
    pub candidate_paths: Vec<String>,
}

impl AnalysisQuery {
    pub fn for_work_item(id: impl Into<String>) -> Self {
        Self {
            candidate_id: Some(id.into()),
            ..Self::default()
        }
    }

    pub fn with_scopes(
        mut self,
        owner_scope: impl Into<String>,
        affected_scopes: Vec<String>,
    ) -> Self {
        self.owner_scope = Some(owner_scope.into());
        self.affected_scopes = affected_scopes;
        self
    }
}

pub struct Analyzer<'a> {
    snapshot: &'a RepositorySnapshot,
    graph: RepositoryGraph,
}

impl<'a> Analyzer<'a> {
    pub fn new(snapshot: &'a RepositorySnapshot) -> Self {
        Self {
            snapshot,
            graph: RepositoryGraph::build(snapshot),
        }
    }

    pub fn analyze(&self, query: &AnalysisQuery) -> AnalysisReport {
        let mut report = AnalysisReport::default();
        self.next_work_id(&mut report);
        self.scopes(query, &mut report);
        self.dependencies(query, &mut report);
        self.evidence(query, &mut report);
        self.findings(query, &mut report);
        self.contracts(query, &mut report);
        self.frozen(query, &mut report);
        self.provenance(query, &mut report);
        self.related_work(query, &mut report);
        report.findings.sort_by(|left, right| {
            (left.kind, left.classification, &left.code, &left.message).cmp(&(
                right.kind,
                right.classification,
                &right.code,
                &right.message,
            ))
        });
        report
    }

    pub fn graph(&self) -> &RepositoryGraph {
        &self.graph
    }

    fn next_work_id(&self, report: &mut AnalysisReport) {
        let used = self
            .snapshot
            .index()
            .work_items
            .iter()
            .filter_map(|record| record.reference.id.parse::<u64>().ok())
            .collect::<BTreeSet<_>>();
        let next = (1..)
            .find(|candidate| !used.contains(candidate))
            .unwrap_or(1);
        let mut basis: Vec<RecordRef> = self
            .snapshot
            .index()
            .work_items
            .iter()
            .map(|record| record.reference.clone())
            .collect();
        if basis.is_empty() {
            basis.push(RecordRef::new(
                RecordKind::Repository,
                "repository",
                "<root>",
            ));
        }
        report.findings.push(fact(
            AnalysisKind::NextWorkId,
            "next-work-id",
            format!("next available work-item id is {next:03}"),
            basis,
        ));
    }

    fn scopes(&self, query: &AnalysisQuery, report: &mut AnalysisReport) {
        let Some(owner) = query.owner_scope.as_deref() else {
            return;
        };
        let (scopes, owners_source) = owner_scopes(self.snapshot.index());
        let candidate_scopes = std::iter::once(owner)
            .chain(query.affected_scopes.iter().map(String::as_str))
            .collect::<Vec<_>>();
        for scope in &candidate_scopes {
            if !scopes.contains(*scope) {
                report.findings.push(constraint(
                    AnalysisKind::Scope,
                    "scope.unknown",
                    format!("candidate references unknown owner/affected scope '{scope}'"),
                    owners_source.clone().into_iter().collect(),
                    Some("select a scope declared in governance/owners.json".to_owned()),
                ));
            }
        }
        let active = self
            .snapshot
            .index()
            .work_items
            .iter()
            .filter_map(|record| {
                Some((
                    record.reference.clone(),
                    serde_json::from_value::<WorkItem>(record.value.clone().ok()?).ok()?,
                ))
            })
            .filter(|(_, item)| matches!(item.status.as_str(), "proposed" | "active" | "blocked"))
            .collect::<Vec<_>>();
        for (reference, item) in active {
            let mut overlap = candidate_scopes.iter().any(|scope| {
                *scope == item.owner_scope
                    || item.affected_scopes.iter().any(|other| other == *scope)
            });
            overlap |= query
                .affected_scopes
                .iter()
                .any(|scope| item.affected_scopes.iter().any(|other| other == scope));
            if overlap {
                report.findings.push(constraint(
                    AnalysisKind::Scope,
                    "scope.overlap",
                    format!(
                        "candidate overlaps non-terminal work item '{}' in declared scopes",
                        item.id
                    ),
                    vec![reference],
                    Some("review the affected scope boundary before activation".to_owned()),
                ));
            }
        }
    }

    fn dependencies(&self, query: &AnalysisQuery, report: &mut AnalysisReport) {
        let candidate_id = query.candidate_id.as_deref().unwrap_or("candidate");
        let all = self
            .snapshot
            .index()
            .work_items
            .iter()
            .filter_map(|record| {
                Some((
                    record.reference.clone(),
                    serde_json::from_value::<WorkItem>(record.value.clone().ok()?).ok()?,
                ))
            })
            .collect::<Vec<_>>();
        let by_id = all
            .iter()
            .map(|(_, item)| (item.id.clone(), item))
            .collect::<BTreeMap<_, _>>();
        for dependency in &query.depends_on {
            if let Some((reference, item)) = all.iter().find(|(_, item)| item.id == *dependency) {
                report.findings.push(fact(
                    AnalysisKind::Dependency,
                    "dependency.candidate",
                    format!("candidate depends on work item '{}'", dependency),
                    vec![reference.clone()],
                ));
                if item.status == "proposed"
                    && matches!(query.status.as_deref(), Some("active" | "blocked"))
                {
                    report.findings.push(constraint(
                        AnalysisKind::Dependency,
                        "dependency.lifecycle",
                        format!("active candidate depends on proposed work item '{dependency}'"),
                        vec![reference.clone()],
                        Some("activate the prerequisite or keep the candidate proposed".to_owned()),
                    ));
                }
            } else {
                report.findings.push(constraint(
                    AnalysisKind::Dependency,
                    "dependency.unknown",
                    format!("candidate depends on unknown work item '{dependency}'"),
                    vec![candidate_reference(candidate_id)],
                    Some("use an existing work-item id".to_owned()),
                ));
            }
        }
        for dependency in self.graph.dependencies(candidate_id) {
            report.findings.push(fact(
                AnalysisKind::Dependency,
                "dependency.forward",
                format!(
                    "dependency edge {} -> {} is source-backed",
                    dependency.from, dependency.to
                ),
                vec![dependency.source],
            ));
        }
        for (reference, item) in &all {
            for dependency in &item.depends_on {
                if dependency == candidate_id {
                    report.findings.push(fact(
                        AnalysisKind::Dependency,
                        "dependency.reverse",
                        format!("work item '{}' depends on candidate", item.id),
                        vec![reference.clone()],
                    ));
                }
            }
        }
        let mut adjacency = BTreeMap::<String, Vec<String>>::new();
        for (_, item) in &all {
            adjacency.insert(item.id.clone(), item.depends_on.clone());
        }
        adjacency.insert(candidate_id.to_owned(), query.depends_on.clone());
        if query
            .depends_on
            .iter()
            .any(|dependency| reaches(&adjacency, dependency, candidate_id, &mut HashSet::new()))
        {
            let mut basis: Vec<RecordRef> = query
                .depends_on
                .iter()
                .filter_map(|id| {
                    all.iter()
                        .find(|(_, item)| item.id == *id)
                        .map(|(reference, _)| reference.clone())
                })
                .collect();
            if basis.is_empty() {
                basis.push(candidate_reference(candidate_id));
            }
            report.findings.push(constraint(
                AnalysisKind::Dependency,
                "dependency.cycle",
                "candidate dependency would create a dependency cycle".to_owned(),
                basis,
                Some("remove the candidate edge or choose a non-dependent item".to_owned()),
            ));
        }
        let _ = by_id;
    }

    fn evidence(&self, query: &AnalysisQuery, report: &mut AnalysisReport) {
        for record in &self.snapshot.index().work_items {
            let Some(item) = record
                .value
                .clone()
                .ok()
                .and_then(|value| serde_json::from_value::<WorkItem>(value).ok())
            else {
                continue;
            };
            if query
                .candidate_id
                .as_deref()
                .is_some_and(|id| id != item.id)
            {
                continue;
            }
            for criterion in &item.acceptance_criteria {
                if criterion.state == "pending" {
                    report.findings.push(constraint(
                        AnalysisKind::Evidence,
                        "criteria.pending",
                        format!("acceptance criterion '{}' is pending", criterion.id),
                        vec![record.reference.clone()],
                        Some("satisfy or explicitly waive the criterion with evidence".to_owned()),
                    ));
                } else if criterion.state == "satisfied" && criterion.evidence.is_empty() {
                    report.findings.push(constraint(
                        AnalysisKind::Evidence,
                        "criteria.evidence-missing",
                        format!("satisfied criterion '{}' has no evidence", criterion.id),
                        vec![record.reference.clone()],
                        Some("link a concrete evidence run".to_owned()),
                    ));
                }
            }
        }
    }

    fn findings(&self, query: &AnalysisQuery, report: &mut AnalysisReport) {
        let requested = std::iter::once(query.owner_scope.as_deref())
            .flatten()
            .chain(query.affected_scopes.iter().map(String::as_str))
            .collect::<BTreeSet<_>>();
        for finding in &self.snapshot.index().audit_findings {
            let Some(value) = finding.value.as_ref().ok() else {
                continue;
            };
            let state = value.get("state").and_then(Value::as_str).unwrap_or("");
            if matches!(state, "resolved" | "closed") {
                continue;
            }
            let scope = value
                .get("scope")
                .or_else(|| value.get("path"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if requested.is_empty()
                || requested.iter().any(|candidate| {
                    scope == *candidate || scope.starts_with(&format!("{candidate}/"))
                })
            {
                report.findings.push(constraint(
                    AnalysisKind::Finding,
                    "finding.unresolved",
                    format!(
                        "unresolved audit finding '{}' is relevant to the candidate scope",
                        finding.reference.id
                    ),
                    vec![finding.reference.clone()],
                    Some(
                        "resolve or explicitly account for the finding before completion"
                            .to_owned(),
                    ),
                ));
            }
        }
    }

    fn contracts(&self, query: &AnalysisQuery, report: &mut AnalysisReport) {
        for candidate in &query.candidate_paths {
            let path = self.snapshot.repository().root().join(candidate);
            let matching = self
                .snapshot
                .index()
                .contracts
                .iter()
                .filter(|contract| {
                    contract
                        .path
                        .parent()
                        .is_some_and(|parent| path.strip_prefix(parent).is_ok())
                })
                .collect::<Vec<_>>();
            for contract in matching {
                report.findings.push(fact(
                    AnalysisKind::Contract,
                    "contract.applicable",
                    format!(
                        "contract '{}' applies to candidate path '{candidate}'",
                        contract.reference.path
                    ),
                    vec![contract.reference.clone()],
                ));
            }
        }
    }

    fn frozen(&self, query: &AnalysisQuery, report: &mut AnalysisReport) {
        let Some(frozen) = &self.snapshot.index().frozen_surface else {
            return;
        };
        let Some(value) = frozen.value.as_ref().ok() else {
            return;
        };
        for candidate in &query.candidate_paths {
            for entry in value
                .get("protected")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let glob = entry.get("glob").and_then(Value::as_str).unwrap_or("");
                if wildcard_match(candidate, glob) {
                    let reason = entry
                        .get("reason")
                        .and_then(Value::as_str)
                        .unwrap_or("protected path");
                    report.findings.push(constraint(AnalysisKind::FrozenSurface, "frozen.intersection", format!("candidate path '{candidate}' intersects frozen surface '{glob}': {reason}"), vec![frozen.reference.clone()], Some("obtain governed operator approval before apply".to_owned())));
                }
            }
        }
    }

    fn provenance(&self, query: &AnalysisQuery, report: &mut AnalysisReport) {
        if query.status.as_deref() == Some("completed")
            && query.provenance.as_deref().unwrap_or("concrete") != "concrete"
        {
            report.findings.push(constraint(
                AnalysisKind::Provenance,
                "provenance.completed",
                "completed work must use concrete provenance".to_owned(),
                vec![candidate_reference(
                    query.candidate_id.as_deref().unwrap_or("candidate"),
                )],
                Some("ratchet provenance before completing the work item".to_owned()),
            ));
        }
        if query.status.as_deref() == Some("completed") {
            report.findings.push(fact(
                AnalysisKind::Provenance,
                "provenance.completion",
                "completion requires post-state validation and complete evidence".to_owned(),
                vec![candidate_reference(
                    query.candidate_id.as_deref().unwrap_or("candidate"),
                )],
            ));
        }
    }

    fn related_work(&self, query: &AnalysisQuery, report: &mut AnalysisReport) {
        let requested = query
            .affected_scopes
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        for record in &self.snapshot.index().work_items {
            let Some(item) = record
                .value
                .clone()
                .ok()
                .and_then(|value| serde_json::from_value::<WorkItem>(value).ok())
            else {
                continue;
            };
            let overlap = requested.contains(item.owner_scope.as_str())
                || item
                    .affected_scopes
                    .iter()
                    .any(|scope| requested.contains(scope.as_str()));
            if overlap {
                report.findings.push(AnalysisFinding {
                    kind: AnalysisKind::RelatedWork,
                    classification: FindingClassification::Suggestion,
                    code: "related.explicit-scope".to_owned(),
                    message: format!(
                        "work item '{}' is structurally related through an explicit scope",
                        item.id
                    ),
                    basis: vec![record.reference.clone()],
                    related_records: vec![item.id],
                    remediation: None,
                });
            }
        }
    }
}

pub fn analyze(snapshot: &RepositorySnapshot, query: &AnalysisQuery) -> AnalysisReport {
    Analyzer::new(snapshot).analyze(query)
}

fn owner_scopes(index: &RecordIndex) -> (BTreeSet<String>, Option<RecordRef>) {
    let Some(owners) = &index.owners else {
        return (BTreeSet::new(), None);
    };
    let scopes = owners
        .value
        .as_ref()
        .ok()
        .and_then(|value| value.get("scopes"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("id").and_then(Value::as_str).map(str::to_owned))
        .collect();
    (scopes, Some(owners.reference.clone()))
}

fn reaches(
    adjacency: &BTreeMap<String, Vec<String>>,
    current: &str,
    target: &str,
    visited: &mut HashSet<String>,
) -> bool {
    if current == target {
        return true;
    }
    if !visited.insert(current.to_owned()) {
        return false;
    }
    adjacency
        .get(current)
        .into_iter()
        .flatten()
        .any(|next| reaches(adjacency, next, target, visited))
}

fn fact(
    kind: AnalysisKind,
    code: impl Into<String>,
    message: impl Into<String>,
    basis: Vec<SourceRef>,
) -> AnalysisFinding {
    AnalysisFinding {
        kind,
        classification: FindingClassification::Fact,
        code: code.into(),
        message: message.into(),
        basis,
        related_records: Vec::new(),
        remediation: None,
    }
}

fn candidate_reference(id: &str) -> RecordRef {
    RecordRef::new(RecordKind::WorkItem, id, "<candidate>")
}

fn constraint(
    kind: AnalysisKind,
    code: impl Into<String>,
    message: impl Into<String>,
    basis: Vec<SourceRef>,
    remediation: Option<String>,
) -> AnalysisFinding {
    AnalysisFinding {
        kind,
        classification: FindingClassification::Constraint,
        code: code.into(),
        message: message.into(),
        basis,
        related_records: Vec::new(),
        remediation,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use repopact_repository::RepositorySession;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn findings_are_classified_and_source_backed() {
        let root = std::env::temp_dir().join(format!(
            "repopact-analysis-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("work/active/001-one")).unwrap();
        fs::write(root.join("work/active/001-one/work-item.json"), r#"{"id":"001","title":"One","status":"active","owner_scope":"work","affected_scopes":[],"depends_on":[],"acceptance_criteria":[{"id":"AC-1","text":"prove","state":"pending","evidence":[]}],"created":"2026-01-01","updated":"2026-01-01"}"#).unwrap();
        let snapshot = RepositorySession::open(&root).snapshot();
        let report = analyze(&snapshot, &AnalysisQuery::for_work_item("001"));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "criteria.pending" && !finding.basis.is_empty()));
        assert!(report.facts().any(|finding| finding.code == "next-work-id"));
        fs::remove_dir_all(root).unwrap();
    }
}
