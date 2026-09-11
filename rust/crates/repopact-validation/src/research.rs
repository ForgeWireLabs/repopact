//! WI059 CVP-005: local `research/metadata.json` validation semantics,
//! ported from `repopact.validate_research`.
//!
//! This module owns *local research-fact validity* only: whether the
//! metadata contract and its configured local document references are
//! internally consistent, using content already materialized into the
//! repository snapshot/index. Research execution, benchmark running, and
//! result interpretation are explicitly out of scope and nothing here
//! spawns a process or touches the network.

use std::collections::BTreeMap;
use std::path::Path;

use regex::Regex;
use repopact_repository::{RecordIndex, Repository};
use repopact_types::Diagnostic;
use serde_json::Value;

use crate::{days_from_civil, is_iso_date, is_semver_core, today_utc, Validator};

const STATUSES: [&str; 5] = ["proposed", "active", "blocked", "deferred", "completed"];

impl Validator {
    pub(crate) fn validate_research(&mut self) {
        let root = self.repository.root().to_path_buf();
        let metadata_path = root.join("research/metadata.json");
        let upstream_record = self.index.text(&root.join("research/paper.md")).is_some()
            && self.index.text(&root.join("research/protocol.md")).is_some();
        let record = self.index.research_metadata.clone();
        if record.is_none() && !upstream_record {
            return;
        }
        let Some(record) = record else {
            self.push(self.at(
                "research.metadata-missing",
                "missing canonical research metadata",
                &metadata_path,
            ));
            return;
        };
        let metadata = match record.value {
            Ok(value) => value,
            Err(error) => {
                self.push(self.at(
                    "research.metadata-unreadable",
                    format!("cannot load canonical research metadata: {error}"),
                    &metadata_path,
                ));
                return;
            }
        };
        if !metadata.is_object() || metadata.get("version").and_then(Value::as_i64) != Some(1) {
            self.push(self.at(
                "research.metadata-version",
                "canonical research metadata version must be 1",
                &metadata_path,
            ));
            return;
        }
        let today = self.today.clone().unwrap_or_else(today_utc);
        let diagnostics = validate_metadata(
            &self.index,
            &self.repository,
            &metadata,
            &metadata_path,
            &today,
        );
        self.diagnostics.extend(diagnostics);
    }
}

fn validate_metadata(
    index: &RecordIndex,
    repository: &Repository,
    metadata: &Value,
    metadata_path: &Path,
    today: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    validate_freshness(index, repository, metadata, metadata_path, today, &mut diagnostics);
    validate_lifecycle(index, repository, metadata, metadata_path, &mut diagnostics);
    validate_benchmark(index, repository, metadata, metadata_path, &mut diagnostics);
    validate_threats(index, repository, metadata, metadata_path, &mut diagnostics);
    validate_trace(index, repository, metadata, metadata_path, &mut diagnostics);
    diagnostics
}

fn at(repository: &Repository, code: &str, message: impl Into<String>, path: &Path) -> Diagnostic {
    Diagnostic::error(code.to_owned(), message).with_path(repository.relative_path(path))
}

fn read_local<'a>(index: &'a RecordIndex, repository: &Repository, relative: &str) -> Option<&'a str> {
    index.text(&repository.root().join(relative))
}

fn document_exists(index: &RecordIndex, repository: &Repository, relative: &str) -> bool {
    !relative.is_empty() && read_local(index, repository, relative).is_some()
}

fn configured_list<'a>(
    data: &'a Value,
    key: &str,
    repository: &Repository,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<&'a Value> {
    match data.get(key).and_then(Value::as_array) {
        Some(list) if !list.is_empty() => list.iter().collect(),
        _ => {
            diagnostics.push(at(
                repository,
                "research.field-invalid",
                format!("metadata field '{key}' must be a non-empty list"),
                path,
            ));
            Vec::new()
        }
    }
}

fn parse_ymd(value: &str) -> Option<(i64, i64, i64)> {
    if !is_iso_date(value) {
        return None;
    }
    Some((
        value[0..4].parse().ok()?,
        value[5..7].parse().ok()?,
        value[8..10].parse().ok()?,
    ))
}

/// The first `##`-level (not `###`+) section named `title`, from immediately
/// after the title on its heading line up to (not including) the next
/// `##`-level heading or end of text. Mirrors Python's
/// `^##\s+{title}\b(?P<body>.*?)(?=^##\s+|\Z)` without lookahead, which the
/// `regex` crate's linear-time engine does not support.
fn markdown_section<'a>(text: &'a str, title: &str) -> Option<&'a str> {
    let mut line_starts = vec![0usize];
    line_starts.extend(text.match_indices('\n').map(|(index, _)| index + 1));
    let mut open_at: Option<usize> = None;
    for (position, &start) in line_starts.iter().enumerate() {
        let line_end = line_starts.get(position + 1).copied().unwrap_or(text.len());
        let line = &text[start..line_end];
        let Some(rest) = line.strip_prefix("##") else {
            continue;
        };
        if !rest.starts_with(char::is_whitespace) {
            continue; // "###..." or "##word" is not a level-2 heading match
        }
        let trimmed_offset = rest.len() - rest.trim_start().len();
        let trimmed = &rest[trimmed_offset..];
        match open_at {
            None => {
                if let Some(remainder) = trimmed.strip_prefix(title) {
                    let boundary_ok = remainder
                        .chars()
                        .next()
                        .is_none_or(|char| !char.is_alphanumeric() && char != '_');
                    if boundary_ok {
                        open_at = Some(start + 2 + trimmed_offset + title.len());
                    }
                }
            }
            Some(_) => return open_at.map(|from| &text[from..start]),
        }
    }
    open_at.map(|from| &text[from..])
}

