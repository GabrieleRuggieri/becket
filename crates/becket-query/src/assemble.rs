//! Context assembly: code snippets + impact + markdown bundle.
//!
//! Relevance is a continuous multi-signal score per candidate symbol:
//! confidence-weighted graph proximity, semantic similarity, git co-change
//! coupling, and structural centrality — task-mode dependent weights. The
//! packer then greedily fills the token budget in score order.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::Path;

use becket_core::build::META_EMBEDDER;
use becket_core::wiki::{sanitize_for_context, wiki_adds_context, WikiPageIndex, WikiStore};
use becket_embed::{current_embedder_id, embed_with_model, symbol_embedding_text};
use becket_schema::artifacts::SymbolRecord;
use becket_schema::edge::EdgeType;
use becket_store::{BecketPaths, IndexStore};

use crate::budget::{
    build_advice, estimate_impact_line_tokens, estimate_snippet_tokens, estimate_tokens,
    impact_display_cap, pack_impact_count, recommend_budget, PackingStats, OVERHEAD_TOKENS,
};
use crate::types::{BudgetAdvice, CodeSnippet, ContextResult, ContextTask, SummarySource};

/// Options for context assembly.
#[derive(Debug, Clone, Copy)]
pub struct AssembleOptions {
    /// `None` selects the recommended budget for this symbol and task.
    pub budget: Option<u32>,
    pub task: ContextTask,
    /// Budget guidance only — skips reading source files for snippets.
    pub plan_only: bool,
}

impl Default for AssembleOptions {
    fn default() -> Self {
        Self {
            budget: None,
            task: ContextTask::Fix,
            plan_only: false,
        }
    }
}

/// Assembles a rich context bundle for a symbol within a token budget.
pub fn assemble_context(
    store: &IndexStore,
    repo_root: &Path,
    root: SymbolRecord,
    budget: Option<u32>,
    task: ContextTask,
) -> Result<ContextResult, crate::error::QueryError> {
    assemble_context_with_options(
        store,
        repo_root,
        root,
        AssembleOptions {
            budget,
            task,
            plan_only: false,
        },
    )
}

