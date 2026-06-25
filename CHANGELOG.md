# Changelog

All notable changes to Becket are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

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

[0.2.2]: https://github.com/GabrieleRuggieri/becket/releases/tag/v0.2.2
[0.2.1]: https://github.com/GabrieleRuggieri/becket/releases/tag/v0.2.1
[0.2.0]: https://github.com/GabrieleRuggieri/becket/releases/tag/v0.2.0
[0.1.0]: https://github.com/GabrieleRuggieri/becket/releases/tag/v0.1.0
