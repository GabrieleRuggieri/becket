//! Tree-sitter grammar registry for built-in and plugin languages.

use std::collections::HashMap;
use std::path::Path;

use tree_sitter::Language;

use crate::error::CoreError;
use crate::language::Language as RepoLanguage;

/// Metadata for a registered tree-sitter grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarSpec {
    /// Canonical language id (e.g. `rust`).
    pub id: &'static str,
    /// File extensions handled by this grammar.
    pub extensions: &'static [&'static str],
    /// Optional notes for plugin authors.
    pub description: &'static str,
}

type GrammarLoader = fn() -> Language;

/// Registry of tree-sitter grammars and extension mappings.
#[derive(Debug, Clone)]
pub struct GrammarRegistry {
    loaders: HashMap<&'static str, GrammarLoader>,
    extension_map: HashMap<String, RepoLanguage>,
    specs: Vec<GrammarSpec>,
}

impl GrammarRegistry {
    /// Returns the default registry with all built-in grammars.
    pub fn builtins() -> Self {
        let mut registry = Self {
            loaders: HashMap::new(),
            extension_map: HashMap::new(),
            specs: Vec::new(),
        };
        registry.register_builtin(
            GrammarSpec {
                id: "rust",
                extensions: &["rs"],
                description: "Rust (tree-sitter-rust)",
            },
            RepoLanguage::Rust,
            || tree_sitter_rust::LANGUAGE.into(),
        );
        registry.register_builtin(
            GrammarSpec {
                id: "python",
                extensions: &["py", "pyi"],
                description: "Python (tree-sitter-python)",
            },
            RepoLanguage::Python,
            || tree_sitter_python::LANGUAGE.into(),
        );
        registry.register_builtin(
            GrammarSpec {
                id: "javascript",
                extensions: &["js", "jsx", "mjs", "cjs"],
                description: "JavaScript (tree-sitter-javascript)",
            },
            RepoLanguage::JavaScript,
            || tree_sitter_javascript::LANGUAGE.into(),
        );
        registry.register_builtin(
            GrammarSpec {
                id: "typescript",
                extensions: &["ts", "tsx"],
                description: "TypeScript (tree-sitter-typescript)",
            },
            RepoLanguage::TypeScript,
            || tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        );
        registry.register_builtin(
            GrammarSpec {
                id: "go",
                extensions: &["go"],
                description: "Go (tree-sitter-go)",
            },
            RepoLanguage::Go,
            || tree_sitter_go::LANGUAGE.into(),
        );
        registry.register_builtin(
            GrammarSpec {
                id: "java",
                extensions: &["java"],
                description: "Java (tree-sitter-java)",
            },
            RepoLanguage::Java,
            || tree_sitter_java::LANGUAGE.into(),
        );
        registry
    }

    /// Loads optional extension overrides from `repoctx.languages.toml`.
    pub fn load_overrides_from_file(mut self, path: &Path) -> Result<Self, CoreError> {
        if !path.is_file() {
            return Ok(self);
        }
        let raw = std::fs::read_to_string(path)
            .map_err(|error| CoreError::InvalidRepository(error.to_string()))?;
        let config: LanguagePluginConfig = toml::from_str(&raw)
            .map_err(|error| CoreError::InvalidRepository(error.to_string()))?;

        for plugin in config.languages {
            let Some(language) = self.language_by_id(&plugin.id) else {
                tracing::warn!(
                    language = %plugin.id,
                    "language plugin references unknown built-in grammar; add tree-sitter crate + register_builtin"
                );
                continue;
            };
            for extension in plugin.extensions {
                self.extension_map.insert(extension, language);
            }
        }
        Ok(self)
    }

    /// Returns all registered grammar specs.
    pub fn specs(&self) -> &[GrammarSpec] {
        &self.specs
    }

    /// Detects language from a file path using the registry.
    pub fn detect_language(&self, path: &Path) -> RepoLanguage {
        let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
            return RepoLanguage::Unknown;
        };
        self.extension_map
            .get(extension)
            .copied()
            .unwrap_or(RepoLanguage::Unknown)
    }

    /// Resolves a RepoCtx language to a tree-sitter `Language` handle.
    pub fn tree_sitter_language(&self, language: RepoLanguage) -> Result<Language, CoreError> {
        if language == RepoLanguage::Unknown {
            return Err(CoreError::Parse("unsupported language".into()));
        }
        let loader = self.loaders.get(language.id()).ok_or_else(|| {
            CoreError::Parse(format!("no grammar registered for {}", language.id()))
        })?;
        Ok(loader())
    }

    fn register_builtin(
        &mut self,
        spec: GrammarSpec,
        language: RepoLanguage,
        loader: GrammarLoader,
    ) {
        self.loaders.insert(spec.id, loader);
        for extension in spec.extensions {
            self.extension_map.insert(extension.to_string(), language);
        }
        self.specs.push(spec);
    }

    fn language_by_id(&self, id: &str) -> Option<RepoLanguage> {
        self.specs
            .iter()
            .find(|spec| spec.id == id)
            .and_then(|spec| match spec.id {
                "rust" => Some(RepoLanguage::Rust),
                "python" => Some(RepoLanguage::Python),
                "javascript" => Some(RepoLanguage::JavaScript),
                "typescript" => Some(RepoLanguage::TypeScript),
                "go" => Some(RepoLanguage::Go),
                "java" => Some(RepoLanguage::Java),
                _ => None,
            })
    }
}

/// Optional language plugin configuration (`repoctx.languages.toml`).
#[derive(Debug, Clone, serde::Deserialize)]
struct LanguagePluginConfig {
    #[serde(default)]
    languages: Vec<LanguagePluginEntry>,
}

/// One language plugin entry mapping extensions to a built-in grammar id.
#[derive(Debug, Clone, serde::Deserialize)]
struct LanguagePluginEntry {
    id: String,
    #[serde(default)]
    extensions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_detects_rust_extension() {
        let registry = GrammarRegistry::builtins();
        assert_eq!(
            registry.detect_language(Path::new("src/main.rs")),
            RepoLanguage::Rust
        );
    }

    #[test]
    fn registry_lists_builtin_grammars() {
        let registry = GrammarRegistry::builtins();
        assert!(registry.specs().iter().any(|spec| spec.id == "rust"));
        assert!(registry.specs().iter().any(|spec| spec.id == "java"));
    }
}
