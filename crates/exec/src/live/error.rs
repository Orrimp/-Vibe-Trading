//! `ExecError` — the full live-exec error taxonomy (R6 / feature.md § A4).
//!
//! The existing paper variants (`UnsupportedMode`, `OrderRejected`,
//! `FillFailed`) are **additively extended** here — `PaperExecRouter` is
//! untouched and still compiles unchanged.
//!
//! **No `f64` in any error field** (AC-9 / ADR-0003 invariant ii):
//! `notional` and `cap` are `Decimal`; `retry_after` is `Duration`.

use rust_decimal::Decimal;
use std::time::Duration;
use thiserror::Error;

/// Execution error — paper-mode stubs plus the full live-mode taxonomy.
#[derive(Debug, Error, Clone)]
pub enum ExecError {
    // ── Paper-mode (existing, untouched) ─────────────────────────────────────
    /// Unsupported execution mode.
    #[error("unsupported mode: {0}")]
    UnsupportedMode(String),
    /// Exchange rejected the order (paper mode).
    #[error("order rejected: {0}")]
    OrderRejected(String),
    /// Fill simulation failed (paper mode).
    #[error("fill failed: {0}")]
    FillFailed(String),

    // ── Live-mode taxonomy (F1 Wave B2) ───────────────────────────────────────
    /// Network timeout or connection failure.
    /// **Retry policy:** query order status BEFORE any retry — never blind-
    /// resubmit a possibly-filled order (AC-8).
    #[error("transport error: {0}")]
    Transport(String),

    /// Rate-limited by the exchange (HTTP 429 / Binance `-1003`).
    /// **Retry policy:** capped exponential backoff with a hard ceiling;
    /// then halt on exhaustion.
    #[error("rate limited (retry after ~{retry_after:?})")]
    RateLimited { retry_after: Duration },

    /// Authentication / signature failure (`-1022` bad sig; `-2014`/`-2015`
    /// invalid key).  **No retry** — a key/sig fault won't self-heal.
    #[error("auth error: {0}")]
    Auth(String),

    /// Clock skew — timestamp outside `recvWindow` (`-1021`).
    /// **Retry policy:** resync `GET /api/v3/time` once, retry once.
    /// Persistent skew → `HaltReason::ClockSkew` (variant already in
    /// `kill_switch.rs:53`).
    #[error("clock skew (timestamp outside recvWindow)")]
    ClockSkew,

    /// Filter reject — either a client-side pre-flight check (R4) or an
    /// exchange-side `-1013` / `-2010` filter rejection.
    /// **No retry**; force-refresh the filter cache on the exchange-side
    /// variant (AQ-2).
    #[error("filter reject: {0}")]
    FilterReject(String),

    /// Insufficient balance at the exchange (`-2010`).
    /// **No retry** — surface to caller.
    #[error("insufficient balance")]
    InsufficientBalance,

    /// The order type is not supported by this client (F1 ships MARKET only;
    /// `OrderKind::Limit` → this error — AQ-3).
    /// **No retry** — typed reject, never silent.
    #[error("unsupported order type: {0}")]
    UnsupportedOrderType(String),

    /// Notional exceeded the exec-side cap (R8 / `check_notional_cap`).
    /// Rejected before the network — faked transport records zero requests
    /// (AC-11).  **No retry** — the order is too large by construction.
    #[error("cap exceeded: notional={notional} > cap={cap}")]
    CapExceeded { notional: Decimal, cap: Decimal },

    /// Any unmapped Binance error code.
    /// **No retry** — log and surface.
    #[error("unknown exchange error: {0}")]
    Unknown(String),
}

/// Map a Binance API error code (as returned in `{"code": -N}` JSON) to an
/// `ExecError` variant.
///
/// # Binance error-code table
/// | Code    | Meaning |
/// |---------|---------|
/// | `-1003` | Too many requests (`RateLimited`) |
/// | `-1013` | Filter failure (exchange-side `FilterReject`) |
/// | `-1021` | Timestamp outside `recvWindow` (`ClockSkew`) |
/// | `-1022` | Signature invalid (`Auth`) |
/// | `-2010` | New order rejected — balance or filter (`InsufficientBalance` or `FilterReject`) |
/// | `-2014` | API-key format invalid (`Auth`) |
/// | `-2015` | Invalid API-key, IP, or permissions (`Auth`) |
#[must_use]
pub fn map_binance_code(code: i32, msg: &str) -> ExecError {
    match code {
        -1003 => ExecError::RateLimited {
            retry_after: Duration::from_secs(1),
        },
        -1013 => ExecError::FilterReject(format!("exchange filter reject (code={code}): {msg}")),
        -1021 => ExecError::ClockSkew,
        -1022 => ExecError::Auth(format!("invalid signature (code={code}): {msg}")),
        -2010 => {
            // -2010 can be either insufficient-balance OR a filter rejection.
            // Discriminate by message content (Binance convention).
            if msg.to_lowercase().contains("filter")
                || msg.to_lowercase().contains("lot")
                || msg.to_lowercase().contains("notional")
            {
                ExecError::FilterReject(format!("exchange filter reject (code={code}): {msg}"))
            } else {
                ExecError::InsufficientBalance
            }
        }
        -2014 | -2015 => ExecError::Auth(format!("API key error (code={code}): {msg}")),
        _ => ExecError::Unknown(format!("code={code}: {msg}")),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Wave B2: `binance_code_maps_to_variant` — every documented code maps
    /// to the correct variant.
    #[test]
    fn binance_code_maps_to_variant() {
        let cases: &[(i32, &str, &str)] = &[
            (-1003, "too many requests", "RateLimited"),
            (-1013, "lot size filter failure", "FilterReject"),
            (-1021, "timestamp outside recvWindow", "ClockSkew"),
            (-1022, "invalid signature", "Auth"),
            (-2010, "balance is insufficient", "InsufficientBalance"),
            (-2010, "LOT_SIZE filter failure", "FilterReject"),
            (-2014, "API-key format invalid", "Auth"),
            (-2015, "Invalid API-key", "Auth"),
            (-9999, "some unknown error", "Unknown"),
        ];
        for (code, msg, expected_variant) in cases {
            let err = map_binance_code(*code, msg);
            let variant_name = match &err {
                ExecError::RateLimited { .. } => "RateLimited",
                ExecError::FilterReject(_) => "FilterReject",
                ExecError::ClockSkew => "ClockSkew",
                ExecError::Auth(_) => "Auth",
                ExecError::InsufficientBalance => "InsufficientBalance",
                ExecError::Unknown(_) => "Unknown",
                other => panic!("unexpected variant: {other:?}"),
            };
            assert_eq!(
                variant_name, *expected_variant,
                "code={code} msg={msg:?} expected={expected_variant} got={variant_name}"
            );
        }
    }
}