/// Assembles context with explicit options.
pub fn assemble_context_with_options(
    store: &IndexStore,
    repo_root: &Path,
    root: SymbolRecord,
    options: AssembleOptions,
) -> Result<ContextResult, crate::error::QueryError> {
    let task = options.task;
    let impact_depth = task.impact_depth();

    let all_symbols = store.load_symbols()?;
    let id_to_symbol: HashMap<_, _> = all_symbols.iter().map(|s| (s.id.as_str(), s)).collect();

    let caller_hits = store.direct_call_neighbors(&root.id, true)?;
    let callee_hits = store.direct_call_neighbors(&root.id, false)?;
    let callers: Vec<String> = caller_hits.iter().map(|(id, _)| id.clone()).collect();
    let callees: Vec<String> = callee_hits.iter().map(|(id, _)| id.clone()).collect();
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

    let test_symbol_ids = if matches!(task, ContextTask::Fix) {
        collect_test_symbol_ids(&all_symbols, &root, &callers, &callees)
    } else {
        Vec::new()
    };

    let related_tests: Vec<String> = test_symbol_ids
        .iter()
        .filter_map(|id| id_to_symbol.get(id.as_str()))
        .map(|s| s.file_path.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let semantic_hits = semantic_neighbor_hits(store, &root, 5)?;
    let semantic_ids: Vec<String> = semantic_hits.iter().map(|(id, _)| id.clone()).collect();

    let signals = RankSignals::collect(
        store,
        &root,
        &id_to_symbol,
        &semantic_hits,
        &test_symbol_ids,
        task,
    )?;
    let ranked = rank_candidates(
        &root,
        &callers,
        &callees,
        &test_symbol_ids,
        &semantic_ids,
        &affected_ids,
        &signals,
        task,
    );

    let max_snippets = match task {
        ContextTask::Onboard => 3,
        _ => usize::MAX,
    };

    let paths = BecketPaths::new(repo_root);
    let wiki_index = WikiPageIndex::load(&WikiStore::new(&paths)).unwrap_or_default();

    let wiki_page = wiki_index.best_for_symbol(&root.id).cloned();
    let flow_wiki_page = if matches!(task, ContextTask::Onboard) {
        wiki_index.best_flow_for_symbol(&root.id).cloned()
    } else {
        None
    };

    let wiki_sanitized = wiki_page
        .as_ref()
        .map(|p| sanitize_for_context(&p.body))
        .filter(|body| wiki_adds_context(body));
    let flow_wiki_sanitized = flow_wiki_page
        .as_ref()
        .map(|p| sanitize_for_context(&p.body))
        .filter(|body| wiki_adds_context(body));

    // Low-confidence (syntactic-guess) neighbors are marked with `?` so the
    // agent can weigh them accordingly.
    let caller_names: Vec<String> = neighbor_display_names(&caller_hits, &id_to_symbol);
    let callee_names: Vec<String> = neighbor_display_names(&callee_hits, &id_to_symbol);

    let impact_line_tokens: Vec<u32> = affected_ids
        .iter()
        .filter_map(|id| id_to_symbol.get(id.as_str()))
        .map(|sym| {
            estimate_impact_line_tokens(&sym.name, &sym.file_path, sym.start_line, sym.end_line)
        })
        .collect();

    let mut candidate_snippets = Vec::new();
    let mut snippet_token_estimates = Vec::new();
    let mut file_cache = FileCache::default();

    if options.plan_only {
        for (sym_id, _) in &ranked {
            if candidate_snippets.len() >= max_snippets && max_snippets != usize::MAX {
                break;
            }
            let Some(sym) = id_to_symbol.get(sym_id.as_str()) else {
                continue;
            };
            let est = estimate_symbol_heuristic_tokens(sym);
            candidate_snippets.push(heuristic_snippet(sym, est));
            snippet_token_estimates.push(est);
        }
    } else {
        let mut seen = HashSet::new();
        for (sym_id, _) in &ranked {
            if candidate_snippets.len() >= max_snippets && max_snippets != usize::MAX {
                break;
            }
            if !seen.insert(sym_id.clone()) {
                continue;
            }
            let Some(sym) = id_to_symbol.get(sym_id.as_str()) else {
                continue;
            };
            let Some(snippet) = slice_symbol_cached(&mut file_cache, repo_root, sym) else {
                continue;
            };
            snippet_token_estimates.push(estimate_snippet_tokens(&snippet));
            candidate_snippets.push(snippet);
        }
    }

    let wiki_tokens = wiki_sanitized
        .as_ref()
        .map(|b| estimate_tokens(b) + estimate_tokens("## Knowledge\n\n"))
        .unwrap_or(0);
    let flow_wiki_tokens = flow_wiki_sanitized
        .as_ref()
        .map(|b| estimate_tokens(b) + estimate_tokens("## Flow overview\n\n"))
        .unwrap_or(0);

    let recommended = recommend_budget(
        task,
        wiki_tokens,
        flow_wiki_tokens,
        &impact_line_tokens,
        &snippet_token_estimates,
        max_snippets,
    );

    let requested_budget = options.budget.unwrap_or(recommended);
    let mut remaining = requested_budget.saturating_sub(OVERHEAD_TOKENS);

    if wiki_tokens > 0 && wiki_tokens <= remaining {
        remaining = remaining.saturating_sub(wiki_tokens);
    } else if wiki_tokens > remaining && wiki_tokens <= requested_budget / 2 {
        remaining = 0;
    }

    if flow_wiki_tokens > 0 && flow_wiki_tokens <= remaining {
        remaining = remaining.saturating_sub(flow_wiki_tokens);
    }

    let impact_cap = impact_display_cap(task);
    let impact_shown = pack_impact_count(&impact_line_tokens, remaining, impact_cap);
    let impact_section_tokens = if impact_shown > 0 {
        let header = estimate_tokens("## Impact\n\nSymbols affected if this changes:\n\n");
        let lines: u32 = impact_line_tokens.iter().take(impact_shown).sum();
        header + lines
    } else {
        0
    };
    remaining = remaining.saturating_sub(impact_section_tokens);

    let mut snippets = Vec::new();
    let mut snippet_tokens_used = 0u32;
    for (snippet, est) in candidate_snippets
        .into_iter()
        .zip(snippet_token_estimates.iter())
    {
        if snippets.is_empty() || snippet_tokens_used + est <= remaining {
            snippet_tokens_used += est;
            snippets.push(snippet);
        } else {
            break;
        }
    }

    let snippets_total = snippet_token_estimates.len();
    let snippets_included = snippets.len();

    let budget_advice = build_advice(
        task,
        requested_budget,
        recommended,
        0,
        PackingStats {
            snippets_included,
            snippets_total,
            impact_shown,
            impact_total: affected_ids.len(),
        },
    );

    let semantic_neighbors: Vec<String> = semantic_ids
        .iter()
        .filter_map(|id| id_to_symbol.get(id.as_str()).map(|s| s.name.clone()))
        .collect();

    if options.plan_only {
        let markdown = render_plan_markdown(&root, task, &budget_advice, &related_tests);
        let mut advice = budget_advice;
        advice.estimated_tokens = estimate_tokens(&markdown);
        return Ok(ContextResult {
            symbol: root,
            responsibility,
            enriched_summary: None,
            summary_source: SummarySource::Deterministic,
            related_components,
            external_dependencies,
            invariants,
            semantic_neighbors,
            snippets: Vec::new(),
            callers: caller_names,
            callees: callee_names,
            related_tests,
            affected_symbol_ids: affected_ids,
            wiki_page_id: wiki_page.as_ref().map(|p| p.meta.id.clone()),
            wiki_body: wiki_sanitized,
            flow_wiki_page_id: flow_wiki_page.as_ref().map(|p| p.meta.id.clone()),
            flow_wiki_body: flow_wiki_sanitized,
            budget_advice: advice,
            markdown,
            task,
            budget_tokens: requested_budget,
        });
    }

    let markdown = render_markdown(&RenderInput {
        root: &root,
        responsibility: &responsibility,
        snippets: &snippets,
        callers: &caller_names,
        callees: &callee_names,
        affected_ids: &affected_ids,
        impact_shown,
        id_to_symbol: &id_to_symbol,
        related: &related_components,
        related_tests: &related_tests,
        wiki_page_id: wiki_page.as_ref().map(|p| p.meta.id.as_str()),
        wiki_body: wiki_sanitized.as_deref(),
        flow_wiki_page_id: flow_wiki_page.as_ref().map(|p| p.meta.id.as_str()),
        flow_wiki_body: flow_wiki_sanitized.as_deref(),
        task,
        budget_advice: Some(&budget_advice),
    });

    let mut budget_advice = budget_advice;
    budget_advice.estimated_tokens = estimate_tokens(&markdown);

    Ok(ContextResult {
        symbol: root,
        responsibility,
        enriched_summary: None,
        summary_source: SummarySource::Deterministic,
        related_components,
        external_dependencies,
        invariants,
        semantic_neighbors,
        snippets,
        callers: caller_names,
        callees: callee_names,
        related_tests,
        affected_symbol_ids: affected_ids,
        wiki_page_id: wiki_page.as_ref().map(|p| p.meta.id.clone()),
        wiki_body: wiki_sanitized,
        flow_wiki_page_id: flow_wiki_page.as_ref().map(|p| p.meta.id.clone()),
        flow_wiki_body: flow_wiki_sanitized,
        budget_advice,
        markdown,
        task,
        budget_tokens: requested_budget,
    })
}

/// Rebuilds the markdown bundle from an assembled context (e.g. after wiki enrichment).
pub fn refresh_context_markdown(context: &ContextResult) -> String {
    let id_to_symbol: HashMap<&str, &SymbolRecord> =
        std::iter::once((context.symbol.id.as_str(), &context.symbol)).collect();
    let impact_shown = context.budget_advice.impact_entries_shown;
    render_markdown(&RenderInput {
        root: &context.symbol,
        responsibility: &context.responsibility,
        snippets: &context.snippets,
        callers: &context.callers,
        callees: &context.callees,
        affected_ids: &context.affected_symbol_ids,
        impact_shown,
        id_to_symbol: &id_to_symbol,
        related: &context.related_components,
        related_tests: &context.related_tests,
        wiki_page_id: context.wiki_page_id.as_deref(),
        wiki_body: context.wiki_body.as_deref(),
        flow_wiki_page_id: context.flow_wiki_page_id.as_deref(),
        flow_wiki_body: context.flow_wiki_body.as_deref(),
        task: context.task,
        budget_advice: Some(&context.budget_advice),
    })
}

/// Returns true for conventional test/spec file paths.
#[must_use]
pub fn is_test_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.contains("test")
        || lower.contains("spec")
        || lower.contains("__tests__")
        || lower.contains("_test.")
}

