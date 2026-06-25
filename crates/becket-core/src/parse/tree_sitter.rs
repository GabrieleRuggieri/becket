//! Tree-sitter parser: symbol extraction, call edges, and entrypoint hints.

use std::path::Path;

use crate::parse::GrammarRegistry;
use becket_schema::artifacts::SymbolRecord;
use becket_schema::edge::EdgeType;
use becket_schema::symbol::{EntrypointKind, SymbolKind, Visibility};
use tree_sitter::{Node, Parser};

use crate::error::CoreError;
use crate::ids::stable_symbol_id;
use crate::language::Language as RepoLanguage;
use crate::parse::http_routes::{self, ParsedHttpRoute};

/// A single unresolved call edge (resolved to symbol ids in the graph builder).
#[derive(Debug, Clone)]
pub struct ParsedCall {
    /// Symbol id of the calling function/method.
    pub caller_symbol_id: String,
    /// Callee name as written in source.
    pub callee_name: String,
}

/// Entry point candidate detected during parsing.
#[derive(Debug, Clone)]
pub struct ParsedEntrypoint {
    /// Symbol id for the entrypoint.
    pub symbol_id: String,
    /// Entrypoint classification.
    pub kind: EntrypointKind,
    /// Optional label (route path, file path, etc.).
    pub label: Option<String>,
}

/// An unresolved import edge (file-level, resolved in the graph builder).
#[derive(Debug, Clone)]
pub struct ParsedImport {
    /// Repository-relative file path containing the import.
    pub file_path: String,
    /// Imported symbol name (last segment of the use path).
    pub imported_name: String,
}

/// An unresolved inheritance edge (extends / implements).
#[derive(Debug, Clone)]
pub struct ParsedInheritance {
    /// Child symbol id at parse time (remapped during build).
    pub child_symbol_id: String,
    /// Parent type or trait name.
    pub parent_name: String,
    /// Extends or implements.
    pub edge_type: EdgeType,
}

/// Symbols and relationships extracted from one source file.
#[derive(Debug, Clone, Default)]
pub struct FileParseResult {
    /// Repository-relative file path.
    pub path: String,
    /// Extracted symbol records (ids pre-assigned).
    pub symbols: Vec<SymbolRecord>,
    /// Unresolved call edges within this file.
    pub calls: Vec<ParsedCall>,
    /// Unresolved import edges declared in this file.
    pub imports: Vec<ParsedImport>,
    /// Unresolved extends/implements edges.
    pub inheritance: Vec<ParsedInheritance>,
    /// HTTP route entrypoints (resolved to symbols at index time).
    pub http_routes: Vec<ParsedHttpRoute>,
    /// Outbound HTTP client calls (for cross-repo linking).
    pub http_clients: Vec<crate::parse::http_clients::ParsedHttpClient>,
    /// Detected entrypoints.
    pub entrypoints: Vec<ParsedEntrypoint>,
}

/// Multi-language tree-sitter parser.
pub struct TreeSitterParser;

impl TreeSitterParser {
    /// Parses a source file and returns symbols, calls, and entrypoints.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Parse`] when tree-sitter fails to parse the file.
    pub fn parse_file(
        relative_path: &str,
        language: RepoLanguage,
        absolute_path: &Path,
    ) -> Result<FileParseResult, CoreError> {
        let source = std::fs::read_to_string(absolute_path)?;
        Self::parse_source(relative_path, language, &source)
    }

    /// Parses in-memory source (used by unit tests).
    pub fn parse_source(
        relative_path: &str,
        language: RepoLanguage,
        source: &str,
    ) -> Result<FileParseResult, CoreError> {
        let ts_language = language_to_tree_sitter(language)?;
        let mut parser = Parser::new();
        parser
            .set_language(&ts_language)
            .map_err(|e| CoreError::Parse(e.to_string()))?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| CoreError::Parse(format!("failed to parse {relative_path}")))?;

        let mut ctx = ParseContext::new(relative_path);
        walk_node(tree.root_node(), source.as_bytes(), &mut ctx);

        Ok(FileParseResult {
            path: relative_path.to_string(),
            symbols: ctx.symbols,
            calls: ctx.calls,
            imports: ctx.imports,
            inheritance: ctx.inheritance,
            http_routes: ctx.http_routes,
            http_clients: ctx.http_clients,
            entrypoints: ctx.entrypoints,
        })
    }
}

