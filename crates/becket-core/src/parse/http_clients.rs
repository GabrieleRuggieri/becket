//! HTTP client call detection for cross-repo linking.

use tree_sitter::Node;

/// An outbound HTTP call detected in source (client side).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedHttpClient {
    /// Repository-relative file path.
    pub file_path: String,
    /// Symbol id of the calling function/method.
    pub caller_symbol_id: String,
    /// HTTP method.
    pub method: String,
    /// Normalized route path (leading slash, no host).
    pub path: String,
}

/// Detects fetch/axios/requests HTTP client calls inside a call expression.
pub fn detect_http_client_call(
    node: Node,
    source: &[u8],
    file_path: &str,
    caller_symbol_id: &str,
) -> Option<ParsedHttpClient> {
    if node.kind() != "call_expression" && node.kind() != "call" {
        return None;
    }

    let callee = node.child_by_field_name("function")?;
    let callee_text = node_text(callee, source)?;

    if callee_text == "fetch" {
        let path = extract_first_string_argument(node, source)?;
        return Some(ParsedHttpClient {
            file_path: file_path.to_string(),
            caller_symbol_id: caller_symbol_id.to_string(),
            method: "GET".to_string(),
            path: normalize_path(&path),
        });
    }

    if matches!(
        callee_text.as_str(),
        "http.Get" | "http.Post" | "http.Put" | "http.Delete"
    ) {
        let method = callee_text
            .strip_prefix("http.")
            .unwrap_or("Get")
            .to_uppercase();
        let path = extract_first_string_argument(node, source)?;
        return Some(ParsedHttpClient {
            file_path: file_path.to_string(),
            caller_symbol_id: caller_symbol_id.to_string(),
            method,
            path: normalize_path(&path),
        });
    }

    if callee_text == "urllib.request.urlopen" {
        let path = extract_first_string_argument(node, source)?;
        return Some(ParsedHttpClient {
            file_path: file_path.to_string(),
            caller_symbol_id: caller_symbol_id.to_string(),
            method: "GET".to_string(),
            path: normalize_path(&extract_path_from_url(&path)),
        });
    }

    if let Some((method, path)) = parse_member_route_call(&callee_text, node, source) {
        return Some(ParsedHttpClient {
            file_path: file_path.to_string(),
            caller_symbol_id: caller_symbol_id.to_string(),
            method,
            path: normalize_path(&path),
        });
    }

    if let Some((method, path)) = parse_requests_call(&callee_text, node, source) {
        return Some(ParsedHttpClient {
            file_path: file_path.to_string(),
            caller_symbol_id: caller_symbol_id.to_string(),
            method,
            path: normalize_path(&path),
        });
    }

    None
}

fn parse_member_route_call(
    callee_text: &str,
    node: Node,
    source: &[u8],
) -> Option<(String, String)> {
    let (object, method_name) = callee_text.rsplit_once('.')?;
    let method = match method_name {
        "get" => "GET",
        "post" => "POST",
        "put" => "PUT",
        "delete" => "DELETE",
        "patch" => "PATCH",
        "head" => "HEAD",
        "options" => "OPTIONS",
        _ => return None,
    };

    if !matches!(
        object,
        "axios" | "http" | "client" | "req" | "httpx" | "got" | "superagent"
    ) {
        return None;
    }

    let path = extract_first_string_argument(node, source)?;
    Some((method.to_string(), path))
}

fn parse_requests_call(callee_text: &str, node: Node, source: &[u8]) -> Option<(String, String)> {
    let method = match callee_text {
        "requests.get" | "httpx.get" => "GET",
        "requests.post" | "httpx.post" => "POST",
        "requests.put" | "httpx.put" => "PUT",
        "requests.delete" | "httpx.delete" => "DELETE",
        "requests.patch" | "httpx.patch" => "PATCH",
        _ => return None,
    };
    let raw = extract_first_string_argument(node, source)?;
    let path = extract_path_from_url(&raw);
    Some((method.to_string(), path))
}

fn extract_path_from_url(raw: &str) -> String {
    if let Some(rest) = raw
        .strip_prefix("http://")
        .or_else(|| raw.strip_prefix("https://"))
    {
        if let Some(path) = rest.split_once('/') {
            return normalize_path(&format!("/{}", path.1));
        }
        return "/".to_string();
    }
    normalize_path(raw)
}

fn normalize_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return "/".to_string();
    }
    if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn extract_first_string_argument(node: Node, source: &[u8]) -> Option<String> {
    let args = node.child_by_field_name("arguments")?;
    let mut cursor = args.walk();
    for child in args.children(&mut cursor) {
        if matches!(
            child.kind(),
            "string" | "string_literal" | "template_string" | "interpreted_string_literal"
        ) {
            return Some(unquote(node_text(child, source)?));
        }
    }
    None
}

fn unquote(value: String) -> String {
    let trimmed = value.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        || (trimmed.starts_with('`') && trimmed.ends_with('`'))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn node_text(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source).ok().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_adds_leading_slash() {
        assert_eq!(normalize_path("users"), "/users");
        assert_eq!(normalize_path("/users"), "/users");
    }

    #[test]
    fn extract_path_from_absolute_url() {
        assert_eq!(
            extract_path_from_url("http://users-service/users"),
            "/users"
        );
    }
}
