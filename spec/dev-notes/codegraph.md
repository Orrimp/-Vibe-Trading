# CodeGraph — code-knowledge-graph for AI agents (dev tooling)

[CodeGraph](https://github.com/colbymchenry/codegraph) is a pre-indexed knowledge
graph of this codebase (symbols, call paths, blast radius) that lets an AI coding
agent get **one-call** answers — "who calls X", "what breaks if I change Y", "show
me the relevant symbols + source for area Z" — instead of grepping and reading files
one at a time. It is a **developer/agent-navigation tool only**: it is NOT a Cargo
dependency, is NOT part of the trading product or runtime, and has **zero** effect on
builds, the test suite, the `verify_anchors` gate, or any shipped artifact.

Full Rust support (`.rs`, tree-sitter); this repo indexes at **723 files / 16,128
nodes / 54,680 edges** (6,381 functions, 656 structs, 252 enums, 26 traits).

## Setup (one-time, per machine)

```bash
# 1. Install the CLI (self-contained — bundles its own Node runtime).
npm i -g @colbymchenry/codegraph        # requires Node 22.5+ for the lib path; CLI bundles its own

# 2. Build the graph for this repo (creates .codegraph/, ~46 MB SQLite DB).
cd <repo> && codegraph init             # auto-sync is on by default; incremental thereafter
```

`.codegraph/` is a **local, per-machine artifact** — gitignored (root `.gitignore`),
never committed, rebuilt with `codegraph init`. It respects the repo `.gitignore`.

## CLI usage

```bash
codegraph status                 # index stats (files / nodes / edges by kind)
codegraph query <search>         # fuzzy symbol search
codegraph explore <query...>     # relevant symbols' source + call paths, one shot
codegraph node <name>            # one symbol's source + caller/callee trail
codegraph callers <symbol>       # who calls this function/method
codegraph callees <symbol>       # what this symbol calls
codegraph impact <symbol>        # blast radius — what a change to <symbol> affects
codegraph sync                   # manual incremental re-index (usually automatic)
```

Real examples on this codebase:

```text
$ codegraph impact rank_candidates
  Impact of changing "rank_candidates" — 27 affected symbols
  crates/backtest/src/bakeoff/rank.rs: rank_candidates, t62..t66 tests, …

$ codegraph callers classify_verdict
  Callers of "classify_verdict" (17): …
```

## Optional: wire it as an MCP server for Claude Code (OPT-IN — you choose)

This makes the `codegraph_*` tools callable by the agent directly (fewer grep/read
round-trips). It **registers a startup MCP server in the agent's config**, so it is
deliberately left as an explicit opt-in — it is NOT applied automatically by this repo.

**Project-scoped** (travels with the repo; prompts each user to trust it on open) —
create `.mcp.json` at the repo root:

```json
{
  "mcpServers": {
    "codegraph": { "type": "stdio", "command": "codegraph", "args": ["serve", "--mcp"] }
  }
}
```

**Global** (this machine only, all repos):

```bash
codegraph install --target claude --location global    # writes ~/.claude.json
# inspect first without writing:  codegraph install --print-config claude
```

Either way it takes effect on the **next** agent session (MCP servers load at start),
and Claude Code prompts to approve the server before it runs. Prerequisite: the
`codegraph` CLI must be installed (step 1 above) or the server fails to start
(harmlessly).

## Notes

- Scope `codegraph install` with `--target claude` so it does not touch other agents
  (Cursor/Codex/Gemini/etc.) it can auto-detect.
- The graph also picked up the 7 `scripts/*.py` and 1 Swift file — harmless; the
  723-file index is overwhelmingly the Rust tree.
