//! Context assembly: code snippets + impact + markdown bundle.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use repoctx_schema::artifacts::SymbolRecord;
use repoctx_store::IndexStore;

use crate::types::{CodeSnippet, ContextResult, ContextTask, SummarySource};

const DEFAULT_BUDGET: u32 = 6000;
const CHARS_PER_TOKEN: u32 = 4;

/// Assembles a rich context bundle for a symbol within a token budget.
pub fn assemble_context(
    store: &IndexStore,
    repo_root: &Path,
    root: SymbolRecord,
    budget: Option<u32>,
    task: ContextTask,
) -> Result<ContextResult, crate::error::QueryError> {
    let budget = budget.unwrap_or(DEFAULT_BUDGET);
    let impact_depth = task.impact_depth();

    let all_symbols = store.load_symbols()?;
    let id_to_symbol: HashMap<_, _> = all_symbols.iter().map(|s| (s.id.as_str(), s)).collect();

    let callers = direct_neighbors(store, &root.id, true)?;
    let callees = direct_neighbors(store, &root.id, false)?;
    let affected_ids = store.downstream_symbols(&root.id, impact_depth)?;

    let related_components: Vec<String> = all_symbols
        .iter()
        .filter(|s| s.file_path == root.file_path && s.id != root.id)
        .map(|s| s.name.clone())
        .take(10)
        .collect();

    let responsibility = format!(
        "{} ({:?}) defined in {} at lines {}-{}",
        root.name, root.kind, root.file_path, root.start_line, root.end_line
    );

    let external_dependencies = root
        .file_path
        .split('/')
        .take(2)
        .map(str::to_string)
        .collect();

    let invariants = vec![format!("visibility: {:?}", root.visibility)];

    let mut ranked = rank_symbols(
        &root,
        &callers,
        &callees,
        &affected_ids,
        &related_components,
    );
    ranked.sort_by_key(|(_, priority)| *priority);

    let mut snippets = Vec::new();
    let mut used_tokens = 0u32;
    let mut seen = HashSet::new();
    let max_snippets = match task {
        ContextTask::Onboard => 3,
        _ => usize::MAX,
    };

    for (sym_id, _) in ranked {
        if snippets.len() >= max_snippets {
            break;
        }
        if seen.contains(&sym_id) {
            continue;
        }
        seen.insert(sym_id.clone());
        let Some(sym) = id_to_symbol.get(sym_id.as_str()) else {
            continue;
        };
        let Some(snippet) = slice_symbol(repo_root, sym) else {
            continue;
        };
        let est = estimate_tokens(&snippet.content);
        if used_tokens + est > budget && !snippets.is_empty() {
            break;
        }
        used_tokens += est;
        snippets.push(snippet);
    }

    let caller_names: Vec<String> = callers
        .iter()
        .filter_map(|id| id_to_symbol.get(id.as_str()).map(|s| s.name.clone()))
        .collect();
    let callee_names: Vec<String> = callees
        .iter()
        .filter_map(|id| id_to_symbol.get(id.as_str()).map(|s| s.name.clone()))
        .collect();

    let markdown = render_markdown(&MarkdownInput {
        root: &root,
        responsibility: &responsibility,
        snippets: &snippets,
        callers: &caller_names,
        callees: &callee_names,
        affected_ids: &affected_ids,
        id_to_symbol: &id_to_symbol,
        related: &related_components,
        task,
    });

    Ok(ContextResult {
        symbol: root,
        responsibility,
        enriched_summary: None,
        summary_source: SummarySource::Deterministic,
        related_components,
        external_dependencies,
        invariants,
        semantic_neighbors: Vec::new(),
        snippets,
        callers: caller_names,
        callees: callee_names,
        affected_symbol_ids: affected_ids,
        markdown,
        task,
        budget_tokens: budget,
    })
}

fn direct_neighbors(
    store: &IndexStore,
    symbol_id: &str,
    upstream: bool,
) -> Result<Vec<String>, crate::error::QueryError> {
    let edges = store.load_call_edges()?;
    Ok(edges
        .into_iter()
        .filter_map(|(src, dst)| {
            if upstream && dst == symbol_id {
                Some(src)
            } else if !upstream && src == symbol_id {
                Some(dst)
            } else {
                None
            }
        })
        .collect())
}

