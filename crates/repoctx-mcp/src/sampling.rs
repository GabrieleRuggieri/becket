//! Lazy LLM enrichment via MCP sampling (host-delegated).

use std::path::Path;

use repoctx_core::redact_secrets;
use repoctx_core::wiki::{replace_prose_slot, WikiStore, PROSE_SLOT};
use repoctx_query::types::{ContextResult, FlowResult, SummarySource};
use repoctx_schema::artifacts::FlowRecord;
use repoctx_store::{EnrichmentRecord, IndexStore, RepoCtxPaths};
use rmcp::{
    model::{CreateMessageRequestParams, CreateMessageResult, SamplingMessage},
    Peer, RoleServer,
};
use tracing::debug;

const ENTITY_SYMBOL: &str = "symbol";
const ENTITY_FLOW: &str = "flow";
const ENTITY_WIKI: &str = "wiki";
const SOURCE_SAMPLING: &str = "mcp_sampling";

/// Returns true when the connected MCP client can handle `sampling/createMessage`.
pub fn client_supports_sampling(peer: &Peer<RoleServer>) -> bool {
    let Some(info) = peer.peer_info() else {
        return false;
    };
    let caps = &info.capabilities;
    if caps
        .tasks
        .as_ref()
        .map(|tasks| tasks.supports_sampling_create_message())
        .unwrap_or(false)
    {
        return true;
    }
    caps.sampling.is_some()
}

/// Loads a cached enrichment or requests one from the host via MCP sampling.
pub async fn enrich_symbol_context(
    peer: &Peer<RoleServer>,
    repo_root: &Path,
    context: &ContextResult,
) -> Option<String> {
    if !client_supports_sampling(peer) {
        return None;
    }

    let paths = RepoCtxPaths::new(repo_root);
    let symbol_id = context.symbol.id.clone();

    if let Some(cached) = load_cached(ENTITY_SYMBOL, &symbol_id, &paths.index_db) {
        return Some(cached);
    }

    let prompt = build_symbol_prompt(context);
    let summary = request_summary(peer, &prompt).await?;
    cache_enrichment(ENTITY_SYMBOL, &symbol_id, &summary, &paths.index_db);
    Some(summary)
}

/// Loads or generates an enriched flow description.
pub async fn enrich_flow_description(
    peer: &Peer<RoleServer>,
    repo_root: &Path,
    flow: &FlowRecord,
) -> Option<String> {
    if !client_supports_sampling(peer) {
        return None;
    }

    let paths = RepoCtxPaths::new(repo_root);
    let flow_id = flow.id.clone();

    if let Some(cached) = load_cached(ENTITY_FLOW, &flow_id, &paths.index_db) {
        return Some(cached);
    }

    let prompt = build_flow_prompt(flow);
    let summary = request_summary(peer, &prompt).await?;
    cache_enrichment(ENTITY_FLOW, &flow_id, &summary, &paths.index_db);
    Some(summary)
}

/// Applies cached or freshly sampled enrichment to a context result.
pub async fn apply_symbol_enrichment(
    peer: &Peer<RoleServer>,
    repo_root: &Path,
    mut context: ContextResult,
) -> ContextResult {
    if let Some(summary) = enrich_symbol_context(peer, repo_root, &context).await {
        context.enriched_summary = Some(summary.clone());
        context.summary_source = SummarySource::McpSampling;
        context.markdown.push_str("\n\n## Enriched summary\n\n");
        context.markdown.push_str(&summary);
    }
    context
}

/// Applies cached or freshly sampled enrichment to a flow result.
pub async fn apply_flow_enrichment(
    peer: &Peer<RoleServer>,
    repo_root: &Path,
    mut result: FlowResult,
) -> FlowResult {
    let Some(flow) = result.flow.as_ref() else {
        return result;
    };
    if let Some(description) = enrich_flow_description(peer, repo_root, flow).await {
        result.enriched_description = Some(description);
        result.description_source = SummarySource::McpSampling;
    }
    result
}

/// Fills the prose slot on a wiki page via MCP sampling when the host supports it.
pub async fn enrich_wiki_prose(
    peer: &Peer<RoleServer>,
    repo_root: &Path,
    mut page: repoctx_schema::wiki::WikiPage,
) -> repoctx_schema::wiki::WikiPage {
    if !client_supports_sampling(peer) || !page.body.contains(PROSE_SLOT) {
        return page;
    }

    let paths = RepoCtxPaths::new(repo_root);
    let page_id = page.meta.id.clone();

    if let Some(cached) = load_cached(ENTITY_WIKI, &page_id, &paths.index_db) {
        page.body = replace_prose_slot(&page.body, &cached);
        page.meta.source = repoctx_schema::wiki::WikiPageSource::McpSampling;
        let _ = WikiStore::new(&paths).write_page(&page.meta, &page.body);
        return page;
    }

    let prompt = build_wiki_prompt(&page);
    let Some(prose) = request_summary(peer, &prompt).await else {
        return page;
    };
    cache_enrichment(ENTITY_WIKI, &page_id, &prose, &paths.index_db);
    page.body = replace_prose_slot(&page.body, &prose);
    page.meta.source = repoctx_schema::wiki::WikiPageSource::McpSampling;
    let _ = WikiStore::new(&paths).write_page(&page.meta, &page.body);
    page
}

