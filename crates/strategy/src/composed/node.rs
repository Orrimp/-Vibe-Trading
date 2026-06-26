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
    /// ADR-0071 — On-Balance Volume (OBV).
    ///
    /// Recurrence: OBV_0 = 0 (seeded on bar 0, available immediately as `Some(0)`);
    /// OBV_t = OBV_{t-1} + sign(close_t − close_{t-1}) · volume_t.
    /// All math in `Decimal` (no f64). `prev_close` = `None` before bar 0.
    Obv {
        prev_close: Option<Decimal>,
        acc: Decimal,
        latest: Option<Decimal>,
    },
    /// ADR-0071 — N-bar simple moving average of the OBV series.
    ///
    /// Owns an inner `Obv` state (the `MacdLine`-owns-EMA pattern). Returns
    /// `None` during warm-up (< N OBV values pushed).
    ObvAvg {
        period: u32,
        obv: Box<IndicatorState>,
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
            | Self::RollingAvg { latest, .. }
            // ADR-0071 OBV indicators.
            | Self::Obv { latest, .. }
            | Self::ObvAvg { latest, .. } => *latest,
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
            // ADR-0071 — OBV recurrence.
            // Bar 0: seed prev_close, set acc = 0, emit Some(0).
            // Bar t≥1: acc += sign(close − prev_close) · volume.
            Self::Obv {
                prev_close,
                acc,
                latest,
            } => {
                let close = bar.close.get();
                let volume = bar.volume.get();
                match *prev_close {
                    None => {
                        // Bar 0: seed, accumulator = 0, available immediately.
                        *prev_close = Some(close);
                        *acc = Decimal::ZERO;
                        *latest = Some(Decimal::ZERO);
                    }
                    Some(prev) => {
                        let delta = close - prev;
                        let sign = if delta > Decimal::ZERO {
                            Decimal::ONE
                        } else if delta < Decimal::ZERO {
                            Decimal::NEGATIVE_ONE
                        } else {
                            Decimal::ZERO
                        };
                        *acc += sign * volume;
                        *prev_close = Some(close);
                        *latest = Some(*acc);
                    }
                }
            }
            // ADR-0071 — OBV moving average: advance inner OBV, roll a window
            // over its values (mirrors RollingAvg but over OBV not a bar field).
            Self::ObvAvg {
                period,
                obv,
                window,
                sum,
                latest,
            } => {
                obv.on_bar(bar);
                if let Some(obv_val) = obv.latest() {
                    if window.len() == *period as usize {
                        *sum -= window.pop_front().unwrap_or(Decimal::ZERO);
                    }
                    window.push_back(obv_val);
                    *sum += obv_val;
                    if window.len() == *period as usize {
                        *latest = Some(*sum / Decimal::from(*period));
                    }
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
        // ADR-0071 — OBV indicator lookup (0-arg call `obv()`).
        "obv" => find_obv(ctx.indicators),
        // ADR-0071 — OBV moving average lookup (`obv_avg(N)`).
        "obv_avg" => {
            let p = num_arg(0)?;
            find_obv_avg(ctx.indicators, p)
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
            && *p == period
        {
            return *latest;
        }
        None
    })
}

fn find_ema(states: &[IndicatorState], period: u32) -> Option<Decimal> {
    states.iter().find_map(|s| {
        if let IndicatorState::Ema {
            period: p, latest, ..
        } = s
            && *p == period
        {
            return *latest;
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
            && *f == fast
            && *sl == slow
        {
            return *latest;
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
            && *f == fast
            && *sl == slow
            && *sp == signal_period
        {
            return *latest;
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
            && *f == fast
            && *sl == slow
            && *sp == signal_period
        {
            return *latest;
        }
        None
    })
}

fn find_rsi(states: &[IndicatorState], period: u32) -> Option<Decimal> {
    states.iter().find_map(|s| {
        if let IndicatorState::Rsi {
            period: p, latest, ..
        } = s
            && *p == period
        {
            return *latest;
        }
        None
    })
}

fn find_bollinger_upper(states: &[IndicatorState], period: u32) -> Option<Decimal> {
    states.iter().find_map(|s| {
        if let IndicatorState::BollingerUpper {
            period: p, latest, ..
        } = s
            && *p == period as usize
        {
            return *latest;
        }
        None
    })
}

fn find_bollinger_mid(states: &[IndicatorState], period: u32) -> Option<Decimal> {
    states.iter().find_map(|s| {
        if let IndicatorState::BollingerMid {
            period: p, latest, ..
        } = s
            && *p == period as usize
        {
            return *latest;
        }
        None
    })
}

fn find_bollinger_lower(states: &[IndicatorState], period: u32) -> Option<Decimal> {
    states.iter().find_map(|s| {
        if let IndicatorState::BollingerLower {
            period: p, latest, ..
        } = s
            && *p == period as usize
        {
            return *latest;
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
                && f == field
                && *w == window_size
            {
                return *latest;
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
                && f == field
                && *w == window_size
            {
                return *latest;
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
                && f == field
                && *w == window_size
            {
                return *latest;
            }
            None
        }),
        _ => None,
    }
}

/// ADR-0071 — find the top-level `Obv` state. Returns its latest value.
fn find_obv(states: &[IndicatorState]) -> Option<Decimal> {
    states.iter().find_map(|s| {
        if let IndicatorState::Obv { latest, .. } = s {
            return *latest;
        }
        None
    })
}

/// ADR-0071 — find the `ObvAvg` state with the given period. Returns its latest value.
fn find_obv_avg(states: &[IndicatorState], period: u32) -> Option<Decimal> {
    states.iter().find_map(|s| {
        if let IndicatorState::ObvAvg {
            period: p, latest, ..
        } = s
            && *p == period
        {
            return *latest;
        }
        None
    })
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
        // ADR-0071 — `obv()` (0-arity): add at most one top-level Obv state.
        "obv" => {
            if !states
                .iter()
                .any(|s| matches!(s, IndicatorState::Obv { .. }))
            {
                states.push(IndicatorState::Obv {
                    prev_close: None,
                    acc: Decimal::ZERO,
                    latest: None,
                });
            }
        }
        // ADR-0071 — `obv_avg(N)` (1-arity): owns its inner Obv state.
        // Also ensure the top-level Obv is present so `obv()` comparisons in the
        // same signal work without a separate `obv()` term (symmetry guarantee).
        "obv_avg" => {
            let period = num_arg(0);
            if !states
                .iter()
                .any(|s| matches!(s, IndicatorState::ObvAvg { period: p, .. } if *p == period))
            {
                states.push(IndicatorState::ObvAvg {
                    period,
                    obv: Box::new(IndicatorState::Obv {
                        prev_close: None,
                        acc: Decimal::ZERO,
                        latest: None,
                    }),
                    window: VecDeque::with_capacity(period as usize),
                    sum: Decimal::ZERO,
                    latest: None,
                });
            }
            // Ensure the independent top-level Obv exists so a bare `obv()` in the
            // same expression (e.g. `obv() > obv_avg(20)`) can be evaluated separately.
            if !states
                .iter()
                .any(|s| matches!(s, IndicatorState::Obv { .. }))
            {
                states.push(IndicatorState::Obv {
                    prev_close: None,
                    acc: Decimal::ZERO,
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
    /// The symbol from the TOML config. Not used in signal emission (signals use
    /// `bar.symbol` for multi-symbol safety) but retained for diagnostic purposes
    /// and to preserve the round-trip TOML → struct → metadata.
    #[allow(dead_code)]
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

    /// Non-mutating read: the last evaluated rule value (true=rule fired, false=rule off).
    ///
    /// Used by `PlanDescribe::describe_plan` (F6, ADR-0062 § D2) to derive the
    /// current stance without advancing any indicator state.
    /// Returns `None` if no bar has been consumed yet (indicators not warmed).
    #[must_use]
    pub fn last_rule_value(&self) -> Option<bool> {
        self.last_rule_value
    }

    /// Non-mutating read: the strategy id string.
    ///
    /// Used by `PlanDescribe` to identify which rule family to describe.
    #[must_use]
    pub fn id_str(&self) -> &str {
        self.id.0.as_str()
    }

    fn emit_signal(&self, bar: &Bar, kind: SignalKind) -> Signal {
        // Use the incoming bar's symbol rather than the TOML-hardcoded `self.symbol`
        // so that a BTC-named config (e.g. `btc_macd_trend`) can be run on any
        // symbol without the signal/position AssetMismatch that silently suppresses
        // all trades.  For BTC bars `bar.symbol == self.symbol`, so the anchored
        // BTC backtest reports remain byte-identical (anchor-safe).
        Signal {
            strategy_id: self.id.clone(),
            symbol: bar.symbol.clone(),
            ts: bar.close_ts,
            kind,
            evidence: SignalEvidence::empty(),
            pair_data: None, // v1.5a — composed strategies don't emit pair signals
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

// ── PlanDescribe impl for ComposedStrategy ────────────────────────────────────

impl crate::plan::PlanDescribe for ComposedStrategy {
    /// Describe the ComposedStrategy's current stance + rule family.
    ///
    /// The stance is derived from `last_rule_value` (true = rule fired → Long).
    /// The rule family is derived from the strategy id (keyed by which TOML
    /// the supervisor loaded via `build_registry_for`).
    ///
    /// ## ID → Rule mapping (faithful to config/strategies/*.toml)
    ///
    /// - `btc_macd_trend`        → `MacdCross { fast:12, slow:26, signal:9 }`
    ///   (entry: MACD hist > 0 AND close > EMA(200); flip-to-false exit)
    /// - `btc_rsi_reversion`     → `RsiReversion { len:14, lower:30 }`
    ///   (entry: RSI < 30 AND close > min(low,20); exits when RSI climbs back
    ///   above 30 — flip-to-false, no RSI-70 threshold)
    /// - `btc_bbands_mean_revert` → `BollingerReversion { len:20, k:2 }`
    ///   (entry: close < bollinger_lower(20,2) AND volume surge; exits when
    ///   price closes back inside the band — flip-to-false exit)
    /// - (unknown)               → `SmaCross { fast_len:20, slow_len:50 }`
    ///   (defensive fallback so the plan never panics on an unrecognised id)
    ///
    /// ## Non-mutation contract
    ///
    /// Reads only `self.last_rule_value` and `self.id` — no indicator push,
    /// no state advance.  `ctx.last_close` is used only for sizing (ADR-0062 § D2).
    fn describe_plan(&self, ctx: &crate::plan::PlanContext) -> crate::plan::StrategyPlan {
        use crate::plan::{PlanRuleShape, PlanSignal, PlanStance, ProjectedSizing, StrategyPlan};
        use rust_decimal_macros::dec;

        // Stance from the last evaluated rule value:
        //   true  → rule fired on the last bar → Long
        //   false → rule was off               → Flat
        //   None  → not yet warmed             → Flat
        let stance = match self.last_rule_value {
            Some(true) => PlanStance::Long,
            _ => PlanStance::Flat,
        };

        // latest_signal: Buy when rule just fired, Sell when rule just turned off,
        // Hold otherwise. Since we don't track the PREVIOUS value here, we derive
        // a conservative signal from the current stance.
        let latest_signal = match self.last_rule_value {
            Some(true) => Some(PlanSignal::Buy),
            Some(false) => Some(PlanSignal::Sell),
            None => None,
        };

        // Rule shape by strategy id — parameters must match config/strategies/*.toml exactly.
        let rule = match self.id_str() {
            // btc_macd_trend: MACD histogram positive AND close > EMA(200); flip-to-false exit.
            "btc_macd_trend" => PlanRuleShape::MacdCross {
                fast: 12,
                slow: 26,
                signal: 9,
            },
            // btc_rsi_reversion: RSI(14) < 30 AND close > min(low,20); exits when RSI climbs
            // back above 30 (the entry condition clears — flip-to-false, NOT an RSI-70 cross).
            "btc_rsi_reversion" => PlanRuleShape::RsiReversion {
                len: 14,
                lower: dec!(30),
            },
            // btc_bbands_mean_revert: close < bollinger_lower(20,2) AND volume surge;
            // exits when price closes back inside the band (flip-to-false exit).
            "btc_bbands_mean_revert" => PlanRuleShape::BollingerReversion {
                len: 20,
                k: dec!(2),
            },
            // Unknown / future id — defensive fallback so the plan never panics.
            _ => PlanRuleShape::SmaCross {
                fast_len: 20,
                slow_len: 50,
            },
        };

        let sizing = ProjectedSizing::compute(ctx.budget, ctx.budget_cap, ctx.last_close);

        StrategyPlan {
            stance,
            latest_signal,
            rule,
            sizing,
        }
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
    use trading_core::{Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

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
                venue: Venue::Binance,
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
            venue: Venue::Binance,
        };
        assert!(strategy.on_tick(&tick).is_empty());
    }
}

// ── ADR-0071 OBV identity / round-trip guard ──────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod obv_identity_tests {
    use super::*;
    use crate::composed::config::ComposedStrategyConfig;
    use rust_decimal_macros::dec;
    use trading_core::{Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

    /// Build a minimal bar with explicit close and volume (all other fields
    /// are set to safe defaults). Used for the hand-computed OBV series.
    fn make_bar(close: Decimal, volume: Decimal) -> Bar {
        use time::OffsetDateTime;
        let epoch = OffsetDateTime::new_utc(
            time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
            time::Time::MIDNIGHT,
        );
        let ts = Timestamp::new(epoch);
        let mk_price = |v: Decimal| Price::new(v.max(dec!(0.001))).unwrap();
        let mk_qty = |v: Decimal| Quantity::new(v.max(dec!(0.001))).unwrap();
        Bar {
            symbol: Symbol::new("BTCUSDT"),
            tf: Timeframe::OneMinute,
            open: mk_price(close),
            high: mk_price(close),
            low: mk_price(close),
            close: mk_price(close),
            volume: mk_qty(volume),
            trade_count: 1,
            local_recv_ts: ts,
            open_ts: ts,
            close_ts: ts,
            venue: Venue::Binance,
        }
    }

    /// ADR-0071 D2.2 — parser unit test for the novel 0-arity `obv()` call.
    ///
    /// Confirms the empty-arg parse path (which was logically correct but
    /// previously unexercised — every existing indicator had arity ≥ 1).
    #[test]
    fn t_obv_parser_zero_arity_roundtrip() {
        use crate::composed::parser::parse_signal;
        use std::collections::BTreeMap;

        let params: BTreeMap<smol_str::SmolStr, Decimal> = BTreeMap::new();

        // `obv()` — 0-arity indicator call — must parse successfully.
        let ast =
            parse_signal("obv() > 0", &params).expect("obv() > 0 must parse (0-arity call path)");
        assert!(
            matches!(ast, RuleAst::Cmp { .. }),
            "obv() > 0 should produce a Cmp node, got {:?}",
            ast
        );

        // `obv_avg(20)` — 1-arity call — must also parse.
        let ast2 = parse_signal("obv_avg(20) > 0", &params).expect("obv_avg(20) > 0 must parse");
        assert!(matches!(ast2, RuleAst::Cmp { .. }));

        // Full btc_obv signal must parse end-to-end.
        let ast3 = parse_signal("obv() > obv_avg(20) AND close > sma(50)", &params)
            .expect("btc_obv signal must parse");
        assert!(matches!(ast3, RuleAst::And(_, _)));

        // A bare `obv` (no parens) must error with UnknownParam — not an indicator call.
        let err = parse_signal("obv > 0", &params)
            .expect_err("bare `obv` without parens must fail (falls to UnknownParam)");
        assert_eq!(
            err.error_code(),
            "unknown_param",
            "bare `obv` should produce unknown_param, got: {:?}",
            err
        );
    }

    /// ADR-0071 D2.1 — OBV round-trip guard:
    ///   (a) `btc_obv` TOML parses via `from_str`, id == stem, indicators set correct.
    ///   (b) Textbook OBV on a hand-built series (exact Decimal, all 3 sign branches).
    ///   (c) `ObvAvg{20}` == SMA of the reference OBV series.
    ///   (d) Warm-up: `Obv.latest() == Some(0)` at bar 0, `ObvAvg{20}.latest() == None`
    ///       until 20 OBV values have been pushed.
    #[test]
    fn t_obv_identity_guard() {
        // ── (a) round-trip ───────────────────────────────────────────────────
        let toml = r#"
id     = "btc_obv"
kind   = "composed"
symbol = "BTCUSDT"
stage  = "research"
signal = "obv() > obv_avg(20) AND close > sma(50)"
size   = "fixed_fraction(0.1)"
"#;
        let cfg = ComposedStrategyConfig::from_str(toml, "btc_obv")
            .expect("btc_obv TOML must parse without error");
        assert_eq!(cfg.id, "btc_obv", "parsed id must equal the stem");

        // Build indicators and assert the required set is present.
        let ast = crate::composed::parser::parse_signal(&cfg.signal_raw, &cfg.params)
            .expect("signal must parse");
        let indicators = build_indicators(&ast);
        let has_obv = indicators
            .iter()
            .any(|s| matches!(s, IndicatorState::Obv { .. }));
        let has_obv_avg_20 = indicators
            .iter()
            .any(|s| matches!(s, IndicatorState::ObvAvg { period: 20, .. }));
        let has_sma_50 = indicators
            .iter()
            .any(|s| matches!(s, IndicatorState::Sma { period: 50, .. }));
        assert!(has_obv, "build_indicators must yield an Obv state");
        assert!(
            has_obv_avg_20,
            "build_indicators must yield an ObvAvg{{period:20}} state"
        );
        assert!(
            has_sma_50,
            "build_indicators must yield an Sma{{period:50}} state"
        );

        // ── (b) textbook OBV on a hand-built ~14-bar series ─────────────────
        // Series design (close, volume):
        //   bar 0: close=100, vol=10  → bar-0 seed, OBV=0  (warm-up)
        //   bar 1: close=105, vol=20  → up   → OBV = 0 + 20 = 20
        //   bar 2: close=100, vol=15  → down → OBV = 20 − 15 = 5
        //   bar 3: close=100, vol=12  → flat → OBV = 5 + 0 = 5
        //   bar 4: close=110, vol=25  → up   → OBV = 5 + 25 = 30
        //   bar 5: close=108, vol=18  → down → OBV = 30 − 18 = 12
        //   bar 6: close=112, vol=30  → up   → OBV = 12 + 30 = 42
        //   bar 7: close=115, vol=22  → up   → OBV = 42 + 22 = 64
        //   bar 8: close=112, vol=14  → down → OBV = 64 − 14 = 50
        //   bar 9: close=112, vol=10  → flat → OBV = 50 + 0  = 50
        //   bar10: close=118, vol=28  → up   → OBV = 50 + 28 = 78
        //   bar11: close=116, vol=16  → down → OBV = 78 − 16 = 62
        //   bar12: close=120, vol=35  → up   → OBV = 62 + 35 = 97
        //   bar13: close=119, vol=11  → down → OBV = 97 − 11 = 86
        let bars_raw: &[(i64, i64)] = &[
            (100, 10),
            (105, 20),
            (100, 15),
            (100, 12),
            (110, 25),
            (108, 18),
            (112, 30),
            (115, 22),
            (112, 14),
            (112, 10),
            (118, 28),
            (116, 16),
            (120, 35),
            (119, 11),
        ];
        let reference_obv: &[Decimal] = &[
            dec!(0),
            dec!(20),
            dec!(5),
            dec!(5),
            dec!(30),
            dec!(12),
            dec!(42),
            dec!(64),
            dec!(50),
            dec!(50),
            dec!(78),
            dec!(62),
            dec!(97),
            dec!(86),
        ];

        let bars: Vec<Bar> = bars_raw
            .iter()
            .map(|&(c, v)| make_bar(Decimal::from(c), Decimal::from(v)))
            .collect();

        // Stand-alone Obv state (as collected by build_indicators).
        let mut obv_state = IndicatorState::Obv {
            prev_close: None,
            acc: Decimal::ZERO,
            latest: None,
        };

        // ── (d) warm-up assertions ─────────────────────────────────────────
        // Before bar 0: latest must be None.
        assert_eq!(
            obv_state.latest(),
            None,
            "Obv.latest() must be None before any bar"
        );

        // ── (b) advance bar-by-bar + check against reference ─────────────
        for (i, bar) in bars.iter().enumerate() {
            obv_state.on_bar(bar);
            assert_eq!(
                obv_state.latest(),
                Some(reference_obv[i]),
                "Obv.latest() at bar {i} must equal reference OBV = {}",
                reference_obv[i]
            );
        }

        // ── (d) warm-up for ObvAvg{20} ───────────────────────────────────
        // ObvAvg{20} needs 20 OBV values → None until bar 19.
        // Our series only has 14 bars → it must still be None after all bars.
        let mut obv_avg_state = IndicatorState::ObvAvg {
            period: 20,
            obv: Box::new(IndicatorState::Obv {
                prev_close: None,
                acc: Decimal::ZERO,
                latest: None,
            }),
            window: VecDeque::with_capacity(20),
            sum: Decimal::ZERO,
            latest: None,
        };
        for bar in &bars {
            obv_avg_state.on_bar(bar);
        }
        assert_eq!(
            obv_avg_state.latest(),
            None,
            "ObvAvg{{20}}.latest() must be None until 20 OBV values pushed (only 14 bars)"
        );

        // ── (c) ObvAvg == SMA of reference OBV (period=5, fully warm) ────
        // Run ObvAvg{5} over the 14-bar series.
        let mut obv_avg5 = IndicatorState::ObvAvg {
            period: 5,
            obv: Box::new(IndicatorState::Obv {
                prev_close: None,
                acc: Decimal::ZERO,
                latest: None,
            }),
            window: VecDeque::with_capacity(5),
            sum: Decimal::ZERO,
            latest: None,
        };
        for bar in &bars {
            obv_avg5.on_bar(bar);
        }
        // After 14 bars, ObvAvg{5} is warm (>= 5 values pushed).
        // Expected: mean of the last 5 reference OBV values:
        //   reference_obv[9..14] = [50, 78, 62, 97, 86]
        //   mean = (50 + 78 + 62 + 97 + 86) / 5 = 373 / 5 = 74.6
        let expected_avg5 =
            (dec!(50) + dec!(78) + dec!(62) + dec!(97) + dec!(86)) / Decimal::from(5u32);
        assert_eq!(
            obv_avg5.latest(),
            Some(expected_avg5),
            "ObvAvg{{5}}.latest() must equal SMA of last 5 reference OBV values = {expected_avg5}"
        );
    }

    /// ADR-0071 — the 3 sign branches (up / down / flat) are all covered
    /// explicitly in `t_obv_identity_guard` (bars 1,5=up; bars 2,8=down;
    /// bars 3,9=flat). This companion test isolates each branch in isolation.
    #[test]
    fn t_obv_sign_branches_isolated() {
        // Up bar: close rises → OBV += volume.
        let mut obv = IndicatorState::Obv {
            prev_close: None,
            acc: Decimal::ZERO,
            latest: None,
        };
        obv.on_bar(&make_bar(dec!(100), dec!(10))); // bar 0 seed → OBV=0
        obv.on_bar(&make_bar(dec!(105), dec!(20))); // up → OBV=20
        assert_eq!(obv.latest(), Some(dec!(20)));

        // Flat bar: close unchanged → OBV unchanged.
        obv.on_bar(&make_bar(dec!(105), dec!(15))); // flat → OBV=20 (unchanged)
        assert_eq!(obv.latest(), Some(dec!(20)));

        // Down bar: close falls → OBV -= volume.
        obv.on_bar(&make_bar(dec!(100), dec!(8))); // down → OBV=20-8=12
        assert_eq!(obv.latest(), Some(dec!(12)));
    }
}
