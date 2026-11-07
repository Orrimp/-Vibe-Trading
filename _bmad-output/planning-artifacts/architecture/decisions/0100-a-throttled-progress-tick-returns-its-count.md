---
adr: 0100
title: A throttled progress tick says so, and the counter's owner takes the count back
status: accepted
date: 2026-11-07
supersedes: none
superseded-by: none
---

# ADR-0100: A throttled progress tick says so, and the counter's owner takes the count back

## Context

`ActivityHandle::tick(current)` rate-limits the activity channel: if less than
`TICK_THROTTLE = 100 ms` has passed since the last emission it returned early — and it
returned `()`, so the caller could not tell "reported later" from "destroyed".

`activity_audit_aggregator` is such a caller, and it is the worst possible one. At each
window boundary it does `let n = counter.swap(0, Relaxed)` — removing the window's count
from the only place it lives — and then `h.tick(n as u64)`. A throttled call therefore
**destroys** those events' visibility. And `AGGREGATION_WINDOW` is also **100 ms**, exactly
`TICK_THROTTLE`, so whether the throttle has expired at a boundary is settled by scheduler
jitter. Under load it loses, repeatedly.

Found 2026-11-07 from a CI red that looked like a flake and was not. The ubuntu-latest lane
failed `audit_aggregator_handles_10k_event_storm` with
`start_label=760 + ticks=0 = 760 / 5000 = 0.152`, on a commit (`9478ad4e`) that changed only
markdown, yaml and toml, while the identical code was green one commit earlier (`bdc2c12a`,
all three OS lanes). Reproduced locally under ordinary background load: 2 of 3 runs failed at
208/5000 and 505/5000. The measurement that settled it was waiting **twenty seconds** and
recovering **nothing** — the events were not pending, they were gone, one window at a time,
with `tick=0` in every failing run.

This is the third instance of one shape in this repo: a value computed, carried one hop, and
then dropped (bug-log `#141`, a portfolio-breach count; `#153`, a ruin witness; now `#155`).
Here it had a user-visible consequence the others did not: the cockpit's "Audit: N writes"
under-reported real ledger writes whenever the machine was busy, and no gate could see it.

## Decision

`ActivityHandle::tick` returns `bool` — `true` emitted, `false` throttled — and its doc states
that a throttled tick drops `current` rather than deferring it, so a caller that removed
`current` from a counter owns the `false` case. The return is not `#[must_use]`: the ~30 call
sites that tick a value they still hold are correct to ignore it, and forcing them to
acknowledge it would add noise where there is no defect.

`activity_audit_aggregator` handles it by putting the count back:

    (n, Some(h)) => {
        if !h.tick(n as u64) {
            agg.counter.fetch_add(n, Ordering::Relaxed);
        }
    }

The REPORT is coalesced; the MEASUREMENT is not lost. The next boundary ticks the sum. This
also stops the `(0, Some(_))` arm from reading a swapped-out window as idle and closing the
activity handle in the middle of a burst.

This makes ADR-0044 § D2's stated intent — "per-window counts appear on every subsequent
non-empty window" — actually true. It was not true under load.

## Alternatives considered

- **Decouple the periods** (interval 100 ms, throttle 50 ms) — rejected: it lowers the
  collision probability without removing the discard. Whenever they do collide the
  measurement is still destroyed, and the collision is scheduler-dependent, so "unlikely" is
  not a guarantee. Treating the trigger and leaving the defect is how this shape survives.
- **Let `tick` buffer the value internally and emit it on the next call** — rejected: the
  count already has an owner (the aggregator's `AtomicU32`). Two accumulators for one
  quantity is how a count gets double-reported, and the handle would acquire state its other
  ~30 callers neither need nor expect.
- **Peek the counter, and only swap once the throttle has expired** — rejected: a
  read-then-swap is not atomic against the recv arm's `fetch_add`, so events arriving in
  between would be lost. `fetch_add` of a known `n` cannot lose; a racy peek can.
- **Widen `TICK_THROTTLE`** — rejected: the throttle is the activity channel's rate cap and
  is itself asserted (the storm test's assertion 2). Loosening a correct constraint to fix an
  accounting bug trades one defect for another.
- **Lower the storm test's 90 % coverage floor** — rejected outright. The test was right and
  the code was wrong; weakening the assertion that caught a real production bug is the move
  this repo's bug log exists to prevent.

## Consequences

Under back-pressure the operator sees fewer, larger Tick events instead of a stream of small
ones that silently omits most of the work. The cumulative count is now complete.

Mechanically checked by:

- `agent::activity::tests::throttled_tick_reports_that_it_dropped_the_value` — asserts
  `tick()` returns `false` inside the throttle and `true` after it, and that the throttled
  value never reaches the wire. Reverting `tick` to `-> ()` makes this fail to COMPILE, which
  is the loudest available red.
- `ui/tests/activity_tape_audit_ledger_event_storm.rs` — the e2e. Before: 2 of 3 runs failed
  under ordinary load. After: 6 of 6 pass at **100.0 % coverage** under *higher* measured load
  (6.78 vs 3.2–3.6), accounting complete in 106–187 ms against a 20 000 ms deadline.
- That test also stopped waiting a fixed 350 ms and now waits for the accounting condition,
  draining as it goes. The fixed sleep is what made the defect look like a flake; the
  condition-based wait is what proved it was not, because waiting longer changed nothing.

Violated if a future caller of `tick` discards the return value while holding a count it has
already removed from its source.

## Changelog
- 2026-11-07 (orchestrator): initial accept. Bug-log `#155` carries the measurements, including
  the two wrong hypotheses discarded on the way (late broadcast subscription — refuted, the
  `subscribe()` at `spawn_aggregator` is synchronous; and scheduler starvation — refuted by the
  20 s wait recovering nothing).
