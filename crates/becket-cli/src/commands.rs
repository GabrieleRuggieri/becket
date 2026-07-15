//! CLI command handlers delegating to core and query crates.

use anyhow::{bail, Context, Result};
use becket_core::{
    wiki::{WikiCompiler, WikiLinter, WikiStore},
    BuildOptions, BuildPipeline, DomainEditor, WorkspacePipeline,
};
use becket_query::{AssembleOptions, ContextTask, QueryEngine};
use becket_schema::wiki::WikiStaleQueue;
use becket_store::{BecketPaths, IndexStore};
use serde::Serialize;
use std::fs;

use crate::watch;
use crate::{Cli, Commands, DomainAction, WikiAction, WorkspaceAction};

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
                        "build complete: {} parsed, {} skipped, {} symbols, {} edges, {} flows, {} wiki pages, {} embeddings → {}",
                        report.files_parsed,
                        report.files_skipped,
                        report.symbols_indexed,
                        report.edges_indexed,
                        report.flows_indexed,
                        report.wiki_pages_indexed,
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
            auto_budget,
            plan,
            task,
            json,
        } => {
            let engine = QueryEngine::new(&cli.repo);
            let task_mode: ContextTask = task.into();
            let resolved_budget = if auto_budget {
                None
            } else {
                Some(budget.unwrap_or_else(|| task_mode.default_budget()))
            };
            let options = AssembleOptions {
                budget: resolved_budget,
                task: task_mode,
                plan_only: plan,
            };
            let result = engine.context_with_options(&symbol, options)?;
            if json {
                print_json(&result)?;
            } else {
                if !plan && !result.budget_advice.within_budget {
                    eprintln!(
                        "hint: budget may be tight — used ~{} tokens, recommended ~{} (try --auto-budget or --budget {})",
                        result.budget_advice.estimated_tokens,
                        result.budget_advice.recommended_tokens,
                        result.budget_advice.recommended_tokens,
                    );
                }
                println!("{}", result.markdown);
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
        Commands::Wiki { action } => {
            match action {
                WikiAction::Sync { all } => {
                    let paths = BecketPaths::new(&cli.repo);
                    let store = IndexStore::open(&paths.index_db)
                        .context("index missing — run `becket build` first")?;
                    let compiler = WikiCompiler::new(paths.clone());
                    let page_ids = if all {
                        Vec::new()
                    } else {
                        load_stale_queue(&paths)?
                    };
                    let count = compiler.sync_pages(&store, &page_ids)?;
                    if all {
                        println!("wiki sync: recompiled all {count} page(s) (structure; prose preserved)");
                    } else if page_ids.is_empty() {
                        println!("wiki sync: no stale pages in queue (use --all to recompile everything)");
                    } else {
                        println!(
                        "wiki sync: recompiled {count} stale page(s) (structure; prose preserved)"
                    );
                    }
                }
                WikiAction::Lint { json, strict } => {
                    let paths = BecketPaths::new(&cli.repo);
                    let store = IndexStore::open(&paths.index_db)
                        .context("index missing — run `becket build` first")?;
                    let report = WikiLinter::new(paths).run(&store)?;
                    if json {
                        print_json(&report)?;
                    } else {
                        println!(
                            "wiki lint: {} stale, {} claim errors, {} broken links, {} orphans",
                            report.stale_page_ids.len(),
                            report.claim_errors.len(),
                            report.broken_links.len(),
                            report.orphan_page_ids.len()
                        );
                    }
                    if strict
                        && (!report.stale_page_ids.is_empty()
                            || !report.claim_errors.is_empty()
                            || !report.broken_links.is_empty())
                    {
                        bail!("wiki lint failed (--strict)");
                    }
                }
                WikiAction::Show { page, json } => {
                    let paths = BecketPaths::new(&cli.repo);
                    let wiki_store = WikiStore::new(&paths);
                    let loaded = resolve_wiki_page(&wiki_store, &page)?
                        .with_context(|| format!("wiki page not found: {page}"))?;
                    if json {
                        print_json(&loaded)?;
                    } else {
                        println!(
                            "---\n{}\n---\n\n{}",
                            toml::to_string(&loaded.meta)?,
                            loaded.body
                        );
                    }
                }
            }
        }
        Commands::Report { json, open } => {
            let metrics = becket_query::generate_report(&cli.repo)?;
            let report_dir = cli.repo.join(".becket/report");
            if json {
                print_json(&metrics)?;
            } else {
                println!(
                    "report: {} symbols, {} edges ({:.0}% high-confidence), {} co-change pairs",
                    metrics.graph.symbols,
                    metrics.graph.edges,
                    metrics.graph.high_confidence_ratio * 100.0,
                    metrics.graph.co_change_pairs,
                );
                if metrics.tokens.sampled_symbols > 0 {
                    println!(
                        "tokens: mean savings {:.0}% (bundle ~{} vs full files ~{}, {} samples)",
                        metrics.tokens.mean_savings_ratio * 100.0,
                        metrics.tokens.mean_bundle_tokens,
                        metrics.tokens.mean_baseline_tokens,
                        metrics.tokens.sampled_symbols,
                    );
                }
                if metrics.wiki.lint_available {
                    println!(
                        "wiki: {} stale, {} claim errors, {} broken links",
                        metrics.wiki.stale_pages,
                        metrics.wiki.claim_errors,
                        metrics.wiki.broken_links,
                    );
                }
                println!(
                    "integrity: {}/{} snippets fresh",
                    metrics.integrity.snippets_checked - metrics.integrity.stale_snippets,
                    metrics.integrity.snippets_checked,
                );
                println!("dashboard: {}", report_dir.join("index.html").display());
            }
            if open {
                let _ = std::process::Command::new("open")
                    .arg(report_dir.join("index.html"))
                    .spawn();
            }
        }
        Commands::Bench { last, json } => {
            let report = becket_query::run_bench(&cli.repo, last)?;
            if json {
                print_json(&report)?;
            } else if report.commits_evaluated == 0 {
                println!(
                    "bench: no evaluable commits in the last {last} (need ≥2 indexed files per commit; is this a git repo?)"
                );
            } else {
                println!(
                    "bench: {} commits evaluated (of {} requested)",
                    report.commits_evaluated, report.commits_requested
                );
                println!(
                    "mean recall: {:.0}%  ·  mean token cost: {:.0}% of full files",
                    report.mean_recall * 100.0,
                    report.mean_token_ratio * 100.0,
                );
                println!(
                    "{:<12} {:<24} {:>7} {:>9} {:>9}",
                    "commit", "seed", "recall", "bundle", "full"
                );
                for case in &report.cases {
                    println!(
                        "{:<12} {:<24} {:>6.0}% {:>9} {:>9}",
                        case.commit,
                        truncate(&case.seed_symbol, 24),
                        case.recall * 100.0,
                        case.bundle_tokens,
                        case.full_files_tokens,
                    );
                }
                println!("saved: .becket/report/bench.json");
            }
        }
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

fn load_stale_queue(paths: &BecketPaths) -> Result<Vec<String>> {
    let path = paths.wiki_stale_queue();
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&path)?;
    let queue: WikiStaleQueue = serde_json::from_str(&raw)?;
    Ok(queue.page_ids)
}

fn resolve_wiki_page(
    store: &WikiStore,
    query: &str,
) -> Result<Option<becket_schema::wiki::WikiPage>> {
    let q = query.trim().to_lowercase();
    if q == "index" || q == "wiki_index" {
        return Ok(store.load_index()?);
    }
    if let Ok(Some(page)) = store.load_page(query) {
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

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