struct ParseContext {
    file_path: String,
    symbols: Vec<SymbolRecord>,
    calls: Vec<ParsedCall>,
    imports: Vec<ParsedImport>,
    inheritance: Vec<ParsedInheritance>,
    http_routes: Vec<ParsedHttpRoute>,
    http_clients: Vec<crate::parse::http_clients::ParsedHttpClient>,
    entrypoints: Vec<ParsedEntrypoint>,
    scope_stack: Vec<String>,
}

impl ParseContext {
    fn new(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            symbols: Vec::new(),
            calls: Vec::new(),
            imports: Vec::new(),
            inheritance: Vec::new(),
            http_routes: Vec::new(),
            http_clients: Vec::new(),
            entrypoints: Vec::new(),
            scope_stack: Vec::new(),
        }
    }

    fn current_scope(&self) -> Option<&str> {
        self.scope_stack.last().map(String::as_str)
    }

    fn push_symbol(
        &mut self,
        name: &str,
        kind: SymbolKind,
        node: Node,
        visibility: Visibility,
    ) -> String {
        let start_line = node.start_position().row as u32 + 1;
        let end_line = node.end_position().row as u32 + 1;
        let kind_str = symbol_kind_label(kind);
        let id = stable_symbol_id(&self.file_path, name, start_line, kind_str);
        let fqn = format!("{}::{}", self.file_path, name);

        if name == "main" {
            self.entrypoints.push(ParsedEntrypoint {
                symbol_id: id.clone(),
                kind: EntrypointKind::Main,
                label: Some(self.file_path.clone()),
            });
        }

        self.symbols.push(SymbolRecord {
            id: id.clone(),
            kind,
            name: name.to_string(),
            fqn,
            file_path: self.file_path.clone(),
            start_line,
            end_line,
            visibility,
            module_id: None,
        });
        id
    }

    fn record_call(&mut self, callee_name: &str) {
        let Some(caller_id) = self.current_scope() else {
            return;
        };
        self.calls.push(ParsedCall {
            caller_symbol_id: caller_id.to_string(),
            callee_name: callee_name.to_string(),
        });
    }

    fn record_inheritance(&mut self, child_id: &str, parent_name: &str, edge_type: EdgeType) {
        self.inheritance.push(ParsedInheritance {
            child_symbol_id: child_id.to_string(),
            parent_name: parent_name.to_string(),
            edge_type,
        });
    }
}

fn walk_node(node: Node, source: &[u8], ctx: &mut ParseContext) {
    match node.kind() {
        // Rust
        "function_item" | "function_definition" | "method_definition" | "function_declaration" => {
            if let Some(name) = node_child_identifier(node, source, &["name", "declarator"]) {
                let vis = visibility_from_node(node);
                let id = ctx.push_symbol(&name, SymbolKind::Function, node, vis);
                ctx.scope_stack.push(id);
                walk_children(node, source, ctx);
                ctx.scope_stack.pop();
                return;
            }
        }
        "decorated_definition" => {
            if let Some(route) = http_routes::detect_decorated_handler(node, source, &ctx.file_path)
            {
                ctx.http_routes.push(route);
            }
            walk_children(node, source, ctx);
            return;
        }
        "struct_item" | "class_definition" | "class_declaration" => {
            if let Some(name) = node_child_identifier(node, source, &["name", "declarator"]) {
                let id = ctx.push_symbol(&name, SymbolKind::Class, node, Visibility::Public);
                record_type_inheritance(node, source, &id, ctx);
            }
        }
        "trait_item" | "interface_declaration" => {
            if let Some(name) = node_child_identifier(node, source, &["name", "declarator"]) {
                ctx.push_symbol(&name, SymbolKind::Type, node, Visibility::Public);
            }
        }
        "impl_item" => {
            record_rust_impl_inheritance(node, source, ctx);
            walk_children(node, source, ctx);
            return;
        }
        // Go
        "method_declaration" => {
            if let Some(name) = node_child_identifier(node, source, &["name"]) {
                let id = ctx.push_symbol(&name, SymbolKind::Method, node, Visibility::Public);
                if let Some(route) =
                    http_routes::detect_java_http_mapping(node, source, &ctx.file_path, &name)
                {
                    ctx.http_routes.push(route);
                }
                ctx.scope_stack.push(id);
                walk_children(node, source, ctx);
                ctx.scope_stack.pop();
                return;
            }
        }
        // Calls (multi-language)
        "call_expression" | "call" => {
            if let Some(route) = http_routes::detect_route_call(node, source, &ctx.file_path) {
                ctx.http_routes.push(route);
            }
            if let Some(scope) = ctx.current_scope() {
                if let Some(client) = crate::parse::http_clients::detect_http_client_call(
                    node,
                    source,
                    &ctx.file_path,
                    scope,
                ) {
                    ctx.http_clients.push(client);
                }
            }
            if let Some(name) = extract_call_name(node, source) {
                ctx.record_call(&name);
            }
        }
        // Rust use declarations
        "use_declaration" | "import_statement" | "import_declaration" => {
            if let Some(name) = extract_import_name(node, source) {
                ctx.imports.push(ParsedImport {
                    file_path: ctx.file_path.clone(),
                    imported_name: name,
                });
            }
        }
        _ => {}
    }

    walk_children(node, source, ctx);
}

