//! CI latency budget guards (ARCHITECTURE.md targets).

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
const QUERY_SAMPLES: usize = 40;

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
#[cfg_attr(
    windows,
    ignore = "latency budget guards target tier-1 platforms (see docs/windows.md)"
)]
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
    let mut content = fs::read_to_string(&target).expect("read billing");
    content.push_str("\n// touch\n");
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
    let elapsed = start.elapsed();

    assert!(
        elapsed <= INCREMENTAL_BUDGET,
        "incremental rebuild took {elapsed:?}, budget {INCREMENTAL_BUDGET:?}"
    );
}

#[test]
#[cfg_attr(
    windows,
    ignore = "latency budget guards target tier-1 platforms (see docs/windows.md)"
)]
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
    assert!(
        p95 <= QUERY_P95_BUDGET,
        "query p95 took {p95:?}, budget {QUERY_P95_BUDGET:?}"
    );
}
