# ADR-0005: ID deterministici e artifact byte-identici

- **Stato:** Accettato
- **Data:** 2026-06-23

## Contesto

Rebuild incrementali e CI devono produrre output confrontabile e cache-friendly. UUID casuali rendono diff e test flaky.

## Decisione

Derivare tutti gli ID da **SHA-256** su chiavi canoniche (`stable_symbol_id`, `stable_edge_id`, …). Ordinare collezioni prima dell'export JSON. Test CI `rebuild_produces_byte_identical_artifacts`.

## Conseguenze

- Stesso input → stessi file `.repoctx/*.json` byte-per-byte.
- Cambio schema o logica di hashing richiede migrazione/version bump documentato.
- Cross-repo edge ids usano prefisso `cross_edge` con repo + symbol ids.