fn first_fenced_block(body: &str) -> Option<&str> {
    static FENCE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let regex = FENCE.get_or_init(|| Regex::new(r"(?s)```(.*?)```").expect("valid fence pattern"));
    regex.captures(body).and_then(|c| c.get(1)).map(|m| m.as_str())
}

fn contains_word(text: &str, word: &str) -> bool {
    let Ok(regex) = Regex::new(&format!(r"\b{}\b", regex::escape(word))) else {
        return false;
    };
    regex.is_match(text)
}

fn validate_freshness(
    index: &RecordIndex,
    repository: &Repository,
    metadata: &Value,
    metadata_path: &Path,
    today: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(freshness) = metadata.get("claim_freshness").filter(|value| value.is_object())
    else {
        diagnostics.push(at(
            repository,
            "research.freshness-invalid",
            "metadata field 'claim_freshness' must be an object",
            metadata_path,
        ));
        return;
    };

    let policy = freshness.get("policy").and_then(Value::as_str).unwrap_or("");
    if policy.is_empty() || !document_exists(index, repository, policy) {
        diagnostics.push(at(
            repository,
            "research.freshness-policy-missing",
            "research claim freshness policy must name an existing file",
            metadata_path,
        ));
    }

    let verified_on = freshness.get("verified_on").and_then(Value::as_str).unwrap_or_default();
    let review_by = freshness.get("review_by").and_then(Value::as_str).unwrap_or_default();
    let verified_ymd = parse_ymd(verified_on);
    let review_ymd = parse_ymd(review_by);
    if verified_ymd.is_none() {
        diagnostics.push(at(
            repository,
            "research.freshness-date-invalid",
            "claim_freshness.verified_on must be an ISO date",
            metadata_path,
        ));
    }
    if review_ymd.is_none() {
        diagnostics.push(at(
            repository,
            "research.freshness-date-invalid",
            "claim_freshness.review_by must be an ISO date",
            metadata_path,
        ));
    }
    if let (Some(verified), Some(review)) = (verified_ymd, review_ymd) {
        let verified_days = days_from_civil(verified.0, verified.1, verified.2);
        let review_days = days_from_civil(review.0, review.1, review.2);
        if review_days < verified_days {
            diagnostics.push(at(
                repository,
                "research.freshness-order-invalid",
                "research claim review deadline precedes its verification date",
                metadata_path,
            ));
        }
        if review_days > verified_days + 30 {
            diagnostics.push(at(
                repository,
                "research.freshness-window-exceeded",
                "research claim review deadline exceeds the 30-day policy maximum",
                metadata_path,
            ));
        }
        if let Some(current) = parse_ymd(today) {
            let today_days = days_from_civil(current.0, current.1, current.2);
            if review_days < today_days {
                diagnostics.push(at(
                    repository,
                    "research.freshness-expired",
                    format!(
                        "research claim freshness expired on {review_by}; re-verify the registered documents and advance the contract"
                    ),
                    metadata_path,
                ));
            }
        }
    }

    let documents_value = freshness.get("documents").and_then(Value::as_array);
    let Some(documents_value) = documents_value.filter(|list| list.iter().all(Value::is_string))
    else {
        diagnostics.push(at(
            repository,
            "research.freshness-documents-invalid",
            "claim_freshness.documents must be a list of paths",
            metadata_path,
        ));
        return;
    };
    let documents: Vec<String> = documents_value
        .iter()
        .map(|value| value.as_str().unwrap_or("").to_owned())
        .collect();
    let unique_count = documents.iter().collect::<std::collections::BTreeSet<_>>().len();
    if unique_count != documents.len() {
        diagnostics.push(at(
            repository,
            "research.freshness-documents-duplicate",
            "claim_freshness.documents must not contain duplicates",
            metadata_path,
        ));
    }
    let expected = top_level_research_documents(index, repository);
    let observed: std::collections::BTreeSet<String> = documents.into_iter().collect();
    let expected_set: std::collections::BTreeSet<String> = expected.into_iter().collect();
    if observed != expected_set {
        let missing: Vec<&String> = expected_set.difference(&observed).collect();
        let unexpected: Vec<&String> = observed.difference(&expected_set).collect();
        let mut details = Vec::new();
        if !missing.is_empty() {
            details.push(format!(
                "missing {}",
                missing.iter().map(|value| value.as_str()).collect::<Vec<_>>().join(", ")
            ));
        }
        if !unexpected.is_empty() {
            details.push(format!(
                "unexpected {}",
                unexpected.iter().map(|value| value.as_str()).collect::<Vec<_>>().join(", ")
            ));
        }
        diagnostics.push(at(
            repository,
            "research.freshness-coverage-incomplete",
            format!("research claim freshness coverage is incomplete: {}", details.join("; ")),
            metadata_path,
        ));
    }
}

