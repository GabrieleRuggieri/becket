//! HTTP route entrypoint detection from tree-sitter nodes.

use tree_sitter::Node;

/// A detected HTTP route bound to a handler symbol name (resolved at index time).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedHttpRoute {
    /// Repository-relative file path.
    pub file_path: String,
    /// Handler function or method name.
    pub handler_name: String,
    /// Human-readable route label (e.g. `GET /users`).
    pub label: String,
}

const HTTP_METHODS: &[&str] = &[
    "GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD", "ALL",
];

/// Detects Express/Fastify/Axum-style route registration calls.
pub fn detect_route_call(node: Node, source: &[u8], file_path: &str) -> Option<ParsedHttpRoute> {
    if node.kind() != "call_expression" && node.kind() != "call" {
        return None;
    }

    let method = extract_callee_route_method(node, source)?;
    let path = extract_first_string_argument(node, source)?;
    let handler_name = extract_handler_argument(node, source)?;

    Some(ParsedHttpRoute {
        file_path: file_path.to_string(),
        handler_name,
        label: format!("{method} {path}"),
    })
}

/// Detects Flask/FastAPI-style decorated handlers.
pub fn detect_decorated_handler(
    node: Node,
    source: &[u8],
    file_path: &str,
) -> Option<ParsedHttpRoute> {
    if node.kind() != "decorated_definition" {
        return None;
    }

    let mut definition: Option<Node> = None;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(
            child.kind(),
            "function_definition" | "function_declaration" | "method_definition"
        ) {
            definition = Some(child);
            break;
        }
    }
    let definition = definition?;
    let handler_name = child_identifier(definition, source, &["name", "declarator"])?;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "decorator" {
            if let Some(route) =
                parse_python_route_decorator(child, source, file_path, &handler_name)
            {
                return Some(route);
            }
        }
    }
    None
}

/// Detects Spring `@GetMapping` / `@PostMapping` style annotations on methods.
pub fn detect_java_http_mapping(
    node: Node,
    source: &[u8],
    file_path: &str,
    handler_name: &str,
) -> Option<ParsedHttpRoute> {
    if node.kind() != "method_declaration" {
        return None;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "modifiers" && child.kind() != "marker_annotation" {
            continue;
        }
        if let Some(route) = find_java_mapping_in_node(child, source, file_path, handler_name) {
            return Some(route);
        }
    }
    None
}

fn find_java_mapping_in_node(
    node: Node,
    source: &[u8],
    file_path: &str,
    handler_name: &str,
) -> Option<ParsedHttpRoute> {
    if node.kind() == "marker_annotation" || node.kind() == "annotation" {
        return parse_java_annotation(node, source, file_path, handler_name);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(route) = find_java_mapping_in_node(child, source, file_path, handler_name) {
            return Some(route);
        }
    }
    None
}

fn parse_java_annotation(
    node: Node,
    source: &[u8],
    file_path: &str,
    handler_name: &str,
) -> Option<ParsedHttpRoute> {
    let name = annotation_name(node, source)?;
    let method = match name.as_str() {
        "GetMapping" => "GET",
        "PostMapping" => "POST",
        "PutMapping" => "PUT",
        "DeleteMapping" => "DELETE",
        "PatchMapping" => "PATCH",
        "RequestMapping" => "HTTP",
        _ => return None,
    };
    let path = annotation_string_value(node, source).unwrap_or_else(|| "/".to_string());
    Some(ParsedHttpRoute {
        file_path: file_path.to_string(),
        handler_name: handler_name.to_string(),
        label: format!("{method} {path}"),
    })
}

fn parse_python_route_decorator(
    decorator: Node,
    source: &[u8],
    file_path: &str,
    handler_name: &str,
) -> Option<ParsedHttpRoute> {
    let call = first_child_of_kind(decorator, &["call"])?;
    let callee = call.child_by_field_name("function")?;
    let callee_name = node_text(callee, source)?;

    let method = if callee_name.ends_with(".route") || callee_name == "route" {
        "HTTP".to_string()
    } else if let Some(rest) = callee_name.strip_prefix("app.") {
        rest.to_uppercase()
    } else {
        callee_name.to_uppercase()
    };

    if method != "HTTP" && !HTTP_METHODS.contains(&method.as_str()) {
        return None;
    }

    let path = extract_first_string_argument(call, source)?;
    Some(ParsedHttpRoute {
        file_path: file_path.to_string(),
        handler_name: handler_name.to_string(),
        label: format!("{method} {path}"),
    })
}

fn extract_callee_route_method(node: Node, source: &[u8]) -> Option<String> {
    let function = node.child_by_field_name("function")?;
    let name = match function.kind() {
        "field_expression" | "member_expression" => function
            .child_by_field_name("field")
            .or_else(|| function.child_by_field_name("property"))
            .and_then(|n| node_text(n, source)),
        "identifier" | "type_identifier" => node_text(function, source),
        "scoped_identifier" => function
            .child_by_field_name("name")
            .and_then(|n| node_text(n, source)),
        _ => None,
    }?;

    let upper = name.to_uppercase();
    if HTTP_METHODS.contains(&upper.as_str()) {
        return Some(upper);
    }
    if upper == "ROUTE" {
        return Some("HTTP".to_string());
    }
    None
}

