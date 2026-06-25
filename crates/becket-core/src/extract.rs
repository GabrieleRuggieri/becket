//! Heuristic symbol extraction pending full tree-sitter integration.

use std::fs;
use std::path::Path;

use becket_schema::artifacts::SymbolRecord;
use becket_schema::symbol::{SymbolKind, Visibility};

use crate::error::CoreError;
use crate::ids::stable_symbol_id;
use crate::language::Language;

/// Extracts symbols from a source file using lightweight heuristics.
///
/// This is the v0 extractor used until tree-sitter grammars are wired in.
pub struct HeuristicExtractor;

impl HeuristicExtractor {
    /// Parses `path` and returns discovered symbols.
    ///
    /// # Arguments
    ///
    /// * `relative_path` - Path relative to repository root.
    /// * `language` - Detected source language.
    /// * `absolute_path` - Absolute path for reading contents.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Io`] if the file cannot be read.
    pub fn extract(
        relative_path: &str,
        language: Language,
        absolute_path: &Path,
    ) -> Result<Vec<SymbolRecord>, CoreError> {
        let contents = fs::read_to_string(absolute_path)?;
        let lines: Vec<&str> = contents.lines().collect();
        let mut symbols = Vec::new();

        for (index, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }

            if let Some(symbol) = match language {
                Language::Rust => extract_rust_line(trimmed, relative_path, index + 1),
                Language::Python => extract_python_line(trimmed, relative_path, index + 1),
                Language::TypeScript | Language::JavaScript => {
                    extract_ts_line(trimmed, relative_path, index + 1)
                }
                Language::Go => extract_go_line(trimmed, relative_path, index + 1),
                Language::Java => extract_java_line(trimmed, relative_path, index + 1),
                Language::Unknown => None,
            } {
                symbols.push(symbol);
            }
        }

        Ok(symbols)
    }
}

fn make_symbol(
    name: &str,
    kind: SymbolKind,
    file_path: &str,
    line: usize,
    visibility: Visibility,
) -> SymbolRecord {
    SymbolRecord {
        id: stable_symbol_id(
            file_path,
            name,
            line as u32,
            match kind {
                SymbolKind::Function => "function",
                SymbolKind::Class => "class",
                SymbolKind::Method => "method",
                SymbolKind::Var => "var",
                SymbolKind::Type => "type",
                SymbolKind::Module => "module",
            },
        ),
        kind,
        name: name.to_string(),
        fqn: format!("{file_path}::{name}"),
        file_path: file_path.to_string(),
        start_line: line as u32,
        end_line: line as u32,
        visibility,
        module_id: None,
    }
}

fn extract_rust_line(line: &str, file_path: &str, line_no: usize) -> Option<SymbolRecord> {
    if line.starts_with("pub fn ") || line.starts_with("fn ") {
        let name = line.split("fn ").nth(1)?.split('(').next()?.trim();
        let vis = if line.starts_with("pub ") {
            Visibility::Public
        } else {
            Visibility::Private
        };
        return Some(make_symbol(
            name,
            SymbolKind::Function,
            file_path,
            line_no,
            vis,
        ));
    }
    if line.starts_with("pub struct ") || line.starts_with("struct ") {
        let name = line
            .split("struct ")
            .nth(1)?
            .split_whitespace()
            .next()?
            .trim_end_matches('{')
            .trim();
        return Some(make_symbol(
            name,
            SymbolKind::Class,
            file_path,
            line_no,
            Visibility::Public,
        ));
    }
    if line.contains("fn main(") {
        return Some(make_symbol(
            "main",
            SymbolKind::Function,
            file_path,
            line_no,
            Visibility::Public,
        ));
    }
    None
}

fn extract_python_line(line: &str, file_path: &str, line_no: usize) -> Option<SymbolRecord> {
    if line.starts_with("def ") {
        let name = line.split("def ").nth(1)?.split('(').next()?.trim();
        return Some(make_symbol(
            name,
            SymbolKind::Function,
            file_path,
            line_no,
            Visibility::Public,
        ));
    }
    if line.starts_with("class ") {
        let name = line
            .split("class ")
            .nth(1)?
            .split('(')
            .next()?
            .split(':')
            .next()?
            .trim();
        return Some(make_symbol(
            name,
            SymbolKind::Class,
            file_path,
            line_no,
            Visibility::Public,
        ));
    }
    None
}

fn extract_ts_line(line: &str, file_path: &str, line_no: usize) -> Option<SymbolRecord> {
    if line.contains("function ") {
        let rest = line.split("function ").nth(1)?;
        let name = rest.split('(').next()?.trim();
        return Some(make_symbol(
            name,
            SymbolKind::Function,
            file_path,
            line_no,
            Visibility::Public,
        ));
    }
    if line.starts_with("export class ") || line.starts_with("class ") {
        let rest = line.trim_start_matches("export ").split("class ").nth(1)?;
        let name = rest.split_whitespace().next()?.trim_end_matches('{').trim();
        return Some(make_symbol(
            name,
            SymbolKind::Class,
            file_path,
            line_no,
            Visibility::Public,
        ));
    }
    None
}

fn extract_go_line(line: &str, file_path: &str, line_no: usize) -> Option<SymbolRecord> {
    if line.starts_with("func ") {
        let rest = line.trim_start_matches("func ").trim_start_matches('(');
        let after_receiver = if let Some(idx) = rest.find(')') {
            rest[idx + 1..].trim()
        } else {
            rest
        };
        let name = after_receiver.split('(').next()?.trim();
        return Some(make_symbol(
            name,
            SymbolKind::Function,
            file_path,
            line_no,
            Visibility::Public,
        ));
    }
    if line.starts_with("type ") && line.contains(" struct") {
        let name = line.split("type ").nth(1)?.split_whitespace().next()?;
        return Some(make_symbol(
            name,
            SymbolKind::Class,
            file_path,
            line_no,
            Visibility::Public,
        ));
    }
    None
}

fn extract_java_line(line: &str, file_path: &str, line_no: usize) -> Option<SymbolRecord> {
    if line.contains(" class ") || line.starts_with("class ") || line.contains("public class ") {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if let Some(pos) = tokens.iter().position(|t| *t == "class") {
            if let Some(name) = tokens.get(pos + 1) {
                let clean = name.trim_end_matches('{');
                return Some(make_symbol(
                    clean,
                    SymbolKind::Class,
                    file_path,
                    line_no,
                    Visibility::Public,
                ));
            }
        }
    }
    if line.contains('(') && (line.contains(" void ") || line.contains("public ")) {
        if let Some(name) = line.split('(').next() {
            let name = name.split_whitespace().last()?;
            return Some(make_symbol(
                name,
                SymbolKind::Function,
                file_path,
                line_no,
                Visibility::Public,
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn extracts_rust_main() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "fn main() {{}}").unwrap();
        let symbols =
            HeuristicExtractor::extract("src/main.rs", Language::Rust, file.path()).unwrap();
        assert!(symbols.iter().any(|s| s.name == "main"));
    }
}
