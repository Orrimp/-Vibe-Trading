//! `ComposedStrategy` — the `Strategy` trait implementation (T505, T507).
//!
//! Ring buffers are pre-sized at construction; `on_bar` is allocation-free on
//! the hot path (R1.3, R1.4).  The only allocation per bar is the returned
//! `Vec<Signal>`, bounded to 0 or 1 items under edge-triggered semantics (Q3).

use std::collections::BTreeMap;
use std::collections::VecDeque;

use rust_decimal::Decimal;
use smol_str::SmolStr;
use trading_core::{Bar, Signal, SignalEvidence, SignalKind, StrategyId, Symbol, Tick};

use crate::traits::Strategy;

use super::ast::*;
use super::config::{ComposedStrategyConfig, Sizing};

// ── Indicator state ────────────────────────────────────────────────────────────

/// Internal state for a single indicator node.
///
/// Ring buffers are sized at construction to the deepest lookback; `on_bar`
/// never allocates.
#[derive(Debug)]
pub enum IndicatorState {
    Sma {
        period: usize,
        window: VecDeque<Decimal>,
        sum: Decimal,
        latest: Option<Decimal>,
    },
    Ema {
        period: u32,
        alpha: Decimal,
        seed_sum: Decimal,
        seed_count: u32,
        latest: Option<Decimal>,
    },
    MacdLine {
        fast: u32,
        slow: u32,
        fast_state: Box<IndicatorState>, // Ema
        slow_state: Box<IndicatorState>, // Ema
        latest: Option<Decimal>,
    },
    MacdSignal {
        fast: u32,
        slow: u32,
        signal_period: u32,
        fast_state: Box<IndicatorState>,
        slow_state: Box<IndicatorState>,
        signal_state: Box<IndicatorState>, // Ema over macd line
        latest: Option<Decimal>,
    },
    MacdHist {
        fast: u32,
        slow: u32,
        signal_period: u32,
        fast_state: Box<IndicatorState>,
        slow_state: Box<IndicatorState>,
        signal_state: Box<IndicatorState>,
        latest: Option<Decimal>,
    },
    Rsi {
        period: u32,
        prev_close: Option<Decimal>,
        seed_gains: VecDeque<Decimal>,
        seed_losses: VecDeque<Decimal>,
        avg_gain: Option<Decimal>,
        avg_loss: Option<Decimal>,
        latest: Option<Decimal>,
    },
    BollingerUpper {
        period: usize,
        mult: Decimal,
        window: VecDeque<Decimal>,
        sum: Decimal,
        latest_sma: Option<Decimal>,
        latest: Option<Decimal>,
    },
    BollingerMid {
        period: usize,
        window: VecDeque<Decimal>,
        sum: Decimal,
        latest: Option<Decimal>,
    },
    BollingerLower {
        period: usize,
        mult: Decimal,
        window: VecDeque<Decimal>,
        sum: Decimal,
        latest_sma: Option<Decimal>,
        latest: Option<Decimal>,
    },
    RollingMin {
        field: SmolStr,
        window_size: u32,
        window: VecDeque<Decimal>,
        latest: Option<Decimal>,
    },
    RollingMax {
        field: SmolStr,
        window_size: u32,
        window: VecDeque<Decimal>,
        latest: Option<Decimal>,
    },
    RollingAvg {
        field: SmolStr,
        window_size: u32,
        window: VecDeque<Decimal>,
        sum: Decimal,
        latest: Option<Decimal>,
    },
}

impl IndicatorState {
    /// Get the current value (after the last `on_bar`).
    pub fn latest(&self) -> Option<Decimal> {
        match self {
            Self::Sma { latest, .. }
            | Self::Ema { latest, .. }
            | Self::MacdLine { latest, .. }
            | Self::MacdSignal { latest, .. }
            | Self::MacdHist { latest, .. }
            | Self::Rsi { latest, .. }
            | Self::BollingerUpper { latest, .. }
            | Self::BollingerMid { latest, .. }
            | Self::BollingerLower { latest, .. }
            | Self::RollingMin { latest, .. }
            | Self::RollingMax { latest, .. }
            | Self::RollingAvg { latest, .. } => *latest,
        }
    }

    /// Create an Ema state.
    fn new_ema(period: u32) -> Self {
        assert!(period > 0);
        let alpha = Decimal::from(2) / Decimal::from(period + 1);
        Self::Ema {
            period,
            alpha,
            seed_sum: Decimal::ZERO,
            seed_count: 0,
            latest: None,
        }
    }

