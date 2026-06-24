# CODEMAP.md — RepoCtx execution map

> Come scorre l'esecuzione end-to-end. Aggiornare quando si aggiungono route, job o crate.

---

## Binari

| Binario | Crate | Ruolo |
|---|---|---|
| `repoctx` | `repoctx-cli` | CLI developer-facing |
| `repoctx-mcp` | `repoctx-mcp` | MCP stdio via rmcp (`get_context`, `get_impact`, `get_flow`, `get_dependencies`) |

---

## `repoctx build`

```
repoctx-cli::main
  └─ commands::execute(Build | Workspace)
       └─ repoctx-core::BuildPipeline::run  (single repo)
       └─ repoctx-core::WorkspacePipeline::run  (multi-repo)
            ├─ BuildPipeline per ogni membro
            ├─ CrossRepoLinker (HTTP client ↔ server route)
            └─ ArtifactWriter → .repoctx/cross_repo.json
```

### `repoctx workspace build`

```
repoctx-cli::workspace build
  └─ WorkspacePipeline::run
       ├─ parse repoctx.workspace.toml
       ├─ BuildPipeline × N repos
       ├─ CrossRepoLinker::link
       └─ .repoctx/cross_repo.json
```

---

## `repoctx build` (dettaglio singolo repo)

```
repoctx-cli::main
  └─ commands::execute(Build)
       └─ repoctx-core::BuildPipeline::run
            ├─ FileWalker::discover          # ignore + .repoctxignore
            ├─ TreeSitterParser::parse_file  # tree-sitter (Rust, Py, TS, JS, Go, Java)
            ├─ IndexStore::delete_symbols_for_path  # incremental purge
            ├─ IndexStore::insert_symbol
            ├─ GraphResolver::resolve_calls  # call edges → DB
            ├─ FlowReconstructor::reconstruct # flows.json
            ├─ index_entrypoints (main)      # entrypoints.json
            ├─ IndexStore::export_artifacts
            └─ ArtifactWriter::write_artifact × 5
                 → .repoctx/symbols.json
                 → .repoctx/dependencies.json
                 → .repoctx/flows.json
                 → .repoctx/entrypoints.json
                 → .repoctx/architecture.json
                 + .repoctx/index.db (cache)
```

---

## Query commands (`impact` | `flow` | `context`)

```
repoctx-cli::commands::execute
  └─ repoctx-query::QueryEngine
       ├─ IndexStore::open (.repoctx/index.db)
       ├─ find_symbols_by_name / find_flow_by_name
       └─ downstream_symbols (recursive CTE)
```

---

## MCP (`repoctx-mcp`)

```
repoctx-mcp::main (tokio)
  └─ server::serve(stdio)
       └─ RepoCtxMcpServer (rmcp tool_router)
            ├─ get_context  → QueryEngine::context
            ├─ get_impact   → QueryEngine::impact
            ├─ get_flow     → QueryEngine::flow
            └─ get_dependencies → QueryEngine::dependencies
```

Env: `REPOCTX_ROOT` = percorso repo (default: cwd). Richiede `repoctx build` eseguito prima.

---

## Dipendenze tra crate

```mermaid
flowchart BT
    CLI[repoctx-cli] --> CORE[repoctx-core]
    CLI --> QUERY[repoctx-query]
    MCP[repoctx-mcp] --> QUERY
    CORE --> STORE[repoctx-store]
    QUERY --> STORE
    STORE --> SCHEMA[repoctx-schema]
    CORE --> SCHEMA
    QUERY --> SCHEMA
```

---

## File system output

```
<repo-root>/
  .repoctx/
    index.db          # SQLite cache (rebuildable)
    architecture.json
    symbols.json
    dependencies.json
    flows.json
    entrypoints.json

<workspace-root>/
  repoctx.workspace.toml
  .repoctx/
    cross_repo.json     # workspace-level cross-repo edges
```