fn top_level_research_documents(index: &RecordIndex, repository: &Repository) -> Vec<String> {
    let research_dir = repository.root().join("research");
    let mut documents: Vec<String> = index
        .text_files
        .keys()
        .filter(|path| {
            path.parent() == Some(research_dir.as_path())
                && path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        })
        .map(|path| repository.relative_path(path))
        .collect();
    documents.sort();
    documents
}

fn validate_lifecycle(
    index: &RecordIndex,
    repository: &Repository,
    metadata: &Value,
    metadata_path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(lifecycle) = metadata.get("lifecycle").filter(|value| value.is_object()) else {
        diagnostics.push(at(
            repository,
            "research.lifecycle-invalid",
            "metadata field 'lifecycle' must be an object",
            metadata_path,
        ));
        return;
    };
    let states_match = lifecycle
        .get("states")
        .and_then(Value::as_array)
        .map(|states| {
            states.iter().map(|value| value.as_str().unwrap_or("")).collect::<Vec<_>>() == STATUSES
        })
        .unwrap_or(false);
    if !states_match {
        diagnostics.push(at(
            repository,
            "research.lifecycle-states-mismatch",
            format!(
                "canonical lifecycle states must match repopact/repo_model.py: {}",
                STATUSES.join(", ")
            ),
            metadata_path,
        ));
        return;
    }

    for entry in configured_list(lifecycle, "set_documents", repository, metadata_path, diagnostics) {
        let path_rel = entry.get("path").and_then(Value::as_str);
        let pattern_src = entry.get("pattern").and_then(Value::as_str);
        let (Some(path_rel), Some(pattern_src)) = (path_rel, pattern_src) else {
            diagnostics.push(at(
                repository,
                "research.lifecycle-set-documents-invalid",
                "lifecycle set_documents entries require path and pattern",
                metadata_path,
            ));
            continue;
        };
        let target_path = repository.root().join(path_rel);
        let Some(text) = read_local(index, repository, path_rel) else {
            diagnostics.push(at(
                repository,
                "research.source-missing",
                format!("missing research fact source: {path_rel}"),
                metadata_path,
            ));
            continue;
        };
        let Ok(pattern) = Regex::new(pattern_src) else {
            diagnostics.push(at(
                repository,
                "research.pattern-invalid",
                format!("lifecycle set_documents pattern is invalid: {path_rel}"),
                metadata_path,
            ));
            continue;
        };
        let Some(captures) = pattern.captures(text).filter(|c| c.get(1).is_some()) else {
            diagnostics.push(at(
                repository,
                "research.lifecycle-set-missing",
                "canonical lifecycle set is missing",
                &target_path,
            ));
            continue;
        };
        let observed: Vec<String> = captures[1]
            .split(',')
            .map(|token| token.trim().trim_matches('`').to_owned())
            .collect();
        if observed != STATUSES {
            diagnostics.push(at(
                repository,
                "research.lifecycle-set-mismatch",
                format!(
                    "lifecycle states contradict metadata: expected {}; observed {}",
                    STATUSES.join(", "),
                    observed.join(", ")
                ),
                &target_path,
            ));
        }
    }

    for entry in configured_list(lifecycle, "figure_documents", repository, metadata_path, diagnostics) {
        let path_rel = entry.get("path").and_then(Value::as_str);
        let section = entry.get("section").and_then(Value::as_str);
        let (Some(path_rel), Some(section)) = (path_rel, section) else {
            diagnostics.push(at(
                repository,
                "research.lifecycle-figure-documents-invalid",
                "lifecycle figure_documents entries require path and section",
                metadata_path,
            ));
            continue;
        };
        let target_path = repository.root().join(path_rel);
        let Some(text) = read_local(index, repository, path_rel) else {
            diagnostics.push(at(
                repository,
                "research.source-missing",
                format!("missing research fact source: {path_rel}"),
                metadata_path,
            ));
            continue;
        };
        let Some(body) = markdown_section(text, section) else {
            diagnostics.push(at(
                repository,
                "research.lifecycle-figure-section-missing",
                format!("missing lifecycle section '{section}'"),
                &target_path,
            ));
            continue;
        };
        let diagram = first_fenced_block(body).unwrap_or(body);
        let observed: Vec<&str> = STATUSES.iter().copied().filter(|state| contains_word(diagram, state)).collect();
        if observed != STATUSES {
            let missing: Vec<&str> = STATUSES.iter().copied().filter(|state| !observed.contains(state)).collect();
            diagnostics.push(at(
                repository,
                "research.lifecycle-figure-mismatch",
                format!("lifecycle figure contradicts metadata; missing state(s): {}", missing.join(", ")),
                &target_path,
            ));
        }
    }

    let release = lifecycle.get("provenance_typing_release").and_then(Value::as_str);
    let release_valid = release.is_some_and(is_semver_core);
    if !release_valid {
        diagnostics.push(at(
            repository,
            "research.provenance-release-invalid",
            "lifecycle provenance_typing_release must be semantic",
            metadata_path,
        ));
    } else if let Some(figures_text) = read_local(index, repository, "research/figures.md") {
        let release = release.unwrap_or_default();
        let body = markdown_section(figures_text, "Figure 3").unwrap_or("");
        let release_core = release.split('.').take(2).collect::<Vec<_>>().join(".");
        if body.to_ascii_lowercase().contains("future escape")
            || !body.contains(&format!("RepoPact {release_core}"))
        {
            diagnostics.push(at(
                repository,
                "research.provenance-figure-regressed",
                format!(
                    "provenance figure must describe provenance typing as shipped in RepoPact {release_core}, not future work"
                ),
                &repository.root().join("research/figures.md"),
            ));
        }
    }
}