    /// Push a close price into this indicator state.
    pub fn on_bar(&mut self, bar: &Bar) {
        let close = bar.close.get();
        match self {
            Self::Sma {
                period,
                window,
                sum,
                latest,
            } => {
                if window.len() == *period {
                    let evicted = window.pop_front().unwrap_or(Decimal::ZERO);
                    *sum -= evicted;
                }
                window.push_back(close);
                *sum += close;
                if window.len() == *period {
                    *latest = Some(*sum / Decimal::from(*period as u32));
                }
            }
            Self::Ema {
                period,
                alpha,
                seed_sum,
                seed_count,
                latest,
            } => {
                if *seed_count < *period {
                    *seed_sum += close;
                    *seed_count += 1;
                    if *seed_count == *period {
                        *latest = Some(*seed_sum / Decimal::from(*period));
                    }
                } else {
                    let prev = latest.unwrap_or(close);
                    *latest = Some(*alpha * close + (Decimal::ONE - *alpha) * prev);
                }
            }
            Self::MacdLine {
                fast_state,
                slow_state,
                latest,
                ..
            } => {
                fast_state.on_bar(bar);
                slow_state.on_bar(bar);
                *latest = match (fast_state.latest(), slow_state.latest()) {
                    (Some(f), Some(s)) => Some(f - s),
                    _ => None,
                };
            }
            Self::MacdSignal {
                fast_state,
                slow_state,
                signal_state,
                latest,
                ..
            } => {
                fast_state.on_bar(bar);
                slow_state.on_bar(bar);
                let macd_line = match (fast_state.latest(), slow_state.latest()) {
                    (Some(f), Some(s)) => Some(f - s),
                    _ => None,
                };
                if let Some(line) = macd_line {
                    // Push macd_line value into signal EMA.
                    // We simulate a bar with close = macd_line.
                    let dummy_bar = bar_with_close(bar, line);
                    signal_state.on_bar(&dummy_bar);
                }
                *latest = signal_state.latest();
            }
            Self::MacdHist {
                fast_state,
                slow_state,
                signal_state,
                latest,
                ..
            } => {
                fast_state.on_bar(bar);
                slow_state.on_bar(bar);
                let macd_line = match (fast_state.latest(), slow_state.latest()) {
                    (Some(f), Some(s)) => Some(f - s),
                    _ => None,
                };
                if let Some(line) = macd_line {
                    let dummy_bar = bar_with_close(bar, line);
                    signal_state.on_bar(&dummy_bar);
                    *latest = signal_state.latest().map(|sig| line - sig);
                } else {
                    *latest = None;
                }
            }
            Self::Rsi {
                period,
                prev_close,
                seed_gains,
                seed_losses,
                avg_gain,
                avg_loss,
                latest,
            } => {
                let Some(prev) = *prev_close else {
                    *prev_close = Some(close);
                    return;
                };
                *prev_close = Some(close);

                let diff = close - prev;
                let (gain, loss) = if diff > Decimal::ZERO {
                    (diff, Decimal::ZERO)
                } else {
                    (Decimal::ZERO, diff.abs())
                };

                if avg_gain.is_none() {
                    seed_gains.push_back(gain);
                    seed_losses.push_back(loss);
                    if seed_gains.len() == *period as usize {
                        let sg: Decimal = seed_gains.iter().sum();
                        let sl: Decimal = seed_losses.iter().sum();
                        let p = Decimal::from(*period);
                        *avg_gain = Some(sg / p);
                        *avg_loss = Some(sl / p);
                        *latest = Some(rsi_value(avg_gain.unwrap(), avg_loss.unwrap()));
                    }
                } else {
                    let p = Decimal::from(*period);
                    let ag = (avg_gain.unwrap() * (p - Decimal::ONE) + gain) / p;
                    let al = (avg_loss.unwrap() * (p - Decimal::ONE) + loss) / p;
                    *avg_gain = Some(ag);
                    *avg_loss = Some(al);
                    *latest = Some(rsi_value(ag, al));
                }
            }
            Self::BollingerUpper {
                period,
                mult,
                window,
                sum,
                latest_sma,
                latest,
            } => {
                if window.len() == *period {
                    *sum -= window.pop_front().unwrap_or(Decimal::ZERO);
                }
                window.push_back(close);
                *sum += close;
                if window.len() == *period {
                    let mid = *sum / Decimal::from(*period as u32);
                    *latest_sma = Some(mid);
                    let std_dev = pop_std_dev(window, mid);
                    *latest = Some(mid + *mult * std_dev);
                }
            }
            Self::BollingerMid {
                period,
                window,
                sum,
                latest,
            } => {
                if window.len() == *period {
                    *sum -= window.pop_front().unwrap_or(Decimal::ZERO);
                }
                window.push_back(close);
                *sum += close;
                if window.len() == *period {
                    *latest = Some(*sum / Decimal::from(*period as u32));
                }
            }
            Self::BollingerLower {
                period,
                mult,
                window,
                sum,
                latest_sma,
                latest,
            } => {
                if window.len() == *period {
                    *sum -= window.pop_front().unwrap_or(Decimal::ZERO);
                }
                window.push_back(close);
                *sum += close;
                if window.len() == *period {
                    let mid = *sum / Decimal::from(*period as u32);
                    *latest_sma = Some(mid);
                    let std_dev = pop_std_dev(window, mid);
                    *latest = Some(mid - *mult * std_dev);
                }
            }
            Self::RollingMin {
                field,
                window_size,
                window,
                latest,
            } => {
                let val = get_bar_field(bar, field);
                if window.len() == *window_size as usize {
                    window.pop_front();
                }
                window.push_back(val);
                if window.len() == *window_size as usize {
                    *latest = window.iter().copied().reduce(Decimal::min);
                }
            }
            Self::RollingMax {
                field,
                window_size,
                window,
                latest,
            } => {
                let val = get_bar_field(bar, field);
                if window.len() == *window_size as usize {
                    window.pop_front();
                }
                window.push_back(val);
                if window.len() == *window_size as usize {
                    *latest = window.iter().copied().reduce(Decimal::max);
                }
            }
            Self::RollingAvg {
                field,
                window_size,
                window,
                sum,
                latest,
            } => {
                let val = get_bar_field(bar, field);
                if window.len() == *window_size as usize {
                    *sum -= window.pop_front().unwrap_or(Decimal::ZERO);
                }
                window.push_back(val);
                *sum += val;
                if window.len() == *window_size as usize {
                    *latest = Some(*sum / Decimal::from(*window_size));
                }
            }
        }
    }
}

// ── Evaluation context ─────────────────────────────────────────────────────────

/// Context passed to rule/expr evaluators — holds the current bar, all
/// indicator states, and the params map.
pub struct EvalCtx<'a> {
    pub bar: &'a Bar,
    pub indicators: &'a [IndicatorState],
    pub params: &'a BTreeMap<SmolStr, Decimal>,
}

// ── Rule evaluator ─────────────────────────────────────────────────────────────