fn rank_symbols(
    root: &SymbolRecord,
    callers: &[String],
    callees: &[String],
    affected: &[String],
    related: &[String],
) -> Vec<(String, u8)> {
    let mut out = Vec::new();
    out.push((root.id.clone(), 0));
    for id in callers {
        out.push((id.clone(), 1));
    }
    for id in callees {
        out.push((id.clone(), 2));
    }
    for id in affected {
        if id != &root.id {
            out.push((id.clone(), 3));
        }
    }
    // same-file related names resolved later via related_components only in markdown
    let _ = related;
    out
}

fn slice_symbol(repo_root: &Path, symbol: &SymbolRecord) -> Option<CodeSnippet> {
    let path = repo_root.join(&symbol.file_path);
    let content = fs::read_to_string(&path).ok()?;
    let lines: Vec<&str> = content.lines().collect();
    let start = symbol.start_line.saturating_sub(1) as usize;
    let end = (symbol.end_line as usize).min(lines.len());
    if start >= lines.len() || start >= end {
        return None;
    }
    let slice = lines[start..end].join("\n");
    let lang = extension_to_lang(path.extension()?.to_str()?);
    Some(CodeSnippet {
        symbol_id: symbol.id.clone(),
        symbol_name: symbol.name.clone(),
        file_path: symbol.file_path.clone(),
        start_line: symbol.start_line,
        end_line: symbol.end_line,
        language: lang.to_string(),
        content: slice,
    })
}

fn extension_to_lang(ext: &str) -> &'static str {
    match ext {
        "rs" => "rust",
        "py" => "python",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "ts" | "tsx" => "typescript",
        "go" => "go",
        "java" => "java",
        _ => "text",
    }
}

fn estimate_tokens(text: &str) -> u32 {
    (text.len() as u32).div_ceil(CHARS_PER_TOKEN) + 20
}

struct MarkdownInput<'a> {
    root: &'a SymbolRecord,
    responsibility: &'a str,
    snippets: &'a [CodeSnippet],
    callers: &'a [String],
    callees: &'a [String],
    affected_ids: &'a [String],
    id_to_symbol: &'a HashMap<&'a str, &'a SymbolRecord>,
    related: &'a [String],
    task: ContextTask,
}

fn render_markdown(input: &MarkdownInput<'_>) -> String {
    let MarkdownInput {
        root,
        responsibility,
        snippets,
        callers,
        callees,
        affected_ids,
        id_to_symbol,
        related,
        task,
    } = input;
    let mut md = String::new();
    md.push_str(&format!("# Context: {}\n\n", root.name));
    md.push_str(&format!("**Task:** {:?}\n\n", task));
    md.push_str("## Symbol\n\n");
    md.push_str(responsibility);
    md.push('\n');

    if !snippets.is_empty() {
        md.push_str("\n## Code\n\n");
        for snip in *snippets {
            md.push_str(&format!(
                "### {} (`{}:{}-{}`)\n\n",
                snip.symbol_name, snip.file_path, snip.start_line, snip.end_line
            ));
            md.push_str(&format!("```{}\n", snip.language));
            md.push_str(&snip.content);
            if !snip.content.ends_with('\n') {
                md.push('\n');
            }
            md.push_str("```\n\n");
        }
    }

    if !callers.is_empty() || !callees.is_empty() {
        md.push_str("## Call graph\n\n");
        if !callers.is_empty() {
            md.push_str(&format!("**Callers:** {}\n\n", callers.join(", ")));
        }
        if !callees.is_empty() {
            md.push_str(&format!("**Callees:** {}\n\n", callees.join(", ")));
        }
    }

    if !affected_ids.is_empty() {
        md.push_str("## Impact\n\n");
        md.push_str("Symbols affected if this changes:\n\n");
        for id in affected_ids.iter().take(20) {
            if let Some(sym) = id_to_symbol.get(id.as_str()) {
                md.push_str(&format!(
                    "- **{}** — `{}:{}-{}`\n",
                    sym.name, sym.file_path, sym.start_line, sym.end_line
                ));
            }
        }
        if affected_ids.len() > 20 {
            md.push_str(&format!("\n_…and {} more_\n", affected_ids.len() - 20));
        }
        md.push('\n');
    }

    if !related.is_empty() {
        md.push_str(&format!("## Related in module\n\n{}\n", related.join(", ")));
    }

    md
}
