# PROGRESS.md — Becket development log

> Cronologia di sviluppo (work in progress e milestone completate).
> Complementare a [BACKLOG.md](./BACKLOG.md) (cosa manca).

---

## 2026-06-23 — Sessione 1: scaffolding v0.1.0

### Completato

- **Workspace Rust** multi-crate allineato ad [ARCHITECTURE.md](./ARCHITECTURE.md):
  - `becket-schema` — tipi artifact versionati (`schemaVersion` 1.0.0)
  - `becket-store` — SQLite (`index.db`) + writer JSON `.becket/`
  - `becket-core` — file walker, hash incrementale, estrazione euristica simboli
  - `becket-query` — motori `impact`, `flow`, `context`, `dependencies`
  - `becket-cli` — CLI `build | impact | flow | context | domain`
  - `becket-mcp` — stub (stdio MCP da integrare con `rmcp`)
- **Pipeline `becket build`**: walk → hash → parse euristico → index → emit 5 JSON
- **CI GitHub Actions**: fmt, clippy, test, build release (Ubuntu + macOS)
- **LICENSE** Apache-2.0
- **`.becketignore`** di esempio

### Verificato

- `cargo build` e `cargo test` (4 unit test) passano
- `becket build` sul repo stesso genera `.becket/*.json`

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
- `becket impact` funziona su catene di chiamate reali

### Verificato

- 8 unit test + 3 integration test passano
- `cargo clippy` e `cargo fmt` puliti

### Note

- Import/extends e flow reconstructor ancora da fare (P0-4)
- MCP server ancora stub (branch `feature/mcp-server` da aprire)

---

## 2026-06-23 — Sessione 3: MCP server + flow reconstructor (branch `feature/mcp-server`)

### Completato

