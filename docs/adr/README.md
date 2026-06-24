# Architecture Decision Records (ADR)

Decisioni architetturali significative per RepoCtx. Ogni ADR è immutabile una volta accettato; nuove decisioni che lo superano ricevono un nuovo numero.

| ADR | Titolo | Stato |
|-----|--------|-------|
| [0001](./0001-rust-first-stack.md) | Rust end-to-end per CLI e MCP | Accettato |
| [0002](./0002-local-first-sqlite.md) | Storage local-first con SQLite + sqlite-vec | Accettato |
| [0003](./0003-mcp-sampling-only-llm.md) | LLM enrichment solo via MCP sampling | Accettato |
| [0004](./0004-cargo-dist-distribution.md) | Distribuzione con cargo-dist | Accettato |
| [0005](./0005-deterministic-artifact-ids.md) | ID deterministici e artifact byte-identici | Accettato |

## Template per nuovi ADR

```markdown
# ADR-NNNN: Titolo

- **Stato:** Proposto | Accettato | Deprecato | Sostituito da ADR-XXXX
- **Data:** YYYY-MM-DD

## Contesto

## Decisione

## Conseguenze
```
