# ADR-0003: LLM enrichment solo via MCP sampling

- **Stato:** Accettato
- **Data:** 2026-06-23

## Contesto

Nomi di dominio e summary leggibili richiedono testo generato da LLM, ma RepoCtx deve restare local-first senza API key, Ollama o provider remoti bundled.

## Decisione

**Nessun LLM bundled.** L'enrichment testuale avviene **esclusivamente via MCP sampling** (modello dell'host: Cursor, Claude Code, ecc.), con cache SQLite lazy al primo accesso. `repoctx build` non richiede mai un modello.

## Conseguenze

- Senza host MCP con sampling, l'output resta deterministico (nomi da path/simboli).
- Redaction regex (`redact.rs`) prima di inviare prompt al host.
- Embeddings ONNX restano separati (indice locale, non chat).
