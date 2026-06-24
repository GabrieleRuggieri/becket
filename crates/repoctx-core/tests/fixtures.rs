//! Integration tests against synthetic fixtures under `tests/fixtures/`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use repoctx_core::{BuildOptions, BuildPipeline, DomainEditor, WorkspacePipeline};
use repoctx_query::QueryEngine;
use repoctx_schema::{validate_artifact_json, ARTIFACT_NAMES};
use tempfile::TempDir;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

/// Isolated copy of a fixture so parallel tests do not share `.repoctx/`.
struct FixtureWorkdir {
    _temp: TempDir,
    root: PathBuf,
}

fn isolated_fixture(name: &str) -> FixtureWorkdir {
    let src = fixture_path(name);
    let temp = tempfile::tempdir().expect("tempdir");
    copy_dir_all(&src, temp.path()).expect("copy fixture");
    let repoctx = temp.path().join(".repoctx");
    if repoctx.exists() {
        fs::remove_dir_all(&repoctx).expect("remove stale .repoctx");
    }
    FixtureWorkdir {
        root: temp.path().to_path_buf(),
        _temp: temp,
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn read_artifacts(root: &Path) -> HashMap<String, String> {
    let dir = root.join(".repoctx");
    let mut out = HashMap::new();
    for name in [
        "symbols.json",
        "dependencies.json",
        "flows.json",
        "entrypoints.json",
        "architecture.json",
    ] {
        let path = dir.join(name);
        out.insert(
            name.to_string(),
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}")),
        );
    }
    out
}

fn validate_artifacts(root: &Path) {
    let dir = root.join(".repoctx");
    for name in ARTIFACT_NAMES {
        let filename = format!("{name}.json");
        let path = dir.join(&filename);
        if !path.is_file() {
            if *name == "cross_repo" {
                continue;
            }
            panic!("missing {filename}");
        }
        let json = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        validate_artifact_json(name, &json)
            .unwrap_or_else(|e| panic!("{filename} failed schema validation: {e}"));
    }
}

#[test]
fn build_outputs_validate_against_json_schema() {
    for fixture in [
        "monorepo-edges",
        "tiny-rust",
        "tiny-python",
        "flows-payment",
    ] {
        let work = isolated_fixture(fixture);
        BuildPipeline::new(
            &work.root,
            BuildOptions {
                incremental: false,
                no_embeddings: true,
            },
        )
        .run()
        .unwrap_or_else(|e| panic!("build {fixture}: {e}"));
        validate_artifacts(&work.root);
    }
}

#[test]
fn rebuild_produces_byte_identical_artifacts() {
    let work = isolated_fixture("monorepo-edges");
    let options = BuildOptions {
        incremental: false,
        no_embeddings: true,
    };

    BuildPipeline::new(&work.root, options.clone())
        .run()
        .expect("first build");
    let first = read_artifacts(&work.root);

    BuildPipeline::new(&work.root, options)
        .run()
        .expect("second build");
    let second = read_artifacts(&work.root);

    assert_eq!(
        first, second,
        "artifacts must be deterministic across rebuilds"
    );
}

#[test]
fn build_monorepo_edges_resolves_call_chain() {
    let work = isolated_fixture("monorepo-edges");
    let report = BuildPipeline::new(
        &work.root,
        BuildOptions {
            incremental: false,
            no_embeddings: true,
        },
    )
    .run()
    .expect("build should succeed");

    assert!(report.symbols_indexed >= 3);
    assert!(report.edges_indexed >= 2, "expected a->b->c edges");

    let engine = QueryEngine::new(&work.root);
    let impact = engine.impact("func_a", 3).expect("impact query");
    assert!(
        impact.affected_symbol_ids.len() >= 2,
        "func_a should reach func_b and func_c"
    );
}

#[test]
fn build_tiny_rust_indexes_symbols() {
    let work = isolated_fixture("tiny-rust");
    let report = BuildPipeline::new(
        &work.root,
        BuildOptions {
            incremental: false,
            no_embeddings: true,
        },
    )
    .run()
    .expect("build should succeed");

    assert!(report.symbols_indexed >= 2);

    let engine = QueryEngine::new(&work.root);
    let ctx = engine
        .context("Greeter", Some(6000), repoctx_query::ContextTask::Fix)
        .expect("context query");
    assert_eq!(ctx.symbol.name, "Greeter");
    assert!(!ctx.snippets.is_empty() || ctx.markdown.contains("Greeter"));
    assert!(ctx.markdown.contains("# Context: Greeter"));
}

#[test]
fn context_assembly_includes_code_snippets() {
    let work = isolated_fixture("flows-payment");
    BuildPipeline::new(
        &work.root,
        BuildOptions {
            incremental: false,
            no_embeddings: true,
        },
    )
    .run()
    .expect("build");

    let engine = QueryEngine::new(&work.root);
    let ctx = engine
        .context("checkout", Some(8000), repoctx_query::ContextTask::Fix)
        .expect("context");

    assert!(!ctx.markdown.is_empty());
    assert!(ctx.markdown.contains("## Code"));
    assert!(!ctx.snippets.is_empty(), "expected real source snippets");
    assert!(
        ctx.snippets
            .iter()
            .any(|s| s.content.contains("fn") || s.content.contains("def")),
        "snippet should contain source"
    );
}

#[test]
fn build_tiny_python_detects_main_entrypoint() {
    let work = isolated_fixture("tiny-python");
    let report = BuildPipeline::new(
        &work.root,
        BuildOptions {
            incremental: false,
            no_embeddings: true,
        },
    )
    .run()
    .expect("build should succeed");

    assert!(report.entrypoints_indexed >= 1);
}

#[test]
fn build_flows_payment_discovers_flow() {
    let work = isolated_fixture("flows-payment");
    let report = BuildPipeline::new(
        &work.root,
        BuildOptions {
            incremental: false,
            no_embeddings: true,
        },
    )
    .run()
    .expect("build should succeed");

    assert!(report.flows_indexed >= 1);

    let engine = QueryEngine::new(&work.root);
    let flow = engine.flow("payment").expect("flow query");
    assert!(flow.flow.is_some(), "payment flow should exist");
    let flow = flow.flow.unwrap();
    assert!(flow.steps.len() >= 2);
}

#[test]
fn domain_rename_persists_and_survives_rebuild() {
    let work = isolated_fixture("flows-payment");
    BuildPipeline::new(
        &work.root,
        BuildOptions {
            incremental: false,
            no_embeddings: true,
        },
    )
    .run()
    .expect("build");

    let engine = QueryEngine::new(&work.root);
    let before = engine.flow("payment").expect("flow").flow.expect("payment");
    let flow_id = before.id.clone();

    let editor = DomainEditor::new(&work.root);
    editor
        .rename(&flow_id, "billing")
        .expect("rename should succeed");

    let after = engine
        .flow("billing")
        .expect("query")
        .flow
        .expect("billing");
    assert_eq!(after.name, "billing");

    BuildPipeline::new(
        &work.root,
        BuildOptions {
            incremental: false,
            no_embeddings: true,
        },
    )
    .run()
    .expect("rebuild");

    let rebuilt = engine
        .flow("billing")
        .expect("query")
        .flow
        .expect("billing");
    assert_eq!(rebuilt.name, "billing");
}

#[test]
fn domain_add_attaches_symbols_and_rebuilds_flow() {
    let work = isolated_fixture("flows-payment");
    BuildPipeline::new(
        &work.root,
        BuildOptions {
            incremental: false,
            no_embeddings: true,
        },
    )
    .run()
    .expect("build");

    let editor = DomainEditor::new(&work.root);
    let flow = editor
        .add(
            "checkout-flow",
            &[
                "src/payment/**".to_string(),
                "checkout".to_string(),
                "charge_card".to_string(),
            ],
        )
        .expect("domain add");

    assert_eq!(flow.name, "checkout-flow");
    assert!(flow.steps.len() >= 2);
}

#[test]
fn build_inheritance_fixture_resolves_extends_and_implements() {
    let work = isolated_fixture("inheritance");
    let report = BuildPipeline::new(
        &work.root,
        BuildOptions {
            incremental: false,
            no_embeddings: true,
        },
    )
    .run()
    .expect("build");

    assert!(
        report.edges_indexed >= 4,
        "expected extends/implements edges across Rust, TS, and Java"
    );

    let deps = fs::read_to_string(work.root.join(".repoctx/dependencies.json")).expect("deps");
    assert!(deps.contains("\"extends\""));
    assert!(deps.contains("\"implements\""));
}

#[test]
fn build_http_routes_fixture_detects_express_flask_and_spring() {
    let work = isolated_fixture("http-routes");
    let report = BuildPipeline::new(
        &work.root,
        BuildOptions {
            incremental: false,
            no_embeddings: true,
        },
    )
    .run()
    .expect("build");

    assert!(
        report.entrypoints_indexed >= 4,
        "expected HTTP routes from TS, Python, and Java fixtures"
    );

    let entrypoints =
        fs::read_to_string(work.root.join(".repoctx/entrypoints.json")).expect("entrypoints");
    assert!(entrypoints.contains("\"http\""));
    assert!(entrypoints.contains("GET /users"));
    assert!(entrypoints.contains("GET /health"));
    assert!(entrypoints.contains("GET /items"));
}

#[test]
fn build_with_embeddings_indexes_symbol_vectors() {
    let work = isolated_fixture("bench-small");
    let report = BuildPipeline::new(
        &work.root,
        BuildOptions {
            incremental: false,
            no_embeddings: false,
        },
    )
    .run()
    .expect("build");

    assert!(
        report.embeddings_indexed >= report.symbols_indexed,
        "expected an embedding per symbol"
    );

    let engine = QueryEngine::new(&work.root);
    let ctx = engine
        .context("capture", Some(6000), repoctx_query::ContextTask::Fix)
        .expect("context");
    assert!(
        !ctx.semantic_neighbors.is_empty(),
        "payment-related symbols should cluster"
    );
}

#[test]
fn workspace_build_links_http_cross_repo_edges() {
    let work = isolated_fixture("workspace");
    let report = WorkspacePipeline::new(
        &work.root,
        BuildOptions {
            incremental: false,
            no_embeddings: true,
        },
    )
    .run()
    .expect("workspace build");

    assert_eq!(report.repos.len(), 2);
    assert!(
        report.cross_repo_edges >= 1,
        "expected HTTP cross-repo edge between gateway and users"
    );

    let cross_repo_path = work.root.join(".repoctx/cross_repo.json");
    let cross_repo_json = fs::read_to_string(&cross_repo_path).expect("cross_repo.json");
    repoctx_schema::validate_artifact_json("cross_repo", &cross_repo_json).expect("schema");
    assert!(cross_repo_json.contains("edgeType"));
    assert!(cross_repo_json.contains("\"http\""));
    assert!(cross_repo_json.contains("gateway"));
    assert!(cross_repo_json.contains("users"));
}

#[test]
fn workspace_build_links_grpc_and_queue_edges() {
    let work = isolated_fixture("workspace-messaging");
    let report = WorkspacePipeline::new(
        &work.root,
        BuildOptions {
            incremental: false,
            no_embeddings: true,
        },
    )
    .run()
    .expect("workspace build");

    assert_eq!(report.repos.len(), 4);
    assert!(
        report.cross_repo_edges >= 2,
        "expected gRPC and queue cross-repo edges"
    );

    let cross_repo_path = work.root.join(".repoctx/cross_repo.json");
    let cross_repo_json = fs::read_to_string(&cross_repo_path).expect("cross_repo.json");
    repoctx_schema::validate_artifact_json("cross_repo", &cross_repo_json).expect("schema");
    assert!(cross_repo_json.contains("\"grpc\""));
    assert!(cross_repo_json.contains("\"queue\""));
    assert!(cross_repo_json.contains("orders.created"));
    assert!(cross_repo_json.contains("UserService"));
}
