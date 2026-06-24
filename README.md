# RepoCtx — AI Context & Impact Engine for Codebases

## Overview

RepoCtx is a developer-first CLI tool and local intelligence layer that builds a structured, semantic representation of a software repository. Its goal is to provide AI coding agents (and developers) with high-quality, minimal, and accurate context about a codebase.

Instead of treating a repository as a collection of files, RepoCtx transforms it into a **queryable system of architecture, flows, dependencies, and domain concepts**.

It acts as a bridge between:
- Large codebases
- AI coding agents (Claude Code, Cursor, Codex, etc.)
- Developers navigating and refactoring systems

---

## The Problem

Modern AI coding tools are powerful but fundamentally limited by context:

### 1. Context Window Limitations
Even large models struggle when:
- repositories exceed hundreds or thousands of files
- multiple layers of abstraction exist
- domain logic is scattered

### 2. Lack of Architectural Understanding
LLMs typically:
- understand local code snippets
- fail to reconstruct system-wide architecture
- hallucinate dependencies or flows

### 3. Poor Impact Awareness
Agents often:
- modify code without understanding side effects
- break unrelated features
- miss hidden dependencies

### 4. No Persistent Repository Memory
Every session is stateless:
- no durable understanding of the system
- repeated analysis cost
- inconsistent reasoning over time

---

## The Solution

RepoCtx introduces a **local intelligence layer** that continuously analyzes a repository and exposes structured knowledge.

It builds a persistent representation of:

- Architectural structure
- Domain concepts
- Execution flows
- Entry points
- Dependencies
- Symbol relationships
- Change impact maps

---

## Core Idea

Instead of asking an AI:

> "Read this repository and understand it"

We provide:

> "Here is a structured, precomputed understanding of the repository"

This reduces:
- token usage
- hallucinations
- context rebuilding cost

and increases:
- accuracy
- consistency
- speed of reasoning

---

## CLI Interface

### Initialize analysis

```bash
repoctx build
```

Generates:

```
.repoctx/
  architecture.json
  symbols.json
  flows.json
  dependencies.json
  entrypoints.json
```

---

### Query impact of changes

```bash
repoctx impact UserService
```

Output:
- modules affected
- downstream dependencies
- related tests
- potential risk zones

---

### Understand a flow

```bash
repoctx flow payment
```

Output:
- end-to-end execution path
- service interactions
- external systems involved

---

### Generate AI-ready context

```bash
repoctx context PaymentService
```

Output:
A minimal structured context optimized for LLM consumption:
- domain responsibility
- related components
- external dependencies
- constraints and invariants

---

## Key Features

### 1. Impact Analysis Engine
Determines what breaks when a component changes.

### 2. Semantic Flow Reconstruction
Reconstructs business flows across services and modules.

### 3. Repository Memory Layer
Maintains persistent structural understanding.

### 4. AI Context Compression
Reduces large codebases into minimal, relevant context for LLMs.

---

## Integration with AI Tools

RepoCtx is designed to be **agent-agnostic**.

### Claude Code / Claude CLI

Agents can call:

```bash
repoctx context <symbol>
repoctx impact <symbol>
repoctx flow <domain>
```

to retrieve precise context before modifying code.

---

### Cursor IDE

Cursor can integrate RepoCtx as a background context provider:
- enrich code suggestions with architectural awareness
- reduce incorrect refactors
- improve multi-file edits

---

### OpenAI Codex / Future CLI Agents

Any agent can use RepoCtx as a tool:

```bash
tools:
  - repoctx.context
  - repoctx.impact
  - repoctx.flow
```

This enables structured reasoning over large codebases.

---

### MCP (Model Context Protocol) Integration

RepoCtx exposes an MCP server:

```
repoctx-mcp
```

Available tools:
- get_context
- get_impact
- get_flow
- get_dependencies

This allows seamless integration with modern AI agents.

---

## Design Principles

### 1. Deterministic First
Core analysis should be deterministic where possible.

### 2. AI-Augmented, Not AI-Dependent
AI enhances interpretation, but structure is derived from code.

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

RepoCtx fills this gap by becoming the **semantic layer between code and intelligence**.

---

## Long-Term Vision

RepoCtx aims to become:

> The standard context layer for all AI coding agents.

In the same way Git became the standard for version control,
RepoCtx aims to become the standard for:

- code understanding
- AI context retrieval
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
git clone <repo-url>
cd repoctx
cargo build --release
./target/release/repoctx build
```

### MCP server (AI agents)

```bash
# Dalla root del repo da analizzare (dopo `repoctx build`)
export REPOCTX_ROOT=.
cargo run --bin repoctx-mcp --release
```

Tools esposti: `get_context`, `get_impact`, `get_flow`, `get_dependencies`.

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
| [RULES.md](./RULES.md) | Git, commit, testing, and code quality conventions |

### License

Apache-2.0 — see [LICENSE](./LICENSE).

---

## Conclusion

RepoCtx is not just another developer tool.

It is a **missing layer between codebases and AI reasoning systems**.

It transforms raw repositories into structured, queryable knowledge systems that both humans and AI agents can rely on consistently.
