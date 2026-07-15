//! `becket report`: measurable index-quality metrics + local HTML dashboard.
//!
//! Everything is computed from the local index and the working tree — no
//! network, no telemetry. Output:
//! - `.becket/report/metrics.json` — machine-readable metrics
//! - `.becket/report/index.html` — self-contained dashboard (embedded JSON)

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use becket_schema::wiki::WikiLintArtifact;
use becket_store::{BecketPaths, IndexStore};
use serde::{Deserialize, Serialize};

use crate::assemble::{assemble_context_with_options, AssembleOptions};
use crate::budget::estimate_tokens;
use crate::error::QueryError;
use crate::types::ContextTask;

/// Number of high-centrality symbols sampled for token metrics.
const SAMPLE_SIZE: usize = 10;

/// Full metrics document (`metrics.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportMetrics {
    /// Artifact schema version.
    pub schema_version: String,
    /// Graph size and confidence profile.
    pub graph: GraphMetrics,
    /// Wiki lint status (code ↔ documentation coherence).
    pub wiki: WikiMetrics,
    /// Token savings across sampled context bundles.
    pub tokens: TokenMetrics,
    /// Bundle integrity check (snippets vs current source on disk).
    pub integrity: IntegrityMetrics,
    /// Last build counters (from index metadata), when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_build: Option<serde_json::Value>,
}

/// Graph size and per-tier edge confidence profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphMetrics {
    /// Symbols in the index.
    pub symbols: usize,
    /// Edges in the index.
    pub edges: usize,
    /// Edge count per resolution tier (import_resolved, file_scoped, …).
    pub resolution_profile: BTreeMap<String, usize>,
    /// Share of edges resolved with high confidence (≥ 0.7).
    pub high_confidence_ratio: f64,
    /// Co-change file pairs mined from git history.
    pub co_change_pairs: usize,
}

/// Wiki lint summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiMetrics {
    /// True when a lint report was found.
    pub lint_available: bool,
    /// Stale pages (fingerprint drift).
    pub stale_pages: usize,
    /// Claim errors (documentation contradicts the graph).
    pub claim_errors: usize,
    /// Broken see-also links.
    pub broken_links: usize,
    /// Orphan pages.
    pub orphan_pages: usize,
}

/// Token savings: Becket bundle vs naive full-file baseline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenMetrics {
    /// Symbols sampled (highest in-degree).
    pub sampled_symbols: usize,
    /// Mean bundle tokens across samples.
    pub mean_bundle_tokens: u32,
    /// Mean baseline tokens (full content of every file the bundle touches).
    pub mean_baseline_tokens: u32,
    /// Mean savings ratio: `1 - bundle/baseline` (0.6 = 60% fewer tokens).
    pub mean_savings_ratio: f64,
    /// Per-symbol breakdown.
    pub samples: Vec<TokenSample>,
}

/// One sampled symbol's token comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenSample {
    /// Symbol name.
    pub symbol: String,
    /// Symbol file.
    pub file: String,
    /// Tokens in the assembled bundle markdown.
    pub bundle_tokens: u32,
    /// Tokens if the agent were given every touched file in full.
    pub baseline_tokens: u32,
    /// `1 - bundle/baseline`.
    pub savings_ratio: f64,
}

/// Snippet freshness check.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityMetrics {
    /// Snippets verified across sampled bundles.
    pub snippets_checked: usize,
    /// Snippets whose source lines no longer match the index (stale index).
    pub stale_snippets: usize,
}

/// Computes metrics and writes `metrics.json` + `index.html` under `.becket/report/`.
pub fn generate_report(repo_root: &Path) -> Result<ReportMetrics, QueryError> {
    let paths = BecketPaths::new(repo_root);
    if !paths.index_db.exists() {
        return Err(QueryError::IndexMissing(
            paths.index_db.display().to_string(),
        ));
    }
    let store = IndexStore::open(&paths.index_db)?;

    let metrics = compute_metrics(&store, &paths, repo_root)?;

    let report_dir = paths.output_dir.join("report");
    fs::create_dir_all(&report_dir).map_err(QueryError::from)?;
    let json =
        serde_json::to_string_pretty(&metrics).map_err(|e| QueryError::Internal(e.to_string()))?;
    fs::write(report_dir.join("metrics.json"), &json).map_err(QueryError::from)?;
    fs::write(report_dir.join("index.html"), render_dashboard(&json)).map_err(QueryError::from)?;

    Ok(metrics)
}

