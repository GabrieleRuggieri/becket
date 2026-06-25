# Contributing to RepoCtx

Thank you for helping improve RepoCtx. This guide covers local development, the test workflow, and how to add new tree-sitter language support.

## Prerequisites

- Rust stable (see `rust-version` in the root `Cargo.toml`)
- `cargo fmt`, `cargo clippy`, and `cargo test`

## Local setup

```bash
git clone https://github.com/GabrieleRuggieri/repo-ctx.git
cd repo-ctx
cargo build
cargo test --all
```

Optional git hooks:

```bash
./scripts/setup-git-hooks.sh
```

## Development workflow

1. Create a feature branch: `feature/<short-description>`
2. Keep changes focused and deterministic (stable IDs, sorted output)
3. Run quality checks before opening a PR:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
REPOCTX_HASH_EMBED=1 cargo test --all
```

4. Update `PROGRESS.md` for notable milestones and `BACKLOG.md` when closing items
5. Use [Conventional Commits](https://www.conventionalcommits.org/) (Italian messages are welcome)

## Project layout

| Crate | Role |
|---|---|
| `repoctx-cli` | CLI (`repoctx build`, `workspace build`, `wiki`, queries) |
| `repoctx-core` | Build pipeline, parsing, graph, workspace linker, **wiki compiler + linter** |
| `repoctx-store` | SQLite index + JSON artifacts |
| `repoctx-query` | Shared query engine for CLI and MCP |
| `repoctx-schema` | Versioned artifact types + JSON Schema |
| `repoctx-embed` | Local embeddings (hash + optional ONNX) |
| `repoctx-mcp` | MCP stdio server |

See [CODEMAP.md](./CODEMAP.md) and [ARCHITECTURE.md](./ARCHITECTURE.md) for execution flow and design decisions.

**Product direction (v0.2):** deterministic graph + graph-grounded wiki + context assembly (markdown bundle). See [ADR-0006](./docs/adr/0006-grounded-knowledge-wiki.md) and [ADR-0007](./docs/adr/0007-no-rlm-in-core.md). Open items: [BACKLOG.md](./BACKLOG.md) (P1-15 release tag, P2 scale).

## Adoption workflow (for contributors testing the tool)

1. `cargo install repoctx-cli repoctx-mcp --locked` (or build from source)
2. `repoctx build` in a fixture repo or this monorepo
3. `repoctx context <Symbol> --budget 6000 --task fix` — verify markdown bundle
4. `repoctx wiki lint --strict` — verify lint passes
5. Wire `repoctx-mcp` in Cursor (see README) and confirm `get_context` / `get_wiki`

## Adding a language (tree-sitter plugin)

RepoCtx uses a **grammar registry** (`crates/repoctx-core/src/parse/registry.rs`) that maps file extensions to tree-sitter grammars. Built-in languages today:

| Language | Crate | Extensions |
|---|---|---|
| Rust | `tree-sitter-rust` | `.rs` |
| Python | `tree-sitter-python` | `.py`, `.pyi` |
| JavaScript | `tree-sitter-javascript` | `.js`, `.jsx`, `.mjs`, `.cjs` |
| TypeScript | `tree-sitter-typescript` | `.ts`, `.tsx` |
| Go | `tree-sitter-go` | `.go` |
| Java | `tree-sitter-java` | `.java` |

### Steps to add a compile-time language

1. **Add the grammar crate** to the workspace `Cargo.toml` and `repoctx-core/Cargo.toml`:

```toml
tree-sitter-ruby = "0.23"
```

2. **Register the grammar** in `GrammarRegistry::builtins()` inside `parse/registry.rs`:

```rust
registry.register_builtin(
    GrammarSpec {
        id: "ruby",
        extensions: &["rb"],
        description: "Ruby (tree-sitter-ruby)",
    },
    RepoLanguage::Ruby, // add variant in language.rs
    || tree_sitter_ruby::LANGUAGE.into(),
);
```

3. **Extend extraction rules** in `parse/tree_sitter.rs` if the language needs custom node handling (calls, imports, HTTP routes).

4. **Add a fixture** under `tests/fixtures/<name>/` and an integration test in `crates/repoctx-core/tests/fixtures.rs`.

5. **Run the full test suite** and update docs if the language changes public behavior.

### Extension aliases without recompiling

Repositories can map extra extensions to an existing built-in grammar via `repoctx.languages.toml` at the repo root:

```toml
[[languages]]
id = "typescript"
extensions = ["mts", "cts"]
```

Unknown `id` values log a warning — dynamic `.so` grammars are not supported yet.

## Workspace / multi-repo development

Workspaces use `repoctx.workspace.toml` at the monorepo root:

```toml
schema_version = "1.0.0"
name = "my-platform"

[[repos]]
name = "gateway"
path = "services/gateway"

[[repos]]
name = "users"
path = "services/users"
```

Build and link cross-repo edges:

```bash
repoctx workspace build --json
```

Output lands in `<workspace>/.repoctx/cross_repo.json`. See `tests/fixtures/workspace/` for a minimal example.

## JSON Schema contract

Artifact schemas live in `schemas/`. After changing `repoctx-schema` types:

```bash
cargo test -p repoctx-schema write_schemas -- --ignored --nocapture
```

CI verifies committed schemas match generated output.

## Release workflow

Maintainers use [cargo-dist](https://axodotdev.github.io/cargo-dist/) — see [packaging/README.md](./packaging/README.md):

```bash
# bump [workspace.package].version, then:
git tag v0.2.0 && git push --tags
```

CI in `.github/workflows/ci.yml` runs tier-1 checks on Ubuntu and macOS; Windows is tier-2 (see [docs/windows.md](./docs/windows.md)).

CI in `.github/workflows/release.yml` builds binaries and publishes npm/Homebrew artifacts.

## Architecture decisions

Significant design choices are recorded in [docs/adr/](./docs/adr/README.md).

## Questions

Open a [GitHub issue](https://github.com/GabrieleRuggieri/repo-ctx/issues) for design questions before large refactors. For new architecture decisions, add an ADR under `docs/adr/`.
