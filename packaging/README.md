# Packaging & distribution

Becket ships via **cargo-dist** (see `dist-workspace.toml` and [ADR-0004](../docs/adr/0004-cargo-dist-distribution.md)).

## Channels

| Channel | Artifact | Notes |
|---------|----------|-------|
| GitHub Releases | `.tar.xz` / `.zip` per target | Triggered by git tag `v*.*.*` |
| Homebrew | Formula in tap repo | `brew install GabrieleRuggieri/becket/becket` |
| npm | `becket`, `becket-mcp` | `npx becket build` — binary downloaded at install |
| Cargo | `becket-cli`, `becket-mcp` crates | `cargo install becket-cli --locked` |

## Cutting a release

1. Bump `version` in root `Cargo.toml` (`[workspace.package]`) and run `cargo update -w`.
2. Update `CHANGELOG.md` / release notes (if present).
3. Commit, tag, push:

```bash
git commit -am "release: version 0.2.0"
git tag v0.2.0
git push && git push --tags
```

4. GitHub Actions `Release` workflow builds artifacts and publishes to GitHub Releases (+ Homebrew tap + npm when configured).
5. Optional: `cargo publish -p becket-cli` and `cargo publish -p becket-mcp`.

## Homebrew tap setup (one-time)

Create an empty GitHub repository:

`https://github.com/GabrieleRuggieri/homebrew-becket`

cargo-dist publishes generated `becket.rb` formulas there on each stable release. Users install with:

```bash
brew tap GabrieleRuggieri/becket
brew install becket
```

## npm packages

cargo-dist generates npm wrapper packages at release time (`becket-cli-npm-package.tar.gz` in the release assets). The published package name is configured in `crates/becket-cli/Cargo.toml`:

```toml
[package.metadata.dist]
npm-package = "becket"
```

## Local dist development

```bash
cargo install cargo-dist --locked
dist plan          # preview release artifacts
dist generate      # refresh CI after config changes
```

Cross-compilation for all targets runs in CI; local `dist build` may require `cargo-zigbuild` / `cargo-xwin` on some hosts.