fn collect_test_symbol_ids(
    all_symbols: &[SymbolRecord],
    root: &SymbolRecord,
    callers: &[String],
    callees: &[String],
) -> Vec<String> {
    let root_name = root.name.to_lowercase();
    let neighbor_names: HashSet<String> = callers
        .iter()
        .chain(callees.iter())
        .filter_map(|id| {
            all_symbols
                .iter()
                .find(|s| s.id == *id)
                .map(|s| s.name.to_lowercase())
        })
        .collect();

    let mut out = Vec::new();
    for sym in all_symbols {
        if !is_test_path(&sym.file_path) {
            continue;
        }
        let name = sym.name.to_lowercase();
        let relevant = name.contains(&root_name)
            || root_name.contains(&name)
            || neighbor_names
                .iter()
                .any(|n| name.contains(n) || n.contains(&name));
        if relevant {
            out.push(sym.id.clone());
        }
    }
    out.sort();
    out.dedup();
    out.truncate(8);
    out
}

/// Confidence below this threshold is flagged as uncertain in the bundle.
const UNCERTAIN_CONFIDENCE: f32 = 0.6;

fn neighbor_display_names(
    hits: &[(String, f32)],
    id_to_symbol: &HashMap<&str, &SymbolRecord>,
) -> Vec<String> {
    hits.iter()
        .filter_map(|(id, confidence)| {
            id_to_symbol.get(id.as_str()).map(|s| {
                if *confidence < UNCERTAIN_CONFIDENCE {
                    format!("{}?", s.name)
                } else {
                    s.name.clone()
                }
            })
        })
        .collect()
}