fn compute_metrics(
    store: &IndexStore,
    paths: &BecketPaths,
    repo_root: &Path,
) -> Result<ReportMetrics, QueryError> {
    let symbols = store.load_symbols()?;
    let profile: BTreeMap<String, usize> = store.edge_resolution_profile()?.into_iter().collect();
    let edges: usize = profile.values().sum();
    let high_confidence: usize = profile
        .iter()
        .filter(|(tier, _)| {
            matches!(
                tier.as_str(),
                "type_resolved" | "import_resolved" | "file_scoped" | "dir_scoped"
            )
        })
        .map(|(_, count)| count)
        .sum();
    let graph = GraphMetrics {
        symbols: symbols.len(),
        edges,
        resolution_profile: profile,
        high_confidence_ratio: if edges > 0 {
            high_confidence as f64 / edges as f64
        } else {
            0.0
        },
        co_change_pairs: store.count_co_change_pairs()?,
    };

    let wiki = read_wiki_metrics(paths);

    // Sample: highest in-degree symbols (the ones agents ask about most).
    let in_degrees = store.symbol_in_degrees()?;
    let mut ranked: Vec<_> = symbols.iter().collect();
    ranked.sort_by(|a, b| {
        let da = in_degrees.get(&a.id).copied().unwrap_or(0);
        let db = in_degrees.get(&b.id).copied().unwrap_or(0);
        db.cmp(&da).then_with(|| a.id.cmp(&b.id))
    });

    let mut samples = Vec::new();
    let mut snippets_checked = 0usize;
    let mut stale_snippets = 0usize;

    for symbol in ranked.into_iter().take(SAMPLE_SIZE) {
        let Ok(context) = assemble_context_with_options(
            store,
            repo_root,
            symbol.clone(),
            AssembleOptions {
                budget: None,
                task: ContextTask::Fix,
                plan_only: false,
            },
        ) else {
            continue;
        };

        let bundle_tokens = estimate_tokens(&context.markdown);

        // Baseline: the full content of every file whose symbols the bundle
        // covers (snippets, tests, impact) — what an agent would read in full
        // without symbol-level slicing.
        let id_to_file: HashMap<&str, &str> = symbols
            .iter()
            .map(|s| (s.id.as_str(), s.file_path.as_str()))
            .collect();
        let mut baseline_files: Vec<&str> = context
            .snippets
            .iter()
            .map(|s| s.file_path.as_str())
            .chain(context.related_tests.iter().map(String::as_str))
            .chain(
                context
                    .affected_symbol_ids
                    .iter()
                    .filter_map(|id| id_to_file.get(id.as_str()).copied()),
            )
            .collect();
        baseline_files.sort_unstable();
        baseline_files.dedup();
        let mut baseline_tokens = 0u32;
        for file in &baseline_files {
            if let Ok(content) = fs::read_to_string(repo_root.join(file)) {
                baseline_tokens += estimate_tokens(&content);
            }
        }

        for snippet in &context.snippets {
            snippets_checked += 1;
            if !snippet_matches_disk(repo_root, snippet) {
                stale_snippets += 1;
            }
        }

        if baseline_tokens > 0 {
            samples.push(TokenSample {
                symbol: symbol.name.clone(),
                file: symbol.file_path.clone(),
                bundle_tokens,
                baseline_tokens,
                savings_ratio: 1.0 - f64::from(bundle_tokens) / f64::from(baseline_tokens),
            });
        }
    }

    let tokens = aggregate_tokens(samples);
    let integrity = IntegrityMetrics {
        snippets_checked,
        stale_snippets,
    };

    let last_build = store
        .get_meta(becket_core::build::META_LAST_BUILD_REPORT)?
        .and_then(|raw| serde_json::from_str(&raw).ok());

    Ok(ReportMetrics {
        schema_version: becket_schema::SCHEMA_VERSION.to_string(),
        graph,
        wiki,
        tokens,
        integrity,
        last_build,
    })
}

fn aggregate_tokens(samples: Vec<TokenSample>) -> TokenMetrics {
    let n = samples.len();
    if n == 0 {
        return TokenMetrics {
            sampled_symbols: 0,
            mean_bundle_tokens: 0,
            mean_baseline_tokens: 0,
            mean_savings_ratio: 0.0,
            samples,
        };
    }
    let mean_bundle = samples
        .iter()
        .map(|s| u64::from(s.bundle_tokens))
        .sum::<u64>()
        / n as u64;
    let mean_baseline = samples
        .iter()
        .map(|s| u64::from(s.baseline_tokens))
        .sum::<u64>()
        / n as u64;
    let mean_savings = samples.iter().map(|s| s.savings_ratio).sum::<f64>() / n as f64;
    TokenMetrics {
        sampled_symbols: n,
        mean_bundle_tokens: mean_bundle as u32,
        mean_baseline_tokens: mean_baseline as u32,
        mean_savings_ratio: mean_savings,
        samples,
    }
}