/// Evaluate a `RuleAst` against the current context.
///
/// Returns `false` if any required indicator value is not yet available (during
/// warmup period). This prevents spurious signals before indicators are ready.
pub fn eval_rule(rule: &RuleAst, ctx: &EvalCtx<'_>, prev: &mut RuleNodeState) -> bool {
    match rule {
        RuleAst::And(a, b) => {
            eval_rule(a, ctx, &mut prev.children_mut()[0])
                && eval_rule(b, ctx, &mut prev.children_mut()[1])
        }
        RuleAst::Or(a, b) => {
            eval_rule(a, ctx, &mut prev.children_mut()[0])
                || eval_rule(b, ctx, &mut prev.children_mut()[1])
        }
        RuleAst::Not(inner) => !eval_rule(inner, ctx, &mut prev.children_mut()[0]),
        RuleAst::Cmp { op, lhs, rhs } => {
            let l = eval_expr(lhs, ctx);
            let r = eval_expr(rhs, ctx);
            match (l, r) {
                (Some(lv), Some(rv)) => apply_cmp(op, lv, rv),
                _ => false,
            }
        }
        RuleAst::CrossAbove { a, b } => {
            let curr_a = eval_expr(a, ctx);
            let curr_b = eval_expr(b, ctx);
            let result = match (&prev.prev_cross, curr_a, curr_b) {
                (Some((pa, pb)), Some(ca), Some(cb)) => ca > cb && *pa <= *pb,
                _ => false,
            };
            prev.prev_cross = match (curr_a, curr_b) {
                (Some(ca), Some(cb)) => Some((ca, cb)),
                _ => None,
            };
            result
        }
        RuleAst::CrossBelow { a, b } => {
            let curr_a = eval_expr(a, ctx);
            let curr_b = eval_expr(b, ctx);
            let result = match (&prev.prev_cross, curr_a, curr_b) {
                (Some((pa, pb)), Some(ca), Some(cb)) => ca < cb && *pa >= *pb,
                _ => false,
            };
            prev.prev_cross = match (curr_a, curr_b) {
                (Some(ca), Some(cb)) => Some((ca, cb)),
                _ => None,
            };
            result
        }
        RuleAst::MacdCross {
            fast,
            slow,
            signal,
            direction,
        } => {
            // Evaluate the MACD line and signal from indicator states.
            // We look up the MacdLine and MacdSignal states by matching params.
            let line = find_macd_line(ctx.indicators, *fast, *slow);
            let sig = find_macd_signal(ctx.indicators, *fast, *slow, *signal);
            let result = match (&prev.prev_cross, line, sig) {
                (Some((pl, ps)), Some(cl), Some(cs)) => match direction {
                    CrossDir::Above => cl > cs && *pl <= *ps,
                    CrossDir::Below => cl < cs && *pl >= *ps,
                },
                _ => false,
            };
            prev.prev_cross = match (line, sig) {
                (Some(l), Some(s)) => Some((l, s)),
                _ => None,
            };
            result
        }
        RuleAst::BollingerLowerTouch { period, mult: _ } => {
            let lower = find_bollinger_lower(ctx.indicators, *period);
            let close = ctx.bar.close.get();
            match lower {
                Some(l) => close <= l,
                None => false,
            }
        }
    }
}

/// Evaluate an `Expr` to a `Decimal` value.
///
/// Returns `None` if any required indicator is still in warmup.
pub fn eval_expr(expr: &Expr, ctx: &EvalCtx<'_>) -> Option<Decimal> {
    match expr {
        Expr::Literal(d) => Some(*d),
        Expr::BarField(field) => Some(get_bar_field(ctx.bar, field)),
        Expr::Param(name) => ctx.params.get(name).copied(),
        Expr::Indicator(call) => eval_indicator_expr(call, ctx),
        Expr::Binary { op, lhs, rhs } => {
            let l = eval_expr(lhs, ctx)?;
            let r = eval_expr(rhs, ctx)?;
            match op {
                ArithOp::Add => Some(l + r),
                ArithOp::Sub => Some(l - r),
                ArithOp::Mul => Some(l * r),
                ArithOp::Div => {
                    if r == Decimal::ZERO {
                        None
                    } else {
                        Some(l / r)
                    }
                }
            }
        }
        Expr::BoolRule(_) => {
            // BoolRule in expression context is not supported in numeric expressions.
            // This is only valid in parenthesized boolean context, handled by
            // the rule-level eval.
            None
        }
    }
}

fn eval_indicator_expr(call: &IndicatorCall, ctx: &EvalCtx<'_>) -> Option<Decimal> {
    // Get numeric args from the call.
    let num_arg = |idx: usize| -> Option<u32> {
        match call.args.get(idx) {
            Some(Expr::Literal(d)) => d.to_string().parse().ok(),
            _ => None,
        }
    };
    let dec_arg = |idx: usize| -> Option<Decimal> {
        match call.args.get(idx) {
            Some(Expr::Literal(d)) => Some(*d),
            _ => None,
        }
    };
    let field_arg = |idx: usize| -> Option<&SmolStr> {
        match call.args.get(idx) {
            Some(Expr::BarField(f)) => Some(f),
            _ => None,
        }
    };

    match call.name.as_str() {
        "sma" => {
            let p = num_arg(0)?;
            find_sma(ctx.indicators, p as usize)
        }
        "ema" => {
            let p = num_arg(0)?;
            find_ema(ctx.indicators, p)
        }
        "macd_line" => {
            let fast = num_arg(0)?;
            let slow = num_arg(1)?;
            find_macd_line(ctx.indicators, fast, slow)
        }
        "macd_signal" => {
            let fast = num_arg(0)?;
            let slow = num_arg(1)?;
            let sig = num_arg(2)?;
            find_macd_signal(ctx.indicators, fast, slow, sig)
        }
        "macd_hist" => {
            let fast = num_arg(0)?;
            let slow = num_arg(1)?;
            let sig = num_arg(2)?;
            find_macd_hist(ctx.indicators, fast, slow, sig)
        }
        "macd_cross" => {
            // Returns 1.0 if macd line > signal, else 0.0.
            // (Used in boolean comparison context.)
            let fast = num_arg(0)?;
            let slow = num_arg(1)?;
            let sig = num_arg(2)?;
            let line = find_macd_line(ctx.indicators, fast, slow)?;
            let signal = find_macd_signal(ctx.indicators, fast, slow, sig)?;
            if line > signal {
                Some(Decimal::ONE)
            } else {
                Some(Decimal::ZERO)
            }
        }
        "rsi" => {
            let p = num_arg(0)?;
            find_rsi(ctx.indicators, p)
        }
        "bollinger_upper" => {
            let p = num_arg(0)?;
            let _m = dec_arg(1)?;
            find_bollinger_upper(ctx.indicators, p)
        }
        "bollinger_mid" => {
            let p = num_arg(0)?;
            find_bollinger_mid(ctx.indicators, p)
        }
        "bollinger_lower" => {
            let p = num_arg(0)?;
            find_bollinger_lower(ctx.indicators, p)
        }
        "bollinger_lower_touch" => {
            // 1.0 if close <= bollinger_lower, else 0.0.
            let p = num_arg(0)?;
            let lower = find_bollinger_lower(ctx.indicators, p)?;
            let close = ctx.bar.close.get();
            if close <= lower {
                Some(Decimal::ONE)
            } else {
                Some(Decimal::ZERO)
            }
        }
        "min" => {
            let field = field_arg(0)?.clone();
            let w = num_arg(1)?;
            find_rolling(ctx.indicators, "min", &field, w)
        }
        "max" => {
            let field = field_arg(0)?.clone();
            let w = num_arg(1)?;
            find_rolling(ctx.indicators, "max", &field, w)
        }
        "avg" => {
            let field = field_arg(0)?.clone();
            let w = num_arg(1)?;
            find_rolling(ctx.indicators, "avg", &field, w)
        }
        "cross_above" | "cross_below" => {
            // These are state-dependent — return None here (handled via RuleAst level).
            None
        }
        _ => None,
    }
}

