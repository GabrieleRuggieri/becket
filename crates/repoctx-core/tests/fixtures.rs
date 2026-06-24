//! Integration tests against synthetic fixtures under `tests/fixtures/`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use repoctx_core::{BuildOptions, BuildPipeline, DomainEditor};
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
    let artifacts = read_artifacts(root);
    for name in ARTIFACT_NAMES {
        let filename = format!("{name}.json");
        let json = artifacts
            .get(&filename)
            .unwrap_or_else(|| panic!("missing {filename}"));
        validate_artifact_json(name, json)
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
    let ctx = engine.context("Greeter", None).expect("context query");
    assert_eq!(ctx.symbol.name, "Greeter");
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
