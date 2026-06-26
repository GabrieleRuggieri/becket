# Becket demo — mini shop

Small sample codebase to try Becket without touching your own project. It models a payment → order → shipping flow across Rust services, plus a Python webhook handler.

## Try it (from this folder)

```bash
# Index the demo (downloads the CLI binary via npm on first run)
npx becket build

# Inspect impact before a change
becket impact capture

# Get an agent-ready context bundle
becket context capture --budget 6000 --task fix

# Trace a business flow
becket flow services

# Check wiki pages against the live graph
becket wiki lint
```

## Use with Cursor / Claude Code

1. Open this `demo/` folder as your workspace (or run `becket build` here from your real repo root).
2. Install MCP globally: `npm install -g becket-mcp`
3. Add `becket-mcp` to your MCP config with `BECKET_ROOT` pointing at this directory.
4. Ask your agent to call `get_context` on `capture` before editing payment code.

See [website docs](../website/docs.html) for full setup.

## What to expect

After `becket build`, Becket writes `.becket/` with JSON artifacts and grounded wiki pages. The `capture` function in `src/services/payments.rs` is a good starting symbol — it sits in the middle of the payment flow.
