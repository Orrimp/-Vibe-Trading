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

---

## Calibration — measured 2026-08-15: caller lists are a LOWER BOUND

CodeGraph is applied and healthy on this repo (801 files · 18,945 nodes · 65,242 edges · 773 Rust +
20 Python; the daemon keeps it synced — `codegraph sync` reported *"Already up to date"* against a
tree edited minutes earlier, and the index contained both a file and a function created that day).
**Freshness is not the caveat. Completeness is.**

**The measurement.** Two calls inside `crates/agent/src/runtime.rs` — `short_exec::plan_open_short`
(`:2275`) and `short_exec::check_and_liquidate` (`:2616`) — are **absent** from `codegraph callers`,
while `grep` finds both. The same commands return correct, complete answers for the sibling call
sites in `crates/backtest/`. Scope was then bounded by experiment:

| probe | result |
|---|---|
| Is the index stale? | **No** — it contains that day's new file (`short_long_friction_parity_forward_e2e.rs`, 45 symbols) and new fn (`dvol_arm_compiled`, `bakeoff/mod.rs:194`). |
| Is `runtime.rs` excluded? | **No** — indexed, 71 symbols. |
| Does *any* caller in `runtime.rs` resolve? | **Yes** — a 4-space-indented call at `:1648` correctly resolves to enclosing fn `run` (`:817`). |
| Is it "very large functions"? | **No** — `run` also spans 800+ lines and resolves fine. |
| What do the two misses share? | Both sit ~24–40 spaces deep, inside nested `async`/closure blocks within `spawn_trading_loop` (`:1993-2692`). |

**The rule that follows:** treat `codegraph callers` as a **lower bound**, not a census. It is
excellent for *finding* call sites (that is its job, and it is far faster than grep at it). It is not
sufficient for *proving absence* — and "this symbol has no production callers" is exactly the claim
this project keeps needing, because the whole declared-vs-executed defect family (#65 → #91) turns on
reachability.

**The near-miss that motivated this note.** Bug-log **#90**'s census concluded that
`check_and_liquidate` has one caller per side — `agent` and `backtest` — and is therefore *symmetric*,
not a repeat of #80's asymmetry. Codegraph alone reports only the `backtest` caller. Had the census
been taken from it, #90 would have read "only the ranking side liquidates," which is **false**, and
the entry would have been wrong in precisely the way **#82** was wrong: asserting reachability without
tracing the caller graph. The grep cross-check is what prevented it.

**Working rule.** Lead with CodeGraph to orient and to find candidates. When a conclusion depends on a
caller set being *complete* — a reachability claim, a dead-code claim, a "no production callers"
claim — confirm with `grep -rn --include='*.rs'` before writing it down. The two tools disagree
exactly where this codebase's defects live: deep inside the async loops that execute real plans.

## Correction (2026-09-14) — the miss pattern is SYNTAX, not depth

The calibration above is right about the **rule** (caller lists are a lower bound; grep before any
completeness claim) and wrong about the **cause**. It attributed the two `runtime.rs` misses to calls
"~24–40 spaces deep inside nested `async`/closure blocks". Re-measured against `grep` across 11 call
sites, depth predicts nothing — a call at indent **28** resolves, calls at indent **4** are missed.
Two syntactic patterns account for every observed miss:

| pattern | example | codegraph |
|---|---|---|
| **callee named via a path that crosses a crate boundary** | `risk::size_portfolio_target(` — `agent/tests/v1_rebalance_reject.rs:88`, indent **4** | **missed** |
| … including a module imported from another crate | `short_exec::plan_open_short(` in `agent` (via `use backtest::short_exec;`) | **missed** |
| the same qualified call from a same-crate module | `short_exec::plan_open_short(` in `backtest/src/scenarios/sma_composed_run.rs`, indent **28** | found |
| a bare name brought in with `use`, even cross-crate | `spawn_aggregator(` in `ui/tests` (agent -> ui) | found |
| **any call inside a macro invocation** | `window.file_slug()` inside `format!(…)` — `reports/src/lib.rs:418`; `.slug()` inside `assert_eq!` | **missed** |
| the same method call outside a macro | `self.slug()`, `period.slug()` | found |

The original two misses (`short_exec::plan_open_short`, `short_exec::check_and_liquidate` in
`runtime.rs`) are both the cross-crate-path pattern — they happened to also be deep, which is the
confound. The `backtest` sibling calls that "resolved correctly" are same-crate calls.

**Why this is worse than a depth problem would be:** the missed form is where production callers
live. Observed misses include the ONLY production caller of `size_portfolio_target`
(`backtest/src/scenarios/montecarlo.rs:520`) — so codegraph reports it as having no production
caller, which is exactly bug-log #69's pre-fix state — plus `spawn_aggregator`'s
(`ui/src/bin/cockpit_live.rs:648`), `file_slug`'s (`reports/src/lib.rs:418`) and `run_path`'s bin
(`backtest/src/bin/param_robustness_sweep.rs:894`). A text census finds **~1,400** call sites of the
cross-crate-path form (**~580 under `src/`**), concentrated in `agent/src/runtime.rs` (65) and
`ui/src/bin/cockpit_live.rs` (63) — the code that executes plans. (Regex estimate; the pattern was
verified on 8 sampled sites, not all 1,400.)

Also observed: `codegraph_explore` (MCP) annotates `file_slug` with "no covering tests found" while
listing a test as its only caller, and a four-term query spent its output budget on an unrelated
`scripts/spec_lint.py` class. Treat its coverage and relevance hints as leads, not facts.

**The working rule above stands unchanged** — orient with CodeGraph, confirm completeness with grep.
What changes is that no call site is safe to trust on depth grounds: shallow, qualified calls are
missed too.