fn extract_first_string_argument(node: Node, source: &[u8]) -> Option<String> {
    let args = node
        .child_by_field_name("arguments")
        .or_else(|| node.child_by_field_name("argument_list"))?;
    let mut cursor = args.walk();
    for child in args.children(&mut cursor) {
        if let Some(text) = string_literal_text(child, source) {
            return Some(text);
        }
    }
    None
}

fn extract_handler_argument(node: Node, source: &[u8]) -> Option<String> {
    let args = node
        .child_by_field_name("arguments")
        .or_else(|| node.child_by_field_name("argument_list"))?;
    let mut cursor = args.walk();
    let mut seen_string = false;
    for child in args.children(&mut cursor) {
        if string_literal_text(child, source).is_some() {
            seen_string = true;
            continue;
        }
        if matches!(child.kind(), "(" | ")" | ",") {
            continue;
        }
        if seen_string {
            if let Some(name) = handler_from_arg_node(child, source) {
                return Some(name);
            }
        }
    }
    // Axum: .route("/path", get(handler)) — handler inside nested call
    let mut cursor = args.walk();
    for child in args.children(&mut cursor) {
        if let Some(name) = handler_from_arg_node(child, source) {
            if string_literal_text(child, source).is_none() {
                return Some(name);
            }
        }
    }
    None
}

fn handler_from_arg_node(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "type_identifier" => node_text(node, source),
        "call_expression" | "call" => {
            let inner = node.child_by_field_name("arguments")?;
            let mut cursor = inner.walk();
            for child in inner.children(&mut cursor) {
                if let Some(name) = node_text(child, source).filter(|n| !n.starts_with('"')) {
                    if matches!(
                        child.kind(),
                        "identifier" | "type_identifier" | "scoped_identifier"
                    ) {
                        return Some(name);
                    }
                }
                if let Some(name) = handler_from_arg_node(child, source) {
                    return Some(name);
                }
            }
            None
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(name) = handler_from_arg_node(child, source) {
                    return Some(name);
                }
            }
            None
        }
    }
}

fn annotation_name(node: Node, source: &[u8]) -> Option<String> {
    if let Some(name) = node
        .child_by_field_name("name")
        .and_then(|n| node_text(n, source))
    {
        return Some(name);
    }
    node_text(node, source).map(|text| {
        text.trim_start_matches('@')
            .split('(')
            .next()
            .unwrap_or(&text)
            .to_string()
    })
}

fn annotation_string_value(node: Node, source: &[u8]) -> Option<String> {
    if let Some(args) = node.child_by_field_name("arguments") {
        let mut cursor = args.walk();
        for child in args.children(&mut cursor) {
            if let Some(text) = string_literal_text(child, source) {
                return Some(text);
            }
        }
    }
    None
}

fn string_literal_text(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "string" | "string_literal" | "interpreted_string_literal" => {
            let raw = node_text(node, source)?;
            Some(trim_quotes(&raw))
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(text) = string_literal_text(child, source) {
                    return Some(text);
                }
            }
            None
        }
    }
}

fn trim_quotes(value: &str) -> String {
    value
        .trim_matches(|c| c == '"' || c == '\'' || c == '`')
        .to_string()
}

fn child_identifier(node: Node, source: &[u8], fields: &[&str]) -> Option<String> {
    for field in fields {
        if let Some(child) = node.child_by_field_name(field) {
            if let Some(text) = node_text(child, source) {
                return Some(text);
            }
        }
    }
    None
}

fn first_child_of_kind<'a>(node: Node<'a>, kinds: &[&str]) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    let found = node
        .children(&mut cursor)
        .find(|&child| kinds.contains(&child.kind()));
    found
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

#[cfg(test)]
mod tests {
    use crate::language::Language as RepoLanguage;
    use crate::parse::TreeSitterParser;

    #[test]
    fn detects_express_get_route() {
        let source = r#"
function listUsers() {}
const app = express();
app.get('/users', listUsers);
"#;
        let result =
            TreeSitterParser::parse_source("src/app.ts", RepoLanguage::TypeScript, source).unwrap();
        assert!(result
            .http_routes
            .iter()
            .any(|r| r.label == "GET /users" && r.handler_name == "listUsers"));
    }

    #[test]
    fn detects_flask_decorator_route() {
        let source = r#"
@app.get("/health")
def health():
    return "ok"
"#;
        let result =
            TreeSitterParser::parse_source("src/app.py", RepoLanguage::Python, source).unwrap();
        assert!(result
            .http_routes
            .iter()
            .any(|r| r.label == "GET /health" && r.handler_name == "health"));
    }
}
