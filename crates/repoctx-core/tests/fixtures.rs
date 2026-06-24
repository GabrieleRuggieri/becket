//! Integration tests against synthetic fixtures under `tests/fixtures/`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use repoctx_core::{BuildOptions, BuildPipeline};
use repoctx_query::QueryEngine;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
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

#[test]
fn rebuild_produces_byte_identical_artifacts() {
    let root = fixture_path("monorepo-edges");
    let options = BuildOptions {
        incremental: false,
        no_embeddings: true,
    };

    BuildPipeline::new(&root, options.clone())
        .run()
        .expect("first build");
    let first = read_artifacts(&root);

    BuildPipeline::new(&root, options)
        .run()
        .expect("second build");
    let second = read_artifacts(&root);

    assert_eq!(first, second, "artifacts must be deterministic across rebuilds");
}

#[test]
fn build_monorepo_edges_resolves_call_chain() {
    let root = fixture_path("monorepo-edges");
    let report = BuildPipeline::new(
        &root,
        BuildOptions {
            incremental: false,
            no_embeddings: true,
        },
    )
    .run()
    .expect("build should succeed");

    assert!(report.symbols_indexed >= 3);
    assert!(report.edges_indexed >= 2, "expected a->b->c edges");

    let engine = QueryEngine::new(&root);
    let impact = engine.impact("func_a", 3).expect("impact query");
    assert!(
        impact.affected_symbol_ids.len() >= 2,
        "func_a should reach func_b and func_c"
    );
}

#[test]
fn build_tiny_rust_indexes_symbols() {
    let root = fixture_path("tiny-rust");
    let report = BuildPipeline::new(
        &root,
        BuildOptions {
            incremental: false,
            no_embeddings: true,
        },
    )
    .run()
    .expect("build should succeed");

    assert!(report.symbols_indexed >= 2);

    let engine = QueryEngine::new(&root);
    let ctx = engine.context("Greeter", None).expect("context query");
    assert_eq!(ctx.symbol.name, "Greeter");
}

#[test]
fn build_tiny_python_detects_main_entrypoint() {
    let root = fixture_path("tiny-python");
    let report = BuildPipeline::new(
        &root,
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
    let root = fixture_path("flows-payment");
    let report = BuildPipeline::new(
        &root,
        BuildOptions {
            incremental: false,
            no_embeddings: true,
        },
    )
    .run()
    .expect("build should succeed");

    assert!(report.flows_indexed >= 1);

    let engine = QueryEngine::new(&root);
    let flow = engine.flow("payment").expect("flow query");
    assert!(flow.flow.is_some(), "payment flow should exist");
    let flow = flow.flow.unwrap();
    assert!(flow.steps.len() >= 2);
}