- **MCP server** (`becket-mcp`) con [rmcp](https://docs.rs/rmcp) su stdio:
  - `get_context`, `get_impact`, `get_flow`, `get_dependencies`
  - Repo root via `BECKET_ROOT` o cwd
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
- **`becket-schema::json_schema`**: `validate_artifact_json`, `parse_artifact`, `root_schema_for`
- **Test contratto**: `committed_schemas_match_generated` previene drift schema/codice
- **Integration test**: `build_outputs_validate_against_json_schema` su tutte le fixture
- **CI**: step dedicato `cargo test -p becket-schema --test schema_validation`

### Verificato

- 23 test totali passano (`cargo test --all`)
- `cargo clippy` pulito

### Note

- Rigenerare schemi: `cargo test -p becket-schema write_schemas -- --ignored --nocapture`
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

- **`becket domain rename`**: rinomina flow per id/nome, persiste in `domains` + `flows`
- **`becket domain add`**: allega path (`src/foo/**`) o simboli, ricostruisce il flow
- **Tabella `domain_members`** + override al rebuild (`apply_domain_overrides`)
- **`clear_all`** non cancella più i domini utente
- Integration test: rename sopravvive a rebuild, domain add con path/symboli
- Fixture test isolate in tempdir (no race su `.becket/`)

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
- **`becket-mcp::sampling`**: enrichment lazy via `sampling/createMessage` del host
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
- **CI**: step dedicato `cargo test -p becket-core --test bench_budget`

### Verificato

- Budget test passano in locale
- `cargo clippy` pulito

---

## 2026-06-23 — Sessione 12: embeddings + sqlite-vec (branch `feature/embeddings-sqlite-vec`)

### Completato

- **`becket-embed`**: embedding deterministico 384-dim (hash) + hook `BECKET_ONNX_MODEL`
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

- **`becket build --watch`**: watcher con `notify-debouncer-mini` (400ms)
- Ignora `.becket`, `.git`, `target`, `node_modules`, `dist`, `build`
- Build iniziale + rebuild incrementale su ogni burst di modifiche
- Unit test filtro path

### Verificato

- `cargo test --all` e `cargo clippy` puliti

---

## 2026-06-23 — Sessione 14: ONNX BGE-small + tokenizer (branch `feature/onnx-bge-small`)

### Completato

- **`fastembed`**: modello `BAAI/bge-small-en-v1.5` via `ort` + tokenizer Hugging Face
- **Download lazy**: cache in `~/.cache/becket/models` (override `BECKET_EMBED_CACHE`)
- **Fallback hash**: `BECKET_HASH_EMBED=1` forza embedder deterministico (CI)
- **`BECKET_ONNX_MODEL`**: supporto modello custom (directory o `.onnx` + tokenizer files)
- **`preload_onnx_model()`** chiamato all'inizio della fase embeddings in build
- Feature `onnx` (default) in `becket-embed`

### Verificato

- 49 test passano con `BECKET_HASH_EMBED=1` in CI
- `cargo clippy --all-features` pulito

---

## 2026-06-23 — Sessione 15: workspace + grammar registry + CONTRIBUTING (branch `feature/workspace-plugins-docs`)

### Completato

- **P1-4 Workspace**: `becket.workspace.toml`, `becket workspace build`, `CrossRepoLinker`
- **HTTP cross-repo**: match client (`axios`/`fetch`/`requests`) ↔ server route entrypoints
- **Artifact** `cross_repo.json` + JSON Schema in `schemas/`
- **P2-1 Grammar registry**: `GrammarRegistry::builtins()`, override `becket.languages.toml`
- **P2-2 Docs**: `CONTRIBUTING.md` con guida plugin lingue
- Fixture `tests/fixtures/workspace/` + integration test

### Verificato

- 57 test passano (`BECKET_HASH_EMBED=1`)
- `cargo clippy --all-features` pulito

---

## 2026-06-23 — Sessione 16: distribuzione + ADR (branch `feature/distribution-adr`)

### Completato

- **P1-5 cargo-dist**: `dist-workspace.toml`, `[profile.dist]`, `.github/workflows/release.yml`
- Installer **shell**, **npm** (`becket`, `becket-mcp`), **Homebrew** (tap `GabrieleRuggieri/homebrew-becket`)
- `packaging/README.md` con workflow release
- **P2-3 ADR**: `docs/adr/` ADR 0001–0005 + indice
- README: sezione install (brew / npm / cargo / releases)

### Verificato

- `dist plan` elenca artifact per 5 target × 2 binari
- `cargo test --all` invariato

---

## 2026-06-23 — Sessione 17: Windows CI + cross-repo linker v2 (branch `feature/p14-plus-windows-ci`)

### Completato

- **P2-4 Windows tier-2**: job `windows-tier2` in CI (`continue-on-error`), tier-1 su Ubuntu/macOS
- **`docs/windows.md`**: policy tier-2, reporting issue, sviluppo locale
- **P1-4+ Cross-repo linker**:
  - Rilevamento gRPC (`ServiceStub`, `Register*Server`, …) e queue (`send`/`subscribe`, Kafka/Rabbit)
  - Contratti manifest `[[contracts.grpc]]` e `[[contracts.queue]]`
  - HTTP client estesi: `httpx`, `http.Get` (Go), `urllib.request.urlopen`, `got`/`superagent`
- Fixture `tests/fixtures/workspace-messaging/` + integration test gRPC + queue

### Verificato

- 60 test passano (`BECKET_HASH_EMBED=1`)
- `cargo clippy --all-features` pulito

---

## 2026-06-25 — Sessione 18: visione Knowledge Layer v1.1 + docs + website

### Completato

- **ADR-0006**: modello a 3 layer (Deterministic Core → Grounded Repo Wiki → Context Assembly)
- **ARCHITECTURE.md v1.1**: §3.5 wiki, §3.6 context assembly, decisioni #17–#18
- **README.md**: visione aggiornata — codice in contesto, wiki verificata, confronto RAG vs LLM Wiki vs Becket
- **BACKLOG.md**: epic P1-8 … P1-13 (Knowledge Layer) + P2-5/P2-6
- **Website** (`website/`): index + docs + i18n EN/IT — sezione tre layer, tabella comparativa, CLI wiki/context aggiornati
- **CODEMAP.md**: roadmap esecuzione wiki + context assembly (planned)

### Decisioni architetturali (migliorative vs LLM Wiki pura)

- Wiki **ancorata al grafo** (`symbol_ids`, `graph_fingerprint`) — l'LLM scrive prosa, non struttura
- **Lint deterministico** contro il grafo live — staleness first-class
- **Context Assembly** con snippet reali sliced da disco, packing greedy a budget token
- Authoring wiki **lazy via MCP sampling** (ADR-0003 invariato)
- Wiki-only esplicitamente **fuori scope**

### Prossimo step implementativo

- P1-R1 … P1-R3 (release v0.1 + docs adozione) → P1-11 + P1-11b (context bundle markdown) → P1-8 … P1-10 (wiki)

---

## 2026-06-25 — Sessione 19: docs adozione-first (shipped vs v0.2)

### Completato

- **README**: sezioni "Use today (v0.1)" e "Coming v0.2", quick start 3 passi, config MCP Cursor, onestà su `context` metadata-only oggi
- **ADR-0006** ampliato: north star bundle unico, template slots, claim blocks, watch sync, task modes, criteri adozione
- **ARCHITECTURE**: §3.5–§3.6 aggiornati, API con `--format md` / `--task`, decisione #19 adoption-first, tabella MCP con status
- **BACKLOG**: epic P1-R (ship & adozione v0.1), priorità P1-11b markdown bundle
- **Website**: sezione "Use Today", badge shipped/v0.2, tabella comparativa onesta, terminal demo v0.1, docs adoption + MCP config
- **CONTRIBUTING**: workflow adozione per contributor

### Principio

Documentazione allineata a **cosa funziona oggi** (impact, flow, MCP) e **north star v0.2** (un markdown bundle per task) — per massimizzare uso reale, non solo visione.

---

## 2026-06-25 — Sessione 20: Context Assembly (P1-11 / P1-11b)

### Completato

- **`becket-query::assemble`**: snippet reali da disco, callers/callees, impact, packing a budget token
- **`ContextResult` esteso**: `snippets`, `markdown`, `task`, `budget_tokens`
- **CLI** `becket context --budget 6000 --task fix` → output markdown (default), `--json` per tooling
- **MCP `get_context`**: restituisce markdown bundle (non solo metadata JSON)
- Test integrazione `context_assembly_includes_code_snippets` su fixture flows-payment

### Prossimo step

- P1-15 — tag v0.2.0

---

## 2026-06-25 — Sessione 21: Knowledge Layer wiki v0.2 (P1-8 … P1-14)

### Completato

- **Schema wiki** (`WikiPage`, `WikiLintArtifact`, `WikiStaleQueue`) + JSON Schema in `schemas/`
- **`WikiCompiler`**: pagine flow/module/service da grafo, `index.md`, claim blocks + prose slot
- **`WikiLinter`**: stale fingerprint, claim errors, broken links, orphans → `wiki_lint.json` + `wiki_stale.json`
- **Build**: compila wiki + lint dopo emit artifact; `wiki_pages_indexed` in `BuildReport`
- **CLI** `wiki sync|lint|show` (+ `--strict` per CI)
- **MCP** `get_wiki` con `enrich=true` per prose slot via sampling
- **Context assembly**: sezione `## Wiki` nel bundle markdown quando simbolo ancorato
- **`build --watch`**: avviso pagine stale in coda dopo rebuild
- **Bump** workspace `0.2.0`
- Test integrazione wiki + context su fixture `flows-payment`

### Verificato

- `cargo test --all` verde

---

## 2026-06-25 — Sessione 22: wiki quality hardening (pre-release)

### Completato

- **`wiki/util`**: nomi leggibili, merge prose, `sanitize_for_context` per bundle agenti
- **Compiler**: step 1-based, `see_also` tra pagine correlate, sync selettivo con prose preservata
- **Lint**: claim `calls` strict (edge diretto con simboli ancorati)
- **Context**: wiki sanitizzata (no claim HTML, no placeholder vuoto)
- **MCP enrich**: persiste prose su `.becket/wiki/*.md`
- **Test**: prose merge, sanitize, sync, context senza claim comments
- **README**: allineato a v0.2 shipped

### Verificato

- `cargo test --all` + `cargo clippy -D warnings` verdi

---

## 2026-06-25 — Sessione 23: allineamento pre-release + ranking semantico

### Completato

- **Docs allineati**: `ARCHITECTURE.md`, `CODEMAP.md`, `CONTRIBUTING.md`, `BACKLOG.md`
- **ADR-0007**: no RLM nel core
- **Context ranking**: semantic neighbors (sqlite-vec) nel greedy pack, non solo post-hoc metadata
- **Flow steps**: order 1-based nel grafo (non solo display wiki)