/// Per-task weights for the multi-signal relevance score.
struct ScoreWeights {
    proximity: f32,
    semantic: f32,
    cochange: f32,
    centrality: f32,
    /// Flat boost for related test symbols.
    test_boost: f32,
}

impl ScoreWeights {
    fn for_task(task: ContextTask) -> Self {
        match task {
            // Fix: local neighborhood + tests + empirical coupling.
            ContextTask::Fix => Self {
                proximity: 0.45,
                semantic: 0.15,
                cochange: 0.20,
                centrality: 0.05,
                test_boost: 0.30,
            },
            // Refactor: graph structure dominates (all transitive deps matter).
            ContextTask::Refactor => Self {
                proximity: 0.55,
                semantic: 0.10,
                cochange: 0.15,
                centrality: 0.20,
                test_boost: 0.10,
            },
            // Onboard: hubs and semantically related overview.
            ContextTask::Onboard => Self {
                proximity: 0.35,
                semantic: 0.25,
                cochange: 0.05,
                centrality: 0.35,
                test_boost: 0.0,
            },
        }
    }
}

/// Normalized per-candidate signals used by the ranker.
struct RankSignals {
    /// Confidence-weighted graph proximity to the root (0..1].
    proximity: HashMap<String, f32>,
    /// Embedding similarity (0..1] for semantic neighbors.
    semantic: HashMap<String, f32>,
    /// Git co-change coupling with the root's file (0..1].
    cochange: HashMap<String, f32>,
    /// Normalized in-degree (0..1].
    centrality: HashMap<String, f32>,
    test_ids: HashSet<String>,
}

/// Depth for confidence-weighted proximity propagation.
const PROXIMITY_DEPTH: u32 = 4;
/// Per-hop decay applied on top of edge confidence.
const PROXIMITY_DECAY: f32 = 0.7;

impl RankSignals {
    fn collect(
        store: &IndexStore,
        root: &SymbolRecord,
        id_to_symbol: &HashMap<&str, &SymbolRecord>,
        semantic_hits: &[(String, f32)],
        test_symbol_ids: &[String],
        _task: ContextTask,
    ) -> Result<Self, crate::error::QueryError> {
        let proximity = proximity_scores(store, &root.id)?;

        let semantic: HashMap<String, f32> = semantic_hits.iter().cloned().collect();

        // Co-change: file-level counts mapped onto candidate symbols.
        let co_rows = store.co_change_for_file(&root.file_path)?;
        let max_count = co_rows.iter().map(|(_, c)| *c).max().unwrap_or(0);
        let file_scores: HashMap<&str, f32> = co_rows
            .iter()
            .map(|(file, count)| {
                (
                    file.as_str(),
                    if max_count > 0 {
                        *count as f32 / max_count as f32
                    } else {
                        0.0
                    },
                )
            })
            .collect();
        let mut cochange = HashMap::new();
        if !file_scores.is_empty() {
            for symbol in id_to_symbol.values() {
                if let Some(score) = file_scores.get(symbol.file_path.as_str()) {
                    cochange.insert(symbol.id.clone(), *score);
                }
            }
        }

        let in_degrees = store.symbol_in_degrees()?;
        let max_degree = in_degrees.values().copied().max().unwrap_or(0);
        let centrality: HashMap<String, f32> = if max_degree > 0 {
            in_degrees
                .into_iter()
                .map(|(id, degree)| (id, degree as f32 / max_degree as f32))
                .collect()
        } else {
            HashMap::new()
        };

        Ok(Self {
            proximity,
            semantic,
            cochange,
            centrality,
            test_ids: test_symbol_ids.iter().cloned().collect(),
        })
    }

