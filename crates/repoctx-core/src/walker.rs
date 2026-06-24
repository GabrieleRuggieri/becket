//! Repository file walker respecting `.gitignore` and `.repoctxignore`.

use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use sha2::{Digest, Sha256};

use crate::error::CoreError;
use crate::language::Language;
use crate::parse::GrammarRegistry;

/// A source file discovered under the repository root.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// Path relative to the repository root (POSIX separators).
    pub relative_path: String,
    /// Absolute path on disk.
    pub absolute_path: PathBuf,
    /// Detected programming language.
    pub language: Language,
    /// Hex-encoded SHA-256 of file contents.
    pub content_hash: String,
    /// Last modification time in whole seconds since UNIX epoch.
    pub mtime_secs: i64,
}

/// Walks analyzable source files under `root`.
pub struct FileWalker {
    root: PathBuf,
    registry: GrammarRegistry,
}

impl FileWalker {
    /// Creates a walker for the given repository root.
    ///
    /// # Arguments
    ///
    /// * `root` - Repository root directory.
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let registry = GrammarRegistry::builtins()
            .load_overrides_from_file(&root.join("repoctx.languages.toml"))
            .unwrap_or_else(|_| GrammarRegistry::builtins());
        Self { root, registry }
    }

    /// Discovers source files, skipping ignored paths.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Io`] if walking or hashing fails.
    pub fn discover(&self) -> Result<Vec<SourceFile>, CoreError> {
        let mut builder = WalkBuilder::new(&self.root);
        builder.hidden(false);
        builder.git_ignore(true);
        builder.git_global(true);
        builder.git_exclude(true);
        builder.filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            if name == ".repoctx" {
                return false;
            }
            true
        });

        let repoctxignore = self.root.join(".repoctxignore");
        if repoctxignore.exists() {
            builder.add_custom_ignore_filename(".repoctxignore");
        }

        let mut files = Vec::new();
        for entry in builder.build() {
            let entry = entry.map_err(|e| CoreError::Walk(e.to_string()))?;
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }

            let absolute_path = entry.into_path();
            let language = self.registry.detect_language(&absolute_path);
            if !language.is_supported() {
                continue;
            }

            let relative_path = absolute_path
                .strip_prefix(&self.root)
                .map_err(|_| {
                    CoreError::InvalidRepository(format!(
                        "path escapes repository root: {}",
                        absolute_path.display()
                    ))
                })?
                .to_string_lossy()
                .replace('\\', "/");

            let metadata = fs::metadata(&absolute_path)?;
            let mtime_secs = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            let contents = fs::read(&absolute_path)?;
            let mut hasher = Sha256::new();
            hasher.update(&contents);
            let content_hash = hex_encode(hasher.finalize());

            files.push(SourceFile {
                relative_path,
                absolute_path,
                language,
                content_hash,
                mtime_secs,
            });
        }

        files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        Ok(files)
    }
}

fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}
