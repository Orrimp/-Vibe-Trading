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

/// The vote method of an ensemble candidate — mirrored from
/// `agent::config::PlanVoteMethod` (itself a mirror of the closed
/// `strategy::PlanVoteMethod`, ADR-0063 § D3) into a closed `ui` enum.
///
/// **Reconciliation note (developer ‖ ui-designer parallel) — RECONCILED:**
/// the developer's canonical `strategy::PlanVoteMethod` (`crates/strategy/
/// src/plan.rs:82`) uses **`u32`** for `k`/`n` (NOT `usize` as the ADR § D1
/// prose said) — their doc rationale: "Field types use `u32` (not `usize`)
/// so the type stays `Copy + Eq` and crosses the `agent`→`ui` boundary
/// without `Decimal` or lifetime issues." This `ui` mirror matches the
/// **shipped `u32`**, the same way the existing `PlanRuleView` length fields
/// are `u32` while the engine carries `usize` (the `agent` boundary narrows
/// `usize` → `u32`). If a name drifts at integration, the single
/// [`super::adapter`] `vote_method_view` mapping is the only edit site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanVoteMethodView {
    /// Majority vote — the ensemble holds LONG when at least `k` of `n` warmed
    /// members are LONG (e.g. ≥ 2 of 3). `k < n`.
    Majority {
        /// The quorum threshold — how many members must agree LONG.
        k: u32,
        /// The membership count — the total number of voting members.
        n: u32,
    },
    /// Unanimous vote — the ensemble holds LONG only when ALL `n` warmed
    /// members agree LONG (the maximally-conservative consensus).
    Unanimous {
        /// The membership count — all `n` must agree.
        n: u32,
    },
}

impl PlanVoteMethodView {
    /// The membership count `n` — used by the copy ("…of {n}…") and to render
    /// the live-tally denominator.
    #[must_use]
    pub fn member_count(self) -> u32 {
        match self {
            PlanVoteMethodView::Majority { n, .. } | PlanVoteMethodView::Unanimous { n } => n,
        }
    }

    /// The quorum threshold — how many members must agree LONG for the ensemble
    /// to hold. For `Unanimous` this is `n` (all members).
    #[must_use]
    pub fn quorum(self) -> u32 {
        match self {
            PlanVoteMethodView::Majority { k, .. } => k,
            PlanVoteMethodView::Unanimous { n } => n,
        }
    }
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
///
/// **Not `Copy`** (it was, before the F6 member-name enrichment): `Ensemble` now
/// carries a `Vec<PlanMemberFamilyView>` so the plan can NAME its members, which
/// makes the enum `Clone`-not-`Copy`. The two by-value `self` helpers
/// (`is_buy_and_hold` / `is_ensemble`) take `&self` accordingly; the owning
/// [`ForwardPlanView`] was already `Clone`-not-`Copy`, so no caller regresses.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Ensemble (signal vote, F8 / ADR-0063) — the candidate is a deterministic
    /// combination of its members' BUY/SELL signals. Holds LONG when the
    /// `method`'s quorum of `member_count` warmed members agree LONG; goes flat
    /// when the consensus flips. The screen renders the method + member count +
    /// the live tally as FAITHFUL copy — NOT a fabricated single rule.
    ///
    /// **Reconciliation note (developer ‖ ui-designer parallel) — RECONCILED to
    /// the shipped agent shape (F6 member-name enrichment):** the developer
    /// enriched `agent::config::PlanRuleKind::Ensemble` (`crates/agent/src/
    /// config.rs`) to carry `members: Vec<smol_str::SmolStr>` — the human-readable
    /// DISPLAY LABEL of each member strategy, in member order (e.g.
    /// `["MACD trend", "RSI reversion", "Bollinger reversion"]`), sourced from
    /// `strategy::EnsembleStrategy::describe_plan`. `PlanRuleKind` dropped `Copy`
    /// for this. The old shape carried only a scalar `member_count: u32`. This
    /// `ui` mirror carries the same `members: Vec<SmolStr>` field-for-field, so
    /// the plan NAMES the members ("≥ 2 of {MACD trend, RSI reversion, Bollinger
    /// reversion} agree…") rather than counting them abstractly. The
    /// authoritative member count is `members.len()` (the method's `n` is a
    /// belt-and-braces cross-check). If a field name drifts at integration, the
    /// single [`super::adapter`] `rule_view` mapping is the only `ui` edit site.
    Ensemble {
        /// The vote method (majority `k`-of-`n` / unanimous `n`-of-`n`).
        method: PlanVoteMethodView,
        /// The member strategies' display labels, in member order — so the plan
        /// can NAME them ("{MACD trend, RSI reversion, Bollinger reversion}") in
        /// the headline vote rule. `members.len()` is the authoritative member
        /// count. Empty only in the defensive/degenerate case (the copy then
        /// falls back to the method's `n` count-based phrasing — never blank).
        members: Vec<SmolStr>,
    },
}

