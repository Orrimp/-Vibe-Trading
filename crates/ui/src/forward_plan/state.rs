//! advisor-forward-plan v0.1.0 (roadmap F6) — the forward buy/sell PLAN's
//! per-`ui` mirror + per-session state.
//!
//! Journey **step 4 (Plan)** of the single-coin investment-advisor
//! (product § journey: pick coin + budget → bake off → rank & pick →
//! **plan** → watch paper-trade). This module holds the conditional,
//! reactive decision plan the screen renders — mirrored into a pure-`ui`
//! shape exactly as [`crate::leaderboard::BakeoffReportMirror`] mirrors
//! `backtest::BakeoffReport`.
//!
//! ## The mirror discipline (INVARIANT — ADR-0062 § D4)
//!
//! `ui` must NOT import `strategy` / `exec` / `forecast` / `llm`, **and**
//! must not gain a *new* edge on `agent` in its default build. The real
//! plan type the developer is landing IN PARALLEL is the `core`-typed
//! `agent::config::ForwardPlan` (`Clone + Debug`, closed `agent`-owned
//! `PlanStance` / `PlanSignal` / `PlanRuleKind` enums + `core` types). We
//! mirror it into [`ForwardPlanView`] + the closed `ui` enums below at the
//! dispatch boundary (the binary's `live` build, which already depends on
//! `agent`) via [`ForwardPlanView::from_plan`] — so the render code threads
//! only plain `ui` types and `cargo tree -p ui` is unchanged.
//!
//! Until the developer's `ForwardPlan` lands, the `ui` lib carries a
//! **fixture-only** mirror of its shape ([`crate::fixtures`] builds it) so
//! the render-layer proof can run headless against a fake. The
//! `from_plan` adapter (which names `agent::config::ForwardPlan`) is
//! compiled only under the `live` feature — exactly the
//! `BakeoffReportMirror::from_report` boundary discipline.
//!
//! ## The honest semantics (OQ-D — the central UX call)
//!
//! The plan is a **conditional, reactive, rule-driven** decision plan —
//! NOT a price forecast. Every field here is read-only descriptive data
//! about *what the engine will do when conditions fire*. The plain-language
//! copy (the IF/THEN rule sentences + the not-a-prediction / not-advice
//! disclaimers) lives in [`crate::strings`] — no engine string crosses the
//! seam.

use rust_decimal::Decimal;
use smol_str::SmolStr;

use crate::state::PanelState;

/// Current stance of the crowned strategy as of the latest bar — mirrored
/// from `agent::config::PlanStance` into a closed `ui` enum.
///
/// `FLAT` = no position held; `LONG` = holding. The stance is dated to the
/// latest bar (`as_of_label`) so the operator sees how current it is (R1 —
/// honest staleness).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanStanceView {
    /// No position — the strategy is waiting for an entry trigger.
    Flat,
    /// Holding a position — the strategy is in the market, watching for an
    /// exit trigger (or, for buy-and-hold, simply holding the horizon).
    Long,
}

/// The crowned strategy's latest signal as of the most recent bar —
/// mirrored from `agent::config::PlanSignal` into a closed `ui` enum.
///
/// `None` on the view means "no current signal" — the buy-and-hold
/// degenerate case (no re-evaluation; D5), rendered as a plain "holding"
/// rather than an active BUY/SELL/HOLD reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanSignalView {
    /// The latest bar fired an entry signal.
    Buy,
    /// The latest bar fired an exit signal.
    Sell,
    /// The latest bar fired no action — conditions unmet, stance unchanged.
    Hold,
}

