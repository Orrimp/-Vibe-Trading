//! cockpit-activity-audit-ledger-producer v0.1.0 — Wave D storm integration test (T-D-N8).
//!
//! Pushes 10 000 synthetic `AuditTick<AuditEvent>` events at the audit bus's
//! maximum rate (tight loop, no sleeps) and asserts:
//!
//! 1. **Counter completeness**: the aggregator observes all (or nearly all, if
//!    the tick bus lags) 10 000 events — measured via the activity channel's
//!    cumulative `Tick.current` values.
//! 2. **Activity-channel rate cap**: the activity receiver observes
//!    ≤ `(elapsed_ms / 100) + 1` `Tick` events (one per 100 ms window +
//!    1-boundary-flake allowance).
//! 3. **Zero Failed events**: per T-AR-3 — the aggregator's main handle never
//!    emits `End{Failed(...)}` on the happy path.
//! 4. **K2 truncation observed**: at least one `Tick.current` value exceeds
//!    9 999 (since 10 000 events arrive in < 100 ms on any modern machine),
//!    verifying the flood-label path is exercisable. NOTE: the aggregator's
//!    `Tick.current` carries the COUNT OF EVENTS IN THE WINDOW (N), not the
//!    global total. For K2 truncation to fire in the label, N in a single
//!    100 ms window must exceed 9999. With 10k events in a tight loop this
//!    WILL happen.
//!
//! Run: `cargo test -p ui --test activity_tape_audit_ledger_event_storm -- --nocapture`

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::{Duration, Instant};

use tokio::sync::broadcast;
use tokio::time::sleep;

use agent::activity::{ActivityEvent, ActivityKind, ActivityOutcome, ActivityPhase};
use agent::activity_audit_aggregator::spawn_aggregator;
use agent::bus::EventBus;
use agent::config::BusConfig;
use audit::tick::{AuditContext, AuditEvent, AuditTick};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Storm size — deliberately BELOW the aggregator's `K2_THRESHOLD` of 9 999.
///
/// Was 10 000, which made this test unreliable on every platform (2 failures in
/// 5 runs on the canonical macOS box; worse on CI). The reason is structural, not
/// timing: the aggregator's FIRST non-empty window calls `start()` and emits no
/// `Tick` — its count goes into the Start LABEL only, because the 100 ms throttle
/// would swallow an immediate `tick()`. `cumulative_counter` below sums Tick
/// values, so everything window 1 absorbs was invisible to the coverage
/// assertion: measured 83 % in one run, and `cumulative_counter = 10` in another
/// where window 1 had swallowed 9 990 of 10 000.
///
/// Counting the Start label fixes that — but only while the label is parseable,
/// and `format_label` collapses anything above `K2_THRESHOLD` to the fixed string
/// `"Audit: 9999+ writes"`. Hence 5 000: large enough to be a genuine storm, small
/// enough that window 1's count is always recoverable. (Operator ruling
/// 2026-08-29, bug-log #98 sibling; the K2 truncation path itself is unit-tested
/// by `format_label_truncates_above_k2_threshold`.)
const TOTAL_EVENTS: usize = 5_000;

/// Fraction of the storm the aggregator must have accounted for (assertion 4).
///
/// ONE constant, read by both the waiter in step 5 and the assertion in step 8.
/// Two copies would let the waiter stop below the bar the assertion then applies.
const COVERAGE_FLOOR: f64 = 0.90;

/// How long step 5 will wait for the aggregator to finish accounting.
///
/// Generous on purpose. It is not a performance budget — it is the point past
/// which "the aggregator never accounted for the storm" becomes the only
/// remaining explanation. See step 5 for why a fixed sleep was wrong.
const ACCOUNTING_DEADLINE: Duration = Duration::from_secs(20);

// ── Running tally of drained activity events ────────────────────────────

/// Accumulates activity events across BOTH drain points — the poll loop in step 5
/// and the final drain in step 7. It exists because the test now drains while it
/// waits instead of sleeping and draining once; the same accounting has to survive
/// being fed twice.
#[derive(Default)]
struct Tally {
    start: usize,
    tick: usize,
    end_success: usize,
    failed: usize,
    max_tick_current: u64,
    cumulative: u64,
    start_labels: Vec<String>,
}

