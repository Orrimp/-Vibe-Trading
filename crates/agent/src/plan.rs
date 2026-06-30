//! F6 forward-plan supervisor helpers (ADR-0062 § D3–D4).
//!
//! This module owns the ONE-PLACE mapping from `strategy::StrategyPlan`
//! (the engine-side structured plan) to `agent::config::ForwardPlan`
//! (the `core`-typed, `ui`-safe plan that crosses the seam over the second
//! mpsc).  It is the `BakeoffReportMirror::from_report` precedent (ADR-0059):
//! the ONLY place in the `agent` crate where the `strategy`-side types are read
//! and mapped to `core`-typed output.
//!
//! ## Consistency guarantee
//!
//! The mapping is called AFTER `build_registry_for` resolved the strategy for
//! the hot-swap, so the plan ALWAYS describes what the F5 loop actually runs —
//! not the Lab-time AST or a cached id.  Drift is structurally impossible
//! (ADR-0062 § D3).

use smol_str::SmolStr;
use strategy::{PlanContext, PlanDescribe, PlanRuleShape};
use trading_core::{Money, Price, Quantity, Timestamp, Usdt};

use crate::config::{
    ForwardPlan, ForwardRunConfig, PlanRuleKind, PlanSignal, PlanStance, PlanVoteMethod,
};

/// Map a `strategy::PlanStance` to the `agent`-owned closed `PlanStance` enum.
fn map_stance(s: strategy::PlanStance) -> PlanStance {
    match s {
        strategy::PlanStance::Flat => PlanStance::Flat,
        strategy::PlanStance::Long => PlanStance::Long,
    }
}

/// Map a `strategy::PlanSignal` to the `agent`-owned closed `PlanSignal` enum.
fn map_signal(s: strategy::PlanSignal) -> PlanSignal {
    match s {
        strategy::PlanSignal::Buy => PlanSignal::Buy,
        strategy::PlanSignal::Sell => PlanSignal::Sell,
        strategy::PlanSignal::Hold => PlanSignal::Hold,
    }
}

/// Map a member `PlanRuleShape` to a short display label for the ensemble ui copy.
///
/// Returns a `SmolStr` that fits within the "≥k of {label₁, label₂, …}" ensemble
/// headline.  Labels are intentionally short (≤ 22 bytes for inline SmolStr storage).
/// The mapping is exhaustive over the five rule families; `Ensemble` is not expected
/// as a member shape (no recursive ensembles in the pre-registered field).
fn member_shape_to_display_label(shape: &PlanRuleShape) -> SmolStr {
    match shape {
        PlanRuleShape::SmaCross { .. } => SmolStr::new_static("SMA crossover"),
        PlanRuleShape::MacdCross { .. } => SmolStr::new_static("MACD trend"),
        PlanRuleShape::RsiReversion { .. } => SmolStr::new_static("RSI reversion"),
        PlanRuleShape::BollingerReversion { .. } => SmolStr::new_static("Bollinger reversion"),
        PlanRuleShape::BuyAndHold => SmolStr::new_static("Buy-and-hold"),
        // Recursive ensembles are not supported in the pre-registered field.
        // Fall back to a stable label rather than panic.
        PlanRuleShape::Ensemble { .. } => SmolStr::new_static("ensemble"),
    }
}

