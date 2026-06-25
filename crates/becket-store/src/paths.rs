//! Canonical paths for Becket output under a repository root.

use std::path::{Path, PathBuf};

/// Resolved paths for `.becket/` output and the embedded index database.
#[derive(Debug, Clone)]
pub struct BecketPaths {
    /// Repository root directory.
    pub root: PathBuf,
    /// `.becket/` output directory.
    pub output_dir: PathBuf,
    /// SQLite index database path.
    pub index_db: PathBuf,
}

impl BecketPaths {
    /// Creates path helpers for the given repository root.
    ///
    /// # Arguments
    ///
    /// * `root` - Absolute or relative path to the repository root.
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let output_dir = root.join(".becket");
        let index_db = output_dir.join("index.db");
        Self {
            root,
            output_dir,
            index_db,
        }
    }

    /// Returns the path for a named JSON artifact file.
    pub fn artifact(&self, name: &str) -> PathBuf {
        self.output_dir.join(format!("{name}.json"))
    }

    /// Returns `.becket/wiki/` grounded knowledge pages directory.
    pub fn wiki_dir(&self) -> PathBuf {
        self.output_dir.join("wiki")
    }

    /// Returns `.becket/wiki_stale.json` sync queue path.
    pub fn wiki_stale_queue(&self) -> PathBuf {
        self.output_dir.join("wiki_stale.json")
    }

    /// Returns `.becket/wiki_lint.json` lint report path.
    pub fn wiki_lint_report(&self) -> PathBuf {
        self.output_dir.join("wiki_lint.json")
    }
}