    fn score_for(&self, id: &str, weights: &ScoreWeights) -> f32 {
        let mut score = 0.0;
        score += weights.proximity * self.proximity.get(id).copied().unwrap_or(0.0);
        score += weights.semantic * self.semantic.get(id).copied().unwrap_or(0.0);
        score += weights.cochange * self.cochange.get(id).copied().unwrap_or(0.0);
        score += weights.centrality * self.centrality.get(id).copied().unwrap_or(0.0);
        if self.test_ids.contains(id) {
            score += weights.test_boost;
        }
        score
    }
}

/// Confidence-weighted proximity: best path product of
/// `confidence × type_weight × decay` per hop, propagated both directions.
///
/// Import edges get a low type weight (file-level coupling), candidate edges
/// are naturally discounted by their 0.25 confidence.
fn proximity_scores(
    store: &IndexStore,
    root_id: &str,
) -> Result<HashMap<String, f32>, crate::error::QueryError> {
    let edges = store.load_weighted_edges()?;
    let mut adjacency: HashMap<&str, Vec<(&str, f32)>> = HashMap::new();
    for (src, dst, edge_type, confidence) in &edges {
        let weight = confidence * edge_type_weight(*edge_type);
        adjacency
            .entry(src.as_str())
            .or_default()
            .push((dst, weight));
        adjacency
            .entry(dst.as_str())
            .or_default()
            .push((src, weight));
    }

    let mut best: HashMap<String, f32> = HashMap::new();
    best.insert(root_id.to_string(), 1.0);
    let mut frontier: VecDeque<(String, f32, u32)> = VecDeque::new();
    frontier.push_back((root_id.to_string(), 1.0, 0));

    while let Some((current, score, depth)) = frontier.pop_front() {
        if depth >= PROXIMITY_DEPTH {
            continue;
        }
        let Some(neighbors) = adjacency.get(current.as_str()) else {
            continue;
        };
        for (next, weight) in neighbors {
            let next_score = score * weight * PROXIMITY_DECAY;
            if next_score < 0.01 {
                continue;
            }
            let entry = best.entry((*next).to_string()).or_insert(0.0);
            if next_score > *entry {
                *entry = next_score;
                frontier.push_back(((*next).to_string(), next_score, depth + 1));
            }
        }
    }

    best.remove(root_id);
    Ok(best)
}

fn edge_type_weight(edge_type: EdgeType) -> f32 {
    match edge_type {
        EdgeType::Calls => 1.0,
        EdgeType::Extends | EdgeType::Implements => 0.9,
        EdgeType::Http | EdgeType::Grpc | EdgeType::Queue => 0.8,
        EdgeType::Reads | EdgeType::Writes | EdgeType::References => 0.5,
        EdgeType::Imports => 0.3,
    }
}

/// Ranks all candidate symbols by the continuous multi-signal score.
///
/// The root always comes first; ties break on symbol id for determinism.
#[allow(clippy::too_many_arguments)]
fn rank_candidates(
    root: &SymbolRecord,
    callers: &[String],
    callees: &[String],
    test_ids: &[String],
    semantic: &[String],
    affected: &[String],
    signals: &RankSignals,
    task: ContextTask,
) -> Vec<(String, f32)> {
    let weights = ScoreWeights::for_task(task);

    let mut candidates: Vec<&String> = Vec::new();
    candidates.extend(callers);
    candidates.extend(callees);
    if !matches!(task, ContextTask::Onboard) {
        candidates.extend(test_ids);
    }
    candidates.extend(semantic);
    candidates.extend(affected);

    let mut seen = HashSet::new();
    let mut scored: Vec<(String, f32)> = Vec::new();
    for id in candidates {
        if *id == root.id || !seen.insert(id.clone()) {
            continue;
        }
        scored.push((id.clone(), signals.score_for(id, &weights)));
    }

    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    let mut out = Vec::with_capacity(scored.len() + 1);
    out.push((root.id.clone(), f32::MAX));
    out.extend(scored);
    out
}

