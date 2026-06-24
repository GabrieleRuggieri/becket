# BACKLOG.md — RepoCtx open work

> Priorità: **P0** = blocca MVP, **P1** = importante post-MVP, **P2** = nice-to-have.

---

## P0 — MVP core

| ID | Area | Task | Note |
|---|---|---|---|
| P0-1 | Parsing | Integrare **tree-sitter** per Rust, TS/JS, Python, Go, Java | ✅ merge su `main` |
| P0-2 | Graph | **Resolver** import/call/extends → popolare tabella `edges` | ✅ call + import + extends/implements |
| P0-3 | Graph | **Entrypoint detector** (main, HTTP route heuristics) | ✅ main + HTTP (Express, Flask, Spring) |
| P0-4 | Flow | **Flow reconstructor** base (clustering call graph + nomi cartelle) | ✅ v0 auto-discovery path |
| P0-5 | MCP | Server **rmcp** con `get_context`, `get_impact`, `get_flow`, `get_dependencies` | ✅ branch `feature/mcp-server` |
| P0-6 | Schema | File **JSON Schema** in `schemas/` + validazione in CI | ✅ schemars + jsonschema |
| P0-7 | CLI | Comandi `domain rename` / `domain add` | ✅ persistenza store + override build |
| P0-8 | Incremental | Fix re-index: eliminare simboli stale quando un file cambia | ✅ `delete_symbols_for_path` |
| P0-9 | Determinism | ID stabili + artifact byte-identici tra rebuild | ✅ SHA-256 ids + test CI |

## P1 — Architettura completa v1

| ID | Area | Task |
|---|---|---|
| P1-1 | Embeddings | ONNX locale (BGE-small) + `sqlite-vec` | ✅ fastembed BGE-small + download HF cache |
| P1-2 | MCP | **Sampling** per enrichment nomi/summary (host model) | ✅ lazy + cache SQLite |
| P1-3 | Security | Secret redaction prima di sampling | ✅ v0 regex in `redact.rs` |
| P1-4 | Workspace | Multi-repo manifest + cross-repo linker | ✅ `repoctx.workspace.toml` + `workspace build` |
| P1-5 | Distribuzione | `cargo-dist`, Homebrew tap, npm wrapper | ✅ dist + release CI + packaging docs |
| P1-6 | Bench | Fixture small→huge + budget CI (200ms incremental, 100ms query p95) | ✅ `bench-small` + test CI |
| P1-7 | Watch | `repoctx build --watch` | ✅ debounce 400ms, ignora `.repoctx`/`.git` |

## P2 — Ecosistema

| ID | Area | Task |
|---|---|---|
| P2-1 | Plugins | Registry grammatiche tree-sitter per nuove lingue | ✅ `GrammarRegistry` + `repoctx.languages.toml` |
| P2-2 | Docs | `CONTRIBUTING.md`, guida language plugin | ✅ CONTRIBUTING.md |
| P2-3 | ADR | `docs/adr/` per decisioni future | ✅ ADR 0001–0005 |
| P2-4 | Windows | Tier-2 CI e triage |

---

## Prossimo consigliato

1. **P2-4** — Windows tier-2 CI e triage
2. **P1-4+** — linker gRPC/queue e client HTTP più ricchi
3. **Prima release** — creare tap `homebrew-repoctx` e tag `v0.1.0`

---

## Blocchi / domande aperte

- ~~Nome org GitHub~~ → [GabrieleRuggieri/repo-ctx](https://github.com/GabrieleRuggieri/repo-ctx)
- Conferma priorità lingue oltre al core set (Rust, TS/JS, Python, Go, Java)