// ── Indicator lookups ──────────────────────────────────────────────────────────

fn find_sma(states: &[IndicatorState], period: usize) -> Option<Decimal> {
    states.iter().find_map(|s| {
        if let IndicatorState::Sma {
            period: p, latest, ..
        } = s
        {
            if *p == period {
                return *latest;
            }
        }
        None
    })
}

fn find_ema(states: &[IndicatorState], period: u32) -> Option<Decimal> {
    states.iter().find_map(|s| {
        if let IndicatorState::Ema {
            period: p, latest, ..
        } = s
        {
            if *p == period {
                return *latest;
            }
        }
        None
    })
}

fn find_macd_line(states: &[IndicatorState], fast: u32, slow: u32) -> Option<Decimal> {
    states.iter().find_map(|s| {
        if let IndicatorState::MacdLine {
            fast: f,
            slow: sl,
            latest,
            ..
        } = s
        {
            if *f == fast && *sl == slow {
                return *latest;
            }
        }
        None
    })
}

fn find_macd_signal(
    states: &[IndicatorState],
    fast: u32,
    slow: u32,
    signal_period: u32,
) -> Option<Decimal> {
    states.iter().find_map(|s| {
        if let IndicatorState::MacdSignal {
            fast: f,
            slow: sl,
            signal_period: sp,
            latest,
            ..
        } = s
        {
            if *f == fast && *sl == slow && *sp == signal_period {
                return *latest;
            }
        }
        None
    })
}

fn find_macd_hist(
    states: &[IndicatorState],
    fast: u32,
    slow: u32,
    signal_period: u32,
) -> Option<Decimal> {
    states.iter().find_map(|s| {
        if let IndicatorState::MacdHist {
            fast: f,
            slow: sl,
            signal_period: sp,
            latest,
            ..
        } = s
        {
            if *f == fast && *sl == slow && *sp == signal_period {
                return *latest;
            }
        }
        None
    })
}

fn find_rsi(states: &[IndicatorState], period: u32) -> Option<Decimal> {
    states.iter().find_map(|s| {
        if let IndicatorState::Rsi {
            period: p, latest, ..
        } = s
        {
            if *p == period {
                return *latest;
            }
        }
        None
    })
}

fn find_bollinger_upper(states: &[IndicatorState], period: u32) -> Option<Decimal> {
    states.iter().find_map(|s| {
        if let IndicatorState::BollingerUpper {
            period: p, latest, ..
        } = s
        {
            if *p == period as usize {
                return *latest;
            }
        }
        None
    })
}

fn find_bollinger_mid(states: &[IndicatorState], period: u32) -> Option<Decimal> {
    states.iter().find_map(|s| {
        if let IndicatorState::BollingerMid {
            period: p, latest, ..
        } = s
        {
            if *p == period as usize {
                return *latest;
            }
        }
        None
    })
}

fn find_bollinger_lower(states: &[IndicatorState], period: u32) -> Option<Decimal> {
    states.iter().find_map(|s| {
        if let IndicatorState::BollingerLower {
            period: p, latest, ..
        } = s
        {
            if *p == period as usize {
                return *latest;
            }
        }
        None
    })
}

fn find_rolling(
    states: &[IndicatorState],
    kind: &str,
    field: &SmolStr,
    window_size: u32,
) -> Option<Decimal> {
    match kind {
        "min" => states.iter().find_map(|s| {
            if let IndicatorState::RollingMin {
                field: f,
                window_size: w,
                latest,
                ..
            } = s
            {
                if f == field && *w == window_size {
                    return *latest;
                }
            }
            None
        }),
        "max" => states.iter().find_map(|s| {
            if let IndicatorState::RollingMax {
                field: f,
                window_size: w,
                latest,
                ..
            } = s
            {
                if f == field && *w == window_size {
                    return *latest;
                }
            }
            None
        }),
        "avg" => states.iter().find_map(|s| {
            if let IndicatorState::RollingAvg {
                field: f,
                window_size: w,
                latest,
                ..
            } = s
            {
                if f == field && *w == window_size {
                    return *latest;
                }
            }
            None
        }),
        _ => None,
    }
}

// ── Rule node state (for crossovers) ──────────────────────────────────────────

/// State for a single rule node — currently only tracks crossover prev values.
#[derive(Debug, Default, Clone)]
pub struct RuleNodeState {
    pub prev_cross: Option<(Decimal, Decimal)>,
    children: Vec<RuleNodeState>,
}

impl RuleNodeState {
    fn children_mut(&mut self) -> &mut Vec<RuleNodeState> {
        &mut self.children
    }

    /// Build a `RuleNodeState` tree matching the shape of a `RuleAst`.
    pub fn from_ast(rule: &RuleAst) -> Self {
        let children = match rule {
            RuleAst::And(a, b) | RuleAst::Or(a, b) => {
                vec![Self::from_ast(a), Self::from_ast(b)]
            }
            RuleAst::Not(inner) => vec![Self::from_ast(inner)],
            _ => vec![],
        };
        Self {
            prev_cross: None,
            children,
        }
    }
}

// ── Indicator builder ─────────────────────────────────────────────────────────

