//! F6 forward-plan read seam — `PlanDescribe` sibling trait + structured rule data.
//!
//! ## Design (ADR-0062 § D1–D2)
//!
//! `PlanDescribe` is a **read-only sibling** of the `Strategy` trait.  It is NOT
//! a method on `Strategy` (which is frozen per ADR-0005) — it is an opt-in
//! trait for the five F6 candidate engines (SMA, MACD, RSI, BBands via
//! `ComposedStrategy`, and buy-and-hold via `AlwaysLongStrategy`).
//!
//! `describe_plan` MUST NOT mutate indicator state — it is a pure snapshot of
//! the engine's current stance derived from already-warmed indicator values.
//! The plan is NOT a price forecast; it describes the standing conditional
//! entry/exit rules + the current stance + a projected €200 next-BUY sizing.
//!
//! The engine emits **structured rule data** only — copy strings live in
//! `ui::strings` (the ADR-0059 `Recommendation`-not-a-`String` precedent).

use rust_decimal::Decimal;
use trading_core::{Money, Price, Quantity, Timestamp, Usdt};

// ── PlanContext ────────────────────────────────────────────────────────────────

/// Input context for `PlanDescribe::describe_plan`.
///
/// All fields are `core` types — the supervisor builds this from the
/// `ForwardRunConfig` budget + the latest-bar close/ts from the feed.
#[derive(Debug, Clone)]
pub struct PlanContext {
    /// The latest bar's close price — the projection price for sizing.
    pub last_close: Price,
    /// The latest bar's close timestamp — shown for honest-staleness labelling.
    pub last_bar_ts: Timestamp,
    /// The user's budget (€200 ≈ 200 USDT, product § D4).
    pub budget: Money<Usdt>,
    /// The F4 budget cap enforced by the paper loop's `FixedFractionSizer`.
    ///
    /// The plan must reflect **the same cap** the paper loop enforces so the
    /// projected sizing and the actual first fill agree (R3 / ADR-0062 § D3).
    pub budget_cap: Money<Usdt>,
}

// ── PlanStance ────────────────────────────────────────────────────────────────

/// The engine's current holding stance.
///
/// Derived from the engine's already-warmed indicators at the latest bar.
/// `Long` means the engine holds (or would immediately buy into) a position.
/// `Flat` means no position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanStance {
    /// No position — the engine is waiting for an entry signal.
    Flat,
    /// Holding a long position — the engine entered and has not yet exited.
    Long,
}

// ── PlanSignal ────────────────────────────────────────────────────────────────

/// The most recent signal kind from the engine on the last bar.
///
/// `None` for buy-and-hold (which has no re-evaluation after the first bar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanSignal {
    Buy,
    Sell,
    Hold,
}

// ── PlanRuleShape ─────────────────────────────────────────────────────────────

// ── PlanVoteMethod ────────────────────────────────────────────────────────────

/// Vote method for an ensemble plan — closed enum (no free-text string).
///
/// ADR-0063 § D3: the `ui` exhaustively matches on this to generate the
/// honest vote description ("≥ k of n" / "all n agree"). The enum is closed
/// so the compiler enforces exhaustiveness when new methods are added.
///
/// Field types use `u32` (not `usize`) so the type stays `Copy + Eq` and
/// crosses the `agent`→`ui` boundary without `Decimal` or lifetime issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanVoteMethod {
    /// Long iff `long_count >= k` (minimum k of n members agree).
    Majority {
        /// Minimum long count required.
        k: u32,
        /// Total number of members.
        n: u32,
    },
    /// Long iff ALL n members agree Long.
    Unanimous {
        /// Total number of members (must all be Long).
        n: u32,
    },
}

// ── PlanRuleShape ─────────────────────────────────────────────────────────────

