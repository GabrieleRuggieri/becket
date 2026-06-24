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

---

## 2026-06-23 — Sessione 5: JSON Schema + validazione CI (branch `feature/json-schema-validation`)

### Completato

- **`schemas/*.schema.json`**: 5 contratti generati da `schemars` sui tipi Rust
- **`repoctx-schema::json_schema`**: `validate_artifact_json`, `parse_artifact`, `root_schema_for`
- **Test contratto**: `committed_schemas_match_generated` previene drift schema/codice
- **Integration test**: `build_outputs_validate_against_json_schema` su tutte le fixture
- **CI**: step dedicato `cargo test -p repoctx-schema --test schema_validation`

### Verificato

- 23 test totali passano (`cargo test --all`)
- `cargo clippy` pulito

### Note

- Rigenerare schemi: `cargo test -p repoctx-schema write_schemas -- --ignored --nocapture`
- Prossimo step consigliato: P0-7 `domain rename` / `domain add`

---

## 2026-06-23 — Sessione 6: hook pre-commit rustfmt (branch `chore/git-pre-commit-hook`)

### Completato

- **`.githooks/pre-commit`**: `cargo fmt --all` su file `.rs` in stage, re-stage automatico
- **`scripts/setup-git-hooks.sh`**: `git config core.hooksPath .githooks`
- README: sezione setup hook

---

## 2026-06-23 — Sessione 7: domain CLI (branch `feature/domain-cli`)

### Completato

- **`repoctx domain rename`**: rinomina flow per id/nome, persiste in `domains` + `flows`
- **`repoctx domain add`**: allega path (`src/foo/**`) o simboli, ricostruisce il flow
- **Tabella `domain_members`** + override al rebuild (`apply_domain_overrides`)
- **`clear_all`** non cancella più i domini utente
- Integration test: rename sopravvive a rebuild, domain add con path/symboli
- Fixture test isolate in tempdir (no race su `.repoctx/`)

### Verificato

- 27 test totali passano
- `cargo clippy` pulito

---

## 2026-06-23 — Sessione 8: edge extends/implements (branch `feature/extends-implements-edges`)

### Completato

- **Parsing tree-sitter** per `extends` / `implements`:
  - Rust: `impl Trait for Type`
  - TypeScript: `class X extends Y`
  - Java: `extends` + `implements`
- **`GraphResolver`**: risoluzione edge `extends` e `implements`
- Fixture `tests/fixtures/inheritance/` (Rust + TS + Java)
- Unit test parser + integration test build

### Verificato

- 30 test totali passano (`cargo test --all`)
- `cargo clippy` pulito

---

## 2026-06-23 — Sessione 9: HTTP entrypoints (branch `feature/http-entrypoints`)

### Completato

- **`parse/http_routes.rs`**: euristiche per route HTTP
  - Express/Fastify/Axum: `app.get('/path', handler)`, `.route(...)`
  - Flask/FastAPI: `@app.get("/path")` su `decorated_definition`
  - Spring: `@GetMapping`, `@PostMapping`, ecc. su `method_declaration`
- **Build pipeline**: `index_entrypoints` risolve handler → simbolo, emette `EntrypointKind::Http`
- Fixture `tests/fixtures/http-routes/` (TS + Python + Java)
- Unit test parser + integration test build

### Verificato

- 32 test totali passano (`cargo test --all`)
- `cargo clippy` pulito

### Note

- P0 MVP core completo — prossimo step: P1-2 MCP sampling o P1-6 benchmark

---

## 2026-06-23 — Sessione 10: MCP sampling enrichment (branch `feature/mcp-sampling-enrichment`)

### Completato

- **Tabella `enrichments`** in SQLite per cache lazy di summary LLM
- **`redact.rs`**: redazione base segreti prima del sampling (API key, Bearer, sk-*, PEM)
- **`repoctx-mcp::sampling`**: enrichment lazy via `sampling/createMessage` del host
  - `get_context` → `enriched_summary` opzionale
  - `get_flow` → `enriched_description` opzionale
  - Fallback deterministico se host senza sampling