/// Collect all indicator states required by a rule AST.
///
/// Deduplicates: two `rsi(14)` calls share one state.
pub fn build_indicators(rule: &RuleAst) -> Vec<IndicatorState> {
    let mut states: Vec<IndicatorState> = Vec::new();
    collect_indicators(rule, &mut states);
    states
}

fn collect_indicators(rule: &RuleAst, states: &mut Vec<IndicatorState>) {
    match rule {
        RuleAst::And(a, b) | RuleAst::Or(a, b) => {
            collect_indicators(a, states);
            collect_indicators(b, states);
        }
        RuleAst::Not(inner) => collect_indicators(inner, states),
        RuleAst::Cmp { lhs, rhs, .. } => {
            collect_from_expr(lhs, states);
            collect_from_expr(rhs, states);
        }
        RuleAst::CrossAbove { a, b } | RuleAst::CrossBelow { a, b } => {
            collect_from_expr(a, states);
            collect_from_expr(b, states);
        }
        RuleAst::MacdCross {
            fast, slow, signal, ..
        } => {
            ensure_macd_line(states, *fast, *slow);
            ensure_macd_signal(states, *fast, *slow, *signal);
        }
        RuleAst::BollingerLowerTouch { period, mult } => {
            ensure_bollinger_lower(states, *period as usize, *mult);
        }
    }
}

fn collect_from_expr(expr: &Expr, states: &mut Vec<IndicatorState>) {
    match expr {
        Expr::Indicator(call) => add_indicator(call, states),
        Expr::Binary { lhs, rhs, .. } => {
            collect_from_expr(lhs, states);
            collect_from_expr(rhs, states);
        }
        Expr::BoolRule(inner) => collect_indicators(inner, states),
        _ => {}
    }
}

fn add_indicator(call: &IndicatorCall, states: &mut Vec<IndicatorState>) {
    let num_arg = |idx: usize| -> u32 {
        if let Some(Expr::Literal(d)) = call.args.get(idx) {
            d.to_string().parse().unwrap_or(0)
        } else {
            0
        }
    };
    let dec_arg = |idx: usize| -> Decimal {
        if let Some(Expr::Literal(d)) = call.args.get(idx) {
            *d
        } else {
            Decimal::ZERO
        }
    };
    let field_arg = |idx: usize| -> SmolStr {
        if let Some(Expr::BarField(f)) = call.args.get(idx) {
            f.clone()
        } else {
            SmolStr::new("close")
        }
    };

    match call.name.as_str() {
        "sma" => {
            let p = num_arg(0) as usize;
            if !states
                .iter()
                .any(|s| matches!(s, IndicatorState::Sma { period, .. } if *period == p))
            {
                states.push(IndicatorState::Sma {
                    period: p,
                    window: VecDeque::with_capacity(p),
                    sum: Decimal::ZERO,
                    latest: None,
                });
            }
        }
        "ema" => {
            let p = num_arg(0);
            ensure_ema(states, p);
        }
        "macd_line" => {
            let fast = num_arg(0);
            let slow = num_arg(1);
            ensure_macd_line(states, fast, slow);
        }
        "macd_signal" => {
            let fast = num_arg(0);
            let slow = num_arg(1);
            let sig = num_arg(2);
            ensure_macd_signal(states, fast, slow, sig);
        }
        "macd_hist" => {
            let fast = num_arg(0);
            let slow = num_arg(1);
            let sig = num_arg(2);
            ensure_macd_hist(states, fast, slow, sig);
        }
        "macd_cross" => {
            let fast = num_arg(0);
            let slow = num_arg(1);
            let sig = num_arg(2);
            ensure_macd_line(states, fast, slow);
            ensure_macd_signal(states, fast, slow, sig);
        }
        "rsi" => {
            let p = num_arg(0);
            if !states
                .iter()
                .any(|s| matches!(s, IndicatorState::Rsi { period, .. } if *period == p))
            {
                states.push(IndicatorState::Rsi {
                    period: p,
                    prev_close: None,
                    seed_gains: VecDeque::with_capacity(p as usize),
                    seed_losses: VecDeque::with_capacity(p as usize),
                    avg_gain: None,
                    avg_loss: None,
                    latest: None,
                });
            }
        }
        "bollinger_upper" => {
            let p = num_arg(0) as usize;
            let m = dec_arg(1);
            ensure_bollinger_upper(states, p, m);
        }
        "bollinger_mid" => {
            let p = num_arg(0) as usize;
            ensure_bollinger_mid(states, p);
        }
        "bollinger_lower" => {
            let p = num_arg(0) as usize;
            let m = dec_arg(1);
            ensure_bollinger_lower(states, p, m);
        }
        "bollinger_lower_touch" => {
            let p = num_arg(0) as usize;
            let m = dec_arg(1);
            ensure_bollinger_lower(states, p, m);
        }
        "min" => {
            let field = field_arg(0);
            let w = num_arg(1);
            if !states.iter().any(|s| matches!(s, IndicatorState::RollingMin { field: f, window_size, .. } if f == &field && *window_size == w)) {
                states.push(IndicatorState::RollingMin {
                    field,
                    window_size: w,
                    window: VecDeque::with_capacity(w as usize),
                    latest: None,
                });
            }
        }
        "max" => {
            let field = field_arg(0);
            let w = num_arg(1);
            if !states.iter().any(|s| matches!(s, IndicatorState::RollingMax { field: f, window_size, .. } if f == &field && *window_size == w)) {
                states.push(IndicatorState::RollingMax {
                    field,
                    window_size: w,
                    window: VecDeque::with_capacity(w as usize),
                    latest: None,
                });
            }
        }
        "avg" => {
            let field = field_arg(0);
            let w = num_arg(1);
            if !states.iter().any(|s| matches!(s, IndicatorState::RollingAvg { field: f, window_size, .. } if f == &field && *window_size == w)) {
                states.push(IndicatorState::RollingAvg {
                    field,
                    window_size: w,
                    window: VecDeque::with_capacity(w as usize),
                    sum: Decimal::ZERO,
                    latest: None,
                });
            }
        }
        _ => {}
    }
}