fn validate_benchmark(
    index: &RecordIndex,
    repository: &Repository,
    metadata: &Value,
    metadata_path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(benchmark) = metadata.get("benchmark").filter(|value| value.is_object()) else {
        diagnostics.push(at(
            repository,
            "research.benchmark-invalid",
            "metadata field 'benchmark' must be an object",
            metadata_path,
        ));
        return;
    };
    let Some(pactbench) = benchmark.get("pactbench").filter(|value| value.is_object()) else {
        diagnostics.push(at(
            repository,
            "research.benchmark-pactbench-invalid",
            "metadata field 'benchmark' must be an object",
            metadata_path,
        ));
        return;
    };
    let Some(count) = pactbench.get("task_count").and_then(Value::as_i64) else {
        diagnostics.push(at(
            repository,
            "research.benchmark-task-count-invalid",
            "benchmark.pactbench.task_count must be an integer",
            metadata_path,
        ));
        return;
    };
    let source = pactbench.get("source").and_then(Value::as_str).unwrap_or("");
    if !document_exists(index, repository, source) {
        diagnostics.push(at(
            repository,
            "research.benchmark-source-missing",
            "benchmark PactBench count source must name an existing artifact",
            metadata_path,
        ));
    }
    for entry in configured_list(pactbench, "documents", repository, metadata_path, diagnostics) {
        let path_rel = entry.get("path").and_then(Value::as_str);
        let pattern_src = entry.get("pattern").and_then(Value::as_str);
        let (Some(path_rel), Some(pattern_src)) = (path_rel, pattern_src) else {
            diagnostics.push(at(
                repository,
                "research.benchmark-documents-invalid",
                "PactBench documents entries require path and pattern",
                metadata_path,
            ));
            continue;
        };
        let target_path = repository.root().join(path_rel);
        let Some(text) = read_local(index, repository, path_rel) else {
            diagnostics.push(at(
                repository,
                "research.source-missing",
                format!("missing research fact source: {path_rel}"),
                metadata_path,
            ));
            continue;
        };
        let Ok(pattern) = Regex::new(pattern_src) else {
            diagnostics.push(at(
                repository,
                "research.pattern-invalid",
                format!("PactBench documents pattern is invalid: {path_rel}"),
                metadata_path,
            ));
            continue;
        };
        let Some(observed) = pattern
            .captures(text)
            .and_then(|c| c.get(1))
            .and_then(|group| group.as_str().parse::<i64>().ok())
        else {
            diagnostics.push(at(
                repository,
                "research.benchmark-task-count-missing",
                "PactBench task-count fact is missing",
                &target_path,
            ));
            continue;
        };
        if observed != count {
            diagnostics.push(at(
                repository,
                "research.benchmark-task-count-mismatch",
                format!("PactBench task count contradicts metadata: expected {count}; observed {observed}"),
                &target_path,
            ));
        }
    }

    let Some(mappings) = benchmark.get("study_hypotheses").and_then(Value::as_object).filter(|m| !m.is_empty())
    else {
        diagnostics.push(at(
            repository,
            "research.benchmark-study-hypotheses-invalid",
            "benchmark.study_hypotheses must be a non-empty object",
            metadata_path,
        ));
        return;
    };
    let mut ordered: Vec<(&String, &Value)> = mappings.iter().collect();
    ordered.sort_by_key(|(key, _)| key.trim_start_matches(|c: char| !c.is_ascii_digit()).parse::<i64>().unwrap_or(0));
    let expected_start = ordered
        .first()
        .and_then(|(_, value)| value.as_str())
        .and_then(|value| value.trim_start_matches('H').parse::<i64>().ok());
    let expected_end = ordered
        .last()
        .and_then(|(_, value)| value.as_str())
        .and_then(|value| value.trim_start_matches('H').parse::<i64>().ok());
    if let (Some(expected_start), Some(expected_end)) = (expected_start, expected_end) {
        for entry in configured_list(benchmark, "range_documents", repository, metadata_path, diagnostics) {
            let path_rel = entry.get("path").and_then(Value::as_str);
            let pattern_src = entry.get("pattern").and_then(Value::as_str);
            let (Some(path_rel), Some(pattern_src)) = (path_rel, pattern_src) else {
                diagnostics.push(at(
                    repository,
                    "research.benchmark-range-documents-invalid",
                    "benchmark range_documents entries require path and pattern",
                    metadata_path,
                ));
                continue;
            };
            let target_path = repository.root().join(path_rel);
            let Some(text) = read_local(index, repository, path_rel) else {
                diagnostics.push(at(
                    repository,
                    "research.source-missing",
                    format!("missing research fact source: {path_rel}"),
                    metadata_path,
                ));
                continue;
            };
            let Ok(pattern) = Regex::new(pattern_src) else {
                diagnostics.push(at(
                    repository,
                    "research.pattern-invalid",
                    format!("benchmark range_documents pattern is invalid: {path_rel}"),
                    metadata_path,
                ));
                continue;
            };
            let observed = pattern.captures(text).and_then(|c| {
                let start = c.get(1)?.as_str().parse::<i64>().ok()?;
                let end = c.get(2)?.as_str().parse::<i64>().ok()?;
                Some((start, end))
            });
            let Some((observed_start, observed_end)) = observed else {
                diagnostics.push(at(
                    repository,
                    "research.benchmark-range-missing",
                    "benchmark hypothesis range is missing",
                    &target_path,
                ));
                continue;
            };
            if (observed_start, observed_end) != (expected_start, expected_end) {
                diagnostics.push(at(
                    repository,
                    "research.benchmark-range-mismatch",
                    format!(
                        "benchmark hypothesis range contradicts metadata: expected H{expected_start}\u{2013}H{expected_end}; observed H{observed_start}\u{2013}H{observed_end}"
                    ),
                    &target_path,
                ));
            }
        }
    }

    static HEADING_PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let heading_pattern = HEADING_PATTERN.get_or_init(|| {
        Regex::new(r"(?m)^###\s+(S[0-9]+)\b[^\n]*[\u{2192}-]+\s*(H[0-9]+)\b").expect("valid heading pattern")
    });
    static TABLE_PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let table_pattern = TABLE_PATTERN
        .get_or_init(|| Regex::new(r"(?m)^\|\s*(S[0-9]+)\s*\|\s*(H[0-9]+)\s*\|").expect("valid table pattern"));

    let expected_mapping: BTreeMap<String, String> = mappings
        .iter()
        .filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_owned())))
        .collect();
    for relative in configured_list(benchmark, "mapping_documents", repository, metadata_path, diagnostics) {
        let Some(relative) = relative.as_str() else {
            diagnostics.push(at(
                repository,
                "research.benchmark-mapping-documents-invalid",
                "benchmark mapping_documents entries must be paths",
                metadata_path,
            ));
            continue;
        };
        let target_path = repository.root().join(relative);
        let Some(text) = read_local(index, repository, relative) else {
            diagnostics.push(at(
                repository,
                "research.source-missing",
                format!("missing research fact source: {relative}"),
                metadata_path,
            ));
            continue;
        };
        let mut observed: BTreeMap<String, String> = heading_pattern
            .captures_iter(text)
            .map(|c| (c[1].to_owned(), c[2].to_owned()))
            .collect();
        if observed.is_empty() {
            observed = table_pattern
                .captures_iter(text)
                .map(|c| (c[1].to_owned(), c[2].to_owned()))
                .collect();
        }
        if observed != expected_mapping {
            diagnostics.push(at(
                repository,
                "research.benchmark-mapping-mismatch",
                "study-to-hypothesis mapping contradicts metadata",
                &target_path,
            ));
        }
    }
}

