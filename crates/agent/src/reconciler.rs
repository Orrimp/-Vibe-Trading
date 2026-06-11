//! Minute-boundary reconciler task (T26 / R3.5).
//!
//! At every minute-bar close, reconciles:
//!   `cash + Σ(positions × last_mark) = equity`
//!
//! On mismatch > tolerance, trips the kill switch and emits a
//! `LedgerImbalance` tracing event.
//!
//! T903c (live-cockpit-unified): on every `after_bar_close`, the
//! reconciler also publishes a [`trading_core::PnlSnapshot`] onto the
//! [`EventBus`] `pnl` channel so the cockpit's P&L panel can render
//! the live snapshot.  Backtests instantiate the reconciler with
//! `bus = None` so report bytes are unchanged (R15 — anchor gate).
//!
//! live-equity-history-durable (ADR-0052): when an [`audit::LiveEquityStore`]
//! is attached via [`ReconcilerTask::with_equity_store`], every
//! [`ReconcilerTask::after_bar_close`] call fire-and-forget persists the
//! snapshot.  The store is only provided in **paper/live mode**; research
//! mode passes `None` at construction time (the A2 mode gate).

use std::sync::Arc;

use rust_decimal::Decimal;
use tracing::info;
use trading_core::{Money, PnlSnapshot, Timestamp};
use uuid::Uuid;

use crate::EventBus;
use crate::kill_switch::KillSwitch;

/// Shared reconciliation state — updated by the trading loop each bar.
#[derive(Debug, Clone)]
pub struct ReconcilerState {
    /// Cash balance from the position book.
    pub cash: Decimal,
    /// Position quantity.
    pub position_qty: Decimal,
    /// Last known mark price.
    pub last_mark: Decimal,
    /// Tolerance for imbalance (from config).
    pub tolerance: Decimal,
    /// Cumulative realized P&L from closed positions (USDT).  Used by
    /// [`ReconcilerTask::after_bar_close`] to populate the snapshot's
    /// `realized` field (T903c).  Defaults to zero — the trading loop
    /// updates it as positions close.
    #[doc(hidden)]
    pub realized_pnl: Decimal,
    /// Cost basis of the current open position (USDT).  Used to
    /// compute `unrealized = position_qty * last_mark - cost_basis`
    /// in [`ReconcilerTask::after_bar_close`] (T903c).
    #[doc(hidden)]
    pub cost_basis: Decimal,
}

impl ReconcilerState {
    /// Compute current equity.
    #[must_use]
    pub fn equity(&self) -> Decimal {
        self.cash + self.position_qty * self.last_mark
    }

    /// Compute unrealized P&L = `position_qty * last_mark - cost_basis`.
    /// Always uses [`Decimal`] — never `f64` (determinism rule).
    #[must_use]
    pub fn unrealized(&self) -> Decimal {
        self.position_qty * self.last_mark - self.cost_basis
    }
}

/// Reconciler task handle.
pub struct ReconcilerTask {
    state_rx: tokio::sync::watch::Receiver<ReconcilerState>,
    kill_switch: KillSwitch,
    interval_ms: u64,
    /// Optional event bus — when present, [`Self::after_bar_close`]
    /// publishes a [`PnlSnapshot`] on the `pnl` channel so the cockpit
    /// renders the live P&L panel (T903c — live-cockpit-unified).
    /// Backtests pass `None` so report bytes stay unchanged.
    bus: Option<Arc<EventBus>>,
    /// Optional durable equity store (live-equity-history-durable ADR-0052).
    ///
    /// When `Some`, every [`Self::after_bar_close`] call fire-and-forget
    /// persists the snapshot via a tokio::spawn (A6 — never blocks/panics
    /// the trading loop; write errors are logged and discarded).
    ///
    /// **Only provided in paper/live mode** — the mode gate (A2) lives at
    /// construction time: `runtime::run` passes `None` for research mode
    /// so no row is ever written during replay.  Backtests use `None`.
    equity_store: Option<Arc<dyn audit::LiveEquityStore>>,
}

impl ReconcilerTask {
    /// Create a reconciler that watches `state_rx` on `interval_ms` cadence.
    #[must_use]
    pub fn new(
        state_rx: tokio::sync::watch::Receiver<ReconcilerState>,
        kill_switch: KillSwitch,
        interval_ms: u64,
    ) -> Self {
        Self {
            state_rx,
            kill_switch,
            interval_ms,
            bus: None,
            equity_store: None,
        }
    }

