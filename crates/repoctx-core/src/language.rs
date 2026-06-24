//! Language detection from file extensions via the grammar registry.

use std::path::Path;

use crate::parse::GrammarRegistry;

/// Supported language identifiers for the v1 heuristic extractor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    /// Rust source files.
    Rust,
    /// TypeScript source files.
    TypeScript,
    /// JavaScript source files.
    JavaScript,
    /// Python source files.
    Python,
    /// Go source files.
    Go,
    /// Java source files.
    Java,
    /// Unknown or unsupported language.
    Unknown,
}

impl Language {
    /// Returns the canonical language id stored in the index.
    pub fn id(self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::TypeScript => "typescript",
            Language::JavaScript => "javascript",
            Language::Python => "python",
            Language::Go => "go",
            Language::Java => "java",
            Language::Unknown => "unknown",
        }
    }

    /// Returns true when the language has a heuristic extractor.
    pub fn is_supported(self) -> bool {
        !matches!(self, Language::Unknown)
    }
}

/// Detects language from a file path extension using the built-in grammar registry.
pub fn detect_language(path: &Path) -> Language {
    GrammarRegistry::builtins().detect_language(path)
}

/// Detects language using a custom registry (e.g. with `repoctx.languages.toml` overrides).
pub fn detect_language_with_registry(path: &Path, registry: &GrammarRegistry) -> Language {
    registry.detect_language(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_rust_and_python() {
        assert_eq!(detect_language(Path::new("src/main.rs")), Language::Rust);
        assert_eq!(detect_language(Path::new("app.py")), Language::Python);
    }
}