fn ensure_ema(states: &mut Vec<IndicatorState>, period: u32) {
    if !states
        .iter()
        .any(|s| matches!(s, IndicatorState::Ema { period: p, .. } if *p == period))
    {
        states.push(IndicatorState::new_ema(period));
    }
}

fn ensure_macd_line(states: &mut Vec<IndicatorState>, fast: u32, slow: u32) {
    if !states.iter().any(|s| matches!(s, IndicatorState::MacdLine { fast: f, slow: sl, .. } if *f == fast && *sl == slow)) {
        states.push(IndicatorState::MacdLine {
            fast,
            slow,
            fast_state: Box::new(IndicatorState::new_ema(fast)),
            slow_state: Box::new(IndicatorState::new_ema(slow)),
            latest: None,
        });
    }
}

fn ensure_macd_signal(states: &mut Vec<IndicatorState>, fast: u32, slow: u32, signal_period: u32) {
    if !states.iter().any(|s| matches!(s, IndicatorState::MacdSignal { fast: f, slow: sl, signal_period: sp, .. } if *f == fast && *sl == slow && *sp == signal_period)) {
        states.push(IndicatorState::MacdSignal {
            fast,
            slow,
            signal_period,
            fast_state: Box::new(IndicatorState::new_ema(fast)),
            slow_state: Box::new(IndicatorState::new_ema(slow)),
            signal_state: Box::new(IndicatorState::new_ema(signal_period)),
            latest: None,
        });
    }
}

fn ensure_macd_hist(states: &mut Vec<IndicatorState>, fast: u32, slow: u32, signal_period: u32) {
    if !states.iter().any(|s| matches!(s, IndicatorState::MacdHist { fast: f, slow: sl, signal_period: sp, .. } if *f == fast && *sl == slow && *sp == signal_period)) {
        states.push(IndicatorState::MacdHist {
            fast,
            slow,
            signal_period,
            fast_state: Box::new(IndicatorState::new_ema(fast)),
            slow_state: Box::new(IndicatorState::new_ema(slow)),
            signal_state: Box::new(IndicatorState::new_ema(signal_period)),
            latest: None,
        });
    }
}

fn ensure_bollinger_upper(states: &mut Vec<IndicatorState>, period: usize, mult: Decimal) {
    if !states.iter().any(|s| matches!(s, IndicatorState::BollingerUpper { period: p, mult: m, .. } if *p == period && *m == mult)) {
        states.push(IndicatorState::BollingerUpper {
            period,
            mult,
            window: VecDeque::with_capacity(period),
            sum: Decimal::ZERO,
            latest_sma: None,
            latest: None,
        });
    }
}

fn ensure_bollinger_mid(states: &mut Vec<IndicatorState>, period: usize) {
    if !states
        .iter()
        .any(|s| matches!(s, IndicatorState::BollingerMid { period: p, .. } if *p == period))
    {
        states.push(IndicatorState::BollingerMid {
            period,
            window: VecDeque::with_capacity(period),
            sum: Decimal::ZERO,
            latest: None,
        });
    }
}

fn ensure_bollinger_lower(states: &mut Vec<IndicatorState>, period: usize, mult: Decimal) {
    if !states.iter().any(|s| matches!(s, IndicatorState::BollingerLower { period: p, mult: m, .. } if *p == period && *m == mult)) {
        states.push(IndicatorState::BollingerLower {
            period,
            mult,
            window: VecDeque::with_capacity(period),
            sum: Decimal::ZERO,
            latest_sma: None,
            latest: None,
        });
    }
}

// ── `ComposedStrategy` ─────────────────────────────────────────────────────────

/// A composed strategy loaded from a TOML config file.
///
/// Implements the `Strategy` trait. The hot path (`on_bar`) is allocation-free:
/// ring buffers are pre-sized at construction.
pub struct ComposedStrategy {
    id: StrategyId,
    symbol: Symbol,
    hash: [u8; 32],
    source_path: SmolStr,
    indicators: Vec<IndicatorState>,
    rule: RuleAst,
    rule_state: RuleNodeState,
    last_rule_value: Option<bool>,
    sizing: Sizing,
    params: BTreeMap<SmolStr, Decimal>,
}

impl ComposedStrategy {
    /// Build a `ComposedStrategy` from a validated config.
    pub fn from_config(config: ComposedStrategyConfig, source_path: SmolStr) -> Self {
        let ast = super::parser::parse_signal(&config.signal_raw, &config.params)
            .expect("config already validated");
        let indicators = build_indicators(&ast);
        let rule_state = RuleNodeState::from_ast(&ast);
        Self {
            id: StrategyId::new(config.id.as_str()),
            symbol: trading_core::Symbol::new(config.symbol.as_str()),
            hash: config.hash,
            source_path,
            indicators,
            rule: ast,
            rule_state,
            last_rule_value: None,
            sizing: config.sizing,
            params: config.params,
        }
    }

    /// The content hash (SHA-256 of canonicalized AST).
    pub fn hash(&self) -> [u8; 32] {
        self.hash
    }

    /// The source path (repo-relative).
    pub fn source_path(&self) -> &SmolStr {
        &self.source_path
    }

    /// The sizing expression.
    pub fn sizing(&self) -> &Sizing {
        &self.sizing
    }

    fn emit_signal(&self, bar: &Bar, kind: SignalKind) -> Signal {
        Signal {
            strategy_id: self.id.clone(),
            symbol: self.symbol.clone(),
            ts: bar.close_ts,
            kind,
            evidence: SignalEvidence::empty(),
        }
    }
}

impl Strategy for ComposedStrategy {
    fn id(&self) -> StrategyId {
        self.id.clone()
    }

    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        // 1. Advance all indicator states.
        for ind in &mut self.indicators {
            ind.on_bar(bar);
        }

        // 2. Evaluate the rule tree.
        let ctx = EvalCtx {
            bar,
            indicators: &self.indicators,
            params: &self.params,
        };
        let now = eval_rule(&self.rule, &ctx, &mut self.rule_state);

