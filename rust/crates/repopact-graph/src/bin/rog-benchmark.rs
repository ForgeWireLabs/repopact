//! ROG-032 deterministic performance/storage benchmark harness (Decision
//! 0053 section 2). Source-controlled and reproducible -- generated
//! fixtures are produced at run time from a fixed seed, never committed
//! as thousands of manually authored files, and no criterion depends on
//! a developer's local cache.
//!
//! Usage:
//! ```text
//! cargo run --release -p repopact-graph --bin rog-benchmark -- <target> [<out.json>]
//! ```
//! `<target>` is one of `real:<path-to-repopact-checkout>`,
//! `medium` (a ~2,000-file deterministic fixture), or `large` (a
//! ~10,000-file deterministic fixture). Output is one JSON object per
//! run, written to `<out.json>` if given, else stdout.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use repopact_graph::{merge_reconcile, RepositoryGraph};
use repopact_repository::Repository;
use serde_json::json;

const GENERATOR_VERSION: &str = "rog-benchmark-fixture-gen-v1";
const SEED_MEDIUM: u64 = 20260914_002_000;
const SEED_LARGE: u64 = 20260914_010_000;

fn main() {
    let mut args = std::env::args().skip(1);
    let target = args.next().unwrap_or_else(|| {
        eprintln!("usage: rog-benchmark <real:<path>|medium|large> [<out.json>]");
        std::process::exit(2);
    });
    let out = args.next();

    let (root, disposable, scale_label, composition) =
        if let Some(path) = target.strip_prefix("real:") {
            (PathBuf::from(path), false, "real_repopact".to_owned(), None)
        } else if target == "medium" {
            let root = temp_root("rog-bench-medium");
            let composition = generate_fixture(&root, 2_000, SEED_MEDIUM);
            (root, true, "medium_2000".to_owned(), Some(composition))
        } else if target == "large" {
            let root = temp_root("rog-bench-large");
            let composition = generate_fixture(&root, 10_000, SEED_LARGE);
            (root, true, "large_10000".to_owned(), Some(composition))
        } else {
            eprintln!("unknown target: {target}");
            std::process::exit(2);
        };

    let result = run_benchmark(&root, &scale_label, composition);
    let text = serde_json::to_string_pretty(&result).unwrap();
    match out {
        Some(path) => fs::write(&path, text).expect("write benchmark output"),
        None => println!("{text}"),
    }

    if disposable {
        let _ = fs::remove_dir_all(&root);
    }
}

fn temp_root(name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("{name}-{suffix}"));
    fs::create_dir_all(&root).unwrap();
    root
}

// ---- deterministic fixture generator -------------------------------------

struct Composition {
    rust_files: usize,
    python_files: usize,
    ts_files: usize,
    manifests: usize,
    packages: usize,
    test_files: usize,
    ci_files: usize,
    generated_boundary_files: usize,
    fixture_boundary_present: bool,
    dependency_edges_declared: usize,
}

