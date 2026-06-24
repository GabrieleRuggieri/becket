# ADR-0004: Distribuzione con cargo-dist

- **Stato:** Accettato
- **Data:** 2026-06-23

## Contesto

RepoCtx deve raggiungere sviluppatori Rust, macOS/Linux nativi e ecosistema JS (`npx repoctx`) con binari firmati e installer ripetibili.

## Decisione

Usare **[cargo-dist](https://axodotdev.github.io/cargo-dist/)** (`dist-workspace.toml`) per:

- Build cross-platform su tag `v*.*.*` → GitHub Releases
- Installer shell, **npm** (`repoctx`, `repoctx-mcp`) e **Homebrew** (tap `GabrieleRuggieri/homebrew-repoctx`)
- CI generata in `.github/workflows/release.yml`

Canali complementari: `cargo install repoctx-cli` e build da sorgente.

## Conseguenze

- Il tap Homebrew va creato come repo separato prima del primo publish automatico.
- `dist init` va rieseguito quando si aggiorna cargo-dist.
- Release = bump versione workspace + tag `v0.x.y` + push.