fn is_threat_identifier(value: &str) -> bool {
    let Some(rest) = value.strip_prefix('T') else {
        return false;
    };
    !rest.is_empty() && !rest.starts_with('0') && rest.chars().all(|char| char.is_ascii_digit())
}

fn validate_threats(
    index: &RecordIndex,
    repository: &Repository,
    metadata: &Value,
    metadata_path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(threats) = metadata.get("threats").filter(|value| value.is_object()) else {
        diagnostics.push(at(
            repository,
            "research.threats-invalid",
            "metadata field 'threats' must be an object",
            metadata_path,
        ));
        return;
    };
    let identifiers: Vec<String> = configured_list(threats, "identifiers", repository, metadata_path, diagnostics)
        .into_iter()
        .filter_map(|value| value.as_str().map(str::to_owned))
        .collect();
    if identifiers.is_empty() {
        return;
    }
    if identifiers.iter().any(|value| !is_threat_identifier(value)) {
        diagnostics.push(at(
            repository,
            "research.threat-identifier-invalid",
            "threat identifiers must use T<number> form",
            metadata_path,
        ));
        return;
    }
    let unique_count = identifiers.iter().collect::<std::collections::BTreeSet<_>>().len();
    if unique_count != identifiers.len() {
        diagnostics.push(at(
            repository,
            "research.threat-identifier-duplicate",
            "canonical threat identifiers must be unique",
            metadata_path,
        ));
    }

    static HEADING_PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let heading_pattern = HEADING_PATTERN
        .get_or_init(|| Regex::new(r"(?m)^#{2,3}\s+(T[0-9]+)\s*(?::|\u{2014}|-)").expect("valid threat heading pattern"));

    for relative in configured_list(threats, "documents", repository, metadata_path, diagnostics) {
        let Some(relative) = relative.as_str() else {
            diagnostics.push(at(
                repository,
                "research.threat-documents-invalid",
                "threat documents entries must be paths",
                metadata_path,
            ));
            continue;
        };
        let target_path = repository.root().join(relative);
        let Some(text) = read_local(index, repository, relative) else {
            diagnostics.push(at(
                repository,
                "research.source-missing",
                format!("missing research fact source: {relative}"),
                metadata_path,
            ));
            continue;
        };
        let observed: Vec<String> = heading_pattern
            .captures_iter(text)
            .map(|c| c[1].to_owned())
            .collect();
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for identifier in &observed {
            *counts.entry(identifier.as_str()).or_insert(0) += 1;
        }
        let repeated: Vec<&str> = counts
            .iter()
            .filter(|(_, count)| **count > 1)
            .map(|(id, _)| *id)
            .collect();
        let missing: Vec<&str> = identifiers
            .iter()
            .filter(|id| !observed.iter().any(|found| found == *id))
            .map(String::as_str)
            .collect();
        let unexpected: Vec<&str> = observed
            .iter()
            .filter(|found| !identifiers.iter().any(|id| id == *found))
            .map(String::as_str)
            .collect();
        if !repeated.is_empty() || !missing.is_empty() || !unexpected.is_empty() {
            let mut details = Vec::new();
            if !repeated.is_empty() {
                details.push(format!("repeated {}", repeated.join(", ")));
            }
            if !missing.is_empty() {
                details.push(format!("missing {}", missing.join(", ")));
            }
            if !unexpected.is_empty() {
                details.push(format!("unexpected {}", unexpected.join(", ")));
            }
            diagnostics.push(at(
                repository,
                "research.threat-identifier-mismatch",
                format!("threat identifiers contradict metadata: {}", details.join("; ")),
                &target_path,
            ));
        }
    }
}