- **`SummarySource`** in output query (`deterministic` | `mcp_sampling`)

### Verificato

- 38 test totali passano (`cargo test --all`)
- `cargo clippy` pulito

---

## 2026-06-23 — Sessione 11: benchmark budget CI (branch `feature/bench-budget-ci`)

### Completato

- **Fixture `bench-small`**: mini-monorepo Rust (~10 file, call graph realistico)
- **`bench_budget.rs`**: guardrail CI su latenza
  - rebuild incrementale dopo touch singolo file **≤ 200 ms**
  - query warm (`impact`/`context`/`flow`) **p95 ≤ 100 ms**
- **CI**: step dedicato `cargo test -p repoctx-core --test bench_budget`

### Verificato

- Budget test passano in locale
- `cargo clippy` pulito

---

## 2026-06-23 — Sessione 12: embeddings + sqlite-vec (branch `feature/embeddings-sqlite-vec`)

### Completato

- **`repoctx-embed`**: embedding deterministico 384-dim (hash) + hook `REPOCTX_ONNX_MODEL`
- **sqlite-vec**: tabella virtuale `symbol_vec`, KNN `nearest_symbol_ids`
- **Build**: `index_symbol_embeddings` quando `--no-embeddings` non è attivo
- **Query**: `semantic_neighbors` in `get_context`
- Integration test `build_with_embeddings_indexes_symbol_vectors`

### Verificato

- Test store vec + integration embeddings passano
- CI continua con `no_embeddings: true` negli altri test

---

## 2026-06-23 — Sessione 13: build watch (branch `feature/build-watch`)

### Completato

- **`repoctx build --watch`**: watcher con `notify-debouncer-mini` (400ms)
- Ignora `.repoctx`, `.git`, `target`, `node_modules`, `dist`, `build`
- Build iniziale + rebuild incrementale su ogni burst di modifiche
- Unit test filtro path

### Verificato

- `cargo test --all` e `cargo clippy` puliti

---

## 2026-06-23 — Sessione 14: ONNX BGE-small + tokenizer (branch `feature/onnx-bge-small`)

### Completato

- **`fastembed`**: modello `BAAI/bge-small-en-v1.5` via `ort` + tokenizer Hugging Face
- **Download lazy**: cache in `~/.cache/repoctx/models` (override `REPOCTX_EMBED_CACHE`)
- **Fallback hash**: `REPOCTX_HASH_EMBED=1` forza embedder deterministico (CI)
- **`REPOCTX_ONNX_MODEL`**: supporto modello custom (directory o `.onnx` + tokenizer files)
- **`preload_onnx_model()`** chiamato all'inizio della fase embeddings in build
- Feature `onnx` (default) in `repoctx-embed`

### Verificato

- 49 test passano con `REPOCTX_HASH_EMBED=1` in CI
- `cargo clippy --all-features` pulito

---

## 2026-06-23 — Sessione 15: workspace + grammar registry + CONTRIBUTING (branch `feature/workspace-plugins-docs`)

### Completato

- **P1-4 Workspace**: `repoctx.workspace.toml`, `repoctx workspace build`, `CrossRepoLinker`
- **HTTP cross-repo**: match client (`axios`/`fetch`/`requests`) ↔ server route entrypoints
- **Artifact** `cross_repo.json` + JSON Schema in `schemas/`
- **P2-1 Grammar registry**: `GrammarRegistry::builtins()`, override `repoctx.languages.toml`
- **P2-2 Docs**: `CONTRIBUTING.md` con guida plugin lingue
- Fixture `tests/fixtures/workspace/` + integration test

### Verificato

- 57 test passano (`REPOCTX_HASH_EMBED=1`)
- `cargo clippy --all-features` pulito