/// A small deterministic linear-congruential generator -- no external
/// `rand` dependency, fully reproducible from `seed` alone.
struct Lcg(u64);
impl Lcg {
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    fn next_range(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

/// Deterministically generates a fixture of approximately `scale` total
/// source/metadata files, composed as documented in Decision 0053
/// section 2 / ROG-032: Rust crates (each with its own Cargo.toml and a
/// test target), a Python package, a TypeScript/npm workspace, CI
/// metadata, a generated-boundary marker, and one fixture-topology
/// boundary. Cross-file dependency edges are created via ordinary
/// `use`/`import` statements referencing sibling modules within each
/// package, so the generated tree exercises real semantic-adapter edges,
/// not just file counts.
fn generate_fixture(root: &Path, scale: usize, seed: u64) -> Composition {
    let mut rng = Lcg(seed);
    let rust_share = scale * 40 / 100;
    let python_share = scale * 20 / 100;
    let ts_share = scale * 15 / 100;
    let test_share = scale * 15 / 100;
    let remainder = scale.saturating_sub(rust_share + python_share + ts_share + test_share);

    let n_rust_packages = (rust_share / 150).max(1);
    let mut rust_files = 0;
    let mut manifests = 0;
    let mut dependency_edges_declared = 0;
    for package_index in 0..n_rust_packages {
        let package_dir = root.join(format!("crates/pkg_{package_index:04}"));
        fs::create_dir_all(package_dir.join("src")).unwrap();
        fs::create_dir_all(package_dir.join("tests")).unwrap();
        let files_here = rust_share / n_rust_packages;
        let mut lib_uses = String::new();
        for file_index in 0..files_here {
            let mod_name = format!("mod_{file_index:04}");
            let calls_prior = file_index > 0 && rng.next_range(3) == 0;
            let mut body = format!("pub fn function_{file_index:04}() -> u32 {{\n");
            if calls_prior {
                body.push_str(&format!(
                    "    super::mod_{:04}::function_{:04}() + 1\n",
                    file_index - 1,
                    file_index - 1
                ));
                dependency_edges_declared += 1;
            } else {
                body.push_str("    0\n");
            }
            body.push_str("}\n");
            fs::write(package_dir.join(format!("src/{mod_name}.rs")), body).unwrap();
            lib_uses.push_str(&format!("pub mod {mod_name};\n"));
            rust_files += 1;
        }
        fs::write(package_dir.join("src/lib.rs"), lib_uses).unwrap();
        fs::write(
            package_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"pkg-{package_index:04}\"\nversion = \"0.1.0\"\n\n[[test]]\nname = \"integration\"\npath = \"tests/integration.rs\"\n"
            ),
        )
        .unwrap();
        fs::write(
            package_dir.join("tests/integration.rs"),
            "#[test]\nfn it_works() { assert!(true); }\n",
        )
        .unwrap();
        manifests += 1;
        rust_files += 1; // lib.rs itself
    }
    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[workspace]\nmembers = [{}]\n",
            (0..n_rust_packages)
                .map(|i| format!("\"crates/pkg_{i:04}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    )
    .unwrap();
    manifests += 1;

    let mut python_files = 0;
    fs::create_dir_all(root.join("python_pkg")).unwrap();
    fs::write(
        root.join("python_pkg/pyproject.toml"),
        "[project]\nname = \"rog-bench-python\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    manifests += 1;
    for index in 0..python_share {
        fs::write(
            root.join(format!("python_pkg/module_{index:04}.py")),
            format!("def function_{index:04}():\n    return {index}\n"),
        )
        .unwrap();
        python_files += 1;
    }

    let mut ts_files = 0;
    fs::create_dir_all(root.join("web/src")).unwrap();
    fs::write(
        root.join("web/package.json"),
        "{\"name\": \"rog-bench-web\", \"version\": \"0.1.0\"}\n",
    )
    .unwrap();
    manifests += 1;
    for index in 0..ts_share {
        fs::write(
            root.join(format!("web/src/component_{index:04}.ts")),
            format!("export function component{index:04}(): number {{ return {index}; }}\n"),
        )
        .unwrap();
        ts_files += 1;
    }

    let mut test_files = 0;
    fs::create_dir_all(root.join("tests")).unwrap();
    for index in 0..test_share {
        fs::write(
            root.join(format!("tests/test_generated_{index:04}.py")),
            format!("def test_generated_{index:04}():\n    assert True\n"),
        )
        .unwrap();
        test_files += 1;
    }

    // Remainder: a handful of CI/manifest/generated-boundary files, a
    // realistic AGENTS.md, and exactly one fixture-topology boundary
    // (Decision 0053 section 1).
    fs::create_dir_all(root.join(".github/workflows")).unwrap();
    fs::write(
        root.join(".github/workflows/ci.yml"),
        "name: CI\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo test\n",
    )
    .unwrap();
    let ci_files = 1;

    fs::create_dir_all(root.join("dist")).unwrap();
    fs::write(
        root.join("dist/.generated-marker"),
        "generated-by=rog-bench\n",
    )
    .unwrap();
    let generated_boundary_files = 1;

    fs::create_dir_all(root.join("tests/fixtures")).unwrap();
    fs::write(
        root.join("tests/fixtures/sample.json"),
        "{\"note\": \"excluded fixture content, never read by the projection\"}\n",
    )
    .unwrap();
    let fixture_boundary_present = true;

    fs::write(
        root.join("AGENTS.md"),
        "# Deterministic ROG-032 benchmark fixture\nGenerated by rog-benchmark; not a real project.\n",
    )
    .unwrap();

    let _ = remainder;
    Composition {
        rust_files,
        python_files,
        ts_files,
        manifests,
        packages: n_rust_packages,
        test_files,
        ci_files,
        generated_boundary_files,
        fixture_boundary_present,
        dependency_edges_declared,
    }
}

// ---- measurement ----------------------------------------------------------

fn peak_working_set_bytes() -> Option<u64> {
    #[cfg(target_os = "windows")]
    {
        let pid = std::process::id();
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("(Get-Process -Id {pid}).PeakWorkingSet64"),
            ])
            .output()
            .ok()?;
        String::from_utf8_lossy(&output.stdout).trim().parse().ok()
    }
    #[cfg(target_os = "linux")]
    {
        let status = fs::read_to_string("/proc/self/status").ok()?;
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("VmHWM:") {
                let kb: u64 = rest.trim().trim_end_matches(" kB").trim().parse().ok()?;
                return Some(kb * 1024);
            }
        }
        None
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        None
    }
}

fn dir_size_bytes(path: &Path) -> u64 {
    let mut total = 0u64;
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            total += dir_size_bytes(&entry_path);
        } else if let Ok(metadata) = entry.metadata() {
            total += metadata.len();
        }
    }
    total
}