/// The standing entry/exit RULE FAMILY — mirrored from
/// `agent::config::PlanRuleKind` (itself a mirror of the
/// `strategy::PlanRuleShape` the engine emits, ADR-0062 § D1) into a closed
/// `ui` enum carrying the rule parameters.
///
/// The screen maps each variant to its plain-language IF/THEN copy (the
/// `ui` owns the words — no engine `String` crosses the seam). The
/// parameters (lengths / thresholds) let the copy be specific without the
/// engine dictating prose.
///
/// **Honesty note:** rule families reflect the REAL strategy rules as loaded
/// by `build_registry_for` from `config/strategies/*.toml`. The exit
/// semantics for RSI and Bollinger are flip-to-false (the entry condition
/// clearing), NOT a reverse-threshold cross.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanRuleView {
    /// SMA crossover — buys when the fast SMA crosses above the slow SMA,
    /// sells on the reverse cross.
    SmaCross {
        /// Fast (short) SMA window length, in bars.
        fast_len: u32,
        /// Slow (long) SMA window length, in bars.
        slow_len: u32,
    },
    /// MACD trend — buys when the MACD histogram is positive AND price is
    /// above EMA(200); exits when that compound condition flips false.
    MacdCross {
        /// Fast EMA length.
        fast: u32,
        /// Slow EMA length.
        slow: u32,
        /// Signal EMA length.
        signal: u32,
    },
    /// RSI mean-reversion — buys when RSI falls below `lower` (oversold) AND
    /// the close is above the recent support floor.  Exits when RSI climbs
    /// back above `lower` (flip-to-false — the entry condition clears).
    /// There is NO upper/overbought threshold in this strategy.
    RsiReversion {
        /// RSI window length.
        len: u32,
        /// Oversold entry threshold; also the flip-to-false exit threshold.
        lower: u32,
    },
    /// Bollinger mean-reversion — buys when price closes below the lower band
    /// AND volume surges; exits when price closes back inside the band
    /// (flip-to-false exit — NOT a reverse upper-band cross).
    BollingerReversion {
        /// Moving-average window length.
        len: u32,
        /// Band width in standard deviations (×10 so it stays integral,
        /// e.g. `k_tenths = 20` → 2.0σ — the mirror keeps the closed enum
        /// `Copy`/`Eq` without a `Decimal` field).
        k_tenths: u32,
    },
    /// Buy-and-hold — buy once and hold the whole horizon; there is no sell
    /// trigger and no re-evaluation (the degenerate plan, D5).
    BuyAndHold,
}

impl PlanRuleView {
    /// `true` for the buy-and-hold degenerate rule — the screen drops the
    /// "sell" half of the IF/THEN copy and the re-evaluation cadence for
    /// this case (there is no sell trigger).
    #[must_use]
    pub fn is_buy_and_hold(self) -> bool {
        matches!(self, PlanRuleView::BuyAndHold)
    }
}

/// The full forward plan the screen renders — a pure-`ui` mirror of the
/// `core`-typed `agent::config::ForwardPlan` (ADR-0062 § D4 struct shape).
///
/// Every field is a plain `ui` type (`SmolStr` / `Decimal` / closed `ui`
/// enum / primitive) — free of every engine type, so the render code is
/// trivially `ui`-pure and the plan is unit-constructible in fixtures +
/// render tests without standing up the engine.
///
/// Field-for-field parallel to the developer's `ForwardPlan`:
/// `strategy` / `symbol` / `stance` / `latest_signal` / `rule` /
/// `last_close` / `last_bar_ts` (here pre-formatted to `as_of_label`) /
/// `budget` / `projected_units` / `sizing_capped` / `horizon_days`.
#[derive(Debug, Clone, PartialEq)]
pub struct ForwardPlanView {
    /// The resolved forward-run strategy display id, e.g. `"v0.sma"`.
    pub strategy: SmolStr,
    /// The coin the plan is for, e.g. `"BTCUSDT"` (echoed for copy).
    pub symbol: SmolStr,
    /// Current stance as of the latest bar (`FLAT` / `LONG`).
    pub stance: PlanStanceView,
    /// The latest signal as of the most recent bar, `None` for the
    /// buy-and-hold degenerate case (no re-evaluation).
    pub latest_signal: Option<PlanSignalView>,
    /// The standing entry/exit rule family (drives the IF/THEN copy).
    pub rule: PlanRuleView,
    /// The latest bar's close price — the projection price for the sizing
    /// ("at the last close").
    pub last_close: Decimal,
    /// Pre-formatted "as of" label for the latest bar (e.g. `"Jun 19 14:00"`)
    /// — the honest-staleness stamp on the stance badge. Pre-formatted in
    /// the adapter so the render code carries no time-zone logic.
    pub as_of_label: SmolStr,
    /// The simulated budget (€200 ≈ 200 USDT, product § D4), as a `Decimal`
    /// of quote-units.
    pub budget: Decimal,
    /// The projected next-BUY deployment (`units ≈ budget / last_close`,
    /// capped by the F4 `budget_cap`).
    pub projected_units: Decimal,
    /// `true` iff the F4 budget cap bound the projected units (the screen
    /// surfaces the cap explicitly when it bit).
    pub sizing_capped: bool,
    /// The planning horizon in days (default 7, range 1–30) — DISPLAY-ONLY
    /// framing (ADR-0062 § D6); does NOT terminate the forward run. Shown in
    /// the horizon framing ("rules in force, checked each bar, for the next
    /// `horizon_days` days").
    pub horizon_days: u16,
    /// Pre-formatted "planned through" date label (e.g. `"Jun 26"`) =
    /// `last_bar_ts + horizon_days`, computed in the adapter so the render
    /// code carries no time-zone / date arithmetic (the consistency gate
    /// forbids inline date formatting in the render layer). Renders as
    /// "planned through <date>" (R4).
    pub horizon_through_label: SmolStr,
}

