# Changelog

All notable changes to Becket are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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

[0.2.0]: https://github.com/GabrieleRuggieri/becket/releases/tag/v0.2.0
[0.1.0]: https://github.com/GabrieleRuggieri/becket/releases/tag/v0.1.0
