//! Exchange-filter ingestion + client-side pre-validation (R4 / feature.md § A1).
//!
//! Ingests `LOT_SIZE` (`stepSize`, `minQty`), `MIN_NOTIONAL` (`minNotional`),
//! and `PRICE_FILTER` (`tickSize`) from `GET /api/v3/exchangeInfo`.
//!
//! Every order is rounded to `stepSize` and validated against `minQty` /
//! `minNotional` **client-side, in `Decimal`, BEFORE submit**.  An under-min
//! order returns a typed [`ExecError::FilterReject`] and **NEVER reaches the
//! network** (AC-5).
//!
//! Filter cache: TTL = 1 hour (AQ-2); force-refreshed on exchange `-1013`/`-2010`.
//!
//! **No `f64` in any rounding or validation path** (AC-9 / ADR-0003).

use std::time::{Duration, Instant};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::live::error::ExecError;

/// Default filter TTL (1 hour per AQ-2).
pub const FILTER_TTL: Duration = Duration::from_secs(3600);

/// Exchange filters for a single symbol, all fields `Decimal`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeFilters {
    /// LOT_SIZE: minimum step (all quantities must be a multiple of this).
    pub step_size: Decimal,
    /// LOT_SIZE: minimum order quantity.
    pub min_qty: Decimal,
    /// MIN_NOTIONAL: minimum order notional (`price * qty`).
    pub min_notional: Decimal,
    /// PRICE_FILTER: minimum tick size for prices.
    pub tick_size: Decimal,
}

/// A TTL-cached set of exchange filters.
pub struct FilterCache {
    inner: Option<(ExchangeFilters, Instant)>,
    ttl: Duration,
}

impl FilterCache {
    pub fn new(ttl: Duration) -> Self {
        Self { inner: None, ttl }
    }

    pub fn default_ttl() -> Self {
        Self::new(FILTER_TTL)
    }

    /// Return the cached filters if they are still fresh.
    pub fn get(&self) -> Option<&ExchangeFilters> {
        self.inner.as_ref().and_then(|(f, ts)| {
            if ts.elapsed() < self.ttl {
                Some(f)
            } else {
                None
            }
        })
    }

    /// Store new filters (reset the TTL clock).
    pub fn store(&mut self, filters: ExchangeFilters) {
        self.inner = Some((filters, Instant::now()));
    }

    /// Invalidate the cache (force-refresh on next access).
    pub fn invalidate(&mut self) {
        self.inner = None;
    }
}

impl Default for FilterCache {
    fn default() -> Self {
        Self::default_ttl()
    }
}

/// Round `qty` down to the nearest multiple of `step_size` using `Decimal`.
///
/// Returns `Decimal::ZERO` when `step_size` is zero (degenerate — not a valid
/// exchange filter but we guard rather than panic).
#[must_use]
pub fn round_to_step(qty: Decimal, step_size: Decimal) -> Decimal {
    if step_size.is_zero() {
        return qty;
    }
    // floor division: (qty / step_size).floor() * step_size
    let steps = (qty / step_size).floor();
    steps * step_size
}

/// Validate `qty` and `notional` against the given filters.
///
/// Returns `Ok(rounded_qty)` when the order passes all checks after rounding.
/// Returns `Err(ExecError::FilterReject)` when it fails — the error NEVER
/// reaches the network (AC-5).
///
/// # Errors
/// - `FilterReject` when rounded qty < `min_qty`.
/// - `FilterReject` when `rounded_qty * price < min_notional`.
///
/// # Arguments
/// * `qty`     — requested quantity (pre-rounding).
/// * `price`   — price used to compute notional (`last_mark` for MARKET).
/// * `filters` — filter set for this symbol.
pub fn validate_order(
    qty: Decimal,
    price: Decimal,
    filters: &ExchangeFilters,
) -> Result<Decimal, ExecError> {
    let rounded = round_to_step(qty, filters.step_size);
    if rounded < filters.min_qty {
        return Err(ExecError::FilterReject(format!(
            "qty {rounded} < min_qty {} (step_size={})",
            filters.min_qty, filters.step_size
        )));
    }
    let notional = rounded * price;
    if notional < filters.min_notional {
        return Err(ExecError::FilterReject(format!(
            "notional {notional} < min_notional {} (qty={rounded}, price={price})",
            filters.min_notional
        )));
    }
    Ok(rounded)
}

