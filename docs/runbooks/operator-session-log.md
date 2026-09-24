# Runbook — The operator session log

**Created:** 2026-09-24 (story 6-11). **Scope:** the cockpit's record of what it did.
Paper/sim only, like everything else here.

## Why this exists

Five defects in this repo's bug log shipped past a green test suite (#65–#69). The
common thread is not that the tests were weak in some general way — it is that
**nothing independent of the tests recorded what the software actually did**.

The canonical case is `#66` A.4. `report::sma` emitted its `strategy:` frontmatter keys
unindented; `compare::cache::parse_frontmatter` filed them top-level; and
`scan_one_root` therefore skipped **every engine-written report**. The Compare screen
rendered perfectly and showed an empty matrix.

An empty Compare matrix is also what a fresh checkout shows. Nothing in the product
could tell those two states apart, and the defect survived for weeks until a code review
found it by reading. This log is the thing that can tell them apart.

## What it records

The unit is **an operator-visible action and its outcome, with a denominator** — not a
tracing line.

| Event | Carries |
|---|---|
| `session_started` | which binary opened the session |
| `screen_opened` | the screen name |
| `run_started` | kind, and the inputs that define the run: strategy, pair, range, source |
| `run_finished` | kind, whether it produced a result, and the error if not |
| `scanned` | what was scanned, how many candidates were **discovered**, how many **admitted**, and every rejection **by named reason** |
| `artifact_written` | kind and path |

`scanned` is the one that matters. `0 admitted of 0 discovered` means "there was nothing
here". `0 admitted of 40 discovered, all `missing_strategy_id`" means "there was plenty
here and I used none of it, and here is why". Those are different sentences, and until
this story the product could only say the first.

The rejection vocabulary is a **closed set** (`session_log::Rejected`). An open string
would let a future skip-path be added without anyone deciding it deserves a name, and an
unnamed skip path is exactly how `#66` A.4 stayed invisible: the scanner had seven bare
`continue`s and no way to distinguish them.

## What it does NOT record, and cannot

No network call of any kind. No telemetry service, no analytics, no crash reporting, no
identifier. Nothing you type into a field, no price, no money figure. One append-only
JSONL file on your own machine, readable with `cat`.

This product's promise is offline honesty, and this log must not dent it.

## Where it lives

`$XDG_STATE_HOME/trading/sessions/`, defaulting to `~/.local/state/trading/sessions/`.
State, not config — these are records of what happened, not settings, and XDG puts them
in different places. The directory is **outside the repository**, so there is no
`.gitignore` rule to forget.

A `README.md` is written beside the logs on first use, saying the same things this
section does, because the person most likely to wonder what those files are is the
person looking at them.

```bash
ls -la ~/.local/state/trading/sessions/
```

```bash
jq -c 'select(.event == "scanned")' ~/.local/state/trading/sessions/session-*.jsonl
```

## Retention

The newest **30** sessions are kept; older files are deleted at the next boot. At a few
KB per session that is a few weeks of ordinary use. The cap exists so the directory
cannot grow without bound on a machine nobody prunes — it is not a privacy control,
because nothing leaves the machine in the first place. Delete any of them at any time;
nothing reads them back.

## Turning it off

```bash
TRADING_SESSION_LOG=0 cargo run -p ui --bin cockpit_live
```

Off costs nothing structurally, not just by a branch: the sink is an `Option` on the
`Cockpit` (the same switch shape as `lab_state_path`, ADR-0094 D3). Every fixture,
gallery and test leaves it `None`, so they cannot write to your real log, and only
`Cockpit::boot` arms it.

If the file cannot be opened — read-only home, full disk, permissions — the cockpit logs
one `warn!` and runs unlogged. A cockpit that refused to start because it could not open
its own diary would be a worse bug than the one this exists to catch.

## The gate

`crates/ui/tests/session_log_replays_66_a4.rs` is the story's real acceptance test. It
builds a corpus of valid reports the scanner admits none of, and asserts that its record
**differs** from an empty corpus's. A log that reports "0 cells" for both is precisely
the log that would not have caught `#66` A.4, and that test fails on it.

One honest note on the replay: `#66` A.4's literal trigger — unindented frontmatter — no
longer reproduces, because its fix was a tolerant-reader change to `parse_frontmatter`
that now accepts that shape (its own regression test covers that). The gate therefore
reconstructs A.4's **outcome** by a different route. Re-breaking the parser to get a more
literal replay would test the parser, not the log.
