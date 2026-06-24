//! RepoCtx CLI — local codebase intelligence layer.

mod commands;
mod watch;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

/// Local intelligence layer for codebases — build, query, and export AI-ready context.
#[derive(Debug, Parser)]
#[command(name = "repoctx", version, about, long_about = None)]
struct Cli {
    /// Repository root (defaults to current directory).
    #[arg(long, global = true, default_value = ".")]
    repo: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Analyze the repository and emit `.repoctx/` artifacts.
    Build {
        /// Re-parse only changed files.
        #[arg(long, default_value_t = true)]
        incremental: bool,
        /// Skip local embedding generation.
        #[arg(long)]
        no_embeddings: bool,
        /// Rebuild incrementally when source files change.
        #[arg(long)]
        watch: bool,
        /// Emit machine-readable JSON to stdout.
        #[arg(long)]
        json: bool,
    },
    /// Show downstream impact for a symbol.
    Impact {
        /// Symbol name or FQN.
        symbol: String,
        /// Traversal depth.
        #[arg(long, default_value_t = 3)]
        depth: u32,
        #[arg(long)]
        json: bool,
    },
    /// Show an end-to-end flow for a domain.
    Flow {
        /// Domain or flow name.
        domain: String,
        #[arg(long)]
        json: bool,
    },
    /// Generate minimal LLM context for a symbol.
    Context {
        /// Symbol name or FQN.
        symbol: String,
        /// Approximate token budget (reserved for future compression).
        #[arg(long)]
        budget: Option<u32>,
        #[arg(long)]
        json: bool,
    },
    /// Refine auto-discovered domains (persisted in the index).
    Domain {
        #[command(subcommand)]
        action: DomainAction,
    },
}

#[derive(Debug, Subcommand)]
enum DomainAction {
    /// Rename an auto-discovered domain.
    Rename {
        /// Auto-generated domain id.
        auto_id: String,
        /// New human-friendly name.
        name: String,
    },
    /// Attach symbols or paths to a domain.
    Add {
        /// Domain name.
        name: String,
        /// Paths or symbol names.
        targets: Vec<String>,
    },
}

fn main() -> ExitCode {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("repoctx=info".parse()?))
        .with_target(false)
        .init();

    let cli = Cli::parse();
    commands::execute(cli)
}