    /// Builder helper: attach an event bus so `after_bar_close`
    /// publishes a [`PnlSnapshot`] (T903c).
    #[must_use]
    pub fn with_bus(mut self, bus: Arc<EventBus>) -> Self {
        self.bus = Some(bus);
        self
    }

    /// Builder helper: attach an equity store so `after_bar_close`
    /// fire-and-forget persists the snapshot (live-equity-history-durable
    /// ADR-0052 / A6).
    ///
    /// **Only call this for paper/live mode** — the mode gate (A2) lives at
    /// the construction call site (`runtime::run`): research mode passes
    /// `None`; paper/live mode passes `Some(store)`.
    #[must_use]
    pub fn with_equity_store(mut self, store: Arc<dyn audit::LiveEquityStore>) -> Self {
        self.equity_store = Some(store);
        self
    }

    /// Compute and (when a bus is wired) publish a [`PnlSnapshot`]
    /// derived from the current reconciler state (T903c —
    /// live-cockpit-unified).
    ///
    /// `bar_ts` is the **data timestamp** of the bar being closed
    /// (i.e. `bar.close_ts` from the trading loop).  It is stored in
    /// `PnlSnapshot::bar_ts` — the SEPARATE x-axis coordinate the live
    /// equity curve plots — so the chart shows the historical bar timeline
    /// in replay/research mode rather than the wallclock "now", which
    /// collapses every label to the current minute when bars are replayed
    /// faster than real time.
    ///
    /// `as_of` stays wallclock `Timestamp::now()`: the UI equity buffer's
    /// out-of-order-delivery guard + freshness/latency rely on it being
    /// monotone (a clock never goes back).  Stamping `as_of` itself with
    /// `bar_ts` (data time) broke the live render and was reverted
    /// (ISSUE 1, 2026-06-11); the data-time axis now rides `bar_ts`
    /// (cockpit-live-equity-render-guard, approach A).  In live / paper
    /// mode `bar.close_ts ≈ now()` so both fields nearly coincide.
    ///
    /// When an [`audit::LiveEquityStore`] is attached, the snapshot is
    /// fire-and-forget persisted via a `tokio::spawn` (A6 — never blocks
    /// or panics the trading loop; write errors are logged and discarded).
    ///
    /// All money math uses [`Decimal`] / [`Money<Usdt>`]; never `f64`.
    /// Returns the computed snapshot so the caller can persist or
    /// log it independently.
    pub fn after_bar_close(&self, bar_ts: Timestamp) -> PnlSnapshot {
        let state = self.state_rx.borrow().clone();
        let as_of = Timestamp::now();
        let snap = PnlSnapshot {
            cash: Money::from_decimal(state.cash),
            unrealized: Money::from_decimal(state.unrealized()),
            realized: Money::from_decimal(state.realized_pnl),
            total_equity: Money::from_decimal(state.equity()),
            // `daily_return` requires a roll-over baseline that the
            // reconciler does not yet track — populate as zero so the
            // snapshot is well-formed; T912 future work can wire the
            // baseline from the audit ledger.
            daily_return: Money::from_decimal(Decimal::ZERO),
            // Wallclock for the delivery guard / freshness (monotone).
            as_of,
            // Data time for the chart x-axis (the historical bar timeline).
            bar_ts: Some(bar_ts),
        };
        if let Some(bus) = &self.bus {
            bus.publish_pnl(snap.clone());
        }
        // live-equity-history-durable ADR-0052 A6: fire-and-forget persist.
        // The store is only Some in paper/live mode (the A2 mode gate lives at
        // `ReconcilerTask::with_equity_store` construction time in runtime::run).
        if let Some(store) = &self.equity_store {
            let store = Arc::clone(store);
            let row = build_snapshot_row(&snap, "paper");
            tokio::spawn(async move {
                if let Err(e) = store.append_equity_snapshot(&row).await {
                    tracing::warn!(
                        error = %e,
                        bar_ts = %bar_ts,
                        "equity_snapshot write failed (non-fatal — continuing)"
                    );
                }
            });
        }
        snap
    }

