const translations = {
  en: {
    "a11y.skip": "Skip to content",
    "nav.problem": "Problem",
    "nav.solution": "Solution",
    "nav.layers": "Layers",
    "nav.product": "Product",
    "nav.features": "Features",
    "nav.integrations": "Integrations",
    "nav.docs": "Docs",
    "nav.getStarted": "Get Started",
    "nav.home": "Home",
    "hero.badge": "Open Source · Local-First · Apache-2.0",
    "hero.title1": "One call.",
    "hero.title2": "The right context.",
    "hero.sub":
      "Becket combines a verified code graph, a graph-grounded knowledge wiki, and real code snippets — giving AI agents one markdown context bundle per task, within your token budget.",
    "hero.cta2": "Read the Docs",
    "hero.install.terminal": "terminal",
    "hero.install.copy": "Copy",
    "hero.install.mcpHint":
      'Built for Cursor and Claude Code — install <code>becket-mcp</code> and follow the <a href="docs.html#quickstart">Quick Start</a>.',
    "hero.stat1": "Local & Free",
    "hero.stat2": "Telemetry",
    "hero.stat3": "Knowledge Layers",
    "problem.tag": "The Problem",
    "problem.title": "AI tools are powerful, but context-blind",
    "problem.sub": "Modern coding agents hit hard limits when dealing with real-world codebases.",
    "problem.c1.title": "Context Window Limits",
    "problem.c1.desc":
      "Repositories with thousands of files and scattered domain logic exceed what any model can hold in memory.",
    "problem.c2.title": "No Architectural View",
    "problem.c2.desc":
      "LLMs understand local snippets but fail to reconstruct system-wide architecture and hallucinate dependencies.",
    "problem.c3.title": "Poor Impact Awareness",
    "problem.c3.desc":
      "Agents modify code without understanding side effects, breaking unrelated features and missing hidden deps.",
    "problem.c4.title": "No Persistent Memory",
    "problem.c4.desc":
      "Every session starts from zero — repeated analysis cost, inconsistent reasoning over time.",
    "solution.tag": "The Solution",
    "solution.title": "Verified structure, compounding knowledge, real code",
    "solution.quote":
      '"The right code, the right understanding, and what breaks if you change it — within your token budget."',
    "solution.desc":
      "Becket combines a deterministic code graph (ground truth), a graph-grounded knowledge wiki (intent & gotchas), and context assembly that returns actual source snippets — not metadata alone.",
    "solution.li1": "Graph verifies the wiki — no silent drift",
    "solution.li2": "Persistent memory that compounds across sessions",
    "solution.li3": "Code snippets + impact packed to your budget",
    "solution.flow1": "Source Code",
    "solution.flow2": "Deterministic Core",
    "solution.flow3": "Repo Wiki",
    "solution.flow4": "Impact",
    "solution.flow5": "Flows",
    "solution.flow6": "Snippets",
    "solution.flow7": "Context Bundle → AI Agents",
    "layers.tag": "Architecture",
    "layers.title": "Three layers, one memory system",
    "layers.sub":
      "Structure from the graph, meaning in the wiki, code in the bundle — three layers that answer what is true, what it means, and what you need right now.",
    "layers.l1.title": "Deterministic Core",
    "layers.l1.q": "What is <em>true</em>?",
    "layers.l1.desc":
      "Symbols, call graph, flows, entrypoints, impact — measured from source with tree-sitter. Byte-identical rebuilds. Zero LLM.",
    "layers.l2.title": "Grounded Repo Wiki",
    "layers.l2.q": "What does it <em>mean</em>?",
    "layers.l2.desc":
      "Compounding markdown pages anchored to symbol IDs. Authored lazily via your agent's model. Stale pages are flagged automatically — the graph is ground truth.",
    "layers.l3.title": "Context Assembly",
    "layers.l3.q": "What code do I need <em>now</em>?",
    "layers.l3.desc":
      "Real source snippets + wiki page + impact set, ranked and packed to your token budget. Code is never model-generated.",
    "product.tag": "The product",
    "product.title": "Everything an agent needs from your repo",
    "product.sub":
      "One local index. Persistent memory. Query-ready bundles — no re-reading the whole codebase every session.",
    "product.p1.title": "Index once",
    "product.p1.desc":
      "<code>becket build</code> parses symbols, calls, flows, and impact. Writes <code>.becket/</code> — no LLM required.",
    "product.p2.title": "Memory persists",
    "product.p2.desc":
      "Graph JSON + grounded wiki pages survive across agent sessions. Incremental rebuild on watch; stale wiki flagged automatically.",
    "product.p3.title": "Query per task",
    "product.p3.desc":
      "MCP <code>get_context</code> in Cursor / Claude Code — or CLI <code>context</code>, <code>impact</code>, <code>flow</code> from the terminal.",
    "product.li1": "MCP in Cursor / Claude Code — agent can call get_context when your rules or prompt ask for it",
    "product.li2": "Local .becket/ cache — gitignore it, rebuild anytime with becket build",
    "product.li3": "Demo folder in the repo — try Becket in 30 seconds",
    "product.li4": "Cross-repo workspace linking for monorepos and polyrepos",
    "features.tag": "Key Features",
    "features.title": "Built for precision, not guesswork",
    "features.f1.title": "Deterministic Code Graph",
    "features.f1.desc": "Symbols, dependencies, calls, flows and entrypoints — measured from source, not guessed.",
    "features.f2.title": "Impact Analysis Engine",
    "features.f2.desc": "Know exactly what breaks when you change a component — modules, tests, risk zones.",
    "features.f3.title": "Grounded Knowledge Wiki",
    "features.f3.desc": "Compounding markdown knowledge anchored to real symbols, with automatic staleness detection.",
    "features.f4.title": "Context Assembly",
    "features.f4.desc": "One markdown bundle with code snippets, wiki, and impact — packed within a token budget.",
    "integrations.tag": "Integrations",
    "integrations.title": "Built for Cursor and Claude Code",
    "integrations.sub": "Primary workflow: MCP server inside your IDE. CLI for indexing, inspection, and CI.",
    "integrations.cursor":
      "Add becket-mcp to .cursor/mcp.json — expose get_context, get_impact, get_wiki to the agent (use a project rule so it calls them when needed).",
    "integrations.claude":
      "Same MCP server in Claude Code settings. Agent pulls verified context before modifying code.",
    "integrations.otherTitle": "Other MCP hosts",
    "integrations.codex":
      "Any MCP-compatible agent can use the same tools over stdio.",
    "integrations.mcp":
      "becket-mcp: get_context (code + wiki + impact), get_wiki, get_impact, get_flow, get_dependencies.",
    "cta.title": "Verified, compounding memory for AI coding agents",
    "cta.sub": "Index once. Let your agent pull the right context every time.",
    "cta.button": "Quick Start Guide",
    "footer.copy": "Open source under Apache-2.0 · 100% local · Zero telemetry",
    "footer.docs": "Documentation",
    "footer.github": "GitHub",
    "docs.nav.quickstart": "Quick Start",
    "docs.nav.mcp": "MCP",
    "docs.nav.cli": "CLI",
    "docs.nav.faq": "FAQ",
    "docs.nav.layers": "Layers",
    "docs.nav.wiki": "Wiki",
    "docs.nav.principles": "Principles",
    "docs.nav.architecture": "Architecture",
    "docs.nav.install": "Install",
    "docs.toc.quickstart": "Quick Start",
    "docs.toc.install": "Installation",
    "docs.toc.overview": "Overview",
    "docs.toc.demo": "Demo",
    "docs.toc.audience": "Who It's For",
    "docs.toc.layers": "Three Layers",
    "docs.toc.cli": "CLI Reference",
    "docs.toc.wiki": "Knowledge Wiki",
    "docs.toc.mcp": "MCP Integration",
    "docs.toc.languages": "Languages",
    "docs.toc.advanced": "Advanced",
    "docs.toc.artifacts": "Artifacts",
    "docs.toc.faq": "FAQ",
    "docs.toc.principles": "Design Principles",
    "docs.toc.architecture": "Architecture",
    "docs.quickstart.badge": "Recommended path",
    "docs.quickstart.title": "Quick Start",
    "docs.quickstart.lead":
      "Becket helps Cursor and Claude Code use fewer tokens with better context: index once locally, then the agent can call MCP tools to pull verified snippets instead of reading whole files.",
    "docs.quickstart.s1": "Install globally: <code>npm install -g becket becket-mcp</code> (or use <code>npx becket build</code> without global install).",
    "docs.quickstart.s2": "Index your repo: <code>cd your-project && becket build</code> (creates <code>.becket/</code>).",
    "docs.quickstart.s3": "Add <code>becket-mcp</code> to your MCP config (see below) with <code>BECKET_ROOT</code> set to your project root.",
    "docs.quickstart.s4": "Add a project rule (see MCP section) or ask the agent to call <code>get_context</code> on the symbol you are editing.",
    "docs.quickstart.callout":
      "<strong>Try first:</strong> clone the repo and run <code>cd demo && npx becket build</code> — a mini shop codebase ready to index.",
    "docs.quickstart.cursorTitle": "Cursor — MCP config",
    "docs.quickstart.claudeTitle": "Claude Code — MCP config",
    "docs.quickstart.claudeNote": "Replace the path with your project root. Claude Code reads the same MCP server as Cursor.",
    "docs.overview.title": "Overview",
    "docs.overview.lead":
      "Becket is a local intelligence layer that combines a verified code graph, a graph-grounded knowledge wiki, and context assembly that returns real source snippets for AI agents and developers.",
    "docs.overview.mcpFirst":
      "Primary workflow: MCP in your IDE so the agent can call get_context when needed — fewer tokens than @-mentioning whole files. The CLI indexes and inspects; it does not replace the agent. Add a project rule so the agent actually uses the tools.",
    "docs.demo.title": "Demo walkthrough",
    "docs.demo.desc":
      "The repository includes a demo/ folder — a mini shop with payment, order, and shipping flows. Use it to try Becket without touching your own codebase.",
    "docs.demo.outputDesc": "Example output (truncated) — a markdown bundle with wiki, real snippets, and impact:",
    "docs.demo.readme":
      'Full commands: see <a href="https://github.com/GabrieleRuggieri/becket/tree/main/demo">demo/README.md</a> in the repository.',
    "docs.audience.title": "Who it's for",
    "docs.audience.desc": "Becket targets developers and teams working on real codebases with AI coding agents.",
    "docs.audience.th.role": "You are…",
    "docs.audience.th.do": "Do this",
    "docs.audience.r1": "Cursor / Claude Code user",
    "docs.audience.d1": "<code>becket build</code> once → MCP config → agent calls <code>get_context</code>",
    "docs.audience.r2": "Developer (CLI only)",
    "docs.audience.d2": "<code>becket build</code> → <code>impact</code> / <code>context</code> / <code>flow</code> before refactors",
    "docs.audience.r3": "Staying fresh after git pull",
    "docs.audience.d3": "<code>becket build</code> or <code>becket build --watch</code> — local cache, not committed",
    "docs.audience.r4": "Monorepo / polyrepo",
    "docs.audience.d4": "<code>becket workspace build</code> for cross-service linking",
    "docs.layers.title": "Three Knowledge Layers",
    "docs.layers.desc":
      "Becket indexes your codebase locally and exposes three coordinated layers.",
    "docs.layers.l1": "Deterministic Core",
    "docs.layers.l1d": "Symbols, graph, flows, impact — from tree-sitter, no LLM",
    "docs.layers.l2": "Grounded Repo Wiki",
    "docs.layers.l2d": "Markdown pages anchored to symbol IDs, MCP-authored, lint-checked",
    "docs.layers.l3": "Context Assembly",
    "docs.layers.l3d": "Code snippets + wiki + impact, token-budgeted",
    "docs.cli.title": "CLI Reference",
    "docs.cli.intro": "The CLI indexes your repo and answers queries. Run it from the repository root (or pass --repo).",
    "docs.cli.build.title": "Index the repository",
    "docs.cli.build.desc": "Generates versioned JSON artifacts and grounded wiki pages under .becket/. No LLM required.",
    "docs.cli.wiki.title": "Knowledge wiki",
    "docs.cli.wiki.desc": "Sync recompiles wiki structure from the graph. Lint checks pages against live call edges — see Wiki section below.",
    "docs.cli.impact.title": "Query impact",
    "docs.cli.impact.desc": "Returns affected modules, downstream symbols, related tests, and risk zones.",
    "docs.cli.flow.title": "Understand a flow",
    "docs.cli.flow.desc": "Shows end-to-end execution path and service interactions.",
    "docs.cli.context.title": "Context bundle",
    "docs.cli.context.desc":
      "Task modes: fix (default, more snippets), refactor (wider impact), onboard (overview, fewer snippets).",
    "docs.cli.domain.title": "Domain refinement (optional)",
    "docs.cli.domain.desc": "Zero-config by default. Refine auto-discovered domains via CLI.",
    "docs.wiki.title": "Grounded Repo Wiki",
    "docs.wiki.desc":
      "Markdown pages for modules, services, and flows — each anchored to real symbol IDs from the deterministic graph. Structure is compiled from the graph; optional prose (intent and gotchas) via MCP sampling.",
    "docs.wiki.lintTitle": "What does <code>wiki lint</code> do?",
    "docs.wiki.lintDesc": "Wiki lint compares every wiki page against the live call graph — the graph is always ground truth. It reports:",
    "docs.wiki.liStale": "<strong>Stale pages</strong> — anchored code changed since the page was written (<code>graph_fingerprint</code> mismatch)",
    "docs.wiki.liClaims": "<strong>Contradictory claims</strong> — page says A calls B, but the graph says otherwise",
    "docs.wiki.liLinks": "<strong>Broken links</strong> — wiki references a symbol or page that no longer exists",
    "docs.wiki.liOrphans": "<strong>Orphan pages</strong> — pages not reachable from <code>index.md</code>",
    "docs.wiki.lintCi":
      "wiki lint --strict in CI is an optional edge case for teams that version wiki pages in git. For the usual workflow — local .becket/, agent via MCP, no commit — you do not need wiki lint.",
    "docs.wiki.li1": "Pages authored lazily via MCP sampling (host model, no bundled LLM)",
    "docs.wiki.li2": "MCP get_wiki with enrich=true fills intent/gotchas and persists to disk",
    "docs.wiki.li3": "index.md routes agents to relevant pages first",
    "docs.languages.title": "Supported languages",
    "docs.languages.desc": "Becket parses source with tree-sitter. Built-in support today:",
    "docs.languages.note":
      "Other extensions can map to a built-in grammar via becket.languages.toml at the repo root. See CONTRIBUTING.md for adding new languages.",
    "docs.mcp.title": "MCP Integration",
    "docs.mcp.desc":
      "Becket exposes an MCP server over stdio. Cursor and Claude Code list its tools alongside other MCP servers — the agent invokes them during a chat turn when relevant.",
    "docs.mcp.th.tool": "Tool",
    "docs.mcp.th.desc": "Description",
    "docs.mcp.th.when": "When to use",
    "docs.mcp.t1": "Wiki + code snippets + impact, packed to budget",
    "docs.mcp.t1w": "Before fixing or refactoring a symbol",
    "docs.mcp.t2": "Change impact analysis",
    "docs.mcp.t2w": "Before renaming or deleting a symbol",
    "docs.mcp.t3": "Business flow reconstruction",
    "docs.mcp.t3w": "Understanding end-to-end paths",
    "docs.mcp.t4": "Direct and transitive dependencies",
    "docs.mcp.t4w": "Dependency exploration",
    "docs.mcp.t5": "Grounded markdown page or index router",
    "docs.mcp.t5w": "Onboarding; enrich=true fills intent/gotchas",
    "docs.mcp.sampling":
      "Wiki prose and summaries use MCP sampling — your host agent's model writes intent and gotchas. Becket ships no LLM, holds no API keys, and never generates code — only slices real source from disk.",
    "docs.mcp.howTitle": "How does the agent call get_context?",
    "docs.mcp.howDesc":
      "Becket does not hook into every keystroke. MCP exposes tools to Cursor or Claude Code; the agent decides when to call them — same as any other MCP tool. Tool names and descriptions tell the model what is available; it invokes get_context when your task needs codebase context (fix a bug, refactor a symbol, etc.).",
    "docs.mcp.howTip":
      "For reliable use, add a project rule or ask explicitly — e.g. “use Becket get_context on PaymentService before editing”. Without that, the agent may skip the tool and guess from open files alone.",
    "docs.mcp.rulesTitle": "Cursor — project rule (recommended)",
    "docs.mcp.rulesNote": "Claude Code: add the same instruction in CLAUDE.md or your project system prompt.",
    "docs.mcp.cliNote":
      "<strong>CLI vs MCP:</strong> becket context returns the same bundle as get_context. MCP lets the agent fetch it inside the chat; the CLI is for you in the terminal. Neither runs by itself — MCP needs an agent turn; CLI needs you to run the command.",
    "docs.advanced.title": "Advanced",
    "docs.advanced.watch.title": "Incremental rebuild",
    "docs.advanced.watch.desc": "Re-parses changed files and auto-syncs stale wiki structure. Run in a terminal while you code.",
    "docs.advanced.workspace.title": "Multi-repo workspaces",
    "docs.advanced.workspace.desc": "Links repositories via HTTP, gRPC, or message queues. Requires a becket.workspace.toml manifest.",
    "docs.advanced.git.title": "Keep <code>.becket/</code> local",
    "docs.advanced.git.desc":
      "Add .becket/ to .gitignore. It is a local cache rebuilt by becket build — not source of truth for the team. The wiki inside is scaffolding for context assembly, not documentation to publish. Re-index after pulls or large refactors; use becket build --watch while coding if you want it always fresh.",
    "docs.advanced.platform.title": "Platform support",
    "docs.advanced.platform.desc":
      "macOS and Linux are tier-1. Windows binaries ship via GitHub Releases but are tier-2 — report issues on GitHub if something breaks.",
    "docs.artifacts.title": "Output Artifacts",
    "docs.artifacts.desc": "All outputs are JSON-compatible, schema-versioned, and machine-readable.",
    "docs.artifacts.th.file": "File",
    "docs.artifacts.th.content": "Content",
    "docs.artifacts.a1": "High-level structural map",
    "docs.artifacts.a2": "Symbol catalog with locations",
    "docs.artifacts.a3": "Dependency graph",
    "docs.artifacts.a4": "Reconstructed business flows",
    "docs.artifacts.a5": "Detected entry points",
    "docs.artifacts.a6": "Wiki router / table of contents",
    "docs.artifacts.a7": "Grounded knowledge pages (symbol-anchored)",
    "docs.faq.title": "FAQ & Troubleshooting",
    "docs.faq.q1": "Do I need Rust installed?",
    "docs.faq.a1":
      "No. npx becket build and npm install -g becket download prebuilt binaries. Rust is only needed to compile from source.",
    "docs.faq.q2": "MCP tools return \"index missing\"",
    "docs.faq.a2":
      "Run becket build in the repository root first. MCP reads from .becket/ — without a build, there is nothing to query.",
    "docs.faq.q3": "becket-mcp command not found",
    "docs.faq.a3":
      "Install globally: npm install -g becket-mcp. Ensure your npm global bin directory is on PATH. Restart Cursor/Claude Code after installing.",
    "docs.faq.q4": "Can I use Becket without Cursor or Claude Code?",
    "docs.faq.a4":
      "Yes — the CLI provides context, impact, and flow for terminal workflows. MCP is for the same queries inside Cursor/Claude when the agent calls the tools. Becket does not edit code or run an LLM by itself.",
    "docs.faq.q5": "Does Becket send my code to the cloud?",
    "docs.faq.a5":
      "No. All analysis runs locally. Optional wiki prose enrichment uses your MCP host's model (Cursor/Claude) — Becket itself holds no API keys and sends no telemetry.",
    "docs.faq.q6": "What symbol name should I use?",
    "docs.faq.a6":
      "Use a function, class, or module name visible in your source — e.g. capture, PaymentService, or a fully qualified name. Run becket build first; symbols come from the indexed graph.",
    "docs.faq.q7": "Does get_context run automatically on every edit?",
    "docs.faq.a7":
      "No. Cursor and Claude Code expose Becket as MCP tools; the agent calls them when it decides they are needed (or when your rules/prompt tell it to). Add a project rule or ask explicitly for best results. Becket is not a background daemon inside the IDE.",
    "docs.faq.q8": "Should I commit .becket/ or .becket/wiki/?",
    "docs.faq.a8":
      "No, for the typical workflow. Keep .becket/ in .gitignore and run becket build locally (or --watch). Committing wiki pages is only for niche teams that want shared prose in git — not required to save tokens or speed up your agent.",
    "docs.principles.title": "Design Principles",
    "docs.principles.p1.title": "Deterministic First",
    "docs.principles.p1.desc": "Core analysis is deterministic. Structure is derived from code, not guessed.",
    "docs.principles.p2.title": "AI-Augmented, Not AI-Dependent",
    "docs.principles.p2.desc": "AI enhances interpretation via MCP sampling. The tool works fully without it.",
    "docs.principles.p3.title": "Local-First",
    "docs.principles.p3.desc": "All analysis runs locally. Privacy, speed, reproducibility. Zero telemetry.",
    "docs.principles.p4.title": "Machine-Readable Outputs",
    "docs.principles.p4.desc": "JSON-compatible, stable schema, versioned artifacts.",
    "docs.principles.p5.title": "Graph-Grounded Wiki",
    "docs.principles.p5.desc": "Wiki pages anchor to symbol IDs and are lint-checked against the live graph.",
    "docs.arch.title": "Architecture",
    "docs.arch.desc":
      "Rust-first deterministic core, graph-grounded wiki, context assembly, SQLite index, MCP server. Multi-repo workspaces and cross-service linking.",
    "docs.arch.l1": "Surfaces",
    "docs.arch.l2": "Store",
    "docs.arch.l3": "Core",
    "docs.arch.l4": "Sources",
    "docs.arch.l5": "Assembly",
    "docs.arch.l5d": "Context bundle builder (snippets + wiki + impact)",
    "docs.arch.l6": "Wiki",
    "docs.arch.l6d": "Grounded markdown + lint engine",
    "docs.arch.note":
      'Full architecture: <a href="https://github.com/GabrieleRuggieri/becket/blob/main/ARCHITECTURE.md">ARCHITECTURE.md</a> in the repository.',
    "docs.install.title": "Installation",
    "docs.install.desc":
      "Becket is free and open source (Apache-2.0). No account, no subscription, no telemetry. You need Node.js/npm for the install wrappers — Rust is only required if you build from source.",
    "docs.install.path1.title": "Cursor / Claude Code",
    "docs.install.path1.desc": "Install both packages globally so MCP finds them on PATH.",
    "docs.install.path2.title": "Try without global install",
    "docs.install.path2.desc": "Downloads the native binary on first run. Good for a one-off index or the demo folder.",
    "docs.install.note":
      '<strong>Order matters:</strong> run <code>becket build</code> in your repo before using MCP. The agent queries <code>.becket/</code> — indexing is always step one.',
  },
  it: {
    "a11y.skip": "Salta al contenuto",
    "nav.problem": "Problema",
    "nav.solution": "Soluzione",
    "nav.layers": "Layer",
    "nav.product": "Prodotto",
    "nav.features": "Funzionalità",
    "nav.integrations": "Integrazioni",
    "nav.docs": "Documentazione",
    "nav.getStarted": "Inizia",
    "nav.home": "Home",
    "hero.badge": "Open Source · Local-First · Apache-2.0",
    "hero.title1": "Una chiamata.",
    "hero.title2": "Il contesto giusto.",
    "hero.sub":
      "Becket unisce un grafo verificato, una wiki ancorata al grafo e snippet di codice reali — un bundle markdown per task, nel tuo budget di token.",
    "hero.cta2": "Leggi la documentazione",
    "hero.install.terminal": "terminale",
    "hero.install.copy": "Copia",
    "hero.install.mcpHint":
      'Pensato per Cursor e Claude Code — installa <code>becket-mcp</code> e segui la <a href="docs.html#quickstart">guida rapida</a>.',
    "hero.stat1": "Locale e gratuito",
    "hero.stat2": "Telemetria",
    "hero.stat3": "Layer di conoscenza",
    "problem.tag": "Il problema",
    "problem.title": "Gli strumenti AI sono potenti, ma ciechi al contesto",
    "problem.sub": "Gli agenti di coding moderni incontrano limiti concreti sulle codebase reali.",
    "problem.c1.title": "Limiti della finestra di contesto",
    "problem.c1.desc":
      "Repository con migliaia di file e logica di dominio sparsa superano ciò che un modello può tenere in memoria.",
    "problem.c2.title": "Nessuna visione architetturale",
    "problem.c2.desc":
      "Gli LLM capiscono snippet locali ma non ricostruiscono l'architettura di sistema e allucinano dipendenze.",
    "problem.c3.title": "Scarsa consapevolezza d'impatto",
    "problem.c3.desc":
      "Gli agenti modificano codice senza capire gli effetti collaterali, rompendo feature non correlate.",
    "problem.c4.title": "Nessuna memoria persistente",
    "problem.c4.desc":
      "Ogni sessione riparte da zero — costi di analisi ripetuti, ragionamento incoerente nel tempo.",
    "solution.tag": "La soluzione",
    "solution.title": "Struttura verificata, conoscenza che cresce, codice reale",
    "solution.quote":
      '"Il codice giusto, la comprensione giusta e cosa si rompe se lo cambi — entro il tuo budget di token."',
    "solution.desc":
      "Becket combina un grafo deterministico (ground truth), una wiki ancorata al grafo (intent e gotcha) e un context assembly che restituisce snippet di sorgente reali — non solo metadati.",
    "solution.li1": "Il grafo verifica la wiki — niente drift silenzioso",
    "solution.li2": "Memoria persistente che si arricchisce tra le sessioni",
    "solution.li3": "Snippet di codice + impact nel tuo budget",
    "solution.flow1": "Codice sorgente",
    "solution.flow2": "Core deterministico",
    "solution.flow3": "Repo Wiki",
    "solution.flow4": "Impatto",
    "solution.flow5": "Flussi",
    "solution.flow6": "Snippet",
    "solution.flow7": "Context bundle → agenti AI",
    "layers.tag": "Architettura",
    "layers.title": "Tre layer, un sistema di memoria",
    "layers.sub":
      "Struttura dal grafo, significato nella wiki, codice nel bundle — tre layer per cosa è vero, cosa significa e cosa ti serve adesso.",
    "layers.l1.title": "Core deterministico",
    "layers.l1.q": "Cosa è <em>vero</em>?",
    "layers.l1.desc":
      "Simboli, call graph, flussi, entrypoint, impact — misurati dal sorgente con tree-sitter. Rebuild byte-identici. Zero LLM.",
    "layers.l2.title": "Repo Wiki ancorata",
    "layers.l2.q": "Cosa <em>significa</em>?",
    "layers.l2.desc":
      "Pagine markdown che crescono nel tempo, ancorate a symbol ID. Autore lazy via il modello del tuo agente. Pagine stale segnalate automaticamente.",
    "layers.l3.title": "Context assembly",
    "layers.l3.q": "Che codice mi serve <em>ora</em>?",
    "layers.l3.desc":
      "Snippet reali + pagina wiki + impact set, classificati e impacchettati nel budget token. Il codice non è mai generato dal modello.",
    "product.tag": "Il prodotto",
    "product.title": "Tutto ciò che un agente serve dal tuo repo",
    "product.sub":
      "Un indice locale. Memoria persistente. Bundle pronti per query — senza rileggere tutto il codebase ogni sessione.",
    "product.p1.title": "Indicizza una volta",
    "product.p1.desc":
      "<code>becket build</code> analizza simboli, call, flussi e impact. Scrive <code>.becket/</code> — senza LLM.",
    "product.p2.title": "Memoria che resta",
    "product.p2.desc":
      "JSON del grafo + pagine wiki ancorate tra le sessioni. Rebuild incrementale in watch; wiki stale segnalata automaticamente.",
    "product.p3.title": "Query per task",
    "product.p3.desc":
      "MCP <code>get_context</code> in Cursor / Claude Code — oppure CLI <code>context</code>, <code>impact</code>, <code>flow</code> da terminale.",
    "product.li1": "MCP in Cursor / Claude Code — l'agente può chiamare get_context quando glielo chiedono rule o prompt",
    "product.li2": "Cache locale .becket/ — mettila in gitignore, rebuild con becket build",
    "product.li3": "Cartella demo nel repo — prova Becket in 30 secondi",
    "product.li4": "Workspace cross-repo per monorepo e polyrepo",
    "features.tag": "Funzionalità chiave",
    "features.title": "Costruito per la precisione, non per indovinare",
    "features.f1.title": "Grafo deterministico",
    "features.f1.desc": "Simboli, dipendenze, call, flussi ed entrypoint — misurati dal sorgente.",
    "features.f2.title": "Motore di analisi d'impatto",
    "features.f2.desc": "Sai esattamente cosa si rompe quando cambi un componente.",
    "features.f3.title": "Wiki di conoscenza ancorata",
    "features.f3.desc": "Conoscenza markdown ancorata a simboli reali, con rilevamento automatico dello stale.",
    "features.f4.title": "Context assembly",
    "features.f4.desc": "Un bundle markdown con snippet, wiki e impact — nel budget token.",
    "integrations.tag": "Integrazioni",
    "integrations.title": "Pensato per Cursor e Claude Code",
    "integrations.sub": "Workflow principale: server MCP nell'IDE. CLI per indicizzazione, ispezione e CI.",
    "integrations.cursor":
      "Aggiungi becket-mcp a .cursor/mcp.json — espone get_context, get_impact, get_wiki all'agente (usa una project rule per farglieli chiamare quando serve).",
    "integrations.claude":
      "Lo stesso server MCP nelle impostazioni di Claude Code. L'agente recupera contesto verificato prima di modificare il codice.",
    "integrations.otherTitle": "Altri host MCP",
    "integrations.codex": "Qualsiasi agente compatibile MCP può usare gli stessi tool su stdio.",
    "integrations.mcp":
      "becket-mcp: get_context (codice + wiki + impact), get_wiki, get_impact, get_flow, get_dependencies.",
    "cta.title": "Memoria verificata per agenti di coding AI",
    "cta.sub": "Indicizza una volta. Lascia che l'agente recuperi il contesto giusto ogni volta.",
    "cta.button": "Guida rapida",
    "footer.copy": "Open source Apache-2.0 · 100% locale · Zero telemetria",
    "footer.docs": "Documentazione",
    "footer.github": "GitHub",
    "docs.nav.quickstart": "Guida rapida",
    "docs.nav.mcp": "MCP",
    "docs.nav.cli": "CLI",
    "docs.nav.faq": "FAQ",
    "docs.nav.layers": "Layer",
    "docs.nav.wiki": "Wiki",
    "docs.nav.principles": "Principi",
    "docs.nav.architecture": "Architettura",
    "docs.nav.install": "Installazione",
    "docs.toc.quickstart": "Guida rapida",
    "docs.toc.install": "Installazione",
    "docs.toc.overview": "Panoramica",
    "docs.toc.demo": "Demo",
    "docs.toc.audience": "Per chi è",
    "docs.toc.layers": "Tre layer",
    "docs.toc.cli": "Riferimento CLI",
    "docs.toc.wiki": "Knowledge Wiki",
    "docs.toc.mcp": "Integrazione MCP",
    "docs.toc.languages": "Linguaggi",
    "docs.toc.advanced": "Avanzato",
    "docs.toc.artifacts": "Artefatti",
    "docs.toc.faq": "FAQ",
    "docs.toc.principles": "Principi di design",
    "docs.toc.architecture": "Architettura",
    "docs.quickstart.badge": "Percorso consigliato",
    "docs.quickstart.title": "Guida rapida",
    "docs.quickstart.lead":
      "Becket aiuta Cursor e Claude Code a usare meno token con contesto migliore: indicizzi una volta in locale, poi l'agente può chiamare i tool MCP per snippet verificati invece di leggere file interi.",
    "docs.quickstart.s1": "Installa globalmente: <code>npm install -g becket becket-mcp</code> (oppure <code>npx becket build</code> senza install globale).",
    "docs.quickstart.s2": "Indicizza il repo: <code>cd your-project && becket build</code> (crea <code>.becket/</code>).",
    "docs.quickstart.s3": "Aggiungi <code>becket-mcp</code> alla config MCP (sotto) con <code>BECKET_ROOT</code> sulla root del progetto.",
    "docs.quickstart.s4": "Aggiungi una project rule (sezione MCP) o chiedi all'agente di chiamare <code>get_context</code> sul simbolo che stai modificando.",
    "docs.quickstart.callout":
      "<strong>Prova subito:</strong> clona il repo ed esegui <code>cd demo && npx becket build</code> — una mini codebase pronta da indicizzare.",
    "docs.quickstart.cursorTitle": "Cursor — config MCP",
    "docs.quickstart.claudeTitle": "Claude Code — config MCP",
    "docs.quickstart.claudeNote": "Sostituisci il path con la root del tuo progetto. Claude Code usa lo stesso server MCP di Cursor.",
    "docs.overview.title": "Panoramica",
    "docs.overview.lead":
      "Becket è un layer di intelligenza locale che combina grafo verificato, wiki ancorata al grafo e context assembly con snippet di sorgente reali per agenti AI e sviluppatori.",
    "docs.overview.mcpFirst":
      "Workflow principale: MCP nell'IDE così l'agente può chiamare get_context quando serve — meno token che @-menzionare file interi. La CLI indicizza e ispeziona; non sostituisce l'agente. Aggiungi una project rule perché usi davvero i tool.",
    "docs.demo.title": "Walkthrough demo",
    "docs.demo.desc":
      "Il repository include una cartella demo/ — un mini shop con flussi payment, order e shipping. Usala per provare Becket senza toccare la tua codebase.",
    "docs.demo.outputDesc": "Output di esempio (troncato) — un bundle markdown con wiki, snippet reali e impact:",
    "docs.demo.readme":
      'Comandi completi: vedi <a href="https://github.com/GabrieleRuggieri/becket/tree/main/demo">demo/README.md</a> nel repository.',
    "docs.audience.title": "Per chi è",
    "docs.audience.desc": "Becket è per sviluppatori e team che lavorano su codebase reali con agenti di coding AI.",
    "docs.audience.th.role": "Sei…",
    "docs.audience.th.do": "Fai così",
    "docs.audience.r1": "Utente Cursor / Claude Code",
    "docs.audience.d1": "<code>becket build</code> una volta → config MCP → l'agente chiama <code>get_context</code>",
    "docs.audience.r2": "Sviluppatore (solo CLI)",
    "docs.audience.d2": "<code>becket build</code> → <code>impact</code> / <code>context</code> / <code>flow</code> prima dei refactor",
    "docs.audience.r3": "Dopo un git pull o refactor grosso",
    "docs.audience.d3": "<code>becket build</code> o <code>becket build --watch</code> — cache locale, non committata",
    "docs.audience.r4": "Monorepo / polyrepo",
    "docs.audience.d4": "<code>becket workspace build</code> per il linking cross-service",
    "docs.layers.title": "Tre layer di conoscenza",
    "docs.layers.desc": "Becket indicizza il codebase in locale ed espone tre layer coordinati.",
    "docs.layers.l1": "Core deterministico",
    "docs.layers.l1d": "Simboli, grafo, flussi, impact — da tree-sitter, senza LLM",
    "docs.layers.l2": "Repo Wiki ancorata",
    "docs.layers.l2d": "Pagine markdown ancorate a symbol ID, via MCP, lint-checked",
    "docs.layers.l3": "Context assembly",
    "docs.layers.l3d": "Snippet + wiki + impact, a budget token",
    "docs.cli.title": "Riferimento CLI",
    "docs.cli.intro": "La CLI indicizza il repo e risponde alle query. Eseguila dalla root del repository (o passa --repo).",
    "docs.cli.build.title": "Indicizza il repository",
    "docs.cli.build.desc": "Genera artefatti JSON versionati e pagine wiki ancorate sotto .becket/. Nessun LLM richiesto.",
    "docs.cli.wiki.title": "Knowledge wiki",
    "docs.cli.wiki.desc": "Sync ricompila la struttura wiki dal grafo. Lint verifica le pagine rispetto agli edge live — vedi sezione Wiki.",
    "docs.cli.impact.title": "Interroga l'impatto",
    "docs.cli.impact.desc": "Restituisce moduli affetti, simboli downstream, test correlati e zone a rischio.",
    "docs.cli.flow.title": "Comprendi un flusso",
    "docs.cli.flow.desc": "Mostra il percorso end-to-end e le interazioni tra servizi.",
    "docs.cli.context.title": "Context bundle",
    "docs.cli.context.desc":
      "Modalità task: fix (default, più snippet), refactor (impact più ampio), onboard (panoramica, meno snippet).",
    "docs.cli.domain.title": "Raffinamento domini (opzionale)",
    "docs.cli.domain.desc": "Zero-config di default. Affina i domini auto-scoperti via CLI.",
    "docs.wiki.title": "Repo Wiki ancorata",
    "docs.wiki.desc":
      "Pagine markdown per moduli, servizi e flussi — ciascuna ancorata a symbol ID reali dal grafo deterministico. La struttura è compilata dal grafo; prosa opzionale (intent e gotcha) via MCP sampling.",
    "docs.wiki.lintTitle": "Cosa fa <code>wiki lint</code>?",
    "docs.wiki.lintDesc": "Wiki lint confronta ogni pagina wiki con il call graph live — il grafo è sempre ground truth. Segnala:",
    "docs.wiki.liStale": "<strong>Pagine stale</strong> — il codice ancorato è cambiato (<code>graph_fingerprint</code> mismatch)",
    "docs.wiki.liClaims": "<strong>Claim contraddittori</strong> — la pagina dice che A chiama B, ma il grafo dice altro",
    "docs.wiki.liLinks": "<strong>Link rotti</strong> — la wiki referenzia un simbolo o pagina che non esiste più",
    "docs.wiki.liOrphans": "<strong>Pagine orfane</strong> — non raggiungibili da <code>index.md</code>",
    "docs.wiki.lintCi":
      "wiki lint --strict in CI è un edge case opzionale per team che versionano le pagine wiki in git. Nel workflow usuale — .becket/ locale, agente via MCP, niente commit — non ti serve wiki lint.",
    "docs.wiki.li1": "Pagine via MCP sampling (modello host, nessun LLM bundled)",
    "docs.wiki.li2": "MCP get_wiki con enrich=true compila intent/gotcha e persiste su disco",
    "docs.wiki.li3": "index.md instrada gli agenti alle pagine rilevanti",
    "docs.languages.title": "Linguaggi supportati",
    "docs.languages.desc": "Becket analizza il sorgente con tree-sitter. Supporto built-in oggi:",
    "docs.languages.note":
      "Altre estensioni possono mappare a una grammatica built-in via becket.languages.toml nella root del repo. Vedi CONTRIBUTING.md per aggiungere linguaggi.",
    "docs.mcp.title": "Integrazione MCP",
    "docs.mcp.desc":
      "Becket espone un server MCP su stdio. Cursor e Claude Code elencano i suoi tool insieme agli altri server MCP — l'agente li invoca durante un turno di chat quando servono.",
    "docs.mcp.th.tool": "Tool",
    "docs.mcp.th.desc": "Descrizione",
    "docs.mcp.th.when": "Quando usarlo",
    "docs.mcp.t1": "Wiki + snippet + impact, nel budget",
    "docs.mcp.t1w": "Prima di fix o refactor su un simbolo",
    "docs.mcp.t2": "Analisi d'impatto",
    "docs.mcp.t2w": "Prima di rinominare o eliminare un simbolo",
    "docs.mcp.t3": "Ricostruzione flussi di business",
    "docs.mcp.t3w": "Capire percorsi end-to-end",
    "docs.mcp.t4": "Dipendenze dirette e transitive",
    "docs.mcp.t4w": "Esplorazione dipendenze",
    "docs.mcp.t5": "Pagina markdown ancorata o router index",
    "docs.mcp.t5w": "Onboarding; enrich=true compila intent/gotcha",
    "docs.mcp.sampling":
      "La prosa wiki e i riassunti usano MCP sampling — il modello del tuo agente scrive intent e gotcha. Becket non include LLM, non ha API key e non genera codice — solo slice reali dal disco.",
    "docs.mcp.howTitle": "Come fa l'agente a chiamare get_context?",
    "docs.mcp.howDesc":
      "Becket non si aggancia a ogni keystroke. MCP espone i tool a Cursor o Claude Code; l'agente decide quando chiamarli — come qualsiasi altro tool MCP. Nome e descrizione del tool dicono al modello cosa c'è; invoca get_context quando il task richiede contesto sulla codebase (fix, refactor, ecc.).",
    "docs.mcp.howTip":
      "Per usarlo in modo affidabile, aggiungi una project rule o chiedi esplicitamente — es. “usa Becket get_context su PaymentService prima di modificare”. Senza questo, l'agente può saltare il tool e indovinare dai file aperti.",
    "docs.mcp.rulesTitle": "Cursor — project rule (consigliata)",
    "docs.mcp.rulesNote": "Claude Code: stessa istruzione in CLAUDE.md o nel system prompt di progetto.",
    "docs.mcp.cliNote":
      "<strong>CLI vs MCP:</strong> becket context restituisce lo stesso bundle di get_context. MCP fa recuperare il bundle all'agente in chat; la CLI è per te da terminale. Nessuno dei due gira da solo — MCP serve un turno agente; la CLI un comando tuo.",
    "docs.advanced.title": "Avanzato",
    "docs.advanced.watch.title": "Rebuild incrementale",
    "docs.advanced.watch.desc": "Ri-analizza i file modificati e sincronizza la struttura wiki stale. Eseguilo in un terminale mentre codifichi.",
    "docs.advanced.workspace.title": "Workspace multi-repo",
    "docs.advanced.workspace.desc": "Collega repository via HTTP, gRPC o code. Richiede un manifest becket.workspace.toml.",
    "docs.advanced.git.title": "Tieni <code>.becket/</code> in locale",
    "docs.advanced.git.desc":
      "Aggiungi .becket/ a .gitignore. È una cache locale rigenerata con becket build — non source of truth del team. La wiki dentro è scaffolding per il context assembly, non documentazione da pubblicare. Re-indicizza dopo pull o refactor grossi; usa becket build --watch mentre codifichi se vuoi sempre fresh.",
    "docs.advanced.platform.title": "Supporto piattaforme",
    "docs.advanced.platform.desc":
      "macOS e Linux sono tier-1. I binari Windows sono su GitHub Releases ma tier-2 — segnala problemi su GitHub.",
    "docs.artifacts.title": "Artefatti di output",
    "docs.artifacts.desc": "Output JSON-compatibili, versionati per schema.",
    "docs.artifacts.th.file": "File",
    "docs.artifacts.th.content": "Contenuto",
    "docs.artifacts.a1": "Mappa strutturale ad alto livello",
    "docs.artifacts.a2": "Catalogo simboli con posizioni",
    "docs.artifacts.a3": "Grafo delle dipendenze",
    "docs.artifacts.a4": "Flussi di business ricostruiti",
    "docs.artifacts.a5": "Entry point rilevati",
    "docs.artifacts.a6": "Router wiki / indice",
    "docs.artifacts.a7": "Pagine di conoscenza ancorate ai simboli",
    "docs.faq.title": "FAQ e risoluzione problemi",
    "docs.faq.q1": "Serve Rust installato?",
    "docs.faq.a1":
      "No. npx becket build e npm install -g becket scaricano binari precompilati. Rust serve solo per compilare da sorgente.",
    "docs.faq.q2": "I tool MCP restituiscono \"index missing\"",
    "docs.faq.a2":
      "Esegui becket build nella root del repository prima. MCP legge da .becket/ — senza build non c'è nulla da interrogare.",
    "docs.faq.q3": "Comando becket-mcp non trovato",
    "docs.faq.a3":
      "Installa globalmente: npm install -g becket-mcp. Verifica che la directory bin globale di npm sia nel PATH. Riavvia Cursor/Claude Code dopo l'installazione.",
    "docs.faq.q4": "Posso usare Becket senza Cursor o Claude Code?",
    "docs.faq.a4":
      "Sì — la CLI offre context, impact e flow da terminale. MCP è per le stesse query in Cursor/Claude quando l'agente chiama i tool. Becket non modifica codice né esegue un LLM da solo.",
    "docs.faq.q5": "Becket invia il mio codice al cloud?",
    "docs.faq.a5":
      "No. Tutta l'analisi gira in locale. L'arricchimento prosa wiki opzionale usa il modello del tuo host MCP (Cursor/Claude) — Becket non ha API key e non invia telemetria.",
    "docs.faq.q6": "Quale nome simbolo devo usare?",
    "docs.faq.a6":
      "Usa un nome di funzione, classe o modulo visibile nel sorgente — es. capture, PaymentService, o un nome fully qualified. Esegui becket build prima; i simboli vengono dal grafo indicizzato.",
    "docs.faq.q7": "get_context parte automaticamente a ogni modifica?",
    "docs.faq.a7":
      "No. Cursor e Claude Code espongono Becket come tool MCP; l'agente li chiama quando decide che servono (o quando glielo dicono rule/prompt). Aggiungi una project rule o chiedi esplicitamente. Becket non è un daemon in background nell'IDE.",
    "docs.faq.q8": "Devo committare .becket/ o .becket/wiki/?",
    "docs.faq.a8":
      "No, nel workflow tipico. Tieni .becket/ in .gitignore e fai becket build in locale (o --watch). Committare la wiki serve solo a team di nicchia che vogliono prosa condivisa in git — non serve per risparmiare token o velocizzare l'agente.",
    "docs.principles.title": "Principi di design",
    "docs.principles.p1.title": "Deterministico prima di tutto",
    "docs.principles.p1.desc": "L'analisi core è deterministica. La struttura deriva dal codice.",
    "docs.principles.p2.title": "AI-augmented, non AI-dependent",
    "docs.principles.p2.desc": "L'AI arricchisce via MCP sampling. Il tool funziona anche senza.",
    "docs.principles.p3.title": "Local-first",
    "docs.principles.p3.desc": "Tutta l'analisi gira in locale. Zero telemetria.",
    "docs.principles.p4.title": "Output machine-readable",
    "docs.principles.p4.desc": "JSON-compatibili, schema stabile, artefatti versionati.",
    "docs.principles.p5.title": "Wiki ancorata al grafo",
    "docs.principles.p5.desc": "Le pagine si ancorano a symbol ID e sono lint-checked. Il grafo è ground truth.",
    "docs.arch.title": "Architettura",
    "docs.arch.desc": "Core Rust deterministico, wiki ancorata, context assembly, SQLite, MCP, workspace multi-repo.",
    "docs.arch.l1": "Superfici",
    "docs.arch.l2": "Store",
    "docs.arch.l3": "Core",
    "docs.arch.l4": "Sorgenti",
    "docs.arch.l5": "Assembly",
    "docs.arch.l5d": "Context bundle (snippet + wiki + impact)",
    "docs.arch.l6": "Wiki",
    "docs.arch.l6d": "Markdown ancorato + lint engine",
    "docs.arch.note":
      'Architettura completa: <a href="https://github.com/GabrieleRuggieri/becket/blob/main/ARCHITECTURE.md">ARCHITECTURE.md</a> nel repository.',
    "docs.install.title": "Installazione",
    "docs.install.desc":
      "Becket è gratuito e open source (Apache-2.0). Nessun account, nessun abbonamento, zero telemetria. Serve Node.js/npm per i wrapper di install — Rust solo se compili da sorgente.",
    "docs.install.path1.title": "Cursor / Claude Code",
    "docs.install.path1.desc": "Installa entrambi i pacchetti globalmente così MCP li trova nel PATH.",
    "docs.install.path2.title": "Prova senza install globale",
    "docs.install.path2.desc": "Scarica il binario nativo al primo run. Ideale per un test o la cartella demo.",
    "docs.install.note":
      "<strong>L'ordine conta:</strong> esegui <code>becket build</code> nel repo prima di usare MCP. L'agente interroga <code>.becket/</code> — l'indicizzazione è sempre il primo passo.",
  },
};