fn validate_trace(
    index: &RecordIndex,
    repository: &Repository,
    metadata: &Value,
    metadata_path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(trace) = metadata.get("proposed_state_trace").filter(|value| value.is_object()) else {
        diagnostics.push(at(
            repository,
            "research.trace-invalid",
            "metadata field 'proposed_state_trace' must be an object",
            metadata_path,
        ));
        return;
    };
    let finding = trace.get("finding").and_then(Value::as_str);
    let capture = trace.get("capture").and_then(Value::as_str);
    if finding != Some("F-014") || capture.is_none() {
        diagnostics.push(at(
            repository,
            "research.trace-target-invalid",
            "proposed-state trace must identify F-014 and capture 013",
            metadata_path,
        ));
        return;
    }
    let capture = capture.unwrap_or_default();

    let decisions: Vec<String> = trace
        .get("decisions")
        .and_then(Value::as_array)
        .filter(|list| !list.is_empty())
        .map(|list| list.iter().filter_map(|value| value.as_str().map(str::to_owned)).collect())
        .unwrap_or_else(|| {
            diagnostics.push(at(
                repository,
                "research.trace-decisions-invalid",
                "proposed-state trace decisions must be a non-empty list",
                metadata_path,
            ));
            Vec::new()
        });

    let local_fields = ["work_item", "implementation_evidence", "rollout_evidence"];
    let mut targets: Vec<String> = vec![capture.to_owned()];
    targets.extend(decisions);
    for field in local_fields {
        targets.push(trace.get(field).and_then(Value::as_str).unwrap_or("").to_owned());
    }
    for relative in &targets {
        if !document_exists(index, repository, relative) {
            diagnostics.push(at(
                repository,
                "research.trace-target-missing",
                format!("proposed-state trace target does not exist: {relative}"),
                metadata_path,
            ));
        }
    }

    if let Some(findings) = read_local(index, repository, "research/findings.md") {
        if !findings.contains("| F-014 |")
            || !findings.contains("captures/013-proposed-lifecycle-adoption-pressure.md")
        {
            diagnostics.push(at(
                repository,
                "research.trace-findings-link-missing",
                "F-014 must link capture 013 in the findings register",
                &repository.root().join("research/findings.md"),
            ));
        }
    }

    if let Some(capture_text) = read_local(index, repository, capture) {
        let required_tokens: Vec<String> = [
            "0023".to_owned(),
            "025".to_owned(),
            "20260629-proposed-lifecycle-state".to_owned(),
            "0024".to_owned(),
            trace.get("release_tag").and_then(Value::as_str).unwrap_or("").to_owned(),
            trace.get("release_commit").and_then(Value::as_str).unwrap_or("").to_owned(),
            trace.get("adopter_commit").and_then(Value::as_str).unwrap_or("").to_owned(),
            "20260718-repopact-2-2-0-adopter-rollout".to_owned(),
        ]
        .into_iter()
        .filter(|token| !token.is_empty())
        .collect();
        let absent: Vec<&str> = required_tokens
            .iter()
            .filter(|token| !capture_text.contains(token.as_str()))
            .map(String::as_str)
            .collect();
        if !absent.is_empty() {
            diagnostics.push(at(
                repository,
                "research.trace-capture-token-missing",
                format!("proposed-state capture is missing trace token(s): {}", absent.join(", ")),
                &repository.root().join(capture),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use repopact_repository::Repository;
    use repopact_types::Diagnostic;
    use serde_json::json;

    use crate::Validator;

    fn temp_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("repopact-rust-research-{name}-{suffix}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write(root: &std::path::Path, relative: &str, content: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    /// A complete, internally-consistent research surface: every fact quoted
    /// in `research/metadata.json` is backed by matching text in the
    /// referenced document, so this fixture is expected to produce zero
    /// `research.*` diagnostics under `today == "2026-01-10"` (before
    /// `review_by`). Individual tests clone this baseline and perturb one
    /// fact so exactly one negative case fires.
    fn valid_fixture(root: &std::path::Path) {
        write(
            root,
            "governance/policies/committed-policy.md",
            "# Semantic Claim Freshness Policy\n",
        );
        write(root, "evidence/runs/example.json", "{}\n");
        write(
            root,
            "research/paper.md",
            "# Paper\n\nstates: {proposed, active, blocked, deferred, completed}\n\ntasks: 5\n\nH1-H2\n\n| S1 | H1 |\n| S2 | H2 |\n\n## T1: threat one\n## T2: threat two\n",
        );
        write(root, "research/protocol.md", "# Protocol\n");
        write(
            root,
            "research/figures.md",
            "## Figure 2\n\n```\nproposed -> active -> blocked -> deferred -> completed\n```\n\n## Figure 3\n\nRepoPact 2.0 ships provenance typing today.\n",
        );
        write(
            root,
            "research/findings.md",
            "| F-014 | links captures/013-proposed-lifecycle-adoption-pressure.md |\n",
        );
        write(
            root,
            "research/captures/013-proposed-lifecycle-adoption-pressure.md",
            "0023 025 20260629-proposed-lifecycle-state 0024 v1.0.0 abc123def 20260718-repopact-2-2-0-adopter-rollout\n",
        );
        write(root, "decisions/0001-example.md", "# Decision 0001\n");
        write(
            root,
            "research/metadata.json",
            &serde_json::to_string_pretty(&json!({
                "version": 1,
                "claim_freshness": {
                    "policy": "governance/policies/committed-policy.md",
                    "verified_on": "2026-01-01",
                    "review_by": "2026-01-15",
                    "documents": [
                        "research/figures.md",
                        "research/findings.md",
                        "research/paper.md",
                        "research/protocol.md"
                    ],
                },
                "lifecycle": {
                    "states": ["proposed", "active", "blocked", "deferred", "completed"],
                    "set_documents": [{"path": "research/paper.md", "pattern": "states: \\{([^}]+)\\}"}],
                    "figure_documents": [{"path": "research/figures.md", "section": "Figure 2"}],
                    "provenance_typing_release": "2.0.0",
                },
                "benchmark": {
                    "pactbench": {
                        "task_count": 5,
                        "source": "evidence/runs/example.json",
                        "documents": [{"path": "research/paper.md", "pattern": "tasks: ([0-9]+)"}],
                    },
                    "study_hypotheses": {"S1": "H1", "S2": "H2"},
                    "range_documents": [{"path": "research/paper.md", "pattern": "H([0-9]+)-H([0-9]+)"}],
                    "mapping_documents": ["research/paper.md"],
                },
                "threats": {
                    "identifiers": ["T1", "T2"],
                    "documents": ["research/paper.md"],
                },
                "proposed_state_trace": {
                    "finding": "F-014",
                    "capture": "research/captures/013-proposed-lifecycle-adoption-pressure.md",
                    "decisions": ["decisions/0001-example.md"],
                    "work_item": "evidence/runs/example.json",
                    "implementation_evidence": "evidence/runs/example.json",
                    "rollout_evidence": "evidence/runs/example.json",
                    "release_tag": "v1.0.0",
                    "release_commit": "abc123def",
                },
            }))
            .unwrap(),
        );
    }

    fn diagnostics_with_today(root: &std::path::Path, today: &str) -> Vec<Diagnostic> {
        let repository = Repository::open(root);
        let snapshot = repository.session().snapshot();
        Validator::from_snapshot(&snapshot)
            .with_today(today)
            .validate()
            .diagnostics
    }

    fn research_codes(root: &std::path::Path, today: &str) -> Vec<String> {
        let diagnostics = diagnostics_with_today(root, today)
            .into_iter()
            .filter(|diagnostic| diagnostic.code.starts_with("research."))
            .map(|diagnostic| diagnostic.code)
            .collect();
        fs::remove_dir_all(root).unwrap();
        diagnostics
    }

    #[test]
    fn no_research_surface_is_silent() {
        let root = temp_root("absent");
        assert!(research_codes(&root, "2026-01-10").is_empty());
    }

    #[test]
    fn upstream_convention_without_metadata_reports_missing() {
        let root = temp_root("upstream-only");
        write(&root, "research/paper.md", "# Paper\n");
        write(&root, "research/protocol.md", "# Protocol\n");
        assert_eq!(research_codes(&root, "2026-01-10"), vec!["research.metadata-missing"]);
    }

    #[test]
    fn valid_current_metadata_is_silent() {
        let root = temp_root("valid");
        valid_fixture(&root);
        assert_eq!(research_codes(&root, "2026-01-10"), Vec::<String>::new());
    }

    #[test]
    fn expired_freshness_with_injected_date_is_rejected() {
        let root = temp_root("expired");
        valid_fixture(&root);
        assert!(research_codes(&root, "2026-02-01").contains(&"research.freshness-expired".to_owned()));
    }

    #[test]
    fn incomplete_top_level_document_registration_is_rejected() {
        let root = temp_root("coverage");
        valid_fixture(&root);
        let metadata_path = root.join("research/metadata.json");
        let mut metadata: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&metadata_path).unwrap()).unwrap();
        metadata["claim_freshness"]["documents"] = json!(["research/paper.md", "research/protocol.md"]);
        fs::write(&metadata_path, serde_json::to_string_pretty(&metadata).unwrap()).unwrap();
        assert!(research_codes(&root, "2026-01-10").contains(&"research.freshness-coverage-incomplete".to_owned()));
    }

    #[test]
    fn lifecycle_representation_drift_is_rejected() {
        let root = temp_root("lifecycle-drift");
        valid_fixture(&root);
        write(
            &root,
            "research/figures.md",
            "## Figure 2\n\n```\nproposed -> active -> completed\n```\n\n## Figure 3\n\nRepoPact 2.0 ships provenance typing today.\n",
        );
        assert!(research_codes(&root, "2026-01-10").contains(&"research.lifecycle-figure-mismatch".to_owned()));
    }

    #[test]
    fn pactbench_count_drift_is_rejected() {
        let root = temp_root("pactbench-drift");
        valid_fixture(&root);
        write(
            &root,
            "research/paper.md",
            "# Paper\n\nstates: {proposed, active, blocked, deferred, completed}\n\ntasks: 999\n\nH1-H2\n\n| S1 | H1 |\n| S2 | H2 |\n\n## T1: threat one\n## T2: threat two\n",
        );
        assert!(research_codes(&root, "2026-01-10").contains(&"research.benchmark-task-count-mismatch".to_owned()));
    }

    #[test]
    fn hypothesis_range_drift_is_rejected() {
        let root = temp_root("range-drift");
        valid_fixture(&root);
        write(
            &root,
            "research/paper.md",
            "# Paper\n\nstates: {proposed, active, blocked, deferred, completed}\n\ntasks: 5\n\nH1-H9\n\n| S1 | H1 |\n| S2 | H2 |\n\n## T1: threat one\n## T2: threat two\n",
        );
        assert!(research_codes(&root, "2026-01-10").contains(&"research.benchmark-range-mismatch".to_owned()));
    }

    #[test]
    fn study_mapping_drift_is_rejected() {
        let root = temp_root("mapping-drift");
        valid_fixture(&root);
        write(
            &root,
            "research/paper.md",
            "# Paper\n\nstates: {proposed, active, blocked, deferred, completed}\n\ntasks: 5\n\nH1-H2\n\n| S1 | H9 |\n| S2 | H2 |\n\n## T1: threat one\n## T2: threat two\n",
        );
        assert!(research_codes(&root, "2026-01-10").contains(&"research.benchmark-mapping-mismatch".to_owned()));
    }

    #[test]
    fn repeated_threat_identifier_is_rejected() {
        let root = temp_root("threat-repeat");
        valid_fixture(&root);
        write(
            &root,
            "research/paper.md",
            "# Paper\n\nstates: {proposed, active, blocked, deferred, completed}\n\ntasks: 5\n\nH1-H2\n\n| S1 | H1 |\n| S2 | H2 |\n\n## T1: threat one\n## T1: threat one again\n## T2: threat two\n",
        );
        assert!(research_codes(&root, "2026-01-10").contains(&"research.threat-identifier-mismatch".to_owned()));
    }

    #[test]
    fn missing_threat_identifier_is_rejected() {
        let root = temp_root("threat-missing");
        valid_fixture(&root);
        write(
            &root,
            "research/paper.md",
            "# Paper\n\nstates: {proposed, active, blocked, deferred, completed}\n\ntasks: 5\n\nH1-H2\n\n| S1 | H1 |\n| S2 | H2 |\n\n## T1: threat one\n",
        );
        assert!(research_codes(&root, "2026-01-10").contains(&"research.threat-identifier-mismatch".to_owned()));
    }

    #[test]
    fn missing_trace_target_is_rejected() {
        let root = temp_root("trace-target-missing");
        valid_fixture(&root);
        let metadata_path = root.join("research/metadata.json");
        let mut metadata: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&metadata_path).unwrap()).unwrap();
        metadata["proposed_state_trace"]["work_item"] = json!("evidence/runs/does-not-exist.json");
        fs::write(&metadata_path, serde_json::to_string_pretty(&metadata).unwrap()).unwrap();
        assert!(research_codes(&root, "2026-01-10").contains(&"research.trace-target-missing".to_owned()));
    }

    #[test]
    fn missing_trace_token_is_rejected() {
        let root = temp_root("trace-token-missing");
        valid_fixture(&root);
        write(
            &root,
            "research/captures/013-proposed-lifecycle-adoption-pressure.md",
            "only some of the required tokens: 0023\n",
        );
        assert!(research_codes(&root, "2026-01-10").contains(&"research.trace-capture-token-missing".to_owned()));
    }

    #[test]
    fn provenance_figure_regressing_to_future_work_is_rejected() {
        let root = temp_root("provenance-regressed");
        valid_fixture(&root);
        write(
            &root,
            "research/figures.md",
            "## Figure 2\n\n```\nproposed -> active -> blocked -> deferred -> completed\n```\n\n## Figure 3\n\nProvenance typing remains a future escape hatch.\n",
        );
        assert!(research_codes(&root, "2026-01-10").contains(&"research.provenance-figure-regressed".to_owned()));
    }
}
