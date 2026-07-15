# Changelog

All notable changes to Becket are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- **Import-aware call resolution** — full per-language import extraction (Rust `use` lists/aliases, Python `from … import … as`, JS/TS named/default/namespace imports, Go import blocks, Java imports) with module-path → file resolution; calls resolve through the importing file's import table before falling back to name matching
- **Edge confidence tiers** — every edge carries an `EdgeResolution` (`import_resolved`, `file_scoped`, `dir_scoped`, `global_unique`, `candidate`) and a derived `confidence`; ambiguous calls now emit capped low-confidence *candidate* edges instead of being silently dropped (schema `1.1.0`, additive)
- **Multi-signal context ranking** — continuous relevance score per candidate: confidence-weighted graph proximity, semantic similarity, git co-change coupling, and structural centrality, with task-specific weights (`fix` / `refactor` / `onboard`)
- **Co-change mining** — `becket build` mines `git log` for files that historically change together (fails soft without git)
- **`becket report`** — measurable index-quality metrics (`.becket/report/metrics.json`) plus a self-contained local HTML dashboard: token savings vs full files, graph confidence profile, wiki lint status, and bundle integrity check
- **`becket bench`** — reproducible retrieval benchmark against recent git history (recall of co-committed files + token cost), so anyone can validate Becket on their own repo
- **Upstream dependencies** — `becket-query` now populates `upstream` (callers/dependents) in dependency results
- Uncertain call-graph neighbors are marked with `?` in context bundles

### Changed

- **True incremental builds** — unchanged files are no longer re-read or re-parsed: raw parse payloads are cached in the index (`raw_refs`) and reused
- **Transactional builds** — all index writes happen in one SQLite transaction; a failed build leaves the previous index intact
- **Cycle-safe traversal** — downstream/upstream impact uses BFS with a visited set instead of a recursive CTE (no row explosion on cyclic graphs)
- **Embedding space guard** — the index records which embedder produced its vectors; queries skip semantic search on mismatch instead of comparing incompatible spaces
- Deleted files are pruned from the index during incremental builds
- **Documentation** aligned across `README.md`, `ARCHITECTURE.md`, `CODEMAP.md`, `BACKLOG.md`, `PROGRESS.md`, `demo/README.md`, `website/docs.html`, and `CONTRIBUTING.md`

### Removed

- Dead `HeuristicExtractor` module (tree-sitter parser has been the only extractor since v0.2)

## [0.2.3] - 2026-06-29

### Added

- **`becket context --plan`** — fast token budget estimate without reading source files
- **`becket context --auto-budget`** — recommended budget from graph size and task
- **Task default budgets** — `fix` 6k, `refactor` 12k, `onboard` 8k tokens when `--budget` is omitted
- **Fix mode test awareness** — test snippets ranked first; `## Related tests` in markdown bundles
- **`WikiPageIndex`** and assemble-time file cache for faster context builds
- **MCP auto-enrich** — `get_context` enriches placeholder knowledge pages via host sampling
- **Onboard task** — attaches the relevant flow wiki page to context bundles

### Changed

- Context bundles use **`## Knowledge`** (wiki pages) instead of a separate enriched summary section
- Smart **budget advice** notice in markdown when packing hits token limits
- Website docs (EN/IT): `--plan`, `--auto-budget`, task budgets, knowledge layer, and agent setup
- Website: broader agent messaging, demo folder, dark-only UI, clarified `npx becket` vs `becket-mcp`

### Fixed

- CI: latency budget guards stabilized on shared runners; skipped on Windows tier-2
- CI: schema drift test independent of Windows line endings
- Website: i18n hint translation and language toggle label

## [0.2.2] - 2026-06-26

### Changed

- Distribution: npm only for package-manager publish (Homebrew tap removed); shell installer and GitHub Releases unchanged
- Docs: removed `cargo install` / crates.io as an install channel; contributors build from a clone
- Website: homepage hero shows `npx becket build` as the install command

## [0.2.1] - 2025-06-25

### Fixed

- Release CI: distributed binaries build without ONNX by default (hash embeddings), fixing cross-platform `cargo-dist` failures
- `hash_embed_forced()` honors `BECKET_HASH_EMBED` when ONNX feature is disabled

### Changed

- Website deploy config: Vercel (`vercel.json`) replaces Netlify
- Local dev: `scripts/becket-local.sh` runs becket from source without install

## [0.2.0] - 2025-06-25

### Added

- **Knowledge Layer** — graph-grounded repo wiki (`.becket/wiki/`) with symbol-anchored pages
- **`becket wiki sync|lint|show`** — recompile stale pages, CI lint, view grounded pages
- **`becket context`** — markdown context bundle (wiki + code snippets + impact) with `--budget` and `--task`
- MCP **`get_context`** and **`get_wiki`** (`enrich=true` via host model sampling)
- Wiki compiler with `graph_fingerprint`, claim blocks, and stale queue
- `becket build --watch` auto-syncs stale wiki structure after incremental rebuilds
- Cross-repo **`becket workspace build`** with HTTP/gRPC/queue linking
- JSON Schema validation in CI; latency budget guards (rebuild ≤200ms, query p95 ≤100ms)
- Distribution via cargo-dist: GitHub Releases, Homebrew tap, npm wrappers

### Changed

- North star: one call → one token-budgeted markdown bundle for agents
- MCP wiki authoring uses sampling only (no bundled LLM) per ADR-0003

## [0.1.0] - 2025-05-01

### Added

- Deterministic code graph: symbols, call edges, flows, entrypoints, impact
- **`becket build`**, **`becket impact`**, **`becket flow`**
- MCP server: `get_impact`, `get_flow`, `get_dependencies`
- Local SQLite index and versioned JSON artifacts under `.becket/`
- tree-sitter parsing for Rust, TypeScript/JavaScript, Python, Go, Java

[0.2.3]: https://github.com/GabrieleRuggieri/becket/releases/tag/v0.2.3
[0.2.2]: https://github.com/GabrieleRuggieri/becket/releases/tag/v0.2.2
[0.2.1]: https://github.com/GabrieleRuggieri/becket/releases/tag/v0.2.1
[0.2.0]: https://github.com/GabrieleRuggieri/becket/releases/tag/v0.2.0
[0.1.0]: https://github.com/GabrieleRuggieri/becket/releases/tag/v0.1.0
