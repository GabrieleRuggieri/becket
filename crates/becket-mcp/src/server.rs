//! MCP tool handlers exposing the Becket query engine to AI agents.

use std::path::PathBuf;
use std::sync::Arc;

use becket_core::wiki::WikiStore;
use becket_query::{ContextTask, QueryEngine};
use becket_store::BecketPaths;
use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ErrorCode},
    tool, tool_handler, tool_router, ErrorData as McpError, Peer, RoleServer, ServerHandler,
    ServiceExt,
};
use schemars::JsonSchema;
use serde::Deserialize;
use tracing::info;

use crate::sampling::{apply_flow_enrichment, apply_symbol_enrichment, enrich_wiki_prose};

/// Shared server state: repository root for query resolution.
#[derive(Clone)]
pub struct BecketMcpServer {
    /// Repository root containing `.becket/`.
    repo_root: Arc<PathBuf>,
    /// Required by rmcp tool routing (accessed via generated handler code).
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

/// Input for `get_context`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetContextParams {
    /// Symbol name or FQN.
    pub symbol: String,
    /// Optional token budget (default 6000).
    pub budget: Option<u32>,
    /// Task mode: fix, refactor, or onboard.
    pub task: Option<String>,
}

/// Input for `get_impact`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetImpactParams {
    /// Symbol name or FQN.
    pub symbol: String,
    /// Downstream traversal depth.
    #[serde(default = "default_depth")]
    pub depth: u32,
}

/// Input for `get_flow`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetFlowParams {
    /// Domain or flow name.
    pub domain: String,
}

/// Input for `get_dependencies`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetDependenciesParams {
    /// Symbol name or FQN.
    pub symbol: String,
    /// Downstream traversal depth.
    #[serde(default = "default_depth")]
    pub depth: u32,
}

/// Input for `get_wiki`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetWikiParams {
    /// Page id, title fragment, or `index` (default).
    pub page: Option<String>,
    /// Enrich prose slot via MCP sampling when supported.
    #[serde(default)]
    pub enrich: bool,
}

fn default_depth() -> u32 {
    3
}

fn parse_context_task(raw: Option<&str>) -> ContextTask {
    match raw.unwrap_or("fix").to_lowercase().as_str() {
        "refactor" => ContextTask::Refactor,
        "onboard" => ContextTask::Onboard,
        _ => ContextTask::Fix,
    }
}

impl BecketMcpServer {
    /// Creates a server bound to the repository at `repo_root`.
    pub fn new(repo_root: PathBuf) -> Self {
        Self {
            repo_root: Arc::new(repo_root),
            tool_router: Self::tool_router(),
        }
    }

    fn engine(&self) -> QueryEngine {
        QueryEngine::new(self.repo_root.as_path())
    }