/// Semantic neighbors with similarity scores.
///
/// Guard: when the index was built in a different embedding space (hash vs
/// ONNX), vector distances are meaningless — skip semantic search entirely.
fn semantic_neighbor_hits(
    store: &IndexStore,
    symbol: &SymbolRecord,
    limit: usize,
) -> Result<Vec<(String, f32)>, crate::error::QueryError> {
    if store.count_symbol_embeddings()? == 0 {
        return Ok(Vec::new());
    }
    if let Ok(Some(indexed_space)) = store.get_meta(META_EMBEDDER) {
        if indexed_space != current_embedder_id() {
            tracing::debug!(
                indexed = %indexed_space,
                current = %current_embedder_id(),
                "embedding space mismatch; skipping semantic neighbors"
            );
            return Ok(Vec::new());
        }
    }
    let query = embed_with_model(&symbol_embedding_text(symbol));
    let hits = store.nearest_symbol_ids(&query, limit + 1)?;
    Ok(hits
        .into_iter()
        .filter(|(id, _)| id != &symbol.id)
        .take(limit)
        .map(|(id, distance)| (id, (1.0 / (1.0 + distance)) as f32))
        .collect())
}

#[derive(Default)]
struct FileCache {
    lines: HashMap<String, Vec<String>>,
}

impl FileCache {
    fn lines_for(&mut self, repo_root: &Path, file_path: &str) -> Option<&[String]> {
        let path = repo_root.join(file_path);
        if !self.lines.contains_key(file_path) {
            let content = fs::read_to_string(&path).ok()?;
            self.lines.insert(
                file_path.to_string(),
                content.lines().map(str::to_string).collect(),
            );
        }
        self.lines.get(file_path).map(Vec::as_slice)
    }
}