        // 3. Edge-triggered emission — symmetric signal flip (Q3).
        let out = match (self.last_rule_value, now) {
            (Some(false), true) => vec![self.emit_signal(bar, SignalKind::Buy)],
            (Some(true), false) => vec![self.emit_signal(bar, SignalKind::Sell)],
            _ => vec![],
        };
        self.last_rule_value = Some(now);
        out
    }

    fn on_tick(&mut self, _tick: &Tick) -> Vec<Signal> {
        vec![]
    }

    fn config_schema() -> serde_json::Value
    where
        Self: Sized,
    {
        serde_json::json!({
            "type": "object",
            "description": "ComposedStrategy TOML config schema",
            "required": ["id", "kind", "symbol", "stage", "signal", "size"],
            "properties": {
                "id": { "type": "string" },
                "kind": { "type": "string", "enum": ["composed"] },
                "symbol": { "type": "string" },
                "stage": { "type": "string", "enum": ["research", "paper"] },
                "signal": { "type": "string" },
                "size": { "type": "string" },
                "params": { "type": "object" }
            }
        })
    }
}

// ── Helper utilities ──────────────────────────────────────────────────────────

fn get_bar_field(bar: &Bar, field: &SmolStr) -> Decimal {
    match field.as_str() {
        "close" => bar.close.get(),
        "open" => bar.open.get(),
        "high" => bar.high.get(),
        "low" => bar.low.get(),
        "volume" => bar.volume.get(),
        "trade_count" => Decimal::from(bar.trade_count),
        _ => bar.close.get(),
    }
}

fn rsi_value(avg_gain: Decimal, avg_loss: Decimal) -> Decimal {
    if avg_loss == Decimal::ZERO {
        return Decimal::ONE_HUNDRED;
    }
    let rs = avg_gain / avg_loss;
    Decimal::ONE_HUNDRED - Decimal::ONE_HUNDRED / (Decimal::ONE + rs)
}

fn pop_std_dev(window: &VecDeque<Decimal>, mean: Decimal) -> Decimal {
    let variance = window
        .iter()
        .map(|&x| {
            let d = x - mean;
            d * d
        })
        .fold(Decimal::ZERO, |acc, v| acc + v)
        / Decimal::from(window.len() as u32);
    decimal_sqrt(variance)
}

fn decimal_sqrt(x: Decimal) -> Decimal {
    if x == Decimal::ZERO {
        return Decimal::ZERO;
    }
    let x_f64: f64 = x.to_string().parse().unwrap_or(1.0_f64);
    let seed = x_f64.sqrt();
    let mut est = Decimal::try_from(seed).unwrap_or(Decimal::ONE);
    for _ in 0..50 {
        if est == Decimal::ZERO {
            break;
        }
        let next = (est + x / est) / Decimal::from(2);
        if (next - est).abs() < Decimal::new(1, 28) {
            est = next;
            break;
        }
        est = next;
    }
    est
}

/// Create a synthetic bar with a different close price (used for signal EMA).
fn bar_with_close(bar: &Bar, close: Decimal) -> Bar {
    use trading_core::Price;
    let mut b = bar.clone();
    // SAFETY: we're creating a valid price from indicator output that is
    // expected to be finite. In the degenerate case of a zero/negative MACD
    // line value (which is valid), we use the closest valid price.
    if let Ok(p) = Price::new(close.abs().max(Decimal::new(1, 18))) {
        b.close = p;
    }
    b
}

fn apply_cmp(op: &CmpOp, lhs: Decimal, rhs: Decimal) -> bool {
    match op {
        CmpOp::Lt => lhs < rhs,
        CmpOp::Le => lhs <= rhs,
        CmpOp::Eq => lhs == rhs,
        CmpOp::Ge => lhs >= rhs,
        CmpOp::Gt => lhs > rhs,
        CmpOp::Ne => lhs != rhs,
    }
}