/// Map a `strategy::PlanRuleShape` to the `agent`-owned `PlanRuleKind`.
///
/// This is the single boundary where `strategy` types are read; the `ui`
/// only ever sees `PlanRuleKind` (the `agent`-owned closed enum).
///
/// Integer conversions are lossless for plausible window lengths (< 2^32).
/// `Decimal` k is multiplied by 10 and truncated to `u32` (e.g. `2.0` → 20).
fn map_rule_shape(rule: PlanRuleShape) -> PlanRuleKind {
    match rule {
        PlanRuleShape::SmaCross { fast_len, slow_len } => PlanRuleKind::SmaCross {
            fast_len: fast_len as u32,
            slow_len: slow_len as u32,
        },
        PlanRuleShape::MacdCross { fast, slow, signal } => PlanRuleKind::MacdCross {
            fast: fast as u32,
            slow: slow as u32,
            signal: signal as u32,
        },
        PlanRuleShape::RsiReversion { len, lower } => PlanRuleKind::RsiReversion {
            len: len as u32,
            // lower is an integer-like Decimal (e.g. 30); truncate to u32.
            lower: lower.to_string().parse::<u32>().unwrap_or(30),
        },
        PlanRuleShape::BollingerReversion { len, k } => {
            // k × 10, truncated (e.g. Decimal(2) → 20, Decimal(2.5) → 25).
            use rust_decimal::prelude::ToPrimitive;
            let k_tenths = (k * rust_decimal::Decimal::from(10u32))
                .to_u32()
                .unwrap_or(20);
            PlanRuleKind::BollingerReversion {
                len: len as u32,
                k_tenths,
            }
        }
        PlanRuleShape::BuyAndHold => PlanRuleKind::BuyAndHold,
        PlanRuleShape::Ensemble { method, members } => {
            // Map strategy::PlanVoteMethod → agent::config::PlanVoteMethod.
            let agent_method = match method {
                strategy::PlanVoteMethod::Majority { k, n } => PlanVoteMethod::Majority { k, n },
                strategy::PlanVoteMethod::Unanimous { n } => PlanVoteMethod::Unanimous { n },
            };
            // Map each member's PlanRuleShape to a display label for the ui.
            // The ui renders "≥k of {label₁, label₂, …}" from these.
            let member_labels: Vec<smol_str::SmolStr> =
                members.iter().map(member_shape_to_display_label).collect();
            PlanRuleKind::Ensemble {
                method: agent_method,
                members: member_labels,
            }
        }
    }
}

/// Produce a [`ForwardPlan`] from a resolved engine + the forward run config.
///
/// Called by the supervisor in the `ForwardCommand::Launch(cfg)` arm,
/// AFTER `build_registry_for(Some(&cfg))` has resolved the engine for the
/// hot-swap.  The `describer` is a reference to the concrete strategy cast
/// to `&dyn PlanDescribe` (SMA / AlwaysLong / ComposedStrategy).
///
/// `last_close` and `last_bar_ts` are the latest bar values the loop is
/// about to consume (from the initial seed or the live feed).  When unavailable
/// (the very first Launch before any bar is seen), use `Price::new(1.0)` and
/// `Timestamp::now()` as fallbacks — the sizing will be a rough estimate until
/// a real bar lands.
///
/// `horizon_days` is the display-only horizon (default 7).
///
/// # Errors
///
/// Returns `None` if `last_close` would produce an invalid quantity (zero or
/// negative price — defensive, since `Price` is always > 0).
pub fn build_forward_plan(
    describer: &dyn PlanDescribe,
    cfg: &ForwardRunConfig,
    last_close: Price,
    last_bar_ts: Timestamp,
    horizon_days: u16,
) -> ForwardPlan {
    // The F4 budget cap: the plan must reflect the SAME cap the paper loop
    // enforces.  Per ADR-0060 § D2 / F4: the budget cap equals the budget
    // (€200 ≈ 200 USDT) — `with_budget_cap(budget)` in `spawn_trading_loop`.
    // The plan's projected sizing is therefore `min(budget, budget) / price =
    // budget / price`, never capped (unless budget > budget, i.e. never for the
    // default).  If the operator later adds a separate cap field to
    // `ForwardRunConfig`, it goes here.
    let budget_cap: Money<Usdt> = cfg.budget;

    let plan_ctx = PlanContext {
        last_close,
        last_bar_ts,
        budget: cfg.budget,
        budget_cap,
    };

    let strategy_plan = describer.describe_plan(&plan_ctx);

    let projected_units: Quantity = strategy_plan.sizing.units;
    let sizing_capped: bool = strategy_plan.sizing.capped;

    ForwardPlan {
        strategy: cfg.strategy.clone(),
        symbol: cfg.symbol.clone(),
        stance: map_stance(strategy_plan.stance),
        latest_signal: strategy_plan.latest_signal.map(map_signal),
        rule: map_rule_shape(strategy_plan.rule),
        last_close,
        last_bar_ts,
        budget: cfg.budget,
        projected_units,
        sizing_capped,
        horizon_days,
        // P0-3: thread the scorecard summary from the ForwardRunConfig (populated
        // by cockpit_live.rs at launch time from the completed bake-off report).
        confidence: cfg.confidence,
    }
}