fn slice_symbol_cached(
    cache: &mut FileCache,
    repo_root: &Path,
    symbol: &SymbolRecord,
) -> Option<CodeSnippet> {
    let lines = cache.lines_for(repo_root, &symbol.file_path)?;
    let start = symbol.start_line.saturating_sub(1) as usize;
    let end = (symbol.end_line as usize).min(lines.len());
    if start >= lines.len() || start >= end {
        return None;
    }
    let slice = lines[start..end].join("\n");
    let path = repo_root.join(&symbol.file_path);
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

fn estimate_symbol_heuristic_tokens(sym: &SymbolRecord) -> u32 {
    let lines = sym.end_line.saturating_sub(sym.start_line) + 1;
    estimate_snippet_tokens(&CodeSnippet {
        symbol_id: sym.id.clone(),
        symbol_name: sym.name.clone(),
        file_path: sym.file_path.clone(),
        start_line: sym.start_line,
        end_line: sym.end_line,
        language: "text".into(),
        content: "x".repeat((lines as usize).saturating_mul(40)),
    })
}

fn heuristic_snippet(sym: &SymbolRecord, _est: u32) -> CodeSnippet {
    CodeSnippet {
        symbol_id: sym.id.clone(),
        symbol_name: sym.name.clone(),
        file_path: sym.file_path.clone(),
        start_line: sym.start_line,
        end_line: sym.end_line,
        language: "text".into(),
        content: String::new(),
    }
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

fn render_plan_markdown(
    root: &SymbolRecord,
    task: ContextTask,
    advice: &BudgetAdvice,
    related_tests: &[String],
) -> String {
    let mut md = format!("# Context plan: {}\n\n", root.name);
    md.push_str(&format!("**Task:** {:?}\n\n", task));
    md.push_str("## Budget guidance\n\n");
    md.push_str(&format!(
        "- **Recommended:** ~{} tokens\n",
        advice.recommended_tokens
    ));
    md.push_str(&format!(
        "- **Default for task:** {} tokens\n",
        task.default_budget()
    ));
    md.push_str(&format!(
        "- **Candidate snippets:** {}\n",
        advice.snippets_included + advice.snippets_omitted
    ));
    md.push_str(&format!(
        "- **Impact symbols (depth {}):** {}\n",
        task.impact_depth(),
        advice.impact_entries_total
    ));
    if !related_tests.is_empty() {
        md.push_str("\n## Related tests\n\n");
        for path in related_tests {
            md.push_str(&format!("- `{path}`\n"));
        }
    }
    md.push_str("\n_Re-run without `--plan` (or MCP `get_context`) for the full bundle._\n");
    md
}

struct RenderInput<'a> {
    root: &'a SymbolRecord,
    responsibility: &'a str,
    snippets: &'a [CodeSnippet],
    callers: &'a [String],
    callees: &'a [String],
    affected_ids: &'a [String],
    impact_shown: usize,
    id_to_symbol: &'a HashMap<&'a str, &'a SymbolRecord>,
    related: &'a [String],
    related_tests: &'a [String],
    wiki_page_id: Option<&'a str>,
    wiki_body: Option<&'a str>,
    flow_wiki_page_id: Option<&'a str>,
    flow_wiki_body: Option<&'a str>,
    task: ContextTask,
    budget_advice: Option<&'a BudgetAdvice>,
}

fn render_markdown(input: &RenderInput<'_>) -> String {
    let RenderInput {
        root,
        responsibility,
        snippets,
        callers,
        callees,
        affected_ids,
        impact_shown,
        id_to_symbol,
        related,
        related_tests,
        wiki_page_id,
        wiki_body,
        flow_wiki_page_id,
        flow_wiki_body,
        task,
        budget_advice,
    } = input;
    let mut md = String::new();
    md.push_str(&format!("# Context: {}\n\n", root.name));
    md.push_str(&format!("**Task:** {:?}\n\n", task));

    if let Some(advice) = budget_advice {
        if !advice.within_budget {
            md.push_str("> **Budget notice:** The requested budget may be too small for this ");
            md.push_str(&format!(
                "symbol and task. Used ~{} tokens; recommended ~{} tokens",
                advice.estimated_tokens, advice.recommended_tokens
            ));
            if advice.snippets_omitted > 0 {
                md.push_str(&format!(" ({} snippet(s) omitted", advice.snippets_omitted));
                if advice.impact_entries_shown < advice.impact_entries_total {
                    md.push_str(&format!(
                        ", impact truncated to {}",
                        advice.impact_entries_shown
                    ));
                }
                md.push(')');
            } else if advice.impact_entries_shown < advice.impact_entries_total {
                md.push_str(&format!(
                    " (impact truncated to {} of {})",
                    advice.impact_entries_shown, advice.impact_entries_total
                ));
            }
            md.push_str(". Re-run with `--auto-budget` or `--budget ");
            md.push_str(&advice.recommended_tokens.to_string());
            md.push_str("`.\n\n");
        }
    }

    md.push_str("## Symbol\n\n");
    md.push_str(responsibility);
    md.push('\n');

    if let (Some(page_id), Some(body)) = (flow_wiki_page_id, flow_wiki_body) {
        if !body.is_empty() {
            md.push_str("\n## Flow overview\n\n");
            md.push_str(&format!("_Grounded markdown: `{page_id}`_\n\n"));
            md.push_str(body);
            if !body.ends_with('\n') {
                md.push('\n');
            }
            md.push('\n');
        }
    }

    if let (Some(page_id), Some(body)) = (wiki_page_id, wiki_body) {
        if !body.is_empty() {
            md.push_str("\n## Knowledge\n\n");
            md.push_str(&format!("_Grounded markdown: `{page_id}`_\n\n"));
            md.push_str(body);
            if !body.ends_with('\n') {
                md.push('\n');
            }
            md.push('\n');
        }
    }

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

    if !related_tests.is_empty() {
        md.push_str("## Related tests\n\n");
        for path in *related_tests {
            md.push_str(&format!("- `{path}`\n"));
        }
        md.push('\n');
    }

    if !callers.is_empty() || !callees.is_empty() {
        md.push_str("## Call graph\n\n");
        if !callers.is_empty() {
            md.push_str(&format!("**Callers:** {}\n\n", callers.join(", ")));
        }
        if !callees.is_empty() {
            md.push_str(&format!("**Callees:** {}\n\n", callees.join(", ")));
        }
        if callers
            .iter()
            .chain(callees.iter())
            .any(|n| n.ends_with('?'))
        {
            md.push_str("_Names marked `?` are uncertain syntactic matches (low confidence)._\n\n");
        }
    }

    if *impact_shown > 0 {
        md.push_str("## Impact\n\n");
        md.push_str("Symbols affected if this changes:\n\n");
        for id in affected_ids.iter().take(*impact_shown) {
            if let Some(sym) = id_to_symbol.get(id.as_str()) {
                md.push_str(&format!(
                    "- **{}** — `{}:{}-{}`\n",
                    sym.name, sym.file_path, sym.start_line, sym.end_line
                ));
            }
        }
        if affected_ids.len() > *impact_shown {
            md.push_str(&format!(
                "\n_…and {} more (increase `--budget` or use `--auto-budget`)_\n",
                affected_ids.len() - impact_shown
            ));
        }
        md.push('\n');
    }

    if !related.is_empty() {
        md.push_str(&format!("## Related in module\n\n{}\n", related.join(", ")));
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use becket_schema::symbol::{SymbolKind, Visibility};

    fn sample_symbol(id: &str, name: &str, path: &str) -> SymbolRecord {
        SymbolRecord {
            id: id.into(),
            kind: SymbolKind::Function,
            name: name.into(),
            fqn: name.into(),
            file_path: path.into(),
            start_line: 1,
            end_line: 5,
            visibility: Visibility::Public,
            module_id: None,
        }
    }

    #[test]
    fn is_test_path_detects_conventional_layouts() {
        assert!(is_test_path("src/payment/checkout_test.rs"));
        assert!(is_test_path("tests/integration/spec.ts"));
        assert!(!is_test_path("src/payment/checkout.rs"));
    }

    #[test]
    fn collect_test_symbols_prefers_name_match() {
        let root = sample_symbol("sym_root", "checkout", "src/checkout.rs");
        let test_sym = sample_symbol("sym_test", "test_checkout", "tests/checkout_test.rs");
        let noise = sample_symbol("sym_noise", "other", "tests/other_test.rs");
        let ids = collect_test_symbol_ids(&[root.clone(), test_sym, noise], &root, &[], &[]);
        assert_eq!(ids, vec!["sym_test".to_string()]);
    }

    fn empty_signals(test_ids: &[&str]) -> RankSignals {
        RankSignals {
            proximity: HashMap::new(),
            semantic: HashMap::new(),
            cochange: HashMap::new(),
            centrality: HashMap::new(),
            test_ids: test_ids.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn fix_task_ranks_tests_before_affected() {
        let root = sample_symbol("sym_root", "pay", "src/pay.rs");
        let ranked = rank_candidates(
            &root,
            &[],
            &[],
            &["sym_test".into()],
            &[],
            &["sym_far".into()],
            &empty_signals(&["sym_test"]),
            ContextTask::Fix,
        );
        let test_pos = ranked.iter().position(|(id, _)| id == "sym_test").unwrap();
        let far_pos = ranked.iter().position(|(id, _)| id == "sym_far").unwrap();
        assert!(
            test_pos < far_pos,
            "test boost should outrank distant impact"
        );
        assert_eq!(ranked[0].0, "sym_root", "root always first");
    }

    #[test]
    fn higher_proximity_ranks_first_with_deterministic_ties() {
        let root = sample_symbol("sym_root", "pay", "src/pay.rs");
        let mut signals = empty_signals(&[]);
        signals.proximity.insert("sym_near".into(), 0.9);
        signals.proximity.insert("sym_mid".into(), 0.4);
        let ranked = rank_candidates(
            &root,
            &["sym_mid".into(), "sym_near".into()],
            &[],
            &[],
            &[],
            &["sym_a_tie".into(), "sym_b_tie".into()],
            &signals,
            ContextTask::Refactor,
        );
        let ids: Vec<&str> = ranked.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["sym_root", "sym_near", "sym_mid", "sym_a_tie", "sym_b_tie"]
        );
    }

    #[test]
    fn refresh_markdown_includes_budget_notice_when_truncated() {
        let symbol = sample_symbol("sym1", "pay", "src/pay.rs");
        let advice = BudgetAdvice {
            requested_budget: 1000,
            recommended_tokens: 5000,
            estimated_tokens: 1200,
            snippets_included: 1,
            snippets_omitted: 2,
            impact_entries_shown: 5,
            impact_entries_total: 20,
            within_budget: false,
        };
        let ctx = ContextResult {
            symbol,
            responsibility: "pay".into(),
            enriched_summary: None,
            summary_source: SummarySource::Deterministic,
            related_components: vec![],
            external_dependencies: vec![],
            invariants: vec![],
            semantic_neighbors: vec![],
            snippets: vec![],
            callers: vec![],
            callees: vec![],
            related_tests: vec![],
            affected_symbol_ids: (0..20).map(|i| format!("sym{i}")).collect(),
            wiki_page_id: None,
            wiki_body: None,
            flow_wiki_page_id: None,
            flow_wiki_body: None,
            budget_advice: advice,
            markdown: String::new(),
            task: ContextTask::Fix,
            budget_tokens: 1000,
        };
        let md = refresh_context_markdown(&ctx);
        assert!(md.contains("Budget notice"));
    }
}
