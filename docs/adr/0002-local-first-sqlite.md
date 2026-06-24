# ADR-0002: Storage local-first con SQLite + sqlite-vec

- **Stato:** Accettato
- **Data:** 2026-06-23

## Contesto

Serve un indice rebuildabile, embedded, senza servizi cloud o daemon. Serve anche ricerca semantica sui simboli.

## Decisione

Usare **SQLite** (`rusqlite`, feature `bundled`) come unico database embedded in `.repoctx/index.db`, con traversal via recursive CTEs. Aggiungere **sqlite-vec** nella stessa file per embedding KNN.

## Conseguenze

- Zero configurazione infra; backup = copiare `.repoctx/`.
- JSON artifact versionati in `.repoctx/*.json` come contratto pubblico.
- Grafo molto profondo potrebbe richiedere in futuro un engine dedicato (KùzuDB) — upgrade path documentato in ARCHITECTURE.md.