fn source_size_bytes(root: &Path, repository: &Repository) -> u64 {
    let snapshot = repository.session().snapshot();
    let projection =
        repopact_graph::projection::SourceProjection::build(repository, snapshot.topology());
    projection
        .files
        .iter()
        .map(|file| {
            fs::metadata(root.join(&file.relative_path))
                .map(|metadata| metadata.len())
                .unwrap_or(0)
        })
        .sum()
}

fn run_benchmark(
    root: &Path,
    scale_label: &str,
    composition: Option<Composition>,
) -> serde_json::Value {
    let git_available = init_git_if_needed(root);

    // --- full build ---
    let repository = Repository::open(root);
    let snapshot = repository.session().snapshot();
    let build_start = Instant::now();
    let manifest = repopact_graph::build_and_write(&snapshot).expect("full build");
    let full_build_wall_ms = build_start.elapsed().as_secs_f64() * 1000.0;
    let peak_after_build = peak_working_set_bytes();

    let durable_bytes = dir_size_bytes(&root.join("rog"));
    let source_bytes = source_size_bytes(root, &repository);
    let ratio = if source_bytes > 0 {
        durable_bytes as f64 / source_bytes as f64
    } else {
        0.0
    };

    // --- cold verify/load + query-index construction + representative queries ---
    let status_start = Instant::now();
    let status = repopact_graph::status::status(&repository);
    let cold_status_ms = status_start.elapsed().as_secs_f64() * 1000.0;

    let load_start = Instant::now();
    let loaded = repopact_graph::query::open_durable_graph(root, false)
        .expect("clean load of the graph just built");
    let cold_load_ms = load_start.elapsed().as_secs_f64() * 1000.0;

    let index_start = Instant::now();
    let engine = repopact_graph::query::GraphQueryEngine::new(&loaded.graph, loaded.context);
    let index_build_ms = index_start.elapsed().as_secs_f64() * 1000.0;

    let bounds = repopact_graph::query::QueryBounds::default();
    let representative_node_id = loaded
        .graph
        .nodes
        .keys()
        .next()
        .cloned()
        .unwrap_or_else(|| "repository".to_owned());

    let query_latencies = json!({
        "resolve_ms": time_ms(|| {
            let _ = engine.resolve(
                &repopact_graph::query::NodeSelector::NodeId(representative_node_id.clone()),
                &bounds,
            );
        }),
        "search_ms": time_ms(|| {
            let _ = engine.search("pkg", &bounds);
        }),
        "orient_ms": time_ms(|| {
            let _ = engine.orient(
                &repopact_graph::query::NodeSelector::NodeId(representative_node_id.clone()),
                &bounds,
            );
        }),
        "neighbors_ms": time_ms(|| {
            let _ = engine.neighbors(&representative_node_id, repopact_graph::query::Direction::Both, &bounds);
        }),
        "dependencies_ms": time_ms(|| {
            let _ = engine.dependencies(&representative_node_id, false, &bounds);
        }),
        "dependents_ms": time_ms(|| {
            let _ = engine.dependents(&representative_node_id, &bounds);
        }),
        "impact_ms": time_ms(|| {
            let _ = engine.impact(&representative_node_id, &bounds);
        }),
        "tests_ms": time_ms(|| {
            let _ = engine.tests(&representative_node_id, &bounds);
        }),
        "governance_ms": time_ms(|| {
            let _ = engine.governance(&representative_node_id, &bounds);
        }),
    });

    // --- incremental update by change class ---
    let incremental_results = measure_incremental_classes(root, &repository);

    // --- working-overlay performance ---
    let overlay_results = measure_overlay(root);

    // --- branch/merge rebuild cost (only when git is available) ---
    let merge_results = if git_available {
        Some(measure_branch_merge(root))
    } else {
        None
    };

    json!({
        "generator_version": GENERATOR_VERSION,
        "scale": scale_label,
        "composition": composition.map(|composition| json!({
            "rust_files": composition.rust_files,
            "python_files": composition.python_files,
            "ts_files": composition.ts_files,
            "manifests": composition.manifests,
            "packages": composition.packages,
            "test_files": composition.test_files,
            "ci_files": composition.ci_files,
            "generated_boundary_files": composition.generated_boundary_files,
            "fixture_boundary_present": composition.fixture_boundary_present,
            "dependency_edges_declared": composition.dependency_edges_declared,
        })),
        "platform": std::env::consts::OS,
        "full_build": {
            "wall_ms": full_build_wall_ms,
            "node_count": manifest.node_count,
            "edge_count": manifest.edge_count,
            "shard_count": manifest.shard_count,
            "graph_schema_version": manifest.graph_schema_version,
            "peak_working_set_bytes": peak_after_build,
        },
        "storage": {
            "durable_bytes": durable_bytes,
            "source_bytes": source_bytes,
            "graph_source_ratio": ratio,
            "local_cache_bytes": 0,
            "local_cache_note": "no persistent local acceleration cache exists in this architecture; the durable Git-tracked graph plus a disposable in-memory query index are the only two representations",
        },
        "clean_load": {
            "status_ms": cold_status_ms,
            "durable_load_ms": cold_load_ms,
            "query_index_build_ms": index_build_ms,
            "durable_freshness": format!("{:?}", status.freshness),
        },
        "representative_query_latency_ms": query_latencies,
        "incremental_by_change_class": incremental_results,
        "working_overlay": overlay_results,
        "branch_merge": merge_results,
        "peak_memory_methodology": if cfg!(target_os = "windows") {
            "Windows: Get-Process -Id <pid>.PeakWorkingSet64 (OS-reported peak working set of this process, not a hard allocator limit)"
        } else if cfg!(target_os = "linux") {
            "Linux: /proc/self/status VmHWM (OS-reported peak resident set size, not a hard allocator limit)"
        } else {
            "unsupported platform for this measurement"
        },
    })
}

