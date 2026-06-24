//! MCP tool handlers exposing the RepoCtx query engine to AI agents.

use std::path::PathBuf;
use std::sync::Arc;

use repoctx_query::{ContextTask, QueryEngine};
use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ErrorCode},
    tool, tool_handler, tool_router, ErrorData as McpError, Peer, RoleServer, ServerHandler,
    ServiceExt,
};
use schemars::JsonSchema;
use serde::Deserialize;
use tracing::info;

use crate::sampling::{apply_flow_enrichment, apply_symbol_enrichment};

/// Shared server state: repository root for query resolution.
#[derive(Clone)]
pub struct RepoCtxMcpServer {
    /// Repository root containing `.repoctx/`.
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

impl RepoCtxMcpServer {
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

    fn map_query_error(err: repoctx_query::QueryError) -> McpError {
        let code = match &err {
            repoctx_query::QueryError::NotFound(_) => ErrorCode::INVALID_PARAMS,
            repoctx_query::QueryError::IndexMissing(_) => ErrorCode::INVALID_REQUEST,
            repoctx_query::QueryError::Store(_) => ErrorCode::INTERNAL_ERROR,
        };
        McpError::new(code, err.to_string(), None)
    }
}

#[tool_router]
impl RepoCtxMcpServer {
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
}

#[tool_handler(
    name = "repoctx-mcp",
    version = "0.1.0",
    instructions = "RepoCtx MCP server. Run `repoctx build` in the target repository first. Tools: get_context, get_impact, get_flow, get_dependencies. When the host supports MCP sampling, get_context and get_flow lazily enrich summaries via the host model and cache them locally."
)]
impl ServerHandler for RepoCtxMcpServer {}

/// Runs the MCP server over stdio for the repository at `repo_root`.
pub async fn serve(repo_root: PathBuf) -> anyhow::Result<()> {
    use rmcp::transport::stdio;

    info!(root = %repo_root.display(), "starting repoctx-mcp");
    let server = RepoCtxMcpServer::new(repo_root);
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