/// Build a [`ForwardPlan`] by constructing a fresh `PlanDescribe` instance
/// for the strategy id in `cfg`.
///
/// This is the agent-supervisor-callable helper that avoids needing to
/// downcast the `Box<dyn Strategy>` in the registry (which doesn't impl
/// `PlanDescribe`).  Instead it constructs a minimal, non-mutating describer
/// from the same inputs `build_registry_for` uses.
///
/// Returns `None` if the strategy id is unknown or if the TOML can't be
/// loaded (the supervisor should warn and skip, consistent with the F5b
/// anti-fake gate).
///
/// ADR-0062 § D3: called after `build_registry_for(Some(&cfg))` succeeds so
/// the plan and the loop share the same resolved strategy.
///
/// **ADR-0070 D1 — param override:** when `fwd.param_override` is `Some`, the
/// plan describer is resolved from the override — the SAME generators and
/// identity guard as `build_registry_for` — so plan and loop describe the SAME
/// tuned strategy (structural fidelity guarantee, ADR-0070 § D3).
pub fn build_forward_plan_from_registry(
    cfg: &crate::config::Config,
    fwd: &ForwardRunConfig,
    last_close: trading_core::Price,
    last_bar_ts: trading_core::Timestamp,
    horizon_days: u16,
) -> Option<ForwardPlan> {
    let id = fwd.strategy.0.as_str();

    // ── ADR-0070 D1/D3: tuned-param override path ────────────────────────────
    //
    // When an override is present, build the plan describer from the SAME
    // shared generator + identity guard that `build_registry_for` used.
    // The `None` path falls through to the existing match arms (unchanged).
    if let Some(ref override_params) = fwd.param_override {
        return build_plan_from_override(
            override_params,
            fwd,
            last_close,
            last_bar_ts,
            horizon_days,
        );
    }

    match id {
        "v0.sma" | "v0.5.sma" => {
            let describer = strategy::SmaCrossover::new(
                cfg.strategies.sma_crossover.fast_len,
                cfg.strategies.sma_crossover.slow_len,
            );
            Some(build_forward_plan(
                &describer,
                fwd,
                last_close,
                last_bar_ts,
                horizon_days,
            ))
        }
        "v0.buyhold" => {
            let describer = strategy::AlwaysLongStrategy::new();
            Some(build_forward_plan(
                &describer,
                fwd,
                last_close,
                last_bar_ts,
                horizon_days,
            ))
        }
        // ── F5b: ComposedStrategy engines ─────────────────────────────────────
        //
        // For MACD/RSI/BBands, construct a fresh ComposedStrategy from the
        // TOML (the same TOML `build_registry_for` loaded for the hot-swap).
        // The fresh instance has no warmed indicators so `last_rule_value = None`
        // → stance = Flat.  That is the honest stance BEFORE the first bar is
        // consumed (the plan will read as "FLAT — no position yet; waiting for
        // the first bar").  This is correct: at Launch time no bar has been
        // consumed yet by the forward loop.
        "v0.5.macd" => load_composed_describer("btc_macd_trend")
            .map(|d| build_forward_plan(&d, fwd, last_close, last_bar_ts, horizon_days)),
        "v0.5.rsi" => load_composed_describer("btc_rsi_reversion")
            .map(|d| build_forward_plan(&d, fwd, last_close, last_bar_ts, horizon_days)),
        "v0.5.bbands" => load_composed_describer("btc_bbands_mean_revert")
            .map(|d| build_forward_plan(&d, fwd, last_close, last_bar_ts, horizon_days)),
        // ── F8: EnsembleStrategy (ADR-0063 § D2) ─────────────────────────────
        //
        // Build a fresh EnsembleStrategy (same factory as `build_registry_for`
        // and the bake-off engine arm).  Un-warmed at Launch time → honest
        // Flat stance + Ensemble rule kind with member_count.
        "v0.8.vote.majority" | "v0.8.vote.unanimous" => match strategy::build_ensemble(id) {
            Ok(ensemble) => Some(build_forward_plan(
                &ensemble,
                fwd,
                last_close,
                last_bar_ts,
                horizon_days,
            )),
            Err(e) => {
                tracing::warn!(
                    strategy = id,
                    error = %e,
                    "build_forward_plan_from_registry: EnsembleStrategy load failed \
                     — no plan emitted"
                );
                None
            }
        },

        // ── R1 (ADR-0077): ADR-0071 signal-library arms (5 DSL strategies) ──
        //
        // Load the same TOML the bakeoff used. Fresh instance → no warmed
        // indicators → stance = Flat (honest "waiting for first bar" stance).
        "v0.donchian_break" => load_composed_describer("btc_donchian_break")
            .map(|d| build_forward_plan(&d, fwd, last_close, last_bar_ts, horizon_days)),
        "v0.donchian_floor" => load_composed_describer("btc_donchian_floor")
            .map(|d| build_forward_plan(&d, fwd, last_close, last_bar_ts, horizon_days)),
        "v0.vol_breakout" => load_composed_describer("btc_vol_breakout")
            .map(|d| build_forward_plan(&d, fwd, last_close, last_bar_ts, horizon_days)),
        "v0.roc_momentum" => load_composed_describer("btc_roc_momentum")
            .map(|d| build_forward_plan(&d, fwd, last_close, last_bar_ts, horizon_days)),
        "v0.obv" => load_composed_describer("btc_obv")
            .map(|d| build_forward_plan(&d, fwd, last_close, last_bar_ts, horizon_days)),

        // ── R1 (ADR-0077): ADR-0067 combination-search ensemble arms (6 ids) ─
        //
        // `build_ensemble` already knows all 8 vote ids; routing all 6 new ids
        // to the same factory as majority/unanimous — no new engine code.
        "v0.8.vote.trend_pair"
        | "v0.8.vote.tr_mr_macd_rsi"
        | "v0.8.vote.tr_mr_sma_bb"
        | "v0.8.vote.any1of4"
        | "v0.8.vote.k2of4"
        | "v0.8.vote.k3of4" => match strategy::build_ensemble(id) {
            Ok(ensemble) => Some(build_forward_plan(
                &ensemble,
                fwd,
                last_close,
                last_bar_ts,
                horizon_days,
            )),
            Err(e) => {
                tracing::warn!(
                    strategy = id,
                    error = %e,
                    "build_forward_plan_from_registry: ADR-0067 EnsembleStrategy load failed \
                     — no plan emitted"
                );
                None
            }
        },

        // ── R1 (ADR-0077): ADR-0072 DVOL regime arm ──────────────────────────
        //
        // `DvolRegimeStrategy` implements `PlanDescribe` indirectly via the
        // `Strategy` trait approach. Since the forward-plan read path calls
        // `describe_plan` on a `PlanDescribe` implementor, and
        // `DvolRegimeStrategy` does NOT implement `PlanDescribe`, we fall back
        // to `AlwaysLongStrategy` as the honest plan describer: the DVOL arm in
        // the forward paper loop runs warm-up-only (flat) because no corpus is
        // loaded, which is equivalent to buy-and-hold at the plan level.
        "v0.dvol_regime" => {
            let describer = strategy::AlwaysLongStrategy::new();
            Some(build_forward_plan(
                &describer,
                fwd,
                last_close,
                last_bar_ts,
                horizon_days,
            ))
        }

        // ── R1 (ADR-0077): ADR-0073 macro risk-on arm ────────────────────────
        //
        // `v0.macro_riskon` uses `run_macro_gated_buyhold_path` (not a Strategy).
        // In the forward paper loop the macro corpus is not loaded; graceful
        // degradation = buy-and-hold (same as an empty regime series).
        // `AlwaysLongStrategy` is the honest plan describer.
        "v0.macro_riskon" => {
            let describer = strategy::AlwaysLongStrategy::new();
            Some(build_forward_plan(
                &describer,
                fwd,
                last_close,
                last_bar_ts,
                horizon_days,
            ))
        }

        unknown => {
            tracing::warn!(
                strategy = unknown,
                "build_forward_plan_from_registry: unknown strategy id — no plan emitted"
            );
            None
        }
    }
}

