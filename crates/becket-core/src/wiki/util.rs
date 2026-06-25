//! Shared wiki helpers (rendering, prose merge, context sanitization).

use std::collections::HashMap;

use regex::Regex;
use becket_schema::artifacts::SymbolRecord;
use becket_schema::wiki::WikiPageKind;

/// Placeholder text shown before MCP prose enrichment.
pub const PROSE_PLACEHOLDER: &str =
    "_Intent and gotchas can be enriched via MCP `get_wiki` with `enrich=true`._";

/// Machine-readable prose slot marker in compiled pages.
pub const PROSE_SLOT: &str = "<!-- becket:slot prose -->";

/// Resolves a symbol id to a human display name.
pub fn sym_name<'a>(id: &str, names: &HashMap<&'a str, &'a str>) -> String {
    names
        .get(id)
        .map(|n| (*n).to_string())
        .unwrap_or_else(|| short_id(id))
}

/// Builds id → name map from symbol records.
pub fn symbol_name_map(symbols: &[SymbolRecord]) -> HashMap<&str, &str> {
    symbols
        .iter()
        .map(|s| (s.id.as_str(), s.name.as_str()))
        .collect()
}

/// Short hash prefix for unknown symbol ids.
pub fn short_id(id: &str) -> String {
    if id.len() <= 12 {
        return id.to_string();
    }
    format!("{}…", &id[..8])
}

/// Renders a verified call edge with claim block (ids in claim, names in prose).
pub fn format_call_edge(src: &str, dst: &str, names: &HashMap<&str, &str>) -> String {
    format!(
        "<!-- becket:claim calls {dst} source=graph -->\n- **{}** → **{}**\n",
        sym_name(src, names),
        sym_name(dst, names)
    )
}

/// Default empty prose slot section for compiled pages.
pub fn prose_slot() -> String {
    format!("\n## Intent & gotchas\n\n{PROSE_SLOT}\n{PROSE_PLACEHOLDER}\n")
}

/// Copies enriched prose from `existing` into `compiled` when the slot was filled.
pub fn merge_preserved_prose(existing: &str, compiled: &str) -> String {
    let Some(existing_prose) = extract_prose_content(existing) else {
        return compiled.to_string();
    };
    if existing_prose == PROSE_PLACEHOLDER || existing_prose.trim().is_empty() {
        return compiled.to_string();
    }
    replace_prose_slot(compiled, &existing_prose)
}

/// Extracts user- or MCP-authored prose from a page body.
pub fn extract_prose_content(body: &str) -> Option<String> {
    let idx = body.find(PROSE_SLOT)?;
    let after = &body[idx + PROSE_SLOT.len()..];
    let end = after.find("\n## ").unwrap_or(after.len());
    let prose = after[..end].trim();
    if prose.is_empty() || prose == PROSE_PLACEHOLDER {
        return None;
    }
    Some(prose.to_string())
}

/// Replaces the prose slot content in a page body.
pub fn replace_prose_slot(body: &str, prose: &str) -> String {
    let Some(idx) = body.find(PROSE_SLOT) else {
        return format!("{body}\n\n{prose}");
    };
    let after = &body[idx + PROSE_SLOT.len()..];
    let end = after
        .find("\n## ")
        .map(|i| idx + PROSE_SLOT.len() + i)
        .unwrap_or(body.len());
    let mut out = String::new();
    out.push_str(&body[..idx + PROSE_SLOT.len()]);
    out.push('\n');
    out.push_str(prose);
    out.push_str(&body[end..]);
    out
}

/// Strips machine blocks and empty prose for agent context bundles.
pub fn sanitize_for_context(body: &str) -> String {
    let claim_re = Regex::new(r"<!--\s*becket:claim[^>]*-->\n?").expect("claim regex");

    let enriched = extract_prose_content(body);
    let mut text = claim_re.replace_all(body, "").into_owned();

    if let Some(prose) = enriched {
        if let Some(start) = text.find("## Intent & gotchas") {
            let rest = &text[start + "## Intent & gotchas".len()..];
            let end = rest
                .find("\n## ")
                .map(|i| start + "## Intent & gotchas".len() + i)
                .unwrap_or(text.len());
            let replacement = format!("## Intent & gotchas\n\n{prose}\n");
            text = format!("{}{}{}", &text[..start], replacement, &text[end..]);
        }
    } else {
        text = text.replace(PROSE_SLOT, "");
        text = text.replace(PROSE_PLACEHOLDER, "");
        if let Some(start) = text.find("## Intent & gotchas") {
            let rest = &text[start + "## Intent & gotchas".len()..];
            let end = rest
                .find("\n## ")
                .map(|i| start + "## Intent & gotchas".len() + i);
            if let Some(end) = end {
                text = format!("{}{}", &text[..start], &text[end..]);
            } else {
                text = text[..start].to_string();
            }
        }
    }

    while text.contains("\n\n\n") {
        text = text.replace("\n\n\n", "\n\n");
    }
    text.trim().to_string()
}

/// Returns true when sanitized wiki body adds non-redundant context.
pub fn wiki_adds_context(sanitized: &str) -> bool {
    let trimmed = sanitized.trim();
    !trimmed.is_empty() && trimmed.len() > 40
}

/// Priority for choosing the best wiki page for a symbol (lower = preferred).
pub fn page_kind_priority(kind: WikiPageKind) -> u8 {
    match kind {
        WikiPageKind::Service => 0,
        WikiPageKind::Module => 1,
        WikiPageKind::Flow => 2,
        WikiPageKind::Concept => 3,
        WikiPageKind::Overview => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_preserves_enriched_prose() {
        let existing = format!(
            "## Intent & gotchas\n\n{PROSE_SLOT}\nCustom team notes about payment edge cases.\n"
        );
        let compiled = format!("# Flow\n{PROSE_SLOT}\n{PROSE_PLACEHOLDER}\n");
        let merged = merge_preserved_prose(&existing, &compiled);
        assert!(merged.contains("Custom team notes"));
        assert!(!merged.contains(PROSE_PLACEHOLDER));
    }

    #[test]
    fn sanitize_strips_claims_and_placeholder() {
        let body = format!(
            "# svc\n\n<!-- becket:claim calls abc source=graph -->\n- **a** → **b**\n{}",
            prose_slot()
        );
        let clean = sanitize_for_context(&body);
        assert!(!clean.contains("becket:claim"));
        assert!(!clean.contains(PROSE_PLACEHOLDER));
        assert!(clean.contains("**a**"));
    }

    #[test]
    fn sanitize_keeps_enriched_prose() {
        let body = replace_prose_slot(&prose_slot(), "Always validate card token first.");
        let clean = sanitize_for_context(&format!("# S\n{body}"));
        assert!(clean.contains("validate card token"));
    }
}
