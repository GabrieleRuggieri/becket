//! CI latency budget guards (ARCHITECTURE.md targets).
//!
//! Product targets: incremental rebuild < 200 ms, query p95 < 100 ms on a warm index.
//! CI uses looser ceilings on shared runners; run with:
//! `cargo test -p becket-core --test bench_budget -- --ignored`

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use becket_core::{BuildOptions, BuildPipeline};
use becket_query::QueryEngine;
use tempfile::TempDir;

/// Incremental rebuild after a single file change (warm index).
const INCREMENTAL_BUDGET: Duration = Duration::from_millis(200);
/// p95 warm query latency (`impact` / `context` / `flow`).
const QUERY_P95_BUDGET: Duration = Duration::from_millis(100);
/// Slack multiplier for shared CI runners (GitHub Actions ubuntu-latest, etc.).
const CI_BUDGET_MULTIPLIER: f64 = 2.5;
const QUERY_SAMPLES: usize = 40;
const INCREMENTAL_SAMPLES: usize = 3;

fn running_on_ci() -> bool {
    std::env::var_os("CI").is_some()
}

fn budget_with_ci_slack(target: Duration) -> Duration {
    if running_on_ci() {
        Duration::from_secs_f64(target.as_secs_f64() * CI_BUDGET_MULTIPLIER)
    } else {
        target
    }
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

struct FixtureWorkdir {
    _temp: TempDir,
    root: PathBuf,
}

fn isolated_fixture(name: &str) -> FixtureWorkdir {
    let src = fixture_path(name);
    let temp = tempfile::tempdir().expect("tempdir");
    copy_dir_all(&src, temp.path()).expect("copy fixture");
    let becket = temp.path().join(".becket");
    if becket.exists() {
        fs::remove_dir_all(&becket).expect("remove stale .becket");
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

fn percentile_ms(samples: &mut [Duration], pct: f64) -> Duration {
    samples.sort();
    let index = ((samples.len() as f64) * pct).ceil() as usize;
    let index = index.saturating_sub(1).min(samples.len() - 1);
    samples[index]
}

#[test]
#[ignore = "latency budget guard; run: cargo test -p becket-core --test bench_budget -- --ignored"]
fn incremental_rebuild_stays_within_budget() {
    let work = isolated_fixture("bench-small");
    BuildPipeline::new(
        &work.root,
        BuildOptions {
            incremental: true,
            no_embeddings: true,
        },
    )
    .run()
    .expect("initial build");

    let target = work.root.join("src/services/billing.rs");
    let budget = budget_with_ci_slack(INCREMENTAL_BUDGET);
    let mut best = Duration::MAX;

    for sample in 0..INCREMENTAL_SAMPLES {
        let mut content = fs::read_to_string(&target).expect("read billing");
        content.push_str(&format!("\n// touch-{sample}\n"));
        fs::write(&target, content).expect("touch file");

        let start = Instant::now();
        BuildPipeline::new(
            &work.root,
            BuildOptions {
                incremental: true,
                no_embeddings: true,
            },
        )
        .run()
        .expect("incremental build");
        best = best.min(start.elapsed());
    }

    assert!(
        best <= budget,
        "incremental rebuild took {best:?}, budget {budget:?} (target {INCREMENTAL_BUDGET:?})"
    );
}

#[test]
#[ignore = "latency budget guard; run: cargo test -p becket-core --test bench_budget -- --ignored"]
fn warm_queries_p95_stays_within_budget() {
    let work = isolated_fixture("bench-small");
    BuildPipeline::new(
        &work.root,
        BuildOptions {
            incremental: false,
            no_embeddings: true,
        },
    )
    .run()
    .expect("warm build");

    let engine = QueryEngine::new(&work.root);
    let mut samples = Vec::with_capacity(QUERY_SAMPLES);

    for i in 0..QUERY_SAMPLES {
        let start = Instant::now();
        match i % 3 {
            0 => {
                engine.impact("capture", 3).expect("impact");
            }
            1 => {
                engine
                    .context("register", Some(6000), becket_query::ContextTask::Fix)
                    .expect("context");
            }
            _ => {
                engine.flow("payment").expect("flow");
            }
        }
        samples.push(start.elapsed());
    }

    let p95 = percentile_ms(&mut samples, 0.95);
    let budget = budget_with_ci_slack(QUERY_P95_BUDGET);
    assert!(
        p95 <= budget,
        "query p95 took {p95:?}, budget {budget:?} (target {QUERY_P95_BUDGET:?})"
    );
}