impl ForwardPlanView {
    /// `true` when this is the buy-and-hold degenerate plan — convenience
    /// for the screen (drops the sell-rule line + the re-evaluation cadence).
    #[must_use]
    pub fn is_buy_and_hold(&self) -> bool {
        self.rule.is_buy_and_hold()
    }
}

/// Per-session Forward-plan screen state. Sibling of
/// [`crate::leaderboard::LeaderboardScreenState`].
///
/// `Default` = `plan: PanelState::Empty` (no crowned pick yet → no plan —
/// the clean tautology guard / "run a bake-off first" prompt). When a
/// bake-off crowns a pick and the F5 `ForwardCommand::Launch` fires, the
/// agent returns a `ForwardPlan` over `forward_plan_rx`; the cockpit
/// mirrors it to [`ForwardPlanView`] and lands it as `Ready(view)`.
#[derive(Debug, Clone)]
pub struct ForwardPlanScreenState {
    /// The forward plan.
    ///
    /// - `Empty` — no crowned pick yet (the "run a bake-off to see the
    ///   plan" prompt — never a blank screen).
    /// - `Loading` — a plan is being computed (the agent is resolving the
    ///   registry at the `Launch` boundary).
    /// - `Ready(view)` — the conditional plan: stance + IF/THEN rules +
    ///   projected €200 sizing + horizon.
    /// - `Error(msg)` — the plan could not be produced (operator-friendly).
    pub plan: PanelState<ForwardPlanView>,
}

impl Default for ForwardPlanScreenState {
    fn default() -> Self {
        Self {
            // Cold start is the honest Empty state (no pick yet), NOT
            // Loading — there is nothing to compute until a bake-off crowns
            // a pick.
            plan: PanelState::Empty,
        }
    }
}

impl ForwardPlanScreenState {
    /// Land a freshly-mirrored plan as `Ready`. Called from the cockpit's
    /// forward-plan receive arm (the agent→iced return path).
    pub fn set_plan(&mut self, view: ForwardPlanView) {
        self.plan = PanelState::Ready(view);
    }

    /// Reset to the cold Empty state (no plan) — called when the pick is
    /// cleared (e.g. a new bake-off is requested before it crowns).
    pub fn clear(&mut self) {
        self.plan = PanelState::Empty;
    }

    /// Land a plan-production failure as `Error`.
    pub fn fail(&mut self, msg: SmolStr) {
        self.plan = PanelState::Error(msg);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn active_view() -> ForwardPlanView {
        ForwardPlanView {
            strategy: SmolStr::new("v0.sma"),
            symbol: SmolStr::new("BTCUSDT"),
            stance: PlanStanceView::Flat,
            latest_signal: Some(PlanSignalView::Hold),
            rule: PlanRuleView::SmaCross {
                fast_len: 12,
                slow_len: 26,
            },
            last_close: dec!(64000.00),
            as_of_label: SmolStr::new("Jun 19 14:00"),
            budget: dec!(200),
            projected_units: dec!(0.003125),
            sizing_capped: false,
            horizon_days: 7,
            horizon_through_label: SmolStr::new("Jun 26"),
        }
    }

    #[test]
    fn default_is_empty() {
        let s = ForwardPlanScreenState::default();
        assert!(
            matches!(s.plan, PanelState::Empty),
            "cold start must be Empty (no pick yet → the run-a-bake-off prompt)"
        );
    }

    #[test]
    fn set_plan_lands_ready() {
        let mut s = ForwardPlanScreenState::default();
        s.set_plan(active_view());
        assert!(matches!(s.plan, PanelState::Ready(_)));
    }

    #[test]
    fn clear_resets_to_empty() {
        let mut s = ForwardPlanScreenState::default();
        s.set_plan(active_view());
        s.clear();
        assert!(matches!(s.plan, PanelState::Empty));
    }

    #[test]
    fn fail_lands_error() {
        let mut s = ForwardPlanScreenState::default();
        s.fail(SmolStr::new("registry unresolved"));
        match &s.plan {
            PanelState::Error(e) => assert_eq!(e.as_str(), "registry unresolved"),
            other => panic!("expected Error, got {}", other.variant_name()),
        }
    }

    #[test]
    fn buy_and_hold_view_is_flagged() {
        let mut v = active_view();
        v.rule = PlanRuleView::BuyAndHold;
        v.stance = PlanStanceView::Long;
        v.latest_signal = None;
        assert!(v.is_buy_and_hold(), "BuyAndHold rule → degenerate plan");
        assert!(PlanRuleView::BuyAndHold.is_buy_and_hold());
        assert!(
            !PlanRuleView::SmaCross {
                fast_len: 12,
                slow_len: 26
            }
            .is_buy_and_hold(),
            "an active rule is not buy-and-hold"
        );
    }
}