const typingLines = {
  en: [
    "$ cd demo && npx becket build",
    "✓ Graph: 21 symbols indexed",
    "✓ Wiki: 4 pages grounded",
    "✓ Flows: services, api",
    "",
    "$ becket context capture --budget 6000",
    "→ wiki page + 3 code snippets",
    "→ 5 modules in impact set",
    "→ ready for AI agent ✓",
  ],
  it: [
    "$ cd demo && npx becket build",
    "✓ Grafo: 21 simboli indicizzati",
    "✓ Wiki: 4 pagine ancorate",
    "✓ Flussi: services, api",
    "",
    "$ becket context capture --budget 6000",
    "→ pagina wiki + 3 snippet di codice",
    "→ 5 moduli nell'impact set",
    "→ pronto per agente AI ✓",
  ],
};

let currentLang = localStorage.getItem("becket-lang");
if (!currentLang) {
  currentLang = (navigator.language || "").startsWith("it") ? "it" : "en";
}

function applyTranslation(el, key, lang) {
  const value = translations[lang]?.[key];
  if (!value) return;
  if (el.hasAttribute("data-i18n-html")) {
    el.innerHTML = value;
  } else {
    el.textContent = value;
  }
}

function setLanguage(lang) {
  currentLang = lang;
  localStorage.setItem("becket-lang", lang);
  document.documentElement.lang = lang;

  document.querySelectorAll("[data-i18n], [data-i18n-html]").forEach((el) => {
    const key = el.getAttribute("data-i18n") || el.getAttribute("data-i18n-html");
    if (key) applyTranslation(el, key, lang);
  });

  const langBtn = document.getElementById("lang-toggle");
  if (langBtn) {
    langBtn.textContent = lang === "it" ? "IT" : "EN";
    langBtn.setAttribute(
      "aria-label",
      lang === "it" ? "Switch to English" : "Passa all'italiano",
    );
  }

  if (typeof window.restartTyping === "function") {
    window.restartTyping();
  }
}

function toggleLanguage() {
  setLanguage(currentLang === "en" ? "it" : "en");
}

document.addEventListener("DOMContentLoaded", () => {
  setLanguage(currentLang);
  const langBtn = document.getElementById("lang-toggle");
  if (langBtn) langBtn.addEventListener("click", toggleLanguage);
});

window.getTypingLines = () => typingLines[currentLang] || typingLines.en;