fn time_ms<F: FnOnce()>(f: F) -> f64 {
    let start = Instant::now();
    f();
    start.elapsed().as_secs_f64() * 1000.0
}

fn init_git_if_needed(root: &Path) -> bool {
    if root.join(".git").exists() {
        return true;
    }
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    };
    if Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        git(&["init", "--quiet"])
            && git(&["config", "user.email", "bench@example.invalid"])
            && git(&["config", "user.name", "Bench"])
            && git(&["add", "-A"])
            && git(&["commit", "--quiet", "-m", "benchmark fixture"])
    } else {
        false
    }
}

fn measure_incremental_classes(root: &Path, repository: &Repository) -> serde_json::Value {
    let snapshot = repository.session().snapshot();
    let projection =
        repopact_graph::projection::SourceProjection::build(repository, snapshot.topology());
    let sample_rust_file = projection
        .files
        .iter()
        .find(|file| file.relative_path.ends_with(".rs") && !file.relative_path.ends_with("lib.rs"))
        .map(|file| file.relative_path.clone());

    let mut classes = BTreeMap::new();

    if let Some(path) = &sample_rust_file {
        classes.insert(
            "non_semantic_text_edit".to_owned(),
            measure_one_change(root, path, |content| {
                format!("// benchmark comment edit\n{content}")
            }),
        );
        classes.insert(
            "semantic_symbol_edit".to_owned(),
            measure_one_change(root, path, |content| {
                format!("{content}\npub fn benchmark_added_function() -> u32 {{ 42 }}\n")
            }),
        );
    }

    classes.insert(
        "single_new_source_file".to_owned(),
        measure_change(
            root,
            |root| {
                fs::write(
                    root.join("crates/pkg_0000/src/benchmark_new_file.rs"),
                    "pub fn added() {}\n",
                )
                .ok();
            },
            |root| {
                let _ = fs::remove_file(root.join("crates/pkg_0000/src/benchmark_new_file.rs"));
            },
        ),
    );

    let manifest_path = root.join("web/package.json");
    let manifest_original = fs::read_to_string(&manifest_path).unwrap_or_default();
    classes.insert(
        "manifest_dependency_edit".to_owned(),
        measure_change(
            root,
            |root| {
                let path = root.join("web/package.json");
                if let Ok(content) = fs::read_to_string(&path) {
                    let edited = content.replace("0.1.0", "0.1.1");
                    let _ = fs::write(&path, edited);
                }
            },
            |root| {
                let _ = fs::write(root.join("web/package.json"), &manifest_original);
            },
        ),
    );

    let ci_path = root.join(".github/workflows/ci.yml");
    let ci_original = fs::read_to_string(&ci_path).unwrap_or_default();
    classes.insert(
        "ci_metadata_edit".to_owned(),
        measure_change(
            root,
            |root| {
                let path = root.join(".github/workflows/ci.yml");
                if let Ok(content) = fs::read_to_string(&path) {
                    let _ = fs::write(&path, format!("{content}      - run: echo benchmark\n"));
                }
            },
            |root| {
                let _ = fs::write(root.join(".github/workflows/ci.yml"), &ci_original);
            },
        ),
    );

    serde_json::to_value(classes).unwrap()
}