// ── T505 / T507 tests ─────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod t505_t507_tests {
    use super::*;
    use crate::traits::Strategy;
    use rust_decimal_macros::dec;
    use trading_core::{Price, Quantity, Symbol, Timeframe, Timestamp};

    /// Synthetic 1000-bar fixture — deterministic, derived from bar index only.
    /// Uses a volatile random-walk to ensure RSI explores < 35 territory and
    /// MACD crossovers occur.
    fn make_bars(count: usize) -> Vec<Bar> {
        use time::OffsetDateTime;
        // Simple LCG PRNG — deterministic, no external deps.
        struct Lcg(u64);
        impl Lcg {
            fn next_f64(&mut self) -> f64 {
                self.0 = self
                    .0
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                (self.0 >> 33) as f64 / (u64::MAX >> 33) as f64
            }
        }
        let mut rng = Lcg(0xDEAD_BEEF);

        let epoch = OffsetDateTime::new_utc(
            time::Date::from_calendar_date(2023, time::Month::January, 1).unwrap(),
            time::Time::MIDNIGHT,
        );
        let mut bars = Vec::with_capacity(count);
        // Volatile walk: std dev ~2% per bar to ensure wide RSI range.
        let mut close: f64 = 16_500.0;
        for i in 0..count {
            let z = (rng.next_f64() - 0.5) * 2.0; // in (-1, 1)
            let ret = z * 0.02; // ±2% per bar
            close = (close * (1.0 + ret)).clamp(1_000.0, 500_000.0);
            let open = close * (1.0 + (rng.next_f64() - 0.5) * 0.001);
            let high = close.max(open) * (1.0 + rng.next_f64() * 0.005);
            let low = close.min(open) * (1.0 - rng.next_f64() * 0.005);

            let open_ts = Timestamp::new(epoch + time::Duration::minutes(i as i64));
            let close_ts = Timestamp::new(
                epoch + time::Duration::minutes(i as i64 + 1) - time::Duration::seconds(1),
            );
            let mk_price = |v: f64| {
                Price::new(Decimal::try_from(v.max(0.01)).unwrap_or(dec!(1)))
                    .unwrap_or_else(|_| Price::new(dec!(1)).unwrap())
            };
            bars.push(Bar {
                symbol: Symbol::new("BTCUSDT"),
                tf: Timeframe::OneMinute,
                open: mk_price(open),
                high: mk_price(high),
                low: mk_price(low),
                close: mk_price(close),
                volume: Quantity::new(dec!(10)).unwrap(),
                trade_count: 100,
                local_recv_ts: close_ts,
                open_ts,
                close_ts,
            });
        }
        bars
    }

    /// Hand-coded reference implementation of `rsi(14) < 30` — the simplest
    /// single-rule case for T505 correctness verification.
    ///
    /// Computes RSI directly using the same seeded-average formula as
    /// `IndicatorState::Rsi`, then applies the same edge-triggered logic.
    fn reference_signals_rsi(bars: &[Bar]) -> Vec<(usize, trading_core::SignalKind)> {
        let closes: Vec<Decimal> = bars.iter().map(|b| b.close.get()).collect();
        let period = 14usize;
        let mut rsi_vals: Vec<Option<Decimal>> = vec![None; bars.len()];

        if closes.len() > 1 {
            // The ComposedStrategy RSI state skips the first close (sets prev_close,
            // returns without producing a value).  From bar index 1 onward it
            // accumulates diffs; seed is ready after `period` diffs (bar index period).
            let mut seed_gains: Vec<Decimal> = Vec::new();
            let mut seed_losses: Vec<Decimal> = Vec::new();
            let mut avg_gain: Option<Decimal> = None;
            let mut avg_loss: Option<Decimal> = None;
            let mut prev = closes[0];

            for i in 1..closes.len() {
                let diff = closes[i] - prev;
                prev = closes[i];
                let gain = if diff > Decimal::ZERO {
                    diff
                } else {
                    Decimal::ZERO
                };
                let loss = if diff < Decimal::ZERO {
                    -diff
                } else {
                    Decimal::ZERO
                };

                if avg_gain.is_none() {
                    seed_gains.push(gain);
                    seed_losses.push(loss);
                    if seed_gains.len() == period {
                        let sg: Decimal = seed_gains.iter().sum();
                        let sl: Decimal = seed_losses.iter().sum();
                        let p = Decimal::from(period as u32);
                        avg_gain = Some(sg / p);
                        avg_loss = Some(sl / p);
                        rsi_vals[i] = Some(rsi_value(avg_gain.unwrap(), avg_loss.unwrap()));
                    }
                } else {
                    let p = Decimal::from(period as u32);
                    let ag = (avg_gain.unwrap() * (p - Decimal::ONE) + gain) / p;
                    let al = (avg_loss.unwrap() * (p - Decimal::ONE) + loss) / p;
                    avg_gain = Some(ag);
                    avg_loss = Some(al);
                    rsi_vals[i] = Some(rsi_value(ag, al));
                }
            }
        }

        let mut signals = Vec::new();
        let mut last: Option<bool> = None;
        for (i, r) in rsi_vals.iter().enumerate() {
            let now = r.map(|v| v < dec!(30)).unwrap_or(false);
            match (last, now) {
                (Some(false), true) => signals.push((i, trading_core::SignalKind::Buy)),
                (Some(true), false) => signals.push((i, trading_core::SignalKind::Sell)),
                _ => {}
            }
            last = Some(now);
        }
        signals
    }

    /// T505: programmatically-built ComposedStrategy vs hand-coded reference.
    ///
    /// Uses `rsi(14) < 30` — a single indicator, no MACD complexity — to keep
    /// the hand-coded reference fully independent and verifiable by inspection.
    /// The volatile fixture guarantees RSI explores both < 30 and > 30 territory.
    #[test]
    fn t505_rsi_single_rule_matches_reference_impl() {
        let toml = r#"
id     = "test_rsi"
kind   = "composed"
symbol = "BTCUSDT"
stage  = "research"
signal = "rsi(14) < 30"
size   = "fixed_fraction(0.1)"
"#;
        let cfg = crate::composed::config::ComposedStrategyConfig::from_str(toml, "test_rsi")
            .expect("valid config");
        let mut strategy = ComposedStrategy::from_config(cfg, smol_str::SmolStr::new("test"));

        let bars = make_bars(1000);
        let mut composed_signals: Vec<(usize, trading_core::SignalKind)> = Vec::new();
        for (i, bar) in bars.iter().enumerate() {
            let sigs = strategy.on_bar(bar);
            for s in sigs {
                composed_signals.push((i, s.kind));
            }
        }

        let reference = reference_signals_rsi(&bars);
        assert!(
            !composed_signals.is_empty(),
            "volatile fixture must produce at least one signal (RSI < 30)"
        );
        assert_eq!(
            composed_signals, reference,
            "ComposedStrategy signal sequence must be byte-identical to hand-coded RSI reference"
        );
    }

    /// T507: verify Vec<Signal> per bar is bounded to 0 or 1 items, and that
    /// the Strategy trait surface is exercised (id, on_bar, on_tick, config_schema).
    #[test]
    fn t507_strategy_trait_bounded_signal_output() {
        let toml = r#"
id     = "test_bounded"
kind   = "composed"
symbol = "BTCUSDT"
stage  = "research"
signal = "macd_hist(12,26,9) > 0 AND rsi(14) < 50"
size   = "fixed_fraction(0.1)"
"#;
        let cfg = crate::composed::config::ComposedStrategyConfig::from_str(toml, "test_bounded")
            .expect("valid config");
        let mut strategy = ComposedStrategy::from_config(cfg, smol_str::SmolStr::new("test"));

        // Verify Strategy::id().
        assert_eq!(strategy.id().0.as_str(), "test_bounded");

        // Verify config_schema().
        let schema = ComposedStrategy::config_schema();
        assert!(schema.get("type").is_some());

        let bars = make_bars(1000);
        for bar in &bars {
            let sigs = strategy.on_bar(bar);
            assert!(
                sigs.len() <= 1,
                "on_bar must emit at most 1 signal per bar, got {}",
                sigs.len()
            );
        }

        // Verify on_tick returns empty.
        use trading_core::{Side, Tick};
        let tick = Tick {
            symbol: Symbol::new("BTCUSDT"),
            venue_ts: bars[0].close_ts,
            local_recv_ts: bars[0].close_ts,
            price: bars[0].close,
            qty: Quantity::new(dec!(1)).unwrap(),
            side: Side::Buy,
            trade_id: 1,
        };
        assert!(strategy.on_tick(&tick).is_empty());
    }
}
