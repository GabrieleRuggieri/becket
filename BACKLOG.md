# BACKLOG.md — RepoCtx open work

> Priorità: **P0** = blocca MVP, **P1** = importante post-MVP, **P2** = nice-to-have.
>
> **North star:** un agente chiama `repoctx context` e riceve **un bundle unico** (markdown) con codice verificato + wiki + impact — vedi [ADR-0006](./docs/adr/0006-grounded-knowledge-wiki.md).

---

## P0 — MVP core

| ID | Area | Task | Note |
|---|---|---|---|
| P0-1 | Parsing | Integrare **tree-sitter** per Rust, TS/JS, Python, Go, Java | ✅ |
| P0-2 | Graph | **Resolver** import/call/extends → popolare tabella `edges` | ✅ |
| P0-3 | Graph | **Entrypoint detector** (main, HTTP route heuristics) | ✅ |
| P0-4 | Flow | **Flow reconstructor** base | ✅ |
| P0-5 | MCP | Server **rmcp** con `get_context`, `get_impact`, `get_flow`, `get_dependencies` | ✅ |
| P0-6 | Schema | File **JSON Schema** in `schemas/` + validazione in CI | ✅ |
| P0-7 | CLI | Comandi `domain rename` / `domain add` | ✅ |
| P0-8 | Incremental | Fix re-index simboli stale | ✅ |
| P0-9 | Determinism | ID stabili + artifact byte-identici | ✅ |

## P1 — Ship & adozione (v0.1)

| ID | Area | Task | Note |
|---|---|---|---|
| P1-5 | Distribuzione | `cargo-dist`, release CI, npm/homebrew | ✅ infra pronta |
| P1-R1 | Release | Tag **v0.1.0** + tap Homebrew + smoke test install | Blocca “uso reale” |
| P1-R2 | Docs | Guida **“Use today”** in README: build → impact → MCP in Cursor | ✅ |
| P1-R3 | MCP | Esempio config Cursor / Claude Code (`repoctx-mcp`, `REPOCTX_ROOT`) | ✅ |
| P1-R4 | DX | `repoctx context` migliora output **oggi**: symbol + file:line + related + `--json` stabile | ✅ |

## P1 — Knowledge Layer v0.2 (bundle unico, non LLM Wiki clone)

> [ADR-0006](./docs/adr/0006-grounded-knowledge-wiki.md) · [ARCHITECTURE.md](./ARCHITECTURE.md) §3.5–§3.6

| ID | Area | Task | Note |
|---|---|---|---|
| P1-8 | Wiki | Modello pagina + frontmatter + `wiki/index.md` auto-generato da tassonomia grafo | ✅ |
| P1-8b | Wiki | **Template a slot** — sezioni grafo compilate, slot prosa per LLM | ✅ |
| P1-8c | Wiki | **Claim blocks** `<!-- repoctx:claim ... -->` + parser lint | ✅ |
| P1-9 | Wiki | Graph-grounded ingest via MCP sampling (sottografo nel prompt) | ✅ `get_wiki enrich` |
| P1-9b | Wiki | **Coda sync su `build --watch`** quando fingerprint cambia | ✅ avviso stale post-build |
| P1-10 | Wiki | Wiki lint deterministico (stale, claims, link, orphan) → `wiki_lint.json` | ✅ `wiki lint --strict` |
| P1-11 | Context | **Context Assembly** + packing greedy a `--budget` | ✅ |
| P1-11b | Context | **`--format md`** default + `--task fix\|refactor\|onboard` | ✅ CLI + MCP |
| P1-12 | MCP/CLI | `wiki sync\|lint\|show`, `get_wiki`, `get_context` → bundle completo | ✅ |
| P1-13 | Schema | `ContextBundle`, `WikiPage`, validazione artifact | ✅ `wiki_lint` / `wiki_stale` |
| P1-14 | Bench | Budget test: bundle quality su fixture `flows-payment` + `bench-small` | ✅ test integrazione |
| P1-15 | Release | Tag **v0.2.0** con Knowledge Layer | Da taggare |

## P1 — Architettura v1 (completato)

| ID | Area | Task | Note |
|---|---|---|---|
| P1-1 | Embeddings | ONNX BGE-small + `sqlite-vec` | ✅ |
| P1-2 | MCP | Sampling enrichment | ✅ |
| P1-3 | Security | Secret redaction | ✅ |
| P1-4 | Workspace | Cross-repo linker + gRPC/queue | ✅ |
| P1-6 | Bench | Fixture + budget CI latenza | ✅ |
| P1-7 | Watch | `repoctx build --watch` | ✅ base per P1-9b |

## P2 — Ecosistema

| ID | Area | Task | Note |
|---|---|---|---|
| P2-1 | Plugins | Grammar registry | ✅ |
| P2-2 | Docs | CONTRIBUTING.md | ✅ |
| P2-3 | ADR | ADR 0001–0007 | ✅ |
| P2-4 | Windows | Tier-2 CI | ✅ |
| P2-5 | Wiki | Router ibrido BM25+vec se wiki > ~200 pagine | Dopo P1-8 |
| P2-6 | Wiki | Semantic lint opzionale (prosa ambigua) via sampling | Dopo P1-10 |

---

## Prossimo consigliato (ordine adozione)

1. **P1-15** — tag v0.2.0 + release notes Knowledge Layer
2. **P1-R1 + P1-R2 + P1-R3** — v0.1 adozione (se non già fatto)
3. **P2-5 / P2-6** — wiki router ibrido e semantic lint (scala)

---

## Blocchi / domande aperte

- ~~Wiki vs solo grafo~~ → bundle unico (ADR-0006)
- ~~LLM Wiki clone~~ → template + claim + lint grafo (ADR-0006)
- Conferma priorità lingue oltre al core set
