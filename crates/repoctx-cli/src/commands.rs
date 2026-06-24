//! CLI command handlers delegating to core and query crates.

use anyhow::Result;
use repoctx_core::{BuildOptions, BuildPipeline, DomainEditor, WorkspacePipeline};
use repoctx_query::QueryEngine;
use serde::Serialize;

use crate::watch;
use crate::{Cli, Commands, DomainAction, WorkspaceAction};

/// Dispatches the parsed CLI to the appropriate handler.
pub fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Build {
            incremental,
            no_embeddings,
            watch,
            json,
        } => {
            let options = BuildOptions {
                incremental,
                no_embeddings,
            };
            if watch {
                watch::run(&cli.repo, options, json)?;
            } else {
                let pipeline = BuildPipeline::new(&cli.repo, options);
                let report = pipeline.run()?;
                if json {
                    print_json(&report)?;
                } else {
                    println!(
                        "build complete: {} parsed, {} skipped, {} symbols, {} edges, {} flows, {} embeddings → {}",
                        report.files_parsed,
                        report.files_skipped,
                        report.symbols_indexed,
                        report.edges_indexed,
                        report.flows_indexed,
                        report.embeddings_indexed,
                        report.output_dir
                    );
                }
            }
        }
        Commands::Workspace { action } => match action {
            WorkspaceAction::Build {
                incremental,
                no_embeddings,
                json,
            } => {
                let options = BuildOptions {
                    incremental,
                    no_embeddings,
                };
                let report = WorkspacePipeline::new(&cli.repo, options).run()?;
                if json {
                    print_json(&report)?;
                } else {
                    println!(
                        "workspace build complete: {} repos, {} cross-repo edges → {}",
                        report.repos.len(),
                        report.cross_repo_edges,
                        report.output_dir
                    );
                    for repo in &report.repos {
                        println!(
                            "  {}: {} symbols, {} edges",
                            repo.name, repo.report.symbols_indexed, repo.report.edges_indexed
                        );
                    }
                }
            }
        },
        Commands::Impact {
            symbol,
            depth,
            json,
        } => {
            let engine = QueryEngine::new(&cli.repo);
            let result = engine.impact(&symbol, depth)?;
            if json {
                print_json(&result)?;
            } else {
                println!(
                    "impact for {} ({})",
                    result.symbol.name, result.symbol.file_path
                );
                println!("  affected modules: {}", result.affected_modules.len());
                println!("  downstream symbols: {}", result.affected_symbol_ids.len());
                for test in &result.related_tests {
                    println!("  related test: {test}");
                }
            }
        }
        Commands::Flow { domain, json } => {
            let engine = QueryEngine::new(&cli.repo);
            let result = engine.flow(&domain)?;
            if json {
                print_json(&result)?;
            } else if let Some(flow) = &result.flow {
                println!("flow: {}", flow.name);
                for step in &flow.steps {
                    println!("  {} → symbol {}", step.order, step.symbol_id);
                }
            } else {
                println!("no flow named '{domain}'");
                if !result.suggestions.is_empty() {
                    println!("suggestions: {}", result.suggestions.join(", "));
                }
            }
        }
        Commands::Context {
            symbol,
            budget,
            json,
        } => {
            let engine = QueryEngine::new(&cli.repo);
            let result = engine.context(&symbol, budget)?;
            if json {
                print_json(&result)?;
            } else {
                println!("{}", result.responsibility);
                if !result.related_components.is_empty() {
                    println!("related: {}", result.related_components.join(", "));
                }
            }
        }
        Commands::Domain { action } => match action {
            DomainAction::Rename { auto_id, name } => {
                let editor = DomainEditor::new(&cli.repo);
                let flow = editor.rename(&auto_id, &name)?;
                println!("renamed domain → {} ({})", flow.name, flow.id);
            }
            DomainAction::Add { name, targets } => {
                let editor = DomainEditor::new(&cli.repo);
                let flow = editor.add(&name, &targets)?;
                println!(
                    "updated domain {} ({} steps, id {})",
                    flow.name,
                    flow.steps.len(),
                    flow.id
                );
            }
        },
    }
    Ok(())
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
