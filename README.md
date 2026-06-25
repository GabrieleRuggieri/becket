# Becket — AI Context & Impact Engine for Codebases

## Overview

Becket is a developer-first CLI and MCP server that turns a repository into **persistent, queryable knowledge** for AI coding agents and developers. Its goal is to give an agent the *right code plus the right understanding* for a task — without dumping the whole repo into the context window, and without re-deriving everything on every query.

**North star:** one call → one context bundle (markdown) with verified code snippets + meaning + impact, within a token budget. See [ADR-0006](./docs/adr/0006-grounded-knowledge-wiki.md).

Instead of treating a repository as a collection of files, Becket maintains three coordinated layers:

1. **Deterministic Core** — a precise, code-derived graph of symbols, dependencies, flows, entry points and change-impact. This is *ground truth*: measured from the source, never guessed.
2. **Knowledge Layer (Repo Wiki)** — a persistent, compounding set of markdown pages (module/service/flow/concept) that explain *intent, conventions and gotchas*. Pages are **anchored to real symbols** and authored lazily by the host agent's model (via MCP sampling) — never re-derived from scratch.
3. **Context Assembly** — on a query, Becket returns the relevant **wiki page + actual code snippets + impact set**, packed within a token budget. This is the "code in context" layer.

The deterministic core keeps the wiki trustworthy: Becket derives the real call graph from source, **grounds** wiki pages on symbol IDs, and **lints** claims against the live graph so stale or wrong relationships are flagged automatically.

It acts as a bridge between:
- Large codebases
- AI coding agents (Claude Code, Cursor, Codex, etc.)
- Developers navigating and refactoring systems