fn build_wiki_prompt(page: &repoctx_schema::wiki::WikiPage) -> String {
    let facts = format!(
        "title: {}\nkind: {:?}\nanchored_symbols: {}\nbody_excerpt:\n{}",
        page.meta.title,
        page.meta.kind,
        page.meta.symbol_ids.join(", "),
        page.body.chars().take(1200).collect::<String>(),
    );
    let redacted = redact_secrets(&facts);
    format!(
        "Write 2-4 sentences of intent and gotchas for this grounded wiki page. \
Use ONLY the facts below. Do not invent APIs or behavior.\n\n{redacted}"
    )
}

fn load_cached(entity_kind: &str, entity_id: &str, db_path: &Path) -> Option<String> {
    let store = IndexStore::open(db_path).ok()?;
    store
        .get_enrichment(entity_kind, entity_id)
        .ok()
        .flatten()
        .map(|record| record.summary)
}

fn cache_enrichment(entity_kind: &str, entity_id: &str, summary: &str, db_path: &Path) {
    if let Ok(store) = IndexStore::open(db_path) {
        let _ = store.upsert_enrichment(&EnrichmentRecord {
            entity_kind: entity_kind.to_string(),
            entity_id: entity_id.to_string(),
            summary: summary.to_string(),
            source: SOURCE_SAMPLING.to_string(),
        });
    }
}

fn build_symbol_prompt(context: &ContextResult) -> String {
    let facts = format!(
        "name: {}\nkind: {:?}\nfile: {}\nlines: {}-{}\ndeterministic_summary: {}\nrelated: {}\ninvariants: {}",
        context.symbol.name,
        context.symbol.kind,
        context.symbol.file_path,
        context.symbol.start_line,
        context.symbol.end_line,
        context.responsibility,
        context.related_components.join(", "),
        context.invariants.join(", "),
    );
    let redacted = redact_secrets(&facts);
    format!(
        "Summarize this code symbol in 1-2 concise sentences for an AI coding agent. \
Use ONLY the facts below. Do not invent behavior.\n\n{redacted}"
    )
}

fn build_flow_prompt(flow: &FlowRecord) -> String {
    let steps: Vec<String> = flow
        .steps
        .iter()
        .map(|step| format!("{} ({})", step.symbol_id, step.order))
        .collect();
    let facts = format!(
        "flow: {}\ndescription: {}\nsteps: {}",
        flow.name,
        flow.description.as_deref().unwrap_or("(none)"),
        steps.join(" -> "),
    );
    let redacted = redact_secrets(&facts);
    format!(
        "Summarize this business flow in 1-2 concise sentences for an AI coding agent. \
Use ONLY the facts below.\n\n{redacted}"
    )
}

async fn request_summary(peer: &Peer<RoleServer>, prompt: &str) -> Option<String> {
    let params = CreateMessageRequestParams::new(vec![SamplingMessage::user_text(prompt)], 256)
        .with_system_prompt(
            "You write concise, factual summaries of code symbols and flows for developers.",
        );

    #[allow(deprecated)]
    let result = peer.create_message(params).await.ok()?;
    let text = extract_sampling_text(&result)?;
    debug!(chars = text.len(), "received MCP sampling enrichment");
    Some(text)
}

fn extract_sampling_text(result: &CreateMessageResult) -> Option<String> {
    result
        .message
        .content
        .iter()
        .find_map(|content| content.as_text().map(|text| text.text.trim().to_string()))
        .filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use repoctx_query::types::{CodeSnippet, ContextTask};
    use repoctx_schema::artifacts::{FlowRecord, FlowStepRecord, SymbolRecord};
    use repoctx_schema::symbol::{SymbolKind, Visibility};

    #[test]
    fn build_symbol_prompt_includes_redacted_facts() {
        let context = ContextResult {
            symbol: SymbolRecord {
                id: "sym1".into(),
                kind: SymbolKind::Function,
                name: "pay".into(),
                fqn: "pay".into(),
                file_path: "src/payment.rs".into(),
                start_line: 1,
                end_line: 5,
                visibility: Visibility::Public,
                module_id: None,
            },
            responsibility: "pay function".into(),
            enriched_summary: None,
            summary_source: SummarySource::Deterministic,
            related_components: vec!["Charge".into()],
            external_dependencies: vec!["src".into()],
            invariants: vec!["visibility: Public".into()],
            semantic_neighbors: vec![],
            snippets: vec![CodeSnippet {
                symbol_id: "sym1".into(),
                symbol_name: "pay".into(),
                file_path: "src/payment.rs".into(),
                start_line: 1,
                end_line: 5,
                language: "rust".into(),
                content: "fn pay() {}".into(),
            }],
            callers: vec![],
            callees: vec![],
            affected_symbol_ids: vec![],
            wiki_page_id: None,
            wiki_body: None,
            markdown: "# Context: pay".into(),
            task: ContextTask::Fix,
            budget_tokens: 6000,
        };
        let prompt = build_symbol_prompt(&context);
        assert!(prompt.contains("pay"));
        assert!(prompt.contains("Charge"));
    }

    #[test]
    fn build_flow_prompt_lists_steps() {
        let flow = FlowRecord {
            id: "flow1".into(),
            name: "payment".into(),
            description: Some("auto".into()),
            steps: vec![
                FlowStepRecord {
                    order: 1,
                    symbol_id: "a".into(),
                    external_system: None,
                },
                FlowStepRecord {
                    order: 2,
                    symbol_id: "b".into(),
                    external_system: None,
                },
            ],
        };
        let prompt = build_flow_prompt(&flow);
        assert!(prompt.contains("payment"));
        assert!(prompt.contains("a"));
    }
}
