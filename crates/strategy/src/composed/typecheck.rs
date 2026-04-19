//! Typecheck the rule AST (T504).
//!
//! Validates indicator numeric ranges after the parser has already
//! validated arity and indicator names.

use rust_decimal::Decimal;

use super::ast::*;
use super::error::StrategyLoadError;

/// Typecheck a parsed `RuleAst`.
///
/// Validates:
/// - RSI period ≥ 2.
/// - MACD fast < slow, all periods > 0.
/// - Bollinger mult > 0, period ≥ 2.
/// - SMA/EMA period ≥ 1.
/// - Rolling window ≥ 1.
///
/// # Errors
///
/// Returns [`StrategyLoadError::InvalidRange`] on any range violation.
pub fn typecheck(rule: &RuleAst) -> Result<(), StrategyLoadError> {
    check_rule(rule)
}

fn check_rule(rule: &RuleAst) -> Result<(), StrategyLoadError> {
    match rule {
        RuleAst::And(a, b) | RuleAst::Or(a, b) => {
            check_rule(a)?;
            check_rule(b)?;
        }
        RuleAst::Not(inner) => check_rule(inner)?,
        RuleAst::Cmp { lhs, rhs, .. } => {
            check_expr(lhs)?;
            check_expr(rhs)?;
        }
        RuleAst::CrossAbove { a, b } | RuleAst::CrossBelow { a, b } => {
            check_expr(a)?;
            check_expr(b)?;
        }
        RuleAst::MacdCross { fast, slow, signal, .. } => {
            check_macd_params(*fast, *slow, *signal)?;
        }
        RuleAst::BollingerLowerTouch { period, mult } => {
            check_bollinger(*period, *mult)?;
        }
    }
    Ok(())
}

fn check_expr(expr: &Expr) -> Result<(), StrategyLoadError> {
    match expr {
        Expr::Indicator(call) => check_indicator(call),
        Expr::Binary { lhs, rhs, .. } => {
            check_expr(lhs)?;
            check_expr(rhs)?;
            Ok(())
        }
        Expr::BoolRule(inner) => check_rule(inner),
        Expr::BarField(_) | Expr::Param(_) | Expr::Literal(_) => Ok(()),
    }
}

fn check_indicator(call: &IndicatorCall) -> Result<(), StrategyLoadError> {
    // Helper to extract positional numeric literals.
    let arg_u32 = |idx: usize| -> Result<u32, StrategyLoadError> {
        match call.args.get(idx) {
            Some(Expr::Literal(d)) => {
                if *d <= Decimal::ZERO || d.fract() != Decimal::ZERO {
                    return Err(StrategyLoadError::InvalidRange(format!(
                        "{} arg[{idx}] must be a positive integer, got {d}",
                        call.name
                    )));
                }
                let v: u32 = d.to_string().parse().map_err(|_| {
                    StrategyLoadError::InvalidRange(format!(
                        "{} arg[{idx}] out of range: {d}",
                        call.name
                    ))
                })?;
                Ok(v)
            }
            Some(other) => Err(StrategyLoadError::InvalidRange(format!(
                "{} arg[{idx}] must be a numeric literal, got {other:?}",
                call.name
            ))),
            None => Err(StrategyLoadError::InvalidRange(format!(
                "{} missing arg[{idx}]",
                call.name
            ))),
        }
    };

    let arg_decimal = |idx: usize| -> Result<Decimal, StrategyLoadError> {
        match call.args.get(idx) {
            Some(Expr::Literal(d)) => Ok(*d),
            Some(other) => Err(StrategyLoadError::InvalidRange(format!(
                "{} arg[{idx}] must be a numeric literal, got {other:?}",
                call.name
            ))),
            None => Err(StrategyLoadError::InvalidRange(format!(
                "{} missing arg[{idx}]",
                call.name
            ))),
        }
    };

    match call.name.as_str() {
        "rsi" => {
            let period = arg_u32(0)?;
            if period < 2 {
                return Err(StrategyLoadError::InvalidRange(format!(
                    "rsi period must be >= 2, got {period}"
                )));
            }
        }
        "macd_line" | "macd_signal" | "macd_hist" | "macd_cross" => {
            let fast = arg_u32(0)?;
            let slow = arg_u32(1)?;
            let signal = arg_u32(2)?;
            check_macd_params(fast, slow, signal)?;
        }
        "bollinger_upper" | "bollinger_mid" | "bollinger_lower" | "bollinger_lower_touch" => {
            let period = arg_u32(0)?;
            let mult = arg_decimal(1)?;
            check_bollinger(period, mult)?;
        }
        "sma" | "ema" => {
            let period = arg_u32(0)?;
            if period < 1 {
                return Err(StrategyLoadError::InvalidRange(format!(
                    "{} period must be >= 1, got {period}",
                    call.name
                )));
            }
        }
        "min" | "max" | "avg" => {
            // arg[0] is a bar field (or expr) — typecheck it.
            check_expr(&call.args[0])?;
            // arg[1] is the window size.
            let window = arg_u32(1)?;
            if window < 1 {
                return Err(StrategyLoadError::InvalidRange(format!(
                    "{} window must be >= 1, got {window}",
                    call.name
                )));
            }
        }
        "cross_above" | "cross_below" => {
            check_expr(&call.args[0])?;
            check_expr(&call.args[1])?;
        }
        _ => {}
    }
    Ok(())
}

fn check_macd_params(fast: u32, slow: u32, signal: u32) -> Result<(), StrategyLoadError> {
    if fast == 0 || slow == 0 || signal == 0 {
        return Err(StrategyLoadError::InvalidRange(
            "MACD periods must be > 0".to_string(),
        ));
    }
    if fast >= slow {
        return Err(StrategyLoadError::InvalidRange(format!(
            "MACD fast({fast}) must be < slow({slow})"
        )));
    }
    Ok(())
}

fn check_bollinger(period: u32, mult: Decimal) -> Result<(), StrategyLoadError> {
    if period < 2 {
        return Err(StrategyLoadError::InvalidRange(format!(
            "Bollinger period must be >= 2, got {period}"
        )));
    }
    if mult <= Decimal::ZERO {
        return Err(StrategyLoadError::InvalidRange(format!(
            "Bollinger mult must be > 0, got {mult}"
        )));
    }
    Ok(())
}