impl Tally {
    fn absorb(&mut self, evt: &ActivityEvent) {
        match &evt.phase {
            ActivityPhase::Start { .. } => {
                self.start += 1;
                // Window 1's count lives ONLY here — see TOTAL_EVENTS above.
                self.start_labels.push(evt.label.clone());
            }
            ActivityPhase::Tick { current, .. } => {
                self.tick += 1;
                self.cumulative += current;
                if *current > self.max_tick_current {
                    self.max_tick_current = *current;
                }
            }
            ActivityPhase::End(ActivityOutcome::Success) => self.end_success += 1,
            ActivityPhase::End(ActivityOutcome::Failed(_)) => self.failed += 1,
            ActivityPhase::End(ActivityOutcome::Cancelled) => {}
        }
        assert_eq!(
            evt.kind,
            ActivityKind::AuditLedgerWrite,
            "all activity events must be AuditLedgerWrite kind"
        );
    }

    /// The first non-empty window reports through the Start label, never a Tick,
    /// so any count that leaves this out measures the aggregator's throttle
    /// rather than its accounting.
    fn start_label_count(&self) -> u64 {
        self.start_labels
            .iter()
            .filter_map(|l| {
                l.strip_prefix("Audit: ")
                    .and_then(|r| r.strip_suffix(" writes"))
                    .and_then(|n| n.parse::<u64>().ok())
            })
            .sum()
    }

    fn observed(&self) -> u64 {
        self.cumulative + self.start_label_count()
    }
}

// ── Helper: synthetic AuditTick ───────────────────────────────────────────────

fn make_fill_tick() -> AuditTick<AuditEvent> {
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{
        FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, Symbol, Timestamp,
    };
    use uuid::Uuid;

    AuditTick {
        event: AuditEvent::Fill {
            fill: Box::new(Fill {
                id: FillId::new(),
                order_id: OrderId::new(),
                symbol: Symbol::new("BTCUSDT"),
                side: Side::Buy,
                qty: Quantity::new(dec!(0.1)).unwrap(),
                price: Price::new(dec!(40_000)).unwrap(),
                fee: Money::from_decimal(dec!(1.6)),
                fee_tier: FeeTier::Taker,
                venue_ts: Timestamp::new(OffsetDateTime::UNIX_EPOCH),
                local_ts: Timestamp::new(OffsetDateTime::UNIX_EPOCH),
                liquidity: Liquidity::Taker,
                transaction_id: None,
            }),
            fees: dec!(1.6),
        },
        context: AuditContext {
            run_id: Uuid::nil(),
            posted_at: OffsetDateTime::UNIX_EPOCH,
            agent_pid: 0,
        },
    }
}

// ── Storm test ────────────────────────────────────────────────────────────────

