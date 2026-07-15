# CODEMAP.md — Becket execution map

> Come scorre l'esecuzione end-to-end. Aggiornare quando si aggiungono route, job o crate.

---

## Binari

| Binario | Crate | Ruolo |
|---|---|---|
| `becket` | `becket-cli` | CLI developer-facing |
| `becket-mcp` | `becket-mcp` | MCP stdio (`get_context`, `get_wiki`, `get_impact`, `get_flow`, `get_dependencies`) |

---

## `becket build`

```
becket-cli::main
  └─ commands::execute(Build | Workspace | Wiki)
       └─ becket-core::BuildPipeline::run  (single repo)
       └─ becket-core::WorkspacePipeline::run  (multi-repo)
            ├─ BuildPipeline per ogni membro
            ├─ CrossRepoLinker (HTTP / gRPC / queue)
            └─ ArtifactWriter → .becket/cross_repo.json
```

### `becket build` (singolo repo)

```
becket-cli::main
  └─ commands::execute(Build)
       └─ becket-core::BuildPipeline::run  (una transazione SQLite)
            ├─ FileWalker::discover
            ├─ IndexStore::prune_missing_files  (incremental)
            ├─ per file: skip check → raw_refs cache | TreeSitterParser::parse_file
            ├─ GraphResolver::resolve_all  (import-aware, confidence tiers)
            ├─ FlowReconstructor::reconstruct
            ├─ index_entrypoints
            ├─ index_symbol_embeddings (optional; embedder id in meta)
            ├─ history::mine_co_change  (git log, fail-soft)
            ├─ IndexStore::export_artifacts
            ├─ ArtifactWriter::write_artifact × 5
            ├─ WikiCompiler::compile_all
            └─ WikiLinter::run
                 → .becket/*.json + wiki/*.md + wiki_lint.json + wiki_stale.json
                 + .becket/index.db
```

---

## Query commands

```
becket-cli::commands::execute
  └─ becket-query::QueryEngine
       ├─ impact / flow / dependencies
       └─ context → assemble_context
```

### `becket context` (Context Assembly)

```
becket-query::assemble_context(symbol, budget, task)
  ├─ direct_call_neighbors (SQL point query, con confidence)
  ├─ downstream_symbols (BFS anti-ciclo)
  ├─ semantic_neighbor_hits (sqlite-vec; skip se embedding space ≠ build)
  ├─ rank multi-segnale: proximity pesata per confidence + semantic
  │   + co-change (git) + centralità, pesi per task
  ├─ slice source snippets from disk (greedy pack to budget)
  ├─ find_page_for_symbol → sanitize_for_context (wiki)
  └─ render markdown bundle (vicini incerti marcati `?`)
```

### `becket report` / `becket bench`

```
becket-cli::commands::execute
  ├─ Report → becket-query::generate_report
  │     → .becket/report/metrics.json + index.html (dashboard locale)
  │       token savings vs full files · profilo confidence archi
  │       stato wiki lint · integrità snippet
  └─ Bench  → becket-query::run_bench --last N
        → .becket/report/bench.json
          recall dei file co-committati + costo token per commit recente
```

### `becket wiki`

```
becket-cli::wiki {sync|lint|show}
  ├─ sync → WikiCompiler::sync_pages (selective or --all; preserves enriched prose)
  ├─ lint → WikiLinter::run → wiki_lint.json + wiki_stale.json
  └─ show  → WikiStore::load_page / resolve by title or stem
```

---

## MCP (`becket-mcp`)

```
becket-mcp::main (tokio)
  └─ server::serve(stdio)
       └─ BecketMcpServer (rmcp tool_router)
            ├─ get_context  → QueryEngine::context (+ optional sampling enrichment)
            ├─ get_wiki     → WikiStore + optional enrich_wiki_prose (persists .md)
            ├─ get_impact   → QueryEngine::impact
            ├─ get_flow     → QueryEngine::flow (+ optional sampling)
            └─ get_dependencies → QueryEngine::dependencies
```

Env: `BECKET_ROOT` (default: cwd). Richiede `becket build` prima.

---

## Dipendenze tra crate

```mermaid
flowchart BT
    CLI[becket-cli] --> CORE[becket-core]
    CLI --> QUERY[becket-query]
    MCP[becket-mcp] --> CORE
    MCP --> QUERY
    QUERY --> CORE
    CORE --> STORE[becket-store]
    QUERY --> STORE
    QUERY --> EMBED[becket-embed]
    CORE --> EMBED
    STORE --> SCHEMA[becket-schema]
    CORE --> SCHEMA
    QUERY --> SCHEMA
```

---

## File system output

```
<repo-root>/
  .becket/
    index.db
    architecture.json
    symbols.json
    dependencies.json
    flows.json
    entrypoints.json
    wiki_lint.json
    wiki_stale.json
    report/
      metrics.json
      bench.json
      index.html
    wiki/
      index.md
      *.md
```