fn read_wiki_metrics(paths: &BecketPaths) -> WikiMetrics {
    let lint_path = paths.wiki_lint_report();
    let Ok(raw) = fs::read_to_string(&lint_path) else {
        return WikiMetrics {
            lint_available: false,
            stale_pages: 0,
            claim_errors: 0,
            broken_links: 0,
            orphan_pages: 0,
        };
    };
    match serde_json::from_str::<WikiLintArtifact>(&raw) {
        Ok(lint) => WikiMetrics {
            lint_available: true,
            stale_pages: lint.stale_page_ids.len(),
            claim_errors: lint.claim_errors.len(),
            broken_links: lint.broken_links.len(),
            orphan_pages: lint.orphan_page_ids.len(),
        },
        Err(_) => WikiMetrics {
            lint_available: false,
            stale_pages: 0,
            claim_errors: 0,
            broken_links: 0,
            orphan_pages: 0,
        },
    }
}

fn snippet_matches_disk(repo_root: &Path, snippet: &crate::types::CodeSnippet) -> bool {
    let Ok(content) = fs::read_to_string(repo_root.join(&snippet.file_path)) else {
        return false;
    };
    let lines: Vec<&str> = content.lines().collect();
    let start = snippet.start_line.saturating_sub(1) as usize;
    let end = (snippet.end_line as usize).min(lines.len());
    if start >= lines.len() || start >= end {
        return false;
    }
    lines[start..end].join("\n") == snippet.content
}

