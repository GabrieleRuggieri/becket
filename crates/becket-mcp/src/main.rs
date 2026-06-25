//! Becket MCP server — JSON-RPC over stdio for AI agent integration.

mod sampling;
mod server;

use std::path::PathBuf;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("becket=info".parse()?))
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();

    let repo_root = std::env::var("BECKET_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().expect("cwd"));

    server::serve(repo_root).await
}