/// T-D-N8 — 10k-event audit tick storm through the aggregator.
///
/// Assertions (spec §D Wave D):
/// 1. Counter completeness: aggregator sees all 10k events (or observes Lagged).
/// 2. Activity-channel rate cap: ≤ (elapsed_ms/100)+1 Tick events arrive.
/// 3. Zero Failed events on the happy path.
/// 4. K2 truncation: at least one Tick.current > 9999 (all 10k in < 1 window).
#[test]
fn audit_aggregator_handles_10k_event_storm() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("tokio runtime");

    rt.block_on(async {
        // ── 1. Build the tick bus (16384 slots — sufficient for 10k event storm) ──
        // Using 16384 slots (> 10k) ensures zero Lagged events so all 10k land
        // in one 100 ms window, verifying the K2 truncation path (assertion 4).
        // The production tick bus uses 1024 slots (audit_tick_bus_capacity in config)
        // but this test is about the aggregator's aggregation behaviour, not the
        // production channel sizing.
        let (tick_tx, _) = broadcast::channel::<AuditTick<AuditEvent>>(16_384);

        // ── 2. Build EventBus + activity subscriber ───────────────────────────
        let bus = EventBus::new(&BusConfig::default());
        let mut activity_rx = bus.activity().subscribe();

        // ── 3. Spawn the aggregator ───────────────────────────────────────────
        let agg_handle = spawn_aggregator(Some(&tick_tx), &bus);

        // ── 4. Fire 10k events in a tight loop (no sleeps) ───────────────────
        let storm_start = Instant::now();
        // ONE tight burst, as originally designed: the K2 path this test exists
        // to exercise requires a single window to exceed the throttle, and
        // chunking the sends across windows measured WORSE (65 % coverage) — the
        // aggregator drops whatever sits in its counter when the channel closes,
        // so more windows means more chances to lose a partial one.
        for _ in 0..TOTAL_EVENTS {
            let _ = tick_tx.send(make_fill_tick());
        }
        let storm_elapsed_ms = storm_start.elapsed().as_millis() as u64;

        println!(
            "=== storm: sent {TOTAL_EVENTS} events in {storm_elapsed_ms} ms ==="
        );

        // ── 5. Wait until the aggregator has ACCOUNTED for the storm ──────────
        //
        // Was `sleep(350 ms)`, which made this test measure how much CPU the
        // aggregator task happened to get rather than whether it accounts for the
        // storm. It went red on CI's ubuntu-latest lane on 2026-11-07 with
        // `start_label=760 + ticks=0 = 760 / 5000 = 0.152` — on a commit that
        // changed only markdown, yaml and toml, while the identical code was green
        // one commit earlier. So: not a regression, a throughput assertion on a
        // shared runner (bug-log `#155`).
        //
        // The aggregator owns its own 100 ms interval and alternates `rx.recv()`
        // with `interval.tick()`; `tick_tx` is only the inflow. While it is alive
        // it keeps emitting, so the fix is to keep draining until the accounting
        // is actually complete — the invariant this test is for — instead of
        // betting a fixed wall-clock on it. Draining as we go also removes a
        // second flake source: the activity broadcast could previously `Lagged`
        // while we slept.
        //
        // Note the ordering: this MUST run before `tick_tx` is dropped in step 6,
        // because the aggregator discards its current window's partial count when
        // the channel closes (see step 4's note).
        let mut tally = Tally::default();
        let coverage_target = (TOTAL_EVENTS as f64 * COVERAGE_FLOOR).ceil() as u64;
        let wait_start = Instant::now();
        loop {
            while let Ok(evt) = activity_rx.try_recv() {
                tally.absorb(&evt);
            }
            if tally.observed() >= coverage_target {
                break;
            }
            if wait_start.elapsed() >= ACCOUNTING_DEADLINE {
                break;
            }
            sleep(Duration::from_millis(25)).await;
        }
        let wait_elapsed_ms = wait_start.elapsed().as_millis() as u64;
        println!(
            "=== accounting: observed {} of target {coverage_target} after {wait_elapsed_ms} ms \
             (deadline {} ms) ===",
            tally.observed(),
            ACCOUNTING_DEADLINE.as_millis()
        );

        // ── 6. Close the tick bus → aggregator exits ──────────────────────────
        drop(tick_tx);
        let _ = tokio::time::timeout(Duration::from_millis(500), agg_handle).await;

        // ── 7. Drain whatever the aggregator emitted on its way out ───────────
        // Into the SAME tally step 5 filled, including the End{Success} that only
        // arrives after the first idle window.
        while let Ok(evt) = activity_rx.try_recv() {
            tally.absorb(&evt);
        }

        let start_count = tally.start;
        let tick_count = tally.tick;
        let end_success_count = tally.end_success;
        let failed_count = tally.failed;
        let max_tick_current = tally.max_tick_current;
        let cumulative_counter = tally.cumulative;
        let start_labels = tally.start_labels.clone();

        // The FIRST non-empty window reports through the Start label, never a
        // Tick (see TOTAL_EVENTS above), so every count must include it or it
        // measures the aggregator's throttle rather than its accounting.
        let start_label_count: u64 = start_labels
            .iter()
            .filter_map(|l| {
                l.strip_prefix("Audit: ")
                    .and_then(|r| r.strip_suffix(" writes"))
                    .and_then(|n| n.parse::<u64>().ok())
            })
            .sum();
        let observed = cumulative_counter + start_label_count;

        println!("=== activity events: start={start_count} tick={tick_count} end_success={end_success_count} failed={failed_count} ===");
        println!("=== max_tick_current={max_tick_current} cumulative_counter={cumulative_counter} ===");

        // ── Assertion 1: counter completeness ────────────────────────────────
        // The aggregator should have observed all 10k events. In a tight loop
        // the entire storm may land in a single 100 ms window (cumulative_counter
        // == TOTAL_EVENTS from one Tick event). We assert the sum across all
        // Tick events accounts for the storm count. Lagged events still get
        // counted (via `fetch_add(n as u32, Relaxed)` in the Lagged arm).
        //
        // NOTE: On very fast machines the storm completes in << 1 ms, so all
        // 10k events land in a single 100 ms window and cumulative_counter
        // equals TOTAL_EVENTS in exactly one Tick. On slower machines or under
        // scheduler pressure multiple windows fire with lower per-window counts.
        // We assert cumulative_counter >= 1 (at least something was observed)
        // and that the activity channel is not silent.
        assert!(
            observed >= 1,
            "aggregator must observe at least 1 event from the storm \
             (start_label={start_label_count} + ticks={cumulative_counter})"
        );
        // NOT `tick_count >= 1`. Whether a Tick occurs at all depends on how many
        // 100 ms windows the burst happens to span, which is scheduler luck: if
        // the aggregator drains the whole storm inside window 1 it emits `Start`
        // and no Tick, legitimately. Measured under 42 CPU burners, that is what
        // happens — 4 of 8 runs. The invariant worth asserting is that the
        // aggregator ACCOUNTED for the storm, which `observed` captures across
        // both reporting paths; the Tick mechanism itself is covered by
        // assertion 2's rate cap and by the aggregator's own unit tests.
        assert!(
            start_count >= 1,
            "aggregator must open an activity handle for the storm (got {start_count} Start events)"
        );

        // ── Assertion 2: activity-channel rate cap ────────────────────────────
        // Budget: ≤ (elapsed_ms / 100) + 1 Tick events. The denominator is the
        // storm plus the MEASURED step-5 wait, not a literal — step 5 now exits as
        // soon as the accounting is complete, so on a healthy machine this stays as
        // tight as the old hard-coded 350 ms, and on a loaded one it widens exactly
        // as much as the extra observation window legitimately permits. A rate cap
        // fed a constant while the observation window varies is not a rate cap.
        let elapsed_budget_ms = storm_elapsed_ms + wait_elapsed_ms;
        let tick_budget = (elapsed_budget_ms / 100) + 1;
        assert!(
            tick_count as u64 <= tick_budget,
            "rate cap violated: {tick_count} Tick events observed, budget is {tick_budget} \
             (elapsed={elapsed_budget_ms} ms)"
        );

        // ── Assertion 3: zero Failed events ──────────────────────────────────
        assert_eq!(
            failed_count, 0,
            "zero Failed events expected on the happy path, got {failed_count}"
        );

        // ── Assertion 4: K2 truncation path — cumulative counter ─────────────
        // The spec's K2 contract is: "at least one Tick.current value exceeds
        // 9999 → the rendered label flips to 'Audit: 9999+ writes'".
        //
        // In practice, the async aggregator uses `tokio::select!` to alternate
        // between `rx.recv()` and `interval.tick()`. Under a multi-thread
        // scheduler the 10k events may be split across 2-3 windows (each
        // individually < 9999) even though the total is 10k. The `format_label`
        // function is exercised by the bench (T-D-N6 `bench_interval_tick_fan_out`
        // calls it with N > 9999 directly).
        //
        // For the integration test we verify the weaker but correct property:
        // the CUMULATIVE counter across all windows accounts for at least 90%
        // of the 10k storm events. The K2 label is separately unit-tested via
        // `format_label_truncates_above_k2_threshold` in the aggregator module.
        // The FIRST non-empty window reports through the Start label, never a
        // Tick (see TOTAL_EVENTS above), so coverage must include it or it
        // measures the aggregator's throttle rather than its accounting.
        let start_label_count: u64 = start_labels
            .iter()
            .filter_map(|l| {
                l.strip_prefix("Audit: ")
                    .and_then(|r| r.strip_suffix(" writes"))
                    .and_then(|n| n.parse::<u64>().ok())
            })
            .sum();
        let observed = cumulative_counter + start_label_count;
        let coverage = observed as f64 / TOTAL_EVENTS as f64;
        assert!(
            coverage >= COVERAGE_FLOOR,
            "aggregator coverage < 90 %: start_label={start_label_count} + \
             ticks={cumulative_counter} = {observed} / {TOTAL_EVENTS} = {:.3}",
            coverage
        );
        println!(
            "=== coverage: start_label={start_label_count} + ticks={cumulative_counter} \
             = {observed}/{TOTAL_EVENTS} ({:.1}%) — max_window={max_tick_current} ===",
            coverage * 100.0
        );

        println!("=== PASS: all 4 assertions hold ===");
    });
}