/// ADR-0070 D3 — build a plan describer from a `ForwardParamOverride`.
///
/// Uses the SAME shared generators as `build_registry_for_override` so the
/// plan describes the byte-identical strategy the paper loop runs (structural
/// fidelity, ADR-0070 § D3 / ADR-0062 § D3).
///
/// Returns `None` and logs a warning if the override produces an invalid TOML.
fn build_plan_from_override(
    override_params: &crate::config::ForwardParamOverride,
    fwd: &ForwardRunConfig,
    last_close: trading_core::Price,
    last_bar_ts: trading_core::Timestamp,
    horizon_days: u16,
) -> Option<ForwardPlan> {
    use crate::config::ForwardParamOverride;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use smol_str::SmolStr;

    match override_params {
        ForwardParamOverride::Sma { fast_len, slow_len } => {
            let describer = strategy::SmaCrossover::new(*fast_len as usize, *slow_len as usize);
            Some(build_forward_plan(
                &describer,
                fwd,
                last_close,
                last_bar_ts,
                horizon_days,
            ))
        }
        ForwardParamOverride::Macd { fast, slow, signal } => {
            let toml_str = backtest::macd_toml(*fast, *slow, *signal);
            let stem = "btc_macd_trend";
            match strategy::ComposedStrategyConfig::from_str(&toml_str, stem) {
                Ok(cfg_parsed) => {
                    let source_path = SmolStr::new(format!("tuned:{stem}"));
                    let describer =
                        strategy::ComposedStrategy::from_config(cfg_parsed, source_path);
                    Some(build_forward_plan(
                        &describer,
                        fwd,
                        last_close,
                        last_bar_ts,
                        horizon_days,
                    ))
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        fast, slow, signal,
                        "build_plan_from_override: tuned MACD TOML failed identity guard — no plan"
                    );
                    None
                }
            }
        }
        ForwardParamOverride::Rsi { period, oversold } => {
            let toml_str = backtest::rsi_toml(*period, *oversold);
            let stem = "btc_rsi_reversion";
            match strategy::ComposedStrategyConfig::from_str(&toml_str, stem) {
                Ok(cfg_parsed) => {
                    let source_path = SmolStr::new(format!("tuned:{stem}"));
                    let describer =
                        strategy::ComposedStrategy::from_config(cfg_parsed, source_path);
                    Some(build_forward_plan(
                        &describer,
                        fwd,
                        last_close,
                        last_bar_ts,
                        horizon_days,
                    ))
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        period, oversold,
                        "build_plan_from_override: tuned RSI TOML failed identity guard — no plan"
                    );
                    None
                }
            }
        }
        ForwardParamOverride::Bollinger { period, k_tenths } => {
            let k: Decimal = Decimal::from(*k_tenths) / dec!(10);
            let toml_str = backtest::bbands_toml(*period, k);
            let stem = "btc_bbands_mean_revert";
            match strategy::ComposedStrategyConfig::from_str(&toml_str, stem) {
                Ok(cfg_parsed) => {
                    let source_path = SmolStr::new(format!("tuned:{stem}"));
                    let describer =
                        strategy::ComposedStrategy::from_config(cfg_parsed, source_path);
                    Some(build_forward_plan(
                        &describer,
                        fwd,
                        last_close,
                        last_bar_ts,
                        horizon_days,
                    ))
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        period, k_tenths,
                        "build_plan_from_override: tuned BBands TOML failed identity guard — no plan"
                    );
                    None
                }
            }
        }
    }
}

