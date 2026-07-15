//! Git co-change mining: files that historically change together.
//!
//! Runs `git log --name-only` over the recent history and counts pairwise
//! co-occurrences of source files within a commit. The result feeds the
//! context ranking as an empirical "hidden coupling" signal that static
//! edges cannot see (config + handler, schema + migration, etc.).
//!
//! Fails soft: repositories without git (or with git unavailable) simply
//! produce no co-change rows.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Maximum commits mined per build (bounds cost on large repos).
const MAX_COMMITS: usize = 500;
/// Commits touching more files than this are skipped (bulk renames/formatting
/// would create a quadratic blowup of meaningless pairs).
const MAX_FILES_PER_COMMIT: usize = 30;
/// Pairs must co-occur at least this often to be stored.
const MIN_PAIR_COUNT: u32 = 2;

/// Mines co-change pairs from git history: `(file_a, file_b, count)` with
/// `file_a < file_b` lexicographically. Returns an empty vector when git is
/// unavailable or the directory is not a repository.
///
/// When `repo_root` is a subdirectory of the git repository, paths are
/// re-rooted to it and files outside are ignored.
pub fn mine_co_change(repo_root: &Path) -> Vec<(String, String, u32)> {
    let prefix = git_prefix(repo_root);
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("log")
        .arg("--name-only")
        .arg("--pretty=format:@@commit@@")
        .arg(format!("--max-count={MAX_COMMITS}"))
        .output();

    let output = match output {
        Ok(out) if out.status.success() => out,
        _ => return Vec::new(),
    };
    let text = String::from_utf8_lossy(&output.stdout);
    mine_from_log(&text, prefix.as_deref())
}

/// Returns the path of `repo_root` relative to the git top level (empty for
/// the top level itself), or `None` when git is unavailable.
fn git_prefix(repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("rev-parse")
        .arg("--show-prefix")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Parses `git log --name-only` output delimited by `@@commit@@` markers.
///
/// `prefix` re-roots paths for sub-directory repositories (files outside the
/// prefix are dropped).
fn mine_from_log(log: &str, prefix: Option<&str>) -> Vec<(String, String, u32)> {
    let prefix = prefix.unwrap_or("");
    let mut pair_counts: HashMap<(String, String), u32> = HashMap::new();

    for commit_block in log.split("@@commit@@") {
        let files: Vec<&str> = commit_block
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && is_source_file(line))
            .filter_map(|line| {
                if prefix.is_empty() {
                    Some(line)
                } else {
                    line.strip_prefix(prefix)
                }
            })
            .collect();
        if files.len() < 2 || files.len() > MAX_FILES_PER_COMMIT {
            continue;
        }
        for i in 0..files.len() {
            for j in (i + 1)..files.len() {
                let (a, b) = if files[i] < files[j] {
                    (files[i], files[j])
                } else {
                    (files[j], files[i])
                };
                *pair_counts
                    .entry((a.to_string(), b.to_string()))
                    .or_insert(0) += 1;
            }
        }
    }

    let mut rows: Vec<(String, String, u32)> = pair_counts
        .into_iter()
        .filter(|(_, count)| *count >= MIN_PAIR_COUNT)
        .map(|((a, b), count)| (a, b, count))
        .collect();
    rows.sort();
    rows
}

fn is_source_file(path: &str) -> bool {
    const EXTENSIONS: &[&str] = &[
        ".rs", ".py", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".go", ".java",
    ];
    EXTENSIONS.iter().any(|ext| path.ends_with(ext))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_pairs_above_threshold() {
        let log = "@@commit@@\nsrc/a.rs\nsrc/b.rs\n@@commit@@\nsrc/a.rs\nsrc/b.rs\n@@commit@@\nsrc/a.rs\nsrc/c.rs\n";
        let rows = mine_from_log(log, None);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], ("src/a.rs".into(), "src/b.rs".into(), 2));
    }

    #[test]
    fn skips_bulk_commits_and_non_source_files() {
        let mut bulk = String::from("@@commit@@\n");
        for i in 0..40 {
            bulk.push_str(&format!("src/f{i}.rs\n"));
        }
        bulk.push_str("@@commit@@\nREADME.md\nsrc/a.rs\n");
        let rows = mine_from_log(&bulk, None);
        assert!(rows.is_empty());
    }

    #[test]
    fn reroots_paths_for_subdirectory_repos() {
        let log = "@@commit@@\ndemo/src/a.rs\ndemo/src/b.rs\nother/x.rs\n@@commit@@\ndemo/src/a.rs\ndemo/src/b.rs\n";
        let rows = mine_from_log(log, Some("demo/"));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], ("src/a.rs".into(), "src/b.rs".into(), 2));
    }
}