impl PlanRuleView {
    /// `true` for the buy-and-hold degenerate rule — the screen drops the
    /// "sell" half of the IF/THEN copy and the re-evaluation cadence for
    /// this case (there is no sell trigger).
    ///
    /// Takes `&self` (the enum is no longer `Copy` — `Ensemble` carries a `Vec`).
    #[must_use]
    pub fn is_buy_and_hold(&self) -> bool {
        matches!(self, PlanRuleView::BuyAndHold)
    }

    /// `true` for an ensemble (signal-vote) rule — the screen renders the
    /// method + named members + live tally instead of a single IF/THEN family.
    #[must_use]
    pub fn is_ensemble(&self) -> bool {
        matches!(self, PlanRuleView::Ensemble { .. })
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

    /// `true` when the crowned pick is an ensemble (signal vote) — convenience
    /// for the screen (renders the vote method + members + live tally instead
    /// of a single IF/THEN family).
    #[must_use]
    pub fn is_ensemble(&self) -> bool {
        self.rule.is_ensemble()
    }

    /// `true` when the crowned pick is one of the FIXED 5-arm short slate
    /// (advisor-short-selling, ADR-0068 § D9) — keyed on the `strategy` id, a
    /// closed `ui`-side predicate (the same discipline the leaderboard's
    /// `is_short_capable_id` uses). No new `agent::config::ForwardPlan` field
    /// crosses the seam — `from_plan` stays byte-identical, so the mirror
    /// discipline (ADR-0062 § D4) and `cargo tree -p ui` are undisturbed. When
    /// true, the forward plan appends the honest sell-to-open / cover /
    /// liquidation short-rule copy + the unbounded-loss disclaimer.
    ///
    /// The `_ls` suffix covers the four symmetric long/short variants
    /// (`sma_cross_ls` / `macd_ls` / `rsi_ls` / `bbands_ls`); `always_short` is
    /// the explicit always-short benchmark control.
    #[must_use]
    pub fn is_short_capable(&self) -> bool {
        let id = self.strategy.as_str();
        id.ends_with("_ls") || id == "always_short"
    }

    /// `true` for the always-short benchmark control specifically — its standing
    /// rule is the degenerate "open a short and hold it" (the down-side mirror of
    /// buy-and-hold), with NO cover trigger. The screen renders a single standing
    /// short rule for it, the way `is_buy_and_hold` collapses the buy-and-hold
    /// degenerate plan.
    #[must_use]
    pub fn is_always_short(&self) -> bool {
        self.strategy.as_str() == "always_short"
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

    #[test]
    fn ensemble_view_is_flagged_and_not_buy_and_hold() {
        let rule = PlanRuleView::Ensemble {
            method: PlanVoteMethodView::Majority { k: 2, n: 3 },
            members: vec![
                SmolStr::new("MACD trend"),
                SmolStr::new("RSI reversion"),
                SmolStr::new("Bollinger reversion"),
            ],
        };
        assert!(rule.is_ensemble(), "Ensemble rule → ensemble plan");
        assert!(
            !rule.is_buy_and_hold(),
            "an ensemble is not the buy-and-hold degenerate plan"
        );
        let mut v = active_view();
        v.rule = rule;
        assert!(v.is_ensemble(), "ForwardPlanView delegates is_ensemble");
        assert!(!v.is_buy_and_hold());
    }

    #[test]
    fn vote_method_view_quorum_and_member_count() {
        let maj = PlanVoteMethodView::Majority { k: 2, n: 3 };
        assert_eq!(maj.quorum(), 2);
        assert_eq!(maj.member_count(), 3);
        let unan = PlanVoteMethodView::Unanimous { n: 4 };
        assert_eq!(unan.quorum(), 4, "unanimous quorum is n (all members)");
        assert_eq!(unan.member_count(), 4);
    }

    /// advisor-short-selling (T-U3) — `is_short_capable` keys on the strategy id
    /// (the closed ui-side predicate, no engine field crosses the seam) and fires
    /// for every short-slate arm, NOT for the long-only / ensemble / benchmark
    /// arms. `is_always_short` is the strict always-short control predicate.
    #[test]
    fn short_capability_keys_on_strategy_id() {
        let mut v = active_view();
        for id in ["sma_cross_ls", "macd_ls", "rsi_ls", "bbands_ls", "always_short"] {
            v.strategy = SmolStr::new(id);
            assert!(v.is_short_capable(), "`{id}` is short-capable");
        }
        // always_short is the strict control; the `_ls` arms are not.
        v.strategy = SmolStr::new("always_short");
        assert!(v.is_always_short(), "always_short is the always-short control");
        v.strategy = SmolStr::new("sma_cross_ls");
        assert!(
            !v.is_always_short(),
            "an `_ls` symmetric variant is not the always-short control"
        );
        // Long-only / ensemble / benchmark arms are NOT short-capable.
        for id in ["v0.sma", "v0.5.macd", "v0.buyhold", "v0.8.vote.majority"] {
            v.strategy = SmolStr::new(id);
            assert!(!v.is_short_capable(), "`{id}` is not short-capable");
        }
    }
}