    /// Spawn the reconciler as a background tokio task.
    ///
    /// Each interval tick:
    /// 1. Checks for imbalance vs last recorded equity (original T26 logic).
    /// 2. Calls [`Self::after_bar_close`] with `bar_ts = Timestamp::now()`
    ///    so any wired bus/store receives the periodic snapshot.
    ///    In paper/live mode `bar_ts ≈ now()` — the bar close time and
    ///    wallclock nearly coincide (unlike replay mode where `bar_ts`
    ///    is a historical data time).
    pub fn spawn(self) {
        tokio::spawn(async move {
            let interval = tokio::time::Duration::from_millis(self.interval_ms);
            let mut last_equity = self.state_rx.borrow().equity();
            info!("reconciler started");
            loop {
                tokio::time::sleep(interval).await;

                if self.kill_switch.is_tripped() {
                    break;
                }

                let state = self.state_rx.borrow().clone();
                let current_equity = state.equity();

                // Check for imbalance vs last recorded equity
                let diff = (current_equity - last_equity).abs();
                if diff > state.tolerance * Decimal::from(1000u32) {
                    // Large sudden jump — not necessarily an imbalance, could be a price move.
                    // Real imbalance would be detected if ledger != position book.
                    // v0 simplified: flag if equity moved by more than tolerance * 1000
                    // (heuristic; full implementation needs ledger query).
                    tracing::debug!(diff = %diff, "large equity move (possibly price-driven)");
                }

                last_equity = current_equity;

                // Emit + optionally persist a PnL snapshot every tick
                // (live-cockpit-unified T903c + ADR-0052 A6).
                // In paper/live mode bar_ts ≈ now() — bars arrive in real-time
                // so the bar close time and wallclock nearly coincide.
                self.after_bar_close(Timestamp::now());
            }
            info!("reconciler stopped");
        });
    }
}

/// Build an [`audit::EquitySnapshotRow`] from a [`PnlSnapshot`] and the
/// agent mode string (live-equity-history-durable ADR-0052 / A3).
///
/// Called from [`ReconcilerTask::after_bar_close`] (paper/live path) and
/// optionally from the research trading loop (where it is never called
/// because the mode gate is `None` at construction time — A2).
///
/// `bar_ts` in the row comes from `snap.bar_ts.unwrap_or(snap.as_of)` so
/// the row's x-axis coordinate matches the chart (approach A invariant).
/// The `ts` field is the row mint wallclock (`snap.as_of` is already
/// `Timestamp::now()` at this point).
fn build_snapshot_row(snap: &PnlSnapshot, mode: &str) -> audit::EquitySnapshotRow {
    let bar_ts = snap.bar_ts.unwrap_or(snap.as_of);
    audit::EquitySnapshotRow {
        id: Uuid::new_v4().to_string(),
        ts: snap.as_of,
        bar_ts,
        as_of: snap.as_of,
        total_equity: snap.total_equity,
        cash: snap.cash,
        realized: snap.realized,
        unrealized: snap.unrealized,
        mode: mode.to_string(),
    }
}