    fn json_result<T: serde::Serialize>(value: &T) -> Result<CallToolResult, McpError> {
        let json = serde_json::to_string_pretty(value)
            .map_err(|e| McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    fn map_query_error(err: becket_query::QueryError) -> McpError {
        let code = match &err {
            becket_query::QueryError::NotFound(_) => ErrorCode::INVALID_PARAMS,
            becket_query::QueryError::IndexMissing(_) => ErrorCode::INVALID_REQUEST,
            becket_query::QueryError::Store(_) => ErrorCode::INTERNAL_ERROR,
        };
        McpError::new(code, err.to_string(), None)
    }
}

#[tool_router]
impl BecketMcpServer {
    /// Returns LLM-optimized context for a symbol.
    #[tool(
        description = "Get context bundle for a symbol: code snippets, impact, and markdown for agents"
    )]
    async fn get_context(
        &self,
        params: Parameters<GetContextParams>,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let engine = self.engine();
        let repo_root = self.repo_root.as_path().to_path_buf();
        let symbol = params.0.symbol;
        let budget = params.0.budget;
        let task = parse_context_task(params.0.task.as_deref());
        let mut result = tokio::task::spawn_blocking(move || engine.context(&symbol, budget, task))
            .await
            .map_err(|e| McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?
            .map_err(Self::map_query_error)?;
        result = apply_symbol_enrichment(&peer, &repo_root, result).await;
        Ok(CallToolResult::success(vec![Content::text(
            result.markdown,
        )]))
    }

    /// Returns downstream impact analysis for a symbol.
    #[tool(description = "Analyze downstream impact if a symbol changes")]
    async fn get_impact(
        &self,
        params: Parameters<GetImpactParams>,
    ) -> Result<CallToolResult, McpError> {
        let engine = self.engine();
        let symbol = params.0.symbol;
        let depth = params.0.depth;
        let result = tokio::task::spawn_blocking(move || engine.impact(&symbol, depth))
            .await
            .map_err(|e| McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?
            .map_err(Self::map_query_error)?;
        Self::json_result(&result)
    }

    /// Returns an end-to-end flow for a domain name.
    #[tool(description = "Get reconstructed business flow for a domain")]
    async fn get_flow(
        &self,
        params: Parameters<GetFlowParams>,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let engine = self.engine();
        let repo_root = self.repo_root.as_path().to_path_buf();
        let domain = params.0.domain;
        let mut result = tokio::task::spawn_blocking(move || engine.flow(&domain))
            .await
            .map_err(|e| McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?
            .map_err(Self::map_query_error)?;
        result = apply_flow_enrichment(&peer, &repo_root, result).await;
        Self::json_result(&result)
    }

    /// Returns direct and transitive dependencies for a symbol.
    #[tool(description = "List downstream dependencies for a symbol")]
    async fn get_dependencies(
        &self,
        params: Parameters<GetDependenciesParams>,
    ) -> Result<CallToolResult, McpError> {
        let engine = self.engine();
        let symbol = params.0.symbol;
        let depth = params.0.depth;
        let result = tokio::task::spawn_blocking(move || engine.dependencies(&symbol, depth))
            .await
            .map_err(|e| McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?
            .map_err(Self::map_query_error)?;
        Self::json_result(&result)
    }

    /// Returns a grounded wiki page or the index router.
    #[tool(description = "Get a graph-grounded wiki page (markdown) or index router")]
    async fn get_wiki(
        &self,
        params: Parameters<GetWikiParams>,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let repo_root = self.repo_root.as_path().to_path_buf();
        let paths = BecketPaths::new(&repo_root);
        let wiki_store = WikiStore::new(&paths);
        let query = params.0.page.unwrap_or_else(|| "index".into());
        let enrich = params.0.enrich;
        let query_for_err = query.clone();

        let mut page = tokio::task::spawn_blocking(move || resolve_wiki_page(&wiki_store, &query))
            .await
            .map_err(|e| McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?
            .map_err(|e| McpError::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?
            .ok_or_else(|| {
                McpError::new(
                    ErrorCode::INVALID_PARAMS,
                    format!("wiki page not found: {query_for_err}"),
                    None,
                )
            })?;

        if enrich {
            page = enrich_wiki_prose(&peer, &repo_root, page).await;
        }

        let markdown = format!(
            "---\n{}\n---\n\n{}",
            toml::to_string(&page.meta).map_err(|e| McpError::new(
                ErrorCode::INTERNAL_ERROR,
                e.to_string(),
                None
            ))?,
            page.body
        );
        Ok(CallToolResult::success(vec![Content::text(markdown)]))
    }
}

fn resolve_wiki_page(
    store: &WikiStore,
    query: &str,
) -> Result<Option<becket_schema::wiki::WikiPage>, becket_core::error::CoreError> {
    let q = query.trim().to_lowercase();
    if q == "index" || q == "wiki_index" {
        return store.load_index();
    }
    if let Some(page) = store.load_page(query)? {
        return Ok(Some(page));
    }
    for id in store.list_page_ids()? {
        let Some(page) = store.load_page(&id)? else {
            continue;
        };
        if page.meta.id.to_lowercase() == q
            || page.meta.title.to_lowercase().contains(&q)
            || page
                .meta
                .id
                .strip_prefix("wiki_")
                .unwrap_or(&page.meta.id)
                .to_lowercase()
                .contains(&q)
        {
            return Ok(Some(page));
        }
    }
    Ok(None)
}

#[tool_handler(
    name = "becket-mcp",
    version = "0.2.0",
    instructions = "Becket MCP server. Run `becket build` in the target repository first. Tools: get_context, get_wiki, get_impact, get_flow, get_dependencies. When the host supports MCP sampling, get_context, get_flow, and get_wiki (enrich=true) lazily enrich summaries via the host model and cache them locally."
)]
impl ServerHandler for BecketMcpServer {}

/// Runs the MCP server over stdio for the repository at `repo_root`.
pub async fn serve(repo_root: PathBuf) -> anyhow::Result<()> {
    use rmcp::transport::stdio;

    info!(root = %repo_root.display(), "starting becket-mcp");
    let server = BecketMcpServer::new(repo_root);
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
