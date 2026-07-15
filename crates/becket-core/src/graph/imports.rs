//! Per-language import → file resolution for import-aware call resolution.
//!
//! Maps a parsed import (`module_path` + `imported_name` + optional alias) to
//! candidate repository files, so calls can be resolved through the import
//! table of the calling file instead of name-only global lookup.

use std::collections::{HashMap, HashSet};

use crate::parse::ParsedImport;

/// Import table for one file: local name (or alias) → target file paths.
#[derive(Debug, Default, Clone)]
pub struct FileImportTable {
    /// Local binding name → repository-relative files that may declare it.
    bindings: HashMap<String, Vec<String>>,
}

impl FileImportTable {
    /// Returns candidate target files for a locally bound name.
    pub fn targets_for(&self, name: &str) -> Option<&[String]> {
        self.bindings.get(name).map(Vec::as_slice)
    }
}

/// Index of repository files by module stem, for fast suffix matching.
struct ModuleIndex {
    /// Last module-path segment → files whose module stem ends with it.
    by_last_segment: HashMap<String, Vec<String>>,
    /// All known files (exact-path lookups for relative imports).
    known_files: HashSet<String>,
}

impl ModuleIndex {
    fn build(known_files: &HashSet<String>) -> Self {
        let mut by_last_segment: HashMap<String, Vec<String>> = HashMap::new();
        for file in known_files {
            if let Some(stem) = module_stem(file) {
                if let Some(last) = stem.rsplit('/').next() {
                    by_last_segment
                        .entry(last.to_string())
                        .or_default()
                        .push(file.clone());
                }
            }
        }
        for files in by_last_segment.values_mut() {
            files.sort();
        }
        Self {
            by_last_segment,
            known_files: known_files.clone(),
        }
    }
}

/// Import tables for every file in the repository.
#[derive(Debug, Default)]
pub struct ImportResolver {
    tables: HashMap<String, FileImportTable>,
}

impl ImportResolver {
    /// Builds import tables by resolving each import's module path to files.
    ///
    /// `known_files` must contain every repository-relative source path.
    pub fn build(imports: &[ParsedImport], known_files: &HashSet<String>) -> Self {
        let index = ModuleIndex::build(known_files);
        let mut tables: HashMap<String, FileImportTable> = HashMap::new();

        for import in imports {
            let Some(module_path) = import.module_path.as_deref() else {
                continue;
            };
            let candidates = resolve_module_to_files(&import.file_path, module_path, &index);
            if candidates.is_empty() {
                continue;
            }
            let local_name = import
                .alias
                .clone()
                .unwrap_or_else(|| import.imported_name.clone());
            let entry = tables
                .entry(import.file_path.clone())
                .or_default()
                .bindings
                .entry(local_name)
                .or_default();
            for candidate in candidates {
                if !entry.contains(&candidate) {
                    entry.push(candidate);
                }
            }
        }

        for table in tables.values_mut() {
            for files in table.bindings.values_mut() {
                files.sort();
            }
        }

        Self { tables }
    }

    /// Returns the import table for a file, if any imports resolved.
    pub fn table_for(&self, file_path: &str) -> Option<&FileImportTable> {
        self.tables.get(file_path)
    }
}

/// Resolves a language-agnostic module path to repository files.
///
/// Strategies, in order:
/// 1. Relative path resolution (`./x`, `../y`) against the importer's
///    directory with common extensions and `index.*` fallbacks (TS/JS style).
/// 2. Dotted / `::`-separated module paths matched as module-stem suffixes
///    (`a.b.c` → `**/a/b/c.py`; `crate::graph::resolve` → `**/graph.rs` or
///    `**/graph/mod.rs` after dropping the trailing symbol segment; Java
///    packages and Go import paths behave the same way).
fn resolve_module_to_files(
    importer_path: &str,
    module_path: &str,
    index: &ModuleIndex,
) -> Vec<String> {
    let module_path = module_path.trim();
    if module_path.is_empty() {
        return Vec::new();
    }

    if module_path.starts_with("./") || module_path.starts_with("../") || module_path == "." {
        return resolve_relative(importer_path, module_path, &index.known_files);
    }

    resolve_suffix(module_path, index)
}

fn resolve_relative(
    importer_path: &str,
    module_path: &str,
    known_files: &HashSet<String>,
) -> Vec<String> {
    let importer_dir = match importer_path.rsplit_once('/') {
        Some((dir, _)) => dir,
        None => "",
    };
    let joined = normalize_path(importer_dir, module_path);

    let mut candidates = Vec::new();
    if known_files.contains(&joined) {
        candidates.push(joined.clone());
    }
    for ext in ["ts", "tsx", "js", "jsx", "mjs", "cjs", "py"] {
        let with_ext = format!("{joined}.{ext}");
        if known_files.contains(&with_ext) {
            candidates.push(with_ext);
        }
    }
    for index_name in [
        "index.ts",
        "index.tsx",
        "index.js",
        "index.jsx",
        "__init__.py",
    ] {
        let index_file = if joined.is_empty() {
            index_name.to_string()
        } else {
            format!("{joined}/{index_name}")
        };
        if known_files.contains(&index_file) {
            candidates.push(index_file);
        }
    }
    candidates
}

