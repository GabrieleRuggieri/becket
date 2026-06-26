# Windows support (tier-2)

Becket treats **Windows** as a **tier-2** platform: binaries are built and tested in CI, but
**Ubuntu** and **macOS** are tier-1 (required green on every PR).

## CI

- **Tier-1** (`.github/workflows/ci.yml` → `tier1`): `ubuntu-latest`, `macos-latest` — must pass.
- **Tier-2** (`windows-tier2`): `windows-latest` with `continue-on-error: true` — failures are
  triaged asynchronously and do not block merges. Latency budget tests (`bench_budget`) are
  ignored in `cargo test --all` and run only on tier-1 via the dedicated CI step.

Release artifacts for `x86_64-pc-windows-msvc` are still produced via `cargo-dist` on tagged
releases.

## Reporting issues

When filing a Windows-only bug, include:

1. Windows version and shell (PowerShell / cmd)
2. `becket --version` and Rust toolchain (`rustc -V`)
3. Minimal repro repo or workspace manifest
4. Link to a failing `windows-tier2` CI run if available

Label suggestions: `platform:windows`, `tier-2`.

## Local development

```powershell
$env:BECKET_HASH_EMBED = "1"
cargo test --all
cargo build --release --bin becket
```

Use `BECKET_HASH_EMBED=1` to avoid downloading ONNX embedding models during tests.
