# ADR-0001: Rust end-to-end per CLI e MCP

- **Stato:** Accettato
- **Data:** 2026-06-23

## Contesto

Becket deve analizzare repository da piccoli a enormi con bassa latenza, distribuzione come binario nativo e integrazione con agenti AI via MCP stdio.

## Decisione

Implementare **tutto il core in Rust**: CLI (`clap`), parsing (`tree-sitter`), store (`rusqlite`), query engine e server MCP (`rmcp`). Nessun runtime Node/Python nel percorso critico.

## Conseguenze

- Binario singolo, nessuna dipendenza da VM esterne in produzione.
- Integrazione tree-sitter nativa e parallelism senza GC pause.
- Curve di apprendimento Rust per contributor; mitigata da `CONTRIBUTING.md` e crate ben delimitati.