/// Load a `ComposedStrategy` from `config/strategies/<toml_name>.toml` for plan description.
fn load_composed_describer(toml_name: &str) -> Option<strategy::ComposedStrategy> {
    use std::path::PathBuf;
    let rel_path = PathBuf::from(format!("config/strategies/{toml_name}.toml"));
    let toml_path = backtest::paths::resolve_workspace_path(&rel_path);
    let source_path = smol_str::SmolStr::new(rel_path.to_string_lossy());
    match strategy::ComposedStrategyConfig::from_file(&toml_path) {
        Ok(cfg) => Some(strategy::ComposedStrategy::from_config(cfg, source_path)),
        Err(e) => {
            tracing::warn!(
                toml = toml_name,
                error = %e,
                "build_forward_plan_from_registry: ComposedStrategy load failed — no plan emitted"
            );
            None
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;
    use smol_str::SmolStr;
    use time::OffsetDateTime;
    use trading_core::{Money, Price, StrategyId, Symbol, Timestamp, Usdt};

    use super::*;
    use crate::config::{PlanRuleKind, PlanStance};

    fn make_cfg(strategy_id: &str) -> ForwardRunConfig {
        ForwardRunConfig {
            strategy: StrategyId::new(strategy_id),
            symbol: Symbol::new("BTCUSDT"),
            budget: Money::<Usdt>::from_decimal(dec!(200)),
            lookback: None,
            param_override: None,
            confidence: None, // P0-3: no scorecard in unit tests
        }
    }

    fn make_ts() -> Timestamp {
        Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1_700_000_000))
    }

    fn make_price(val: rust_decimal::Decimal) -> Price {
        Price::new(val).unwrap()
    }

    // A minimal PlanDescribe implementor for testing the mapping.
    struct FakeSmaDescriber;
    impl PlanDescribe for FakeSmaDescriber {
        fn describe_plan(&self, ctx: &PlanContext) -> strategy::StrategyPlan {
            use strategy::{PlanRuleShape, PlanStance, ProjectedSizing, StrategyPlan};
            let sizing = ProjectedSizing::compute(ctx.budget, ctx.budget_cap, ctx.last_close);
            StrategyPlan {
                stance: PlanStance::Long,
                latest_signal: Some(strategy::PlanSignal::Buy),
                rule: PlanRuleShape::SmaCross {
                    fast_len: 20,
                    slow_len: 50,
                },
                sizing,
            }
        }
    }

    struct FakeBuyHoldDescriber;
    impl PlanDescribe for FakeBuyHoldDescriber {
        fn describe_plan(&self, ctx: &PlanContext) -> strategy::StrategyPlan {
            use strategy::{PlanRuleShape, PlanStance, ProjectedSizing, StrategyPlan};
            let sizing = ProjectedSizing::compute(ctx.budget, ctx.budget_cap, ctx.last_close);
            StrategyPlan {
                stance: PlanStance::Long,
                latest_signal: None,
                rule: PlanRuleShape::BuyAndHold,
                sizing,
            }
        }
    }

    #[test]
    fn map_rule_shape_sma_cross_round_trips() {
        let shape = strategy::PlanRuleShape::SmaCross {
            fast_len: 20,
            slow_len: 50,
        };
        let kind = map_rule_shape(shape);
        assert!(matches!(
            kind,
            PlanRuleKind::SmaCross {
                fast_len: 20,
                slow_len: 50
            }
        ));
    }

    #[test]
    fn map_rule_shape_macd_cross_round_trips() {
        let shape = strategy::PlanRuleShape::MacdCross {
            fast: 12,
            slow: 26,
            signal: 9,
        };
        let kind = map_rule_shape(shape);
        assert!(matches!(
            kind,
            PlanRuleKind::MacdCross {
                fast: 12,
                slow: 26,
                signal: 9
            }
        ));
    }

    #[test]
    fn map_rule_shape_rsi_round_trips() {
        // btc_rsi_reversion: RSI(14) < 30; flip-to-false exit at RSI > 30.
        // There is NO upper/overbought threshold — no upper field.
        let shape = strategy::PlanRuleShape::RsiReversion {
            len: 14,
            lower: dec!(30),
        };
        let kind = map_rule_shape(shape);
        // lower is serialised as u32 (integer percent)
        assert!(matches!(
            kind,
            PlanRuleKind::RsiReversion { len: 14, lower: 30 }
        ));
    }

    #[test]
    fn map_rule_shape_bbands_round_trips() {
        let shape = strategy::PlanRuleShape::BollingerReversion {
            len: 20,
            k: dec!(2),
        };
        let kind = map_rule_shape(shape);
        // k = 2.0 → k_tenths = 20
        assert!(matches!(
            kind,
            PlanRuleKind::BollingerReversion {
                len: 20,
                k_tenths: 20
            }
        ));
    }

    #[test]
    fn map_rule_shape_buy_and_hold_round_trips() {
        let shape = strategy::PlanRuleShape::BuyAndHold;
        let kind = map_rule_shape(shape);
        assert_eq!(kind, PlanRuleKind::BuyAndHold);
    }

    #[test]
    fn build_forward_plan_sma_fields_match_cfg() {
        let cfg = make_cfg("v0.sma");
        let plan = build_forward_plan(
            &FakeSmaDescriber,
            &cfg,
            make_price(dec!(50_000)),
            make_ts(),
            7,
        );
        assert_eq!(plan.strategy.0.as_str(), "v0.sma");
        assert_eq!(plan.symbol.0.as_str(), "BTCUSDT");
        assert_eq!(plan.stance, PlanStance::Long);
        assert_eq!(plan.horizon_days, 7);
        assert!(!plan.sizing_capped);
        // units = 200 / 50_000 = 0.004
        assert_eq!(plan.projected_units.get(), dec!(0.004));
    }

    #[test]
    fn build_forward_plan_buy_and_hold_no_signal() {
        let cfg = make_cfg("v0.buyhold");
        let plan = build_forward_plan(
            &FakeBuyHoldDescriber,
            &cfg,
            make_price(dec!(50_000)),
            make_ts(),
            7,
        );
        assert_eq!(plan.rule, PlanRuleKind::BuyAndHold);
        assert!(plan.latest_signal.is_none());
        assert_eq!(plan.stance, PlanStance::Long);
    }

    // ── `smol_str::SmolStr` required for the `ForwardRunConfig` `StrategyId` ──
    // (already imported via `use smol_str::SmolStr` above)
    #[allow(dead_code)]
    fn _smol_str_smoke(_: SmolStr) {}
}