fn measure_one_change(
    root: &Path,
    relative_path: &str,
    edit: impl Fn(&str) -> String,
) -> serde_json::Value {
    let absolute = root.join(relative_path);
    let original = fs::read_to_string(&absolute).unwrap_or_default();
    let repository = Repository::open(root);
    let snapshot = repository.session().snapshot();
    fs::write(&absolute, edit(&original)).unwrap();
    let updated_snapshot = Repository::open(root).session().snapshot();
    let start = Instant::now();
    let result =
        repopact_graph::incremental::update(&updated_snapshot).expect("incremental update");
    let wall_ms = start.elapsed().as_secs_f64() * 1000.0;
    fs::write(&absolute, original).unwrap();
    let _ = snapshot;
    json!({
        "wall_ms": wall_ms,
        "mode": format!("{:?}", result.mode),
        "semantic_reparsed": result.semantic_reparsed,
        "semantic_reused": result.semantic_reused,
        "files_modified": result.files_modified,
    })
}

fn measure_change(root: &Path, apply: impl Fn(&Path), revert: impl Fn(&Path)) -> serde_json::Value {
    apply(root);
    let updated_snapshot = Repository::open(root).session().snapshot();
    let start = Instant::now();
    let result =
        repopact_graph::incremental::update(&updated_snapshot).expect("incremental update");
    let wall_ms = start.elapsed().as_secs_f64() * 1000.0;
    revert(root);
    // Re-converge the durable graph to the reverted state so subsequent
    // measurements start clean.
    if let Ok(snapshot) = std::panic::catch_unwind(|| Repository::open(root).session().snapshot()) {
        let _ = repopact_graph::incremental::update(&snapshot);
    }
    json!({
        "wall_ms": wall_ms,
        "mode": format!("{:?}", result.mode),
        "semantic_reparsed": result.semantic_reparsed,
        "semantic_reused": result.semantic_reused,
        "files_added": result.files_added,
        "files_modified": result.files_modified,
        "files_deleted": result.files_deleted,
    })
}