/// The engine's rule family — structured data the `ui` maps to plain-language copy.
///
/// A **closed enum** — the `ui` exhaustively matches on this to generate
/// IF/THEN rule sentences.  No free-text string crosses the seam (ADR-0059
/// `Recommendation`-not-a-`String` precedent).
///
/// Variants:
/// - `SmaCross` — fast/slow SMA crossover.
/// - `MacdCross` — `btc_macd_trend`: MACD histogram positive AND price above
///   EMA(200); exits when that compound condition flips false.
/// - `RsiReversion` — `btc_rsi_reversion`: RSI(14) < 30 AND close above
///   recent support floor; exits when RSI climbs back above 30 (the entry
///   condition clears — a flip-to-false exit, NOT an RSI-70 threshold).
/// - `BollingerReversion` — `btc_bbands_mean_revert`: close below the lower
///   Bollinger band AND volume surge; exits when price closes back inside the
///   band (flip-to-false exit).
/// - `BuyAndHold` — the `AlwaysLongStrategy` degenerate plan.
/// - `Ensemble` — a signal-vote ensemble (ADR-0063 § D3): carries the vote
///   method + each member's own rule shape.  No free-text string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanRuleShape {
    /// SMA crossover — buys when fast > slow, sells on reverse.
    SmaCross {
        /// Fast SMA window length (bars).
        fast_len: usize,
        /// Slow SMA window length (bars).
        slow_len: usize,
    },
    /// MACD trend — buys when the MACD histogram is positive AND price is
    /// above EMA(200); exits when that compound condition flips false.
    MacdCross {
        /// Fast EMA length.
        fast: usize,
        /// Slow EMA length.
        slow: usize,
        /// Signal EMA length.
        signal: usize,
    },
    /// RSI mean-reversion — buys when RSI falls below `lower` (oversold) AND
    /// the close is above the recent support floor.  Exits when the RSI climbs
    /// back above `lower` (the entry condition clears — a flip-to-false exit).
    /// There is NO upper/overbought threshold in this strategy.
    RsiReversion {
        /// RSI window length.
        len: usize,
        /// Oversold entry threshold; also the exit threshold (flip-to-false).
        lower: Decimal,
    },
    /// Bollinger-band reversion — buys when price closes below the lower band
    /// AND volume surges; exits when price closes back inside the band
    /// (flip-to-false exit — NOT a reverse upper-band cross).
    BollingerReversion {
        /// Band window length.
        len: usize,
        /// Standard-deviation multiplier.
        k: Decimal,
    },
    /// Buy-and-hold — buy on the first bar, hold forever, no sell trigger.
    ///
    /// The degenerate case: the `ui` renders "buy now, hold the whole horizon,
    /// no sell trigger, deploy the full €200".
    BuyAndHold,
    /// Signal-vote ensemble (ADR-0063 § D3).
    ///
    /// Carries structured data only — NO copy string.  The `ui` exhaustively
    /// matches on `method` to generate the honest vote description, and on each
    /// element of `members` to list each member's own rule.
    ///
    /// The `members` field carries the real per-member `PlanRuleShape` so the
    /// UI can render e.g. "Holds when ≥ 2 of {MACD trend, RSI reversion,
    /// Bollinger reversion} agree" with the live tally.
    Ensemble {
        /// The vote arbitration method (closed enum — no free-text).
        method: PlanVoteMethod,
        /// Each member strategy's own rule shape (in member order).
        members: Vec<PlanRuleShape>,
    },
}

// ── ProjectedSizing ───────────────────────────────────────────────────────────

/// Projected next-BUY sizing at the current price (R3 / ADR-0062 § D3).
///
/// `units ≈ budget / last_close`, **capped by the F4 `budget_cap`**.
/// This is a current-price estimate, NOT a promised fill — the `ui` must label
/// it as such ("at the last close; the actual fill price will be the next bar's").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectedSizing {
    /// Projected units: `min(budget / last_close, budget_cap / last_close)`.
    pub units: Quantity,
    /// `true` iff the F4 `budget_cap` constrained the projected units
    /// (i.e. `budget > budget_cap`).
    pub capped: bool,
}

impl ProjectedSizing {
    /// Compute the projected sizing from budget + close price + cap.
    ///
    /// Returns `units = min(budget, budget_cap) / last_close` and sets
    /// `capped = budget > budget_cap`.
    ///
    /// Returns `units = 0` if `last_close` is zero (defensive; Price is always > 0).
    #[must_use]
    pub fn compute(budget: Money<Usdt>, budget_cap: Money<Usdt>, last_close: Price) -> Self {
        let budget_dec = budget.amount();
        let cap_dec = budget_cap.amount();
        let price = last_close.get();

        // Guard against zero price (Price::new ensures > 0, so this is defensive only).
        let units_dec = if price.is_zero() {
            Decimal::ZERO
        } else {
            let effective_budget = budget_dec.min(cap_dec);
            effective_budget / price
        };

        let units = Quantity::new(units_dec).unwrap_or(Quantity::zero());
        let capped = budget_dec > cap_dec;

        Self { units, capped }
    }
}

// ── StrategyPlan ──────────────────────────────────────────────────────────────

/// The structured plan emitted by `PlanDescribe::describe_plan`.
///
/// Contains structured rule data only — NO copy strings (see module doc).
/// The supervisor maps this to `agent::config::ForwardPlan` (core-typed) which
/// the `ui` mirrors into a `ForwardPlanView` to generate the rendered copy.
#[derive(Debug, Clone)]
pub struct StrategyPlan {
    /// Current holding stance derived from the warmed indicators (non-mutating read).
    pub stance: PlanStance,
    /// The most recent signal kind on the last-seen bar.
    ///
    /// `None` for buy-and-hold (no re-evaluation signal; the engine held immediately).
    pub latest_signal: Option<PlanSignal>,
    /// The engine's rule family — the structured data the `ui` maps to IF/THEN copy.
    pub rule: PlanRuleShape,
    /// Projected next-BUY sizing at `ctx.last_close`, bounded by `ctx.budget_cap`.
    pub sizing: ProjectedSizing,
}