/// Matches dotted / `::` module paths as module-stem suffixes.
///
/// Tries the deepest interpretation first, then progressively drops trailing
/// segments (they may name a symbol rather than a module, e.g.
/// `use crate::graph::resolve`).
fn resolve_suffix(module_path: &str, index: &ModuleIndex) -> Vec<String> {
    let segments: Vec<&str> = module_path
        .split(['.', ':', '/'])
        .filter(|s| !s.is_empty() && !matches!(*s, "crate" | "self" | "super"))
        .collect();
    if segments.is_empty() {
        return Vec::new();
    }

    for end in (1..=segments.len()).rev() {
        let last = segments[end - 1];
        let Some(candidate_files) = index.by_last_segment.get(last) else {
            continue;
        };
        // Prefer the longest matching suffix among files sharing the last segment.
        for start in 0..end {
            let suffix = segments[start..end].join("/");
            let mut matches: Vec<String> = candidate_files
                .iter()
                .filter(|file| file_matches_module_suffix(file, &suffix))
                .cloned()
                .collect();
            if !matches.is_empty() {
                matches.sort();
                matches.truncate(4);
                return matches;
            }
        }
    }
    Vec::new()
}

fn file_matches_module_suffix(file: &str, suffix: &str) -> bool {
    let Some(stem) = module_stem(file) else {
        return false;
    };
    stem == suffix || stem.ends_with(&format!("/{suffix}"))
}

/// Returns a file's module stem: `a/b/c.rs` → `a/b/c`, `a/b/mod.rs` → `a/b`,
/// `a/b/__init__.py` → `a/b`.
fn module_stem(file: &str) -> Option<String> {
    let stem = strip_source_extension(file)?;
    let stem = stem
        .strip_suffix("/mod")
        .or_else(|| stem.strip_suffix("/__init__"))
        .unwrap_or(stem);
    Some(stem.to_string())
}

fn strip_source_extension(file: &str) -> Option<&str> {
    for ext in [
        ".rs", ".py", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".go", ".java",
    ] {
        if let Some(stem) = file.strip_suffix(ext) {
            return Some(stem);
        }
    }
    None
}

/// Joins and normalizes `dir` + relative `path` (resolving `.` and `..`).
fn normalize_path(dir: &str, relative: &str) -> String {
    let mut parts: Vec<&str> = if dir.is_empty() {
        Vec::new()
    } else {
        dir.split('/').collect()
    };
    for segment in relative.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(paths: &[&str]) -> HashSet<String> {
        paths.iter().map(|p| p.to_string()).collect()
    }

    fn import(file: &str, name: &str, module: &str) -> ParsedImport {
        ParsedImport {
            file_path: file.into(),
            imported_name: name.into(),
            module_path: Some(module.into()),
            alias: None,
        }
    }

    #[test]
    fn resolves_relative_ts_import() {
        let known = files(&["src/app.ts", "src/services/billing.ts"]);
        let imports = vec![import("src/app.ts", "charge", "./services/billing")];
        let resolver = ImportResolver::build(&imports, &known);
        let table = resolver.table_for("src/app.ts").expect("table");
        assert_eq!(
            table.targets_for("charge").unwrap(),
            &["src/services/billing.ts".to_string()]
        );
    }

    #[test]
    fn resolves_parent_relative_import_with_index() {
        let known = files(&["src/api/handler.ts", "src/shared/index.ts"]);
        let imports = vec![import("src/api/handler.ts", "helper", "../shared")];
        let resolver = ImportResolver::build(&imports, &known);
        let table = resolver.table_for("src/api/handler.ts").expect("table");
        assert_eq!(
            table.targets_for("helper").unwrap(),
            &["src/shared/index.ts".to_string()]
        );
    }

    #[test]
    fn resolves_python_dotted_module() {
        let known = files(&["app/payment/gateway.py", "app/main.py"]);
        let imports = vec![import("app/main.py", "charge", "app.payment.gateway")];
        let resolver = ImportResolver::build(&imports, &known);
        let table = resolver.table_for("app/main.py").expect("table");
        assert_eq!(
            table.targets_for("charge").unwrap(),
            &["app/payment/gateway.py".to_string()]
        );
    }

    #[test]
    fn resolves_rust_use_path_dropping_symbol_segment() {
        let known = files(&["src/graph/mod.rs", "src/build.rs"]);
        let imports = vec![import("src/build.rs", "resolve", "crate::graph::resolve")];
        let resolver = ImportResolver::build(&imports, &known);
        let table = resolver.table_for("src/build.rs").expect("table");
        assert_eq!(
            table.targets_for("resolve").unwrap(),
            &["src/graph/mod.rs".to_string()]
        );
    }

    #[test]
    fn alias_binds_local_name() {
        let known = files(&["app/payment/gateway.py", "app/main.py"]);
        let imports = vec![ParsedImport {
            file_path: "app/main.py".into(),
            imported_name: "charge".into(),
            module_path: Some("app.payment.gateway".into()),
            alias: Some("do_charge".into()),
        }];
        let resolver = ImportResolver::build(&imports, &known);
        let table = resolver.table_for("app/main.py").expect("table");
        assert!(table.targets_for("do_charge").is_some());
        assert!(table.targets_for("charge").is_none());
    }

    #[test]
    fn unresolvable_module_produces_no_table() {
        let known = files(&["src/a.rs"]);
        let imports = vec![import("src/a.rs", "x", "external_dep::x")];
        let resolver = ImportResolver::build(&imports, &known);
        assert!(resolver.table_for("src/a.rs").is_none());
    }
}