/// Renders the self-contained HTML dashboard with the metrics JSON embedded.
fn render_dashboard(metrics_json: &str) -> String {
    // `</script>` inside JSON strings would break the inline block.
    let safe_json = metrics_json.replace("</", "<\\/");
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Becket Report</title>
<style>
  :root {{ --bg:#0f1115; --card:#181b22; --text:#e6e8ee; --muted:#9aa1b0; --accent:#5ac8fa; --good:#39d98a; --warn:#ffcc66; --bad:#ff6b6b; }}
  * {{ box-sizing: border-box; }}
  body {{ margin:0; padding:32px; font:15px/1.5 -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; background:var(--bg); color:var(--text); }}
  h1 {{ font-size:22px; margin:0 0 4px; }}
  .sub {{ color:var(--muted); margin-bottom:28px; }}
  .grid {{ display:grid; grid-template-columns:repeat(auto-fit, minmax(260px, 1fr)); gap:16px; }}
  .card {{ background:var(--card); border-radius:12px; padding:20px; }}
  .card h2 {{ font-size:13px; text-transform:uppercase; letter-spacing:.08em; color:var(--muted); margin:0 0 12px; }}
  .big {{ font-size:32px; font-weight:700; }}
  .good {{ color:var(--good); }} .warn {{ color:var(--warn); }} .bad {{ color:var(--bad); }}
  .bar-row {{ display:flex; align-items:center; gap:8px; margin:6px 0; font-size:13px; }}
  .bar-label {{ width:130px; color:var(--muted); }}
  .bar-track {{ flex:1; height:10px; background:#242833; border-radius:5px; overflow:hidden; }}
  .bar-fill {{ height:100%; background:var(--accent); }}
  table {{ width:100%; border-collapse:collapse; font-size:13px; margin-top:8px; }}
  th, td {{ text-align:left; padding:6px 8px; border-bottom:1px solid #242833; }}
  th {{ color:var(--muted); font-weight:600; }}
  td.num {{ text-align:right; font-variant-numeric:tabular-nums; }}
  .footer {{ margin-top:28px; color:var(--muted); font-size:12px; }}
</style>
</head>
<body>
<h1>Becket Report</h1>
<div class="sub" id="subtitle">Local index quality &amp; token savings</div>
<div class="grid" id="cards"></div>
<div class="footer">Generated by <code>becket report</code> — all data local, from <code>.becket/report/metrics.json</code>.</div>
<script>
const METRICS = {safe_json};

function el(tag, cls, html) {{
  const node = document.createElement(tag);
  if (cls) node.className = cls;
  if (html !== undefined) node.innerHTML = html;
  return node;
}}

function pct(x) {{ return (x * 100).toFixed(1) + '%'; }}

const cards = document.getElementById('cards');

// Token savings
const tok = METRICS.tokens;
const tokenCard = el('div', 'card');
tokenCard.appendChild(el('h2', null, 'Token savings (bundle vs full files)'));
const savingsCls = tok.meanSavingsRatio >= 0.5 ? 'good' : tok.meanSavingsRatio >= 0.2 ? 'warn' : 'bad';
tokenCard.appendChild(el('div', 'big ' + savingsCls, tok.sampledSymbols ? pct(tok.meanSavingsRatio) : 'n/a'));
tokenCard.appendChild(el('div', null,
  `mean bundle: <b>${{tok.meanBundleTokens}}</b> tok &nbsp;·&nbsp; baseline: <b>${{tok.meanBaselineTokens}}</b> tok &nbsp;·&nbsp; ${{tok.sampledSymbols}} sampled symbols`));
if (tok.samples.length) {{
  const table = el('table');
  table.innerHTML = '<tr><th>Symbol</th><th class="num">Bundle</th><th class="num">Baseline</th><th class="num">Saved</th></tr>' +
    tok.samples.map(s =>
      `<tr><td>${{s.symbol}}<br><span style="color:var(--muted)">${{s.file}}</span></td>` +
      `<td class="num">${{s.bundleTokens}}</td><td class="num">${{s.baselineTokens}}</td>` +
      `<td class="num">${{pct(s.savingsRatio)}}</td></tr>`).join('');
  tokenCard.appendChild(table);
}}
cards.appendChild(tokenCard);

// Graph confidence
const graph = METRICS.graph;
const graphCard = el('div', 'card');
graphCard.appendChild(el('h2', null, 'Graph confidence'));
graphCard.appendChild(el('div', 'big', graph.symbols + ' <span style="font-size:15px;color:var(--muted)">symbols</span> · ' + graph.edges + ' <span style="font-size:15px;color:var(--muted)">edges</span>'));
graphCard.appendChild(el('div', null, `high-confidence edges: <b class="${{graph.highConfidenceRatio >= 0.7 ? 'good' : 'warn'}}">${{pct(graph.highConfidenceRatio)}}</b> · co-change pairs: <b>${{graph.coChangePairs}}</b>`));
const tiers = Object.entries(graph.resolutionProfile);
const maxTier = Math.max(1, ...tiers.map(([, c]) => c));
for (const [tier, count] of tiers) {{
  const row = el('div', 'bar-row');
  row.appendChild(el('div', 'bar-label', tier));
  const track = el('div', 'bar-track');
  const fill = el('div', 'bar-fill');
  fill.style.width = (count / maxTier * 100) + '%';
  track.appendChild(fill);
  row.appendChild(track);
  row.appendChild(el('div', null, String(count)));
  graphCard.appendChild(row);
}}
cards.appendChild(graphCard);

// Wiki lint
const wiki = METRICS.wiki;
const wikiCard = el('div', 'card');
wikiCard.appendChild(el('h2', null, 'Wiki lint (code ↔ docs coherence)'));
if (!wiki.lintAvailable) {{
  wikiCard.appendChild(el('div', 'big warn', 'no lint report'));
  wikiCard.appendChild(el('div', null, 'Run <code>becket build</code> or <code>becket wiki lint</code>.'));
}} else {{
  const issues = wiki.stalePages + wiki.claimErrors + wiki.brokenLinks;
  wikiCard.appendChild(el('div', 'big ' + (issues === 0 ? 'good' : 'warn'), issues === 0 ? 'clean' : issues + ' issue(s)'));
  wikiCard.appendChild(el('div', null,
    `stale: <b>${{wiki.stalePages}}</b> · claim errors: <b>${{wiki.claimErrors}}</b> · broken links: <b>${{wiki.brokenLinks}}</b> · orphans: <b>${{wiki.orphanPages}}</b>`));
}}
cards.appendChild(wikiCard);

// Bundle integrity
const integ = METRICS.integrity;
const integCard = el('div', 'card');
integCard.appendChild(el('h2', null, 'Bundle integrity'));
const fresh = integ.snippetsChecked - integ.staleSnippets;
integCard.appendChild(el('div', 'big ' + (integ.staleSnippets === 0 ? 'good' : 'bad'),
  integ.snippetsChecked ? `${{fresh}}/${{integ.snippetsChecked}} fresh` : 'n/a'));
integCard.appendChild(el('div', null, integ.staleSnippets === 0
  ? 'All sampled snippets match the source on disk.'
  : `<b class="bad">${{integ.staleSnippets}}</b> stale snippet(s) — re-run <code>becket build</code>.`));
cards.appendChild(integCard);

// Last build
if (METRICS.lastBuild) {{
  const b = METRICS.lastBuild;
  const buildCard = el('div', 'card');
  buildCard.appendChild(el('h2', null, 'Last build'));
  buildCard.appendChild(el('div', null,
    `files: <b>${{b.files_parsed}}</b> parsed / <b>${{b.files_skipped}}</b> cached · ` +
    `edges: <b>${{b.edges_indexed}}</b> · unresolved calls: <b>${{b.unresolved_calls ?? 0}}</b> · ` +
    `flows: <b>${{b.flows_indexed}}</b> · wiki pages: <b>${{b.wiki_pages_indexed}}</b>`));
  cards.appendChild(buildCard);
}}
</script>
</body>
</html>
"#
    )
}