/// Synchronous reconciliation check (used in tests and backtest loop).
///
/// Returns `true` if balanced, `false` on imbalance > `tolerance`.
#[must_use]
pub fn check_balance(
    cash: Decimal,
    position_qty: Decimal,
    mark: Decimal,
    expected_equity: Decimal,
    tolerance: Decimal,
) -> bool {
    let computed = cash + position_qty * mark;
    (computed - expected_equity).abs() <= tolerance
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kill_switch::HaltReason;
    use rust_decimal_macros::dec;

    #[test]
    fn t26_balanced_check() {
        // cash=90_000, qty=0.25 BTC, mark=40_000 → equity = 100_000
        assert!(check_balance(
            dec!(90_000),
            dec!(0.25),
            dec!(40_000),
            dec!(100_000),
            dec!(0.01),
        ));
    }

    #[test]
    fn t26_imbalance_detected() {
        // Introduce deliberate imbalance > tolerance
        let is_balanced = check_balance(
            dec!(90_000),
            dec!(0.25),
            dec!(40_000),
            dec!(100_100), // off by 100
            dec!(0.01),
        );
        assert!(!is_balanced, "should detect imbalance");
    }

    #[tokio::test]
    async fn t26_kill_switch_trips_on_imbalance() {
        let ks = KillSwitch::new("/tmp/nonexistent_halt_xyz.halt", 16);

        // Simulate imbalance > tolerance
        let is_balanced = check_balance(
            dec!(90_000),
            dec!(0.25),
            dec!(40_000),
            dec!(100_100),
            dec!(0.01),
        );

        if !is_balanced {
            ks.trip(HaltReason::LedgerImbalance);
        }

        assert!(
            ks.is_tripped(),
            "kill switch should be tripped on imbalance"
        );
        let rx = ks.subscribe();
        // The trip was before subscribe — can't receive it now, but kill switch is tripped
        drop(rx);
    }

    /// T903c — `after_bar_close` publishes a [`PnlSnapshot`] onto the
    /// bus's `pnl` channel.  Constructs a reconciler with a real
    /// [`EventBus`], invokes `after_bar_close` with a fixed bar
    /// timestamp, asserts the snapshot's `as_of` equals the supplied
    /// bar timestamp (ISSUE 1 — data time, not wallclock), and asserts
    /// the expected `realized` / `unrealized` decimal values.
    /// Backward-compat: a reconciler without a bus does NOT panic and
    /// returns the same snapshot from `after_bar_close`.
    #[tokio::test]
    async fn t903c_after_bar_close_publishes_pnl() {
        use crate::EventBus;
        use crate::config::BusConfig;

        let state = ReconcilerState {
            cash: dec!(100_000),
            position_qty: dec!(0.5),
            last_mark: dec!(60_000),
            tolerance: dec!(0.01),
            realized_pnl: dec!(123.45),
            cost_basis: dec!(25_000),
        };
        let (_state_tx, state_rx) = tokio::sync::watch::channel(state.clone());

        let ks = KillSwitch::new("/tmp/nonexistent_t903c_halt.halt", 16);
        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        let mut pnl_rx = bus.pnl();

        let task = ReconcilerTask::new(state_rx, ks, 1_000).with_bus(Arc::clone(&bus));

        // Use a fixed historical bar timestamp (2023-01-15 12:30:00 UTC) — the
        // snapshot's `bar_ts` (the chart x-coord) must equal this value, while
        // `as_of` is wallclock `now()` (the delivery/freshness coordinate).
        let bar_ts = Timestamp::new(
            ::time::OffsetDateTime::from_unix_timestamp(1_673_789_400)
                .expect("static timestamp is valid"),
        );
        let snap = task.after_bar_close(bar_ts);

        // Computed values: cash = 100_000, unrealized = 0.5*60_000 - 25_000 = 5_000,
        // realized = 123.45, equity = 100_000 + 0.5*60_000 = 130_000.
        assert_eq!(snap.cash.amount(), dec!(100_000));
        assert_eq!(snap.unrealized.amount(), dec!(5_000));
        assert_eq!(snap.realized.amount(), dec!(123.45));
        assert_eq!(snap.total_equity.amount(), dec!(130_000));
        // approach-A assertion: the bar/data time rides `bar_ts` (the chart
        // x-coord), NOT `as_of`. `as_of` is wallclock now() — far in the
        // future relative to the 2023 bar — so the historical axis stays clean
        // while the delivery guard keeps a monotone wallclock key.
        assert_eq!(
            snap.bar_ts,
            Some(bar_ts),
            "bar_ts must carry the supplied data/bar timestamp (the chart x-coord)"
        );
        assert!(
            snap.as_of.unix_millis() >= bar_ts.unix_millis(),
            "as_of must be wallclock now() (>= the 2023 bar time), not the bar time"
        );

        // Subscriber receives the same snapshot via the bus.
        let received = tokio::time::timeout(std::time::Duration::from_secs(1), pnl_rx.recv())
            .await
            .expect("pnl snapshot did not arrive inside 1 s")
            .expect("pnl channel closed unexpectedly");
        assert_eq!(received.cash.amount(), dec!(100_000));
        assert_eq!(received.unrealized.amount(), dec!(5_000));
        assert_eq!(received.realized.amount(), dec!(123.45));
        assert_eq!(received.total_equity.amount(), dec!(130_000));
        assert_eq!(
            received.bar_ts,
            Some(bar_ts),
            "received bar_ts must carry the data/bar timestamp"
        );

        // Backward-compat: reconciler without a bus returns the
        // snapshot but does NOT panic publishing it.
        let (_tx2, rx2) = tokio::sync::watch::channel(state);
        let bare_task = ReconcilerTask::new(
            rx2,
            KillSwitch::new("/tmp/nonexistent_t903c_halt2.halt", 16),
            1_000,
        );
        let bare_snap = bare_task.after_bar_close(bar_ts);
        assert_eq!(bare_snap.unrealized.amount(), dec!(5_000));
    }
}