// ── PlanDescribe trait ────────────────────────────────────────────────────────

/// Read-only sibling trait to `Strategy` (ADR-0062 § D1).
///
/// Implemented by the F6 candidate engines:
/// - `SmaCrossover` (the SMA arm)
/// - `AlwaysLongStrategy` (the buy-and-hold arm)
/// - `ComposedStrategy` (MACD / RSI / BBands — describes the REAL rule shape
///   from the loaded TOML, NOT a proxy)
///
/// ## Non-mutation contract
///
/// `describe_plan` takes `&self` (not `&mut self`) — it MUST NOT advance any
/// indicator state.  The stance + rule data are derived from the engine's
/// already-warmed internal values (read-only getters on the concrete struct),
/// with `ctx.last_close` as the current-bar reference price for sizing and
/// stance evaluation.  DO NOT call `push` or any other mutating indicator method.
pub trait PlanDescribe {
    /// Snapshot the engine's current stance + standing rules + projected sizing.
    ///
    /// # Contract
    ///
    /// - Pure read (`&self`) — MUST NOT push any value into any indicator.
    /// - Deterministic — two calls with the same `ctx` MUST return identical output.
    /// - Describes **what the engine resolves to in `build_registry_for`** —
    ///   NOT the Lab-time strategy id (ADR-0062 § D3 honesty invariant).
    fn describe_plan(&self, ctx: &PlanContext) -> StrategyPlan;
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{Price, Timestamp};

    use super::*;

    fn make_ctx(close: Decimal) -> PlanContext {
        PlanContext {
            last_close: Price::new(close).unwrap(),
            last_bar_ts: Timestamp::new(
                OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1_700_000_000),
            ),
            budget: Money::from_decimal(dec!(200)),
            budget_cap: Money::from_decimal(dec!(200)),
        }
    }

    #[test]
    fn projected_sizing_no_cap() {
        // budget 200 USDT, cap 200 USDT, price 50_000
        // units = 200 / 50_000 = 0.004; capped = false
        let ctx = make_ctx(dec!(50_000));
        let sizing = ProjectedSizing::compute(ctx.budget, ctx.budget_cap, ctx.last_close);
        assert_eq!(sizing.units.get(), dec!(0.004));
        assert!(!sizing.capped);
    }

    #[test]
    fn projected_sizing_with_cap() {
        // budget 300, cap 200, price 50_000 → effective = 200
        // units = 200 / 50_000 = 0.004; capped = true
        let budget: Money<Usdt> = Money::from_decimal(dec!(300));
        let budget_cap: Money<Usdt> = Money::from_decimal(dec!(200));
        let price = Price::new(dec!(50_000)).unwrap();
        let sizing = ProjectedSizing::compute(budget, budget_cap, price);
        assert_eq!(sizing.units.get(), dec!(0.004));
        assert!(sizing.capped);
    }

    #[test]
    fn plan_stance_is_non_copy_able_but_eq() {
        // Ensure the stance enum works as expected — used in anti-drift tests.
        assert_eq!(PlanStance::Flat, PlanStance::Flat);
        assert_ne!(PlanStance::Flat, PlanStance::Long);
    }

    #[test]
    fn plan_rule_shape_sma_cross_fields() {
        let rule = PlanRuleShape::SmaCross {
            fast_len: 20,
            slow_len: 50,
        };
        if let PlanRuleShape::SmaCross { fast_len, slow_len } = rule {
            assert_eq!(fast_len, 20);
            assert_eq!(slow_len, 50);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn buy_and_hold_rule_shape() {
        let rule = PlanRuleShape::BuyAndHold;
        assert_eq!(rule, PlanRuleShape::BuyAndHold);
    }

    #[test]
    fn sma_current_getter_non_mutating() {
        // Validate the SmaStream::current() getter (the non-mutating read)
        // does not change the result of subsequent pushes.
        use features::Sma;
        let mut sma = Sma::new(3);
        sma.push(dec!(10));
        sma.push(dec!(20));
        sma.push(dec!(30));
        // Window full: current = 20
        let c1 = sma.current();
        let c2 = sma.current();
        assert_eq!(c1, c2, "current() must be idempotent (non-mutating)");
        assert_eq!(c1, Some(dec!(20)));
        // Push another value: current advances
        sma.push(dec!(40));
        let c3 = sma.current();
        assert_eq!(c3, Some(dec!(30)));
    }
}