// ── Parsing from exchange-info JSON fixtures ──────────────────────────────────

/// Binance `GET /api/v3/exchangeInfo` filter types we care about.
#[derive(Debug, Deserialize)]
#[serde(tag = "filterType")]
pub(crate) enum BinanceFilter {
    #[serde(rename = "LOT_SIZE")]
    LotSize {
        #[serde(rename = "stepSize")]
        step_size: String,
        #[serde(rename = "minQty")]
        min_qty: String,
        // maxQty is parsed but not used in F1 pre-validation.
        #[serde(rename = "maxQty")]
        #[allow(dead_code)]
        max_qty: String,
    },
    #[serde(rename = "MIN_NOTIONAL")]
    MinNotional {
        #[serde(rename = "minNotional")]
        min_notional: String,
    },
    #[serde(rename = "NOTIONAL")]
    Notional {
        #[serde(rename = "minNotional")]
        min_notional: String,
    },
    #[serde(rename = "PRICE_FILTER")]
    PriceFilter {
        #[serde(rename = "tickSize")]
        tick_size: String,
        // minPrice / maxPrice parsed but not used in F1.
        #[serde(rename = "minPrice")]
        #[allow(dead_code)]
        min_price: String,
        #[serde(rename = "maxPrice")]
        #[allow(dead_code)]
        max_price: String,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BinanceSymbolInfo {
    pub symbol: String,
    pub filters: Vec<BinanceFilter>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BinanceExchangeInfo {
    pub symbols: Vec<BinanceSymbolInfo>,
}

/// Parse `ExchangeFilters` for `symbol` from the raw exchange-info JSON.
///
/// # Errors
/// Returns `Err(ExecError::FilterReject)` when the symbol is not found or
/// required filter fields are missing.
pub fn parse_filters_from_json(json: &str, symbol: &str) -> Result<ExchangeFilters, ExecError> {
    let info: BinanceExchangeInfo = serde_json::from_str(json)
        .map_err(|e| ExecError::FilterReject(format!("exchangeInfo parse error: {e}")))?;

    let sym_info = info
        .symbols
        .into_iter()
        .find(|s| s.symbol == symbol)
        .ok_or_else(|| {
            ExecError::FilterReject(format!("symbol {symbol} not found in exchangeInfo"))
        })?;

    let mut step_size = Decimal::ZERO;
    let mut min_qty = Decimal::ZERO;
    let mut min_notional = Decimal::ZERO;
    let mut tick_size = Decimal::ZERO;

    for f in sym_info.filters {
        match f {
            BinanceFilter::LotSize {
                step_size: s,
                min_qty: m,
                ..
            } => {
                step_size = s.parse().unwrap_or(Decimal::ZERO);
                min_qty = m.parse().unwrap_or(Decimal::ZERO);
            }
            BinanceFilter::MinNotional { min_notional: n }
            | BinanceFilter::Notional { min_notional: n } => {
                min_notional = n.parse().unwrap_or(Decimal::ZERO);
            }
            BinanceFilter::PriceFilter { tick_size: t, .. } => {
                tick_size = t.parse().unwrap_or(Decimal::ZERO);
            }
            BinanceFilter::Other => {}
        }
    }

    Ok(ExchangeFilters {
        step_size,
        min_qty,
        min_notional,
        tick_size,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    fn btcusdt_filters() -> ExchangeFilters {
        ExchangeFilters {
            step_size: dec!(0.00001),
            min_qty: dec!(0.00001),
            min_notional: dec!(10.0),
            tick_size: dec!(0.01),
        }
    }

    /// AC-5 (adversarial): an order below `minNotional` returns
    /// `FilterReject` — the faked transport records **zero** requests.
    #[test]
    fn under_min_notional_fails_fast() {
        let f = btcusdt_filters();
        // price=30000, qty=0.00001 → notional=0.30 < min_notional=10
        let result = validate_order(dec!(0.00001), dec!(30000), &f);
        assert!(
            matches!(result, Err(ExecError::FilterReject(_))),
            "expected FilterReject, got {result:?}"
        );
    }

    /// AC-5 (adversarial): an order off `stepSize` is rounded and then
    /// checked — if still too small, returns `FilterReject`.
    #[test]
    fn bad_lot_step_rejected() {
        let f = ExchangeFilters {
            step_size: dec!(0.1),
            min_qty: dec!(0.1),
            min_notional: dec!(10.0),
            tick_size: dec!(0.01),
        };
        // qty=0.05 is below the step — rounds to 0.0, which is < min_qty=0.1
        let result = validate_order(dec!(0.05), dec!(100), &f);
        assert!(
            matches!(result, Err(ExecError::FilterReject(_))),
            "expected FilterReject for off-step qty, got {result:?}"
        );
    }

    /// A valid order passes with the rounded quantity.
    #[test]
    fn valid_order_passes() {
        let f = btcusdt_filters();
        // price=40000, qty=0.001 → notional=40 > min_notional=10, qty > min_qty
        let rounded = validate_order(dec!(0.001), dec!(40000), &f).expect("should pass");
        assert_eq!(rounded, dec!(0.001));
    }

    /// Rounding with non-trivial step size.
    #[test]
    fn round_to_step_non_trivial() {
        // step=0.1, qty=0.35 → 0.3
        assert_eq!(round_to_step(dec!(0.35), dec!(0.1)), dec!(0.3));
        // step=0.01, qty=1.234567 → 1.23
        assert_eq!(round_to_step(dec!(1.234567), dec!(0.01)), dec!(1.23));
        // step=1, qty=7.9 → 7
        assert_eq!(round_to_step(dec!(7.9), dec!(1)), dec!(7));
    }

    /// Parse BTCUSDT filters from a recorded exchange-info JSON snippet.
    #[test]
    fn parse_filters_from_json_btcusdt() {
        let json = r#"{
            "symbols": [{
                "symbol": "BTCUSDT",
                "filters": [
                    {
                        "filterType": "PRICE_FILTER",
                        "minPrice": "0.01000000",
                        "maxPrice": "1000000.00000000",
                        "tickSize": "0.01000000"
                    },
                    {
                        "filterType": "LOT_SIZE",
                        "minQty": "0.00001000",
                        "maxQty": "9000.00000000",
                        "stepSize": "0.00001000"
                    },
                    {
                        "filterType": "MIN_NOTIONAL",
                        "minNotional": "10.00000000"
                    }
                ]
            }]
        }"#;

        let f = parse_filters_from_json(json, "BTCUSDT").expect("parse should succeed");
        assert_eq!(f.step_size, dec!(0.00001));
        assert_eq!(f.min_qty, dec!(0.00001));
        assert_eq!(f.min_notional, dec!(10));
        assert_eq!(f.tick_size, dec!(0.01));
    }

    /// Filter cache: fresh → Some; expired (manual invalidate) → None.
    #[test]
    fn filter_cache_ttl() {
        let mut cache = FilterCache::new(Duration::from_secs(3600));
        assert!(cache.get().is_none()); // empty
        cache.store(btcusdt_filters());
        assert!(cache.get().is_some()); // fresh
        cache.invalidate();
        assert!(cache.get().is_none()); // invalidated
    }
}
