//! `becket bench --from-git`: reproducible retrieval benchmark.
//!
//! Uses real commits as ground truth: files changed together in one commit
//! approximate "the context a developer actually needed". For each recent
//! commit we pick a seed symbol from one touched file, assemble a context
//! bundle, and measure how many of the commit's *other* touched files the
//! bundle surfaces (recall) and how many tokens it costs versus handing the
//! agent every touched file in full.
//!
//! Anyone can validate Becket on their own repository:
//! `becket build && becket bench --last 30`.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use becket_store::{BecketPaths, IndexStore};
use serde::{Deserialize, Serialize};

use crate::assemble::{assemble_context_with_options, AssembleOptions};
use crate::budget::estimate_tokens;
use crate::error::QueryError;
use crate::types::ContextTask;

/// Benchmark output (`.becket/report/bench.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchReport {
    /// Commits requested.
    pub commits_requested: usize,
    /// Commits actually evaluated (≥ 2 indexed files touched).
    pub commits_evaluated: usize,
    /// Mean file recall across evaluated commits (0..1).
    pub mean_recall: f64,
    /// Mean token cost ratio: bundle / full-touched-files (lower is better).
    pub mean_token_ratio: f64,
    /// Per-commit results.
    pub cases: Vec<BenchCase>,
}

/// One evaluated commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchCase {
    /// Abbreviated commit id.
    pub commit: String,
    /// Seed symbol used for context assembly.
    pub seed_symbol: String,
    /// Files touched by the commit (indexed source files only).
    pub touched_files: usize,
    /// Touched files (other than the seed's) surfaced by the bundle.
    pub recalled_files: usize,
    /// Recall for this commit (0..1).
    pub recall: f64,
    /// Bundle tokens.
    pub bundle_tokens: u32,
    /// Tokens of all touched files in full.
    pub full_files_tokens: u32,
}

/// Runs the benchmark and writes `bench.json` under `.becket/report/`.
pub fn run_bench(repo_root: &Path, last: usize) -> Result<BenchReport, QueryError> {
    let paths = BecketPaths::new(repo_root);
    if !paths.index_db.exists() {
        return Err(QueryError::IndexMissing(
            paths.index_db.display().to_string(),
        ));
    }
    let store = IndexStore::open(&paths.index_db)?;

    let symbols = store.load_symbols()?;
    let symbols_by_file: HashMap<&str, Vec<&becket_schema::artifacts::SymbolRecord>> = {
        let mut map: HashMap<&str, Vec<_>> = HashMap::new();
        for symbol in &symbols {
            map.entry(symbol.file_path.as_str())
                .or_default()
                .push(symbol);
        }
        map
    };
    let id_to_file: HashMap<&str, &str> = symbols
        .iter()
        .map(|s| (s.id.as_str(), s.file_path.as_str()))
        .collect();
    let in_degrees = store.symbol_in_degrees()?;

    let commits = recent_commits(repo_root, last);
    let mut cases = Vec::new();

    for (commit, files) in &commits {
        // Only files present in the index count as ground truth.
        let indexed_files: Vec<&str> = files
            .iter()
            .map(String::as_str)
            .filter(|f| symbols_by_file.contains_key(f))
            .collect();
        if indexed_files.len() < 2 {
            continue;
        }

        // Seed: most-referenced symbol in the first touched file (deterministic).
        let seed_file = indexed_files[0];
        let seed = symbols_by_file[seed_file]
            .iter()
            .max_by(|a, b| {
                let da = in_degrees.get(&a.id).copied().unwrap_or(0);
                let db = in_degrees.get(&b.id).copied().unwrap_or(0);
                da.cmp(&db).then_with(|| b.id.cmp(&a.id))
            })
            .copied();
        let Some(seed) = seed else { continue };

        // Default task budget: measures what an agent gets in real usage,
        // not an unbounded auto-budget bundle.
        let Ok(context) = assemble_context_with_options(
            &store,
            repo_root,
            seed.clone(),
            AssembleOptions {
                budget: Some(ContextTask::Fix.default_budget()),
                task: ContextTask::Fix,
                plan_only: false,
            },
        ) else {
            continue;
        };

        // Files the bundle surfaces (snippets, tests, impact symbols).
        let mut surfaced: HashSet<&str> = HashSet::new();
        for snippet in &context.snippets {
            surfaced.insert(snippet.file_path.as_str());
        }
        for test in &context.related_tests {
            surfaced.insert(test.as_str());
        }
        for id in &context.affected_symbol_ids {
            if let Some(file) = id_to_file.get(id.as_str()) {
                surfaced.insert(file);
            }
        }

        let targets: Vec<&str> = indexed_files
            .iter()
            .copied()
            .filter(|f| *f != seed_file)
            .collect();
        let recalled = targets.iter().filter(|f| surfaced.contains(**f)).count();
        let recall = recalled as f64 / targets.len() as f64;

        let bundle_tokens = estimate_tokens(&context.markdown);
        let mut full_files_tokens = 0u32;
        for file in &indexed_files {
            if let Ok(content) = fs::read_to_string(repo_root.join(file)) {
                full_files_tokens += estimate_tokens(&content);
            }
        }

        cases.push(BenchCase {
            commit: commit.chars().take(10).collect(),
            seed_symbol: seed.name.clone(),
            touched_files: indexed_files.len(),
            recalled_files: recalled,
            recall,
            bundle_tokens,
            full_files_tokens,
        });
    }

    let evaluated = cases.len();
    let mean_recall = if evaluated > 0 {
        cases.iter().map(|c| c.recall).sum::<f64>() / evaluated as f64
    } else {
        0.0
    };
    let mean_token_ratio = if evaluated > 0 {
        cases
            .iter()
            .filter(|c| c.full_files_tokens > 0)
            .map(|c| f64::from(c.bundle_tokens) / f64::from(c.full_files_tokens))
            .sum::<f64>()
            / evaluated as f64
    } else {
        0.0
    };

    let report = BenchReport {
        commits_requested: last,
        commits_evaluated: evaluated,
        mean_recall,
        mean_token_ratio,
        cases,
    };

    let report_dir = paths.output_dir.join("report");
    fs::create_dir_all(&report_dir)?;
    let json =
        serde_json::to_string_pretty(&report).map_err(|e| QueryError::Internal(e.to_string()))?;
    fs::write(report_dir.join("bench.json"), json)?;

    Ok(report)
}

/// Returns `(commit_hash, touched_files)` for the last `n` commits.
fn recent_commits(repo_root: &Path, n: usize) -> Vec<(String, Vec<String>)> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("log")
        .arg("--name-only")
        .arg("--pretty=format:@@commit@@ %H")
        .arg(format!("--max-count={n}"))
        .output();
    let output = match output {
        Ok(out) if out.status.success() => out,
        _ => return Vec::new(),
    };
    let text = String::from_utf8_lossy(&output.stdout);

    let mut commits = Vec::new();
    for block in text.split("@@commit@@") {
        let mut lines = block.lines().map(str::trim).filter(|l| !l.is_empty());
        let Some(hash) = lines.next() else { continue };
        let files: Vec<String> = lines.map(str::to_string).collect();
        if !files.is_empty() {
            commits.push((hash.to_string(), files));
        }
    }
    commits
}