**Website:** [becket.dev](https://becket.dev) (or browse [`website/`](./website/) locally) · **Releases:** [GitHub Releases](https://github.com/GabrieleRuggieri/becket/releases)

---

## Use today (v0.2 — shipped)

These work **now**. No bundled LLM — optional prose enrichment uses your MCP host's model.

| Command / tool | What you get |
|---|---|
| `becket build` | Deterministic graph + **grounded wiki** → `.becket/*.json` + `.becket/wiki/` |
| `becket impact <symbol>` | What breaks downstream (call graph + modules) |
| `becket flow <domain>` | End-to-end execution path across services |
| `becket context <symbol>` | **Markdown bundle**: wiki + real code snippets + impact (`--budget`, `--task`) |
| `becket wiki sync\|lint\|show` | Recompile stale pages, CI lint, view grounded pages |
| `becket build --watch` | Incremental rebuild; auto-syncs stale wiki structure |
| `becket workspace build` | Cross-repo linking (HTTP/gRPC/queue) |
| MCP `get_context` | Same markdown bundle for agents |
| MCP `get_wiki` | Grounded wiki page; `enrich=true` fills prose via sampling |
| MCP `get_impact`, `get_flow`, `get_dependencies` | Same queries for Cursor / Claude Code |

### Quick start (3 steps)

```bash
# 1. Install (pick one)
cargo install becket-cli becket-mcp --locked
# or: npx becket build   (downloads binary)

# 2. Index your repo
cd your-project && becket build

# 3. Get agent-ready context before you edit
becket context PaymentService --budget 6000 --task fix
becket wiki lint --strict   # optional CI gate
```

### Cursor / Claude Code (MCP)

Add to your MCP config (`.cursor/mcp.json` or Claude Code settings):

```json
{
  "mcpServers": {
    "becket": {
      "command": "becket-mcp",
      "env": {
        "BECKET_ROOT": "${workspaceFolder}"
      }
    }
  }
}
```

Run `becket build` once per repo (or use `becket build --watch` in a terminal). The agent can call `get_context` / `get_wiki` / `get_impact` before modifying code.

### Wiki prose enrichment

- `becket wiki sync` — recompiles **structure** from the graph (preserves enriched prose)
- MCP `get_wiki` with `enrich=true` — fills intent/gotchas via host model and **persists** to `.becket/wiki/`

---

## Legacy v0.1 surface

Impact, flow, and MCP queries were the primary v0.1 workflow. They remain fully supported; v0.2 adds the wiki layer and markdown context bundle on top.

---

## What Becket does

Becket is a **local intelligence layer for codebases**. It indexes your repository once, keeps a **persistent memory** under `.becket/`, and answers agent-ready queries with verified structure, optional human-readable wiki prose, and **real source snippets** — packed to your token budget.

### The problem

When an AI agent (or a developer) works on a large repo, four things go wrong:

1. **Too much code** — you cannot fit the whole tree in the context window.
2. **No system view** — local snippets are not enough; callers, flows, and modules matter.
3. **No impact awareness** — a small change can break distant code nobody thought to load.
4. **No durable memory** — every session re-discovers the same architecture from scratch.

Becket addresses all four with one local index and one query surface.

### How it works

```
your repo  →  becket build  →  .becket/ (graph + wiki)
                                      ↓
              becket context | impact | flow | wiki …
                                      ↓
                         markdown bundle for agents
```

1. **`becket build`** — walks and parses the repo (tree-sitter). Builds symbols, call graph, flows, entrypoints, impact maps, and grounded wiki pages. **No LLM required.**
2. **Persistent memory** — JSON artifacts + `.becket/wiki/*.md` survive across sessions. Incremental rebuild on `build --watch`; wiki pages get stale fingerprints when anchored symbols change.
3. **Per-task queries** — `context`, `impact`, `flow`, and MCP tools return what you need *now*, not the whole repo.

### Capabilities

| Capability | What you get |
|---|---|
| **Code graph** | Symbols, dependencies, call edges, entrypoints — measured from source, reproducible |
| **Impact analysis** | Downstream callers and modules affected by a change |
| **Flow reconstruction** | End-to-end paths across services (e.g. `payment` → handler → client → queue) |
| **Grounded wiki** | Module / service / flow pages anchored to symbol IDs; structure from the graph |
| **Wiki lint** | Stale pages, broken claims, orphan links — deterministic checks vs live graph; `--strict` for CI |
| **Context assembly** | One markdown bundle: wiki + ranked code snippets + impact, within `--budget` and `--task` |
| **MCP server** | `get_context`, `get_wiki`, `get_impact`, `get_flow`, `get_dependencies` for Cursor / Claude Code |
| **Workspace mode** | Cross-repo linking (HTTP, gRPC, queues) for monorepos and polyrepos |

### Typical workflows

**Before fixing a bug** — agent calls `get_context` on the failing symbol with `--task fix`: callers, relevant snippets, impact depth 2, wiki gotchas.

**Before a refactor** — `becket impact SymbolName` (or MCP `get_impact`) to see blast radius; `wiki lint --strict` in CI so docs do not lie about call relationships.

**Onboarding** — `becket flow <domain>` plus `context` with `--task onboard` for flows and overview with fewer snippets.

**Enriching intent** — MCP `get_wiki` with `enrich=true` fills prose slots (intent & gotchas) using the host model; structure stays graph-compiled.

### Design principles

- **Local-first** — no cloud, no telemetry, no API keys in the core path
- **Graph is ground truth** — wiki structure and lint claims come from the deterministic core
- **Code is never generated** — snippets are sliced from disk; only optional wiki prose is model-authored
- **One call for agents** — north star: `get_context` → single markdown bundle, not manual file `@` juggling

---

## CLI Interface

### Initialize analysis

```bash
becket build
```

Generates:

```
.becket/
  architecture.json
  symbols.json
  flows.json
  dependencies.json
  entrypoints.json
  wiki/               # grounded knowledge pages (symbol-anchored)
    index.md
    <page>.md
```

The JSON artifacts are produced deterministically with **no model required**.

---

### Knowledge wiki

```bash
becket wiki sync     # recompile stale pages (structure; preserves enriched prose)
becket wiki lint     # flag stale, contradictory, or orphan pages against the graph
becket wiki show payment
```

`lint` is the differentiator: because the deterministic graph is ground truth, Becket can detect when
a page claims a relationship the code no longer has. Use `wiki lint --strict` in CI.

Prose enrichment: MCP `get_wiki` with `enrich=true` (host model required).

### Query impact of changes

```bash
becket impact UserService
```

Output:
- modules affected
- downstream dependencies
- related tests
- potential risk zones

---

### Understand a flow

```bash
becket flow payment
```

Output:
- end-to-end execution path
- service interactions
- external systems involved

---

### Generate AI-ready context

```bash
becket context PaymentService --budget 6000 --task fix
becket context PaymentService --json   # structured output for tooling
```

One markdown bundle within the token budget:

- relevant **wiki page** (intent, conventions, gotchas when enriched)
- **actual code snippets** (callers/callees, sliced from disk)
- **impact set** and related tests

Task modes: `fix` (default), `refactor`, `onboard`.

---

## Integration with AI Tools

Becket is designed to be **agent-agnostic**.

### Claude Code / Claude CLI

Agents can call:

```bash
becket context <symbol>
becket impact <symbol>
becket flow <domain>
```

to retrieve precise context before modifying code.

---

### Cursor IDE

Cursor can integrate Becket as a background context provider:
- enrich code suggestions with architectural awareness
- reduce incorrect refactors
- improve multi-file edits

---

### OpenAI Codex / Future CLI Agents

Any agent can use Becket as a tool:

```bash
tools:
  - becket.context
  - becket.impact
  - becket.flow
```

This enables structured reasoning over large codebases.

---

### MCP (Model Context Protocol) Integration

Becket exposes an MCP server:

```
becket-mcp
```

Available tools:
- **get_impact**, **get_flow**, **get_dependencies** — shipped
- **get_context** — markdown bundle (wiki + snippets + impact); optional MCP sampling enrichment
- **get_wiki** — grounded wiki page; `enrich=true` for prose via host model

This allows seamless integration with modern AI agents. Wiki authoring/enrichment runs through the
host agent's model via **MCP sampling** — Becket bundles no LLM and holds no API keys.

---

## Design Principles

### 1. Deterministic First
Core analysis should be deterministic where possible.

### 2. AI-Augmented, Not AI-Dependent
AI enhances interpretation (wiki prose, names, summaries), but **structure is derived from code** and
the wiki is always **verifiable against the graph**. Remove the AI layer and you lose prose, never
correctness.

### 3. Local-First
All analysis runs locally to ensure:
- privacy
- speed
- reproducibility

### 4. Machine-Readable Outputs
All outputs must be:
- JSON-compatible
- stable schema
- versioned

---

## Why This Matters

The future of software development is:
- AI-assisted
- multi-agent
- context-heavy

But current systems lack:
- persistent understanding of codebases
- structured architectural memory
- reliable impact reasoning

Becket fills this gap by becoming the **semantic layer between code and intelligence**.

---

## Long-Term Vision

Becket aims to become:

> The standard context layer for all AI coding agents — a **verified, compounding memory** of a codebase.

In the same way Git became the standard for version control,
Becket aims to become the standard for:

- code understanding
- AI context retrieval (code + knowledge, not just metadata)
- a self-verifying knowledge wiki grounded in the code
- architectural reasoning

---

## Success Criteria

The tool is successful if:

- developers use it daily before commits/PRs
- AI agents call it before modifying code
- it reduces debugging and refactoring errors
- it becomes part of standard dev workflow

---

## Development

### Prerequisites

- Rust 1.75+ (`rustup`)

### Build from source

```bash
git clone https://github.com/GabrieleRuggieri/becket.git
cd becket
cargo build --release
./target/release/becket build
```

### Install (prebuilt)

**Homebrew** (after tap is published):

```bash
brew tap GabrieleRuggieri/becket
brew install becket
```

**npm** (downloads native binary from GitHub Releases):

```bash
npx becket build
```

**Cargo** (builds from source):

```bash
cargo install becket-cli --locked
cargo install becket-mcp --locked
```

**GitHub Releases**: download the archive for your platform from [Releases](https://github.com/GabrieleRuggieri/becket/releases), or use the shell installer attached to each release.

See [packaging/README.md](./packaging/README.md) for maintainers cutting a new version.

### MCP server (AI agents)

```bash
# Dalla root del repo da analizzare (dopo `becket build`)
export BECKET_ROOT=.
cargo run --bin becket-mcp --release
```

Tools esposti: `get_context`, `get_impact`, `get_flow`, `get_dependencies`, `get_wiki`.

### Run tests & lint

```bash
cargo test --all
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

### Git hooks (rustfmt pre-commit)

Setup una tantum (formatta automaticamente prima di ogni commit):

```bash
./scripts/setup-git-hooks.sh
```

Il gate CI `cargo fmt --check` resta attivo come rete di sicurezza.

### Documentation map

| Document | Purpose |
|---|---|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | Stack, data model, API contracts (source of truth) |
| [CODEMAP.md](./CODEMAP.md) | Execution flow and crate graph |
| [PROGRESS.md](./PROGRESS.md) | Development log / completed milestones |
| [BACKLOG.md](./BACKLOG.md) | Open work prioritized P0–P2 |
| [CONTRIBUTING.md](./CONTRIBUTING.md) | Development setup, tests, and contribution guide |
| [CHANGELOG.md](./CHANGELOG.md) | Release history |

### License

Apache-2.0 — see [LICENSE](./LICENSE).

---

## Conclusion

Becket is not just another developer tool.

It is a **missing layer between codebases and AI reasoning systems**.

It transforms raw repositories into a verified graph, a compounding knowledge wiki grounded in that
graph, and on-demand code bundles — a queryable knowledge system that both humans and AI agents can
rely on consistently, without re-reading the whole repo or trusting an unverifiable wiki.
