# PROGRESS.md — RepoCtx development log

> Cronologia di sviluppo (work in progress e milestone completate).
> Complementare a [BACKLOG.md](./BACKLOG.md) (cosa manca).

---

## 2026-06-23 — Sessione 1: scaffolding v0.1.0

### Completato

- **Workspace Rust** multi-crate allineato ad [ARCHITECTURE.md](./ARCHITECTURE.md):
  - `repoctx-schema` — tipi artifact versionati (`schemaVersion` 1.0.0)
  - `repoctx-store` — SQLite (`index.db`) + writer JSON `.repoctx/`
  - `repoctx-core` — file walker, hash incrementale, estrazione euristica simboli
  - `repoctx-query` — motori `impact`, `flow`, `context`, `dependencies`
  - `repoctx-cli` — CLI `build | impact | flow | context | domain`
  - `repoctx-mcp` — stub (stdio MCP da integrare con `rmcp`)
- **Pipeline `repoctx build`**: walk → hash → parse euristico → index → emit 5 JSON
- **CI GitHub Actions**: fmt, clippy, test, build release (Ubuntu + macOS)
- **LICENSE** Apache-2.0
- **`.repoctxignore`** di esempio

### Verificato

- `cargo build` e `cargo test` (4 unit test) passano
- `repoctx build` sul repo stesso genera `.repoctx/*.json`

### Note

- L'estrazione simboli è **euristica per linea** — tree-sitter è il prossimo step P0
- Grafo dipendenze / flow / entrypoint non ancora popolati (tabelle pronte)
- Embeddings ONNX e MCP sampling non implementati

---

## 2026-06-23 — Sessione 2: tree-sitter + grafo call (branch `feature/tree-sitter-parser`)

### Completato

- **tree-sitter** integrato per Rust, Python, JS, TS, Go, Java
- **`GraphResolver`**: risoluzione call edges (`func_a → func_b → func_c`)
- **Entrypoint detector** v0: simboli `main` → `entrypoints.json`
- **Fix incrementale**: purge simboli per file prima del re-index
- **Fixture sintetiche** in `tests/fixtures/` + 3 integration test
- `repoctx impact` funziona su catene di chiamate reali

### Verificato

- 8 unit test + 3 integration test passano
- `cargo clippy` e `cargo fmt` puliti

### Note

- Import/extends e flow reconstructor ancora da fare (P0-4)
- MCP server ancora stub (branch `feature/mcp-server` da aprire)

---

## 2026-06-23 — Sessione 3: MCP server + flow reconstructor (branch `feature/mcp-server`)

### Completato

- **MCP server** (`repoctx-mcp`) con [rmcp](https://docs.rs/rmcp) su stdio:
  - `get_context`, `get_impact`, `get_flow`, `get_dependencies`
  - Repo root via `REPOCTX_ROOT` o cwd
- **`FlowReconstructor`**: auto-discovery domini da path + BFS sul call graph
- Fixture `tests/fixtures/flows-payment/` + integration test
- Store: `insert_flow`, `clear_flows`, `load_call_edges`

### Verificato

- 13 test totali passano (`cargo test --all`)
- `cargo clippy` pulito

### Note

- MCP sampling (P1-2) non ancora implementato
- JSON Schema artifacts (P0-6) prossimo step consigliato

---

## 2026-06-23 — Sessione 4: determinismo e algoritmi (branch `feature/determinism-and-algorithms`)

### Completato

- **ID deterministici** (`ids.rs`): SHA-256 su chiavi canoniche per simboli, edge, flow, file, entrypoint
- **`GraphResolver` v2**: indice multi-livello (file → directory → globale) con disambiguazione `public`
- **Import edges** v0: parsing `use`/`import` tree-sitter + risoluzione nel grafo
- **`FlowReconstructor` v2**: filtro domini (≥2 simboli, skip cartelle generiche), BFS ordinato, ID stabili
- **Rimossi UUID** da pipeline build/store (artifact byte-identici tra rebuild)
- **Test determinismo**: `rebuild_produces_byte_identical_artifacts`

### Verificato

- 18 test totali passano (`cargo test --all`)
- `cargo clippy` pulito

### Note

- Resolver globale ancora conservativo (ambiguità → nessun edge)
- Flow reconstructor usa solo edge `calls`, non `imports`
- `HeuristicExtractor` deprecato ma ancora presente in `extract.rs`