fn record_rust_impl_inheritance(node: Node, source: &[u8], ctx: &mut ParseContext) {
    let Some(trait_node) = node.child_by_field_name("trait") else {
        return;
    };
    let Some(type_node) = node.child_by_field_name("type") else {
        return;
    };
    let Some(trait_name) = extract_type_name(trait_node, source) else {
        return;
    };
    let Some(type_name) = extract_type_name(type_node, source) else {
        return;
    };
    let child_id = ctx
        .symbols
        .iter()
        .find(|s| s.file_path == ctx.file_path && s.name == type_name)
        .map(|s| s.id.clone())
        .unwrap_or_else(|| {
            stable_symbol_id(
                &ctx.file_path,
                &type_name,
                node.start_position().row as u32 + 1,
                "class",
            )
        });
    ctx.record_inheritance(&child_id, &trait_name, EdgeType::Implements);
}

fn record_type_inheritance(node: Node, source: &[u8], child_id: &str, ctx: &mut ParseContext) {
    if let Some(superclass) = node.child_by_field_name("superclass") {
        if let Some(name) = extract_type_name(superclass, source) {
            ctx.record_inheritance(child_id, &name, EdgeType::Extends);
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "class_heritage" {
            record_class_heritage(child, source, child_id, ctx);
        }
    }

    if let Some(interfaces) = node.child_by_field_name("interfaces") {
        for name in interface_names(interfaces, source) {
            ctx.record_inheritance(child_id, &name, EdgeType::Implements);
        }
    }
}

fn record_class_heritage(node: Node, source: &[u8], child_id: &str, ctx: &mut ParseContext) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "extends_clause" | "extends_type_clause" => {
                if let Some(value) = child
                    .child_by_field_name("value")
                    .or_else(|| child.child_by_field_name("type"))
                {
                    if let Some(name) = extract_type_name(value, source) {
                        ctx.record_inheritance(child_id, &name, EdgeType::Extends);
                    }
                }
            }
            "implements_clause" | "implements_type_clause" => {
                for name in interface_names(child, source) {
                    ctx.record_inheritance(child_id, &name, EdgeType::Implements);
                }
            }
            _ => {}
        }
    }
}

fn interface_names(node: Node, source: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    collect_interface_names(node, source, &mut names);
    names
}

fn collect_interface_names(node: Node, source: &[u8], names: &mut Vec<String>) {
    if matches!(
        node.kind(),
        "type_identifier" | "identifier" | "scoped_type_identifier" | "generic_type"
    ) {
        if let Some(name) = extract_type_name(node, source) {
            names.push(name);
            return;
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_interface_names(child, source, names);
    }
}

fn extract_type_name(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "type_identifier" | "identifier" | "property_identifier" => node_text(node, source),
        "scoped_type_identifier" | "scoped_identifier" => node
            .child_by_field_name("name")
            .and_then(|n| node_text(n, source)),
        "generic_type" => node
            .child_by_field_name("type")
            .and_then(|n| extract_type_name(n, source)),
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(name) = extract_type_name(child, source) {
                    return Some(name);
                }
            }
            None
        }
    }
}

fn walk_children(node: Node, source: &[u8], ctx: &mut ParseContext) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_node(child, source, ctx);
    }
}

