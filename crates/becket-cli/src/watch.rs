//! `becket build --watch`: debounced incremental rebuilds on file changes.

use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEvent};
use becket_core::{BuildOptions, BuildPipeline, BuildReport};
use becket_schema::wiki::WikiStaleQueue;
use tracing::info;

const DEBOUNCE_MS: u64 = 400;

/// Returns true when a filesystem event should trigger a rebuild.
pub fn should_trigger_rebuild(path: &Path, repo_root: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(repo_root) else {
        return false;
    };
    for component in relative.components() {
        let name = component.as_os_str().to_string_lossy();
        if matches!(
            name.as_ref(),
            ".becket" | ".git" | "target" | "node_modules" | "dist" | "build"
        ) {
            return false;
        }
    }
    true
}

/// Runs an initial build, then rebuilds incrementally when relevant files change.
pub fn run(repo: &Path, options: BuildOptions, json: bool) -> Result<()> {
    let repo = repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf());
    run_build(&repo, &options, json)?;

    let (tx, rx) = mpsc::channel();
    let watch_root = repo.clone();
    let mut debouncer = new_debouncer(
        Duration::from_millis(DEBOUNCE_MS),
        move |result: notify_debouncer_mini::DebounceEventResult| {
            let Ok(events) = result else {
                return;
            };
            if events
                .iter()
                .any(|event| should_trigger_rebuild(event_path(event), &watch_root))
            {
                let _ = tx.send(());
            }
        },
    )
    .context("failed to create filesystem watcher")?;

    debouncer
        .watcher()
        .watch(&repo, RecursiveMode::Recursive)
        .context("failed to watch repository")?;

    eprintln!("watching {} for changes (Ctrl+C to stop)", repo.display());

    while let Ok(()) = rx.recv() {
        while rx.try_recv().is_ok() {}
        info!("change detected, rebuilding");
        run_build(&repo, &options, json)?;
    }
    Ok(())
}

fn event_path(event: &DebouncedEvent) -> &Path {
    &event.path
}

fn run_build(repo: &Path, options: &BuildOptions, json: bool) -> Result<()> {
    let pipeline = BuildPipeline::new(repo, options.clone());
    let report = pipeline.run().context("build failed")?;
    print_report(&report, json);
    Ok(())
}

fn print_report(report: &BuildReport, json: bool) {
    if json {
        if let Ok(text) = serde_json::to_string_pretty(report) {
            println!("{text}");
        }
    } else {
        println!(
            "build complete: {} parsed, {} skipped, {} symbols, {} edges, {} flows, {} wiki pages, {} embeddings → {}",
            report.files_parsed,
            report.files_skipped,
            report.symbols_indexed,
            report.edges_indexed,
            report.flows_indexed,
            report.wiki_pages_indexed,
            report.embeddings_indexed,
            report.output_dir
        );
        if let Some(stale) = read_stale_count(Path::new(&report.output_dir)) {
            if stale > 0 {
                eprintln!("wiki: {stale} stale page(s) queued — run `becket wiki sync`");
            }
        }
    }
}

fn read_stale_count(becket_dir: &Path) -> Option<usize> {
    let path = becket_dir.join("wiki_stale.json");
    let raw = fs::read_to_string(path).ok()?;
    let queue: WikiStaleQueue = serde_json::from_str(&raw).ok()?;
    Some(queue.page_ids.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_becket_and_git_paths() {
        let root = std::path::PathBuf::from("/repo");
        assert!(!should_trigger_rebuild(
            &std::path::PathBuf::from("/repo/.becket/index.db"),
            &root
        ));
        assert!(!should_trigger_rebuild(
            &std::path::PathBuf::from("/repo/.git/HEAD"),
            &root
        ));
        assert!(should_trigger_rebuild(
            &std::path::PathBuf::from("/repo/src/main.rs"),
            &root
        ));
    }
}
