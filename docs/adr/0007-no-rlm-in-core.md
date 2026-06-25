# ADR-0007: Nessun RLM nel core — agente host come orchestratore

- **Stato:** Accettato
- **Data:** 2026-06-25

## Contesto

I **Recursive Language Models (RLM)** esplorano contesti grandi con sub-chiamate LLM ricorsive. Becket punta a un **IR deterministico** (grafo) + **query engine** + **bundle verificato** per agent.

L'agente host (Cursor, Claude Code) implementa già loop tool-calling ricorsivi.

## Decisione

1. **Nessun RLM embedded** in `becket-core`, `becket-query` o `becket-mcp`.
2. **Espansione ricorsiva strutturale** solo sul grafo: `impact --depth`, `flow` (BFS/DFS), `dependencies`.
3. **Context assembly** = single-shot bundle (snippet + wiki + impact) entro budget token.
4. **LLM** solo per prosa opzionale via MCP sampling (ADR-0003), mai per struttura o snippet.
5. L'**agente host** può chiamare più tool MCP in sequenza; Becket resta source of truth.

## Conseguenze

- Correttezza e CI testabile sul grafo e `wiki lint`.
- Latenza e costo query bassi post-`build`.
- Nessuna duplicazione dell'orchestrazione agente nel core.
- Esplorazione vaga su repo non indicizzato resta fuori scope (fare `build` prima).

## Riferimenti

- [ADR-0006](./0006-grounded-knowledge-wiki.md) — Knowledge Layer + Context Assembly
