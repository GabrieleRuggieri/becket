# ADR-0006: Knowledge Layer (Repo Wiki) + Context Assembly

- **Stato:** Accettato
- **Data:** 2026-06-25
- **Revisione:** 2026-06-25 — differenziazione vs LLM Wiki, bundle unico, adozione agenti

## Contesto

Gli agenti AI hanno bisogno di *codice reale* e *comprensione persistente* per bug, refactor e feature — non metadati sparsi né un intero repo nel prompt.

Due pattern emergenti coprono metà del problema ciascuno:

- **RAG** — recupera chunk a ogni query; nessuna memoria compounding; chunking fragile sul codice.
- **LLM Wiki** (Karpathy / Microsoft `llmwiki`) — markdown persistente, ma **non verificabile** contro il codice; ingest manuale/conversazionale; lint affidata all’LLM.

Becket ha già il **Deterministic Core** (grafo, impact, flow, entrypoint). Il rischio è costruire “LLM Wiki + JSON a lato” — copia del pattern gist con un layer in più, difficile da adottare e non migliore di un buon `CONTEXT.md` su repo piccoli.

## Obiettivo prodotto (north star)

> **Un agente chiama un comando, riceve un unico context bundle pronto all’uso** (markdown o JSON), con codice verificato + significato + impact — entro un budget token. Zero orchestrazione manuale di più tool.

Becket non chiede all’agente di leggere `AGENTS.md`, navigare Obsidian, o assemblare pezzi. **Compila** il contesto come farebbe un buon documento `.md`, ma da grafo + wiki verificata.

## Decisione

Modello **a tre layer** (v1.1, additivo rispetto al core v1.0):

| Layer | Responsabilità | Owner |
|---|---|---|
| **Deterministic Core** | Ground truth strutturale | Becket (tree-sitter, no LLM) |
| **Knowledge Layer (Repo Wiki)** | Intent, gotcha, convenzioni | Host LLM via MCP sampling, **compilata dal grafo** |
| **Context Assembly** | Deliverable per l’agente | Becket — **un bundle per task** |

### Perché non è “LLM Wiki + grafo”

| LLM Wiki (pattern generico) | Becket (Repo Wiki) |
|---|---|
| Ingest di *documenti* curati dall’umano | Ingest di *sottografi* (simboli, edge, flow) |
| Schema in `AGENTS.md` co-evoluto | Tassonomia **derivata** da directory, entrypoint, `flows.json` |
| LLM scrive pagine intere | **Template a slot**: sezioni struttura dal grafo, LLM solo su slot prosa |
| Lint = LLM rilegge tutto | **Lint deterministico** vs grafo + claim machine-readable |
| Query → risposta in chat | Query → **`context.md`** o JSON bundle |
| Manutenzione su richiesta | **`build --watch`** → fingerprint stale → re-sync mirato |

### Scelte di design

1. **Graph-grounded compilation** — frontmatter: `symbol_ids`, `graph_fingerprint`, `stale`. L’LLM non inventa relazioni; le riceve come fatti nel prompt.

2. **Template a slot per `kind`** — es. `service`: sezioni Routes / Callers / Impact compilate dal grafo; slot `## Intent & gotchas` solo LLM.

3. **Claim machine-readable** — per lint senza capire la prosa:
   ```markdown
   <!-- becket:claim calls sym_billing_client source=graph -->
   ```

4. **Staleness incrementale** — stesso content-hash dell’incremental build; solo pagine ancorate a simboli toccati vanno in coda sync.

5. **Sync event-driven** — dopo `build --watch`, pagine stale in coda; opzionale `wiki sync` eager. Nessun “processa questo PDF”.

6. **Context Assembly** — ranking: call-graph neighborhood + impact + embedding; packing greedy a budget. Codice **sliced da disco**, mai generato.

7. **Task modes** — bundle diverso per obiettivo:
   - `fix` — snippet focali, callers, test, impact depth 2
   - `refactor` — impact profondo, cross-module
   - `onboard` — flow + overview, meno snippet

8. **Output primario markdown** — `becket context X --format md` (default per agenti). `--json` per tooling. Un file, non quattro API da incollare.

9. **Authoring lazy via MCP sampling** (ADR-0003). `becket build` non richiede mai un modello.

10. **Storage** — `.becket/wiki/` regenerabile; commit opzionale in git per team.

### Fuori scope

- Tool **wiki-only** senza grafo verificante
- Wiki pensata per browsing umano in Obsidian come prodotto principale
- Più tool MCP da orchestrare manualmente invece di un bundle unificato

## Conseguenze

- `get_context` / `becket context` → bundle unico (markdown default)
- `becket wiki sync|lint|show` + MCP `get_wiki`
- Core v1.0 invariato; layer wiki/assembly strictly additive
- Implementazione: BACKLOG P1-8 … P1-15; release **v0.2.0**

## Criteri di successo (adozione)

- Un agente risolve un bug con **una chiamata** a `get_context` senza `@` manuali su 10+ file
- Wiki stale segnalata entro un `build --watch`, non scoperta a refactor fallito
- Su repo medio, bundle < budget con snippet + impact corretti (test fixture + integrazione)
- Developer usa `becket impact` prima del commit (workflow documentato)