fn measure_overlay(root: &Path) -> serde_json::Value {
    let snapshot = Repository::open(root).session().snapshot();
    let mut overlay = repopact_graph::overlay::SessionGraphState::open(&snapshot);

    let sample_path = "crates/pkg_0000/src/mod_0000.rs";
    let absolute = root.join(sample_path);
    let single_edit_ms = if absolute.exists() {
        let original = fs::read_to_string(&absolute).unwrap();
        fs::write(&absolute, format!("{original}\n// overlay bench edit\n")).unwrap();
        let updated_snapshot = Repository::open(root).session().snapshot();
        let start = Instant::now();
        overlay.reconcile(&updated_snapshot, &[sample_path.to_owned()]);
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        fs::write(&absolute, original).unwrap();
        Some(elapsed)
    } else {
        None
    };

    let refresh_ms = {
        let updated_snapshot = Repository::open(root).session().snapshot();
        let start = Instant::now();
        overlay.refresh(&updated_snapshot);
        start.elapsed().as_secs_f64() * 1000.0
    };

    json!({
        "single_file_watcher_edit_ms": single_edit_ms,
        "explicit_full_session_refresh_ms": refresh_ms,
    })
}

fn measure_branch_merge(root: &Path) -> serde_json::Value {
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    };

    if !git(&["checkout", "-b", "rog-bench-branch-a"]) {
        return json!({"skipped": "could not create branch A"});
    }
    fs::write(
        root.join("crates/pkg_0000/src/branch_a_marker.rs"),
        "pub fn a() {}\n",
    )
    .ok();
    let snapshot_a = Repository::open(root).session().snapshot();
    let _ = repopact_graph::build_and_write(&snapshot_a);
    git(&["add", "-A"]);
    git(&["commit", "--quiet", "-m", "branch a"]);

    if !(git(&["checkout", "main"]) || git(&["checkout", "master"])) {
        return json!({"skipped": "could not return to base branch"});
    }
    if !git(&["checkout", "-b", "rog-bench-branch-b"]) {
        return json!({"skipped": "could not create branch B"});
    }
    fs::write(
        root.join("crates/pkg_0000/src/branch_b_marker.rs"),
        "pub fn b() {}\n",
    )
    .ok();
    let snapshot_b = Repository::open(root).session().snapshot();
    let _ = repopact_graph::build_and_write(&snapshot_b);
    git(&["add", "-A"]);
    git(&["commit", "--quiet", "-m", "branch b"]);

    git(&["checkout", "rog-bench-branch-a"]);
    let _ = git(&["merge", "--no-edit", "rog-bench-branch-b"]);

    let repository = Repository::open(root);
    let snapshot = repository.session().snapshot();
    let start = Instant::now();
    let outcome = merge_reconcile::reconcile_merge(&repository, &snapshot);
    let wall_ms = start.elapsed().as_secs_f64() * 1000.0;

    json!({
        "reconcile_wall_ms": wall_ms,
        "outcome": format!("{outcome:?}").chars().take(80).collect::<String>(),
    })
}

// silence unused-import warning if RepositoryGraph is not otherwise
// referenced under some feature combination
#[allow(dead_code)]
fn _keep_repository_graph_import(_: &RepositoryGraph) {}