fn node_child_identifier(node: Node, source: &[u8], field_names: &[&str]) -> Option<String> {
    for field in field_names {
        if let Some(child) = node.child_by_field_name(field) {
            if let Some(text) = node_text(child, source) {
                return Some(text);
            }
        }
    }

    // Fallback: first identifier/type_identifier in node
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(
            child.kind(),
            "identifier" | "type_identifier" | "property_identifier"
        ) {
            if let Some(text) = node_text(child, source) {
                return Some(text);
            }
        }
    }
    None
}

fn extract_call_name(node: Node, source: &[u8]) -> Option<String> {
    if let Some(function) = node.child_by_field_name("function") {
        return extract_call_target_name(function, source);
    }
    if let Some(name) = node.child_by_field_name("name") {
        return node_text(name, source);
    }
    None
}

fn extract_call_target_name(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "type_identifier" => node_text(node, source),
        "field_expression" | "member_expression" => node
            .child_by_field_name("field")
            .or_else(|| node.child_by_field_name("property"))
            .and_then(|n| node_text(n, source)),
        "scoped_identifier" => node
            .child_by_field_name("name")
            .and_then(|n| node_text(n, source)),
        _ => {
            let mut cursor = node.walk();
            let children: Vec<Node> = node.children(&mut cursor).collect();
            children.last().and_then(|n| node_text(*n, source))
        }
    }
}

fn visibility_from_node(node: Node) -> Visibility {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" || child.kind() == "pub" {
            return Visibility::Public;
        }
    }
    Visibility::Private
}

fn node_text(node: Node, source: &[u8]) -> Option<String> {
    let start = node.start_byte();
    let end = node.end_byte();
    if end <= source.len() {
        std::str::from_utf8(&source[start..end])
            .ok()
            .map(str::to_string)
    } else {
        None
    }
}

fn extract_import_name(node: Node, source: &[u8]) -> Option<String> {
    // Walk identifiers and take the last meaningful segment (imported symbol).
    let mut last: Option<String> = None;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_last_identifier(child, source, &mut last);
    }
    last
}

fn collect_last_identifier(node: Node, source: &[u8], last: &mut Option<String>) {
    if matches!(
        node.kind(),
        "identifier" | "type_identifier" | "property_identifier"
    ) {
        if let Some(text) = node_text(node, source) {
            *last = Some(text);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_last_identifier(child, source, last);
    }
}

fn symbol_kind_label(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Function => "function",
        SymbolKind::Class => "class",
        SymbolKind::Method => "method",
        SymbolKind::Var => "var",
        SymbolKind::Type => "type",
        SymbolKind::Module => "module",
    }
}

fn language_to_tree_sitter(language: RepoLanguage) -> Result<tree_sitter::Language, CoreError> {
    GrammarRegistry::builtins().tree_sitter_language(language)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rust_functions_and_calls() {
        let source = r#"
pub fn func_a() {
    func_b();
}

fn func_b() {
    func_c();
}

fn func_c() {}
"#;
        let result =
            TreeSitterParser::parse_source("src/graph.rs", RepoLanguage::Rust, source).unwrap();
        let names: Vec<_> = result.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"func_a"));
        assert!(names.contains(&"func_b"));
        assert!(names.contains(&"func_c"));
        assert!(!result.calls.is_empty());
    }

    #[test]
    fn detects_rust_main_entrypoint() {
        let source = "fn main() {}";
        let result =
            TreeSitterParser::parse_source("src/main.rs", RepoLanguage::Rust, source).unwrap();
        assert_eq!(result.entrypoints.len(), 1);
        assert_eq!(result.entrypoints[0].kind, EntrypointKind::Main);
    }

    #[test]
    fn detects_rust_trait_implementation() {
        let source = r#"
trait Speakable {}
struct Dog;
impl Speakable for Dog {}
"#;
        let result =
            TreeSitterParser::parse_source("src/traits.rs", RepoLanguage::Rust, source).unwrap();
        assert_eq!(result.inheritance.len(), 1);
        assert_eq!(result.inheritance[0].edge_type, EdgeType::Implements);
        assert_eq!(result.inheritance[0].parent_name, "Speakable");
    }

    #[test]
    fn detects_typescript_class_extends() {
        let source = "class Shape {}\nclass Circle extends Shape {}";
        let result =
            TreeSitterParser::parse_source("src/shapes.ts", RepoLanguage::TypeScript, source)
                .unwrap();
        assert!(
            result
                .inheritance
                .iter()
                .any(|e| e.edge_type == EdgeType::Extends && e.parent_name == "Shape"),
            "expected Circle extends Shape"
        );
    }
}
