//! Coinbase Exchange public REST `get-product-candles` klines fetcher
//! (library module) — ADR-0084 D2.a.
//!
//! Mirrors `crates/data/src/binance_klines.rs` byte-for-byte in shape. This
//! is the historical **REST backfiller** for Coinbase — distinct from
//! [`crate::coinbase::CoinbaseFeed`], which is a live-WS feed (Advanced
//! Trade `candles` channel) and CANNOT backfill deep history.
//!
//! # Symbol → product-id mapping ([`coinbase_product_id_for_symbol`])
//!
//! **Correction discovered during T1 unit-testing (developer, this
//! session):** the shipped `crate::coinbase::coinbase_symbol_map` is NOT
//! reused for this mapping. That helper is designed for the live-feed's
//! own USDC/USDT/USD symbol space and checks `"USDC"` → `"USDT"` → `"USD"`
//! suffixes IN THAT ORDER — so `coinbase_symbol_map(&Symbol::new("BTCUSDT"))`
//! returns `"BTC-USDT"` (Coinbase DOES list a thin `BTC-USDT` product), NOT
//! `"BTC-USD"`. ADR-0084 is explicit throughout that the P2 corpus is
//! **`BTC-USD`** (the deep, trust-blessed, `VenueTrust::HighReconcilable`
//! product Coinbase serves back to ~2015-16) — a different, much deeper
//! and more liquid market than `BTC-USDT`. Because the on-disk canonical
//! symbol per D2.a is always USDT-quoted (`BTCUSDT`, mirroring Binance's
//! convention), the correct mapping strips the on-disk symbol's OWN quote
//! suffix to recover the base asset, then always appends the FIXED
//! `-USD` quote — never propagating the on-disk symbol's own USDT/USDC
//! suffix into the Coinbase product-id. [`coinbase_product_id_for_symbol`]
//! implements this; it is the ONLY symbol-mapping fn this module uses for
//! the REST call (`coinbase_symbol_map` is not imported here).
//!
//! # The one real seam (ADR-0084 D2.a — four differences from Binance)
//!
//! | Concern            | Binance                                             | Coinbase (this module)                                        |
//! |---------------------|------------------------------------------------------|----------------------------------------------------------------|
//! | On-disk symbol dir | `BTCUSDT` (canonical `Symbol`)                       | **`BTCUSDT`** (normalized) — REST call maps via `coinbase_product_id_for_symbol` → `BTC-USD` (fixed quote, never `BTC-USDT`) |
//! | Page size          | 1000 candles                                          | **300** candles (Coinbase rejects >300-point spans)             |
//! | Candle order       | `[open_time,open,high,low,close,vol,close_time,…]`    | `[time,low,high,open,close,volume]` — mapped positionally      |
//! | Timestamp unit     | millis                                                | **seconds** → ×1000; `close_time = open_time + granularity_ms − 1` |
//! | `trade_count`      | real                                                   | absent → `0` (the `coinbase.rs:299` live-feed sentinel)        |
//! | Pace               | 200 ms                                                | ≥200 ms (Coinbase public rate limit ~10 req/s → 5 req/s safe)   |
//!
//! **Fifth discovered difference (T2 live-network dry-run, 2026-07-10):**
//! Coinbase Exchange rejects requests with no `User-Agent` header
//! (`HTTP 400 {"message":"User-Agent header is required."}`); Binance's
//! public REST does not enforce this. `HttpCoinbaseKlineFetcher::fetch` sets
//! a fixed `User-Agent` on every request (see `COINBASE_USER_AGENT`).
//!
//! Everything else is reused verbatim from `binance_klines`: the shared
//! `Kline` struct, `write_parquet` (identical 8-col `replay_feed.rs`
//! schema), `should_skip`-style content-SHA idempotency (implemented in the
//! `fetch_coinbase_klines` bin, mirroring `fetch_binance_klines.rs`),
//! `expected_bars_per_month`, `data::revision::write_revision_manifest`.
//!
//! # Output layout (identical to Binance)
//!
//! ```text
//! <out>/<SYMBOL>/<YEAR>/<MONTH-padded>.parquet
//! ```
//!
//! # Mock seam
//!
//! [`CoinbaseKlineFetcher`] is the trait every test injects; the real HTTP
//! impl is [`HttpCoinbaseKlineFetcher`]. No test hits a live socket.

use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use time::OffsetDateTime;
use tracing::info;

// Reuse the shared, venue-neutral Kline struct + parquet writer verbatim.
pub use crate::binance_klines::{Kline, write_parquet};

// ── Symbol → product-id mapping (the corrected D2.a seam) ───────────────────

/// Map the on-disk canonical `Symbol` (always USDT-quoted, e.g. `BTCUSDT`)
/// to the Coinbase Exchange product-id with a FIXED `-USD` quote (e.g.
/// `BTC-USD`) — never the on-disk symbol's own USDT/USDC suffix.
///
/// This is DELIBERATELY NOT `crate::coinbase::coinbase_symbol_map` (see the
/// module doc's "Symbol → product-id mapping" section for the discovered
/// discrepancy: that helper would return `BTC-USDT`, a thinner, non-blessed
/// Coinbase product, for a `BTCUSDT` input). Strips the FIRST matching
/// known quote suffix (`USDT`/`USDC`/`USD`) to recover the base asset, then
/// always appends `-USD`.
///
/// # Errors
///
/// Returns an error if `symbol` does not end in any recognized quote suffix
/// (defensive — every P2 corpus symbol is Binance-style USDT-quoted).
pub fn coinbase_product_id_for_symbol(symbol: &trading_core::Symbol) -> Result<String> {
    let raw = symbol.0.as_str().to_uppercase();
    for q in ["USDT", "USDC", "USD"] {
        if let Some(base) = raw.strip_suffix(q)
            && !base.is_empty()
        {
            return Ok(format!("{base}-USD"));
        }
    }
    Err(anyhow!(
        "symbol '{raw}' has no recognized quote suffix (USDT/USDC/USD) — cannot derive a Coinbase product-id"
    ))
}

// ── URL builder ───────────────────────────────────────────────────────────────

pub const COINBASE_EXCHANGE_BASE: &str = "https://api.exchange.coinbase.com";

/// Coinbase `get-product-candles` granularity for hourly bars, in seconds.
pub const GRANULARITY_1H_SECS: u64 = 3600;

/// Build a Coinbase Exchange `get-product-candles` query URL.
///
/// `start_iso` / `end_iso` are RFC3339 timestamps (e.g.
/// `"2024-01-01T00:00:00Z"`). Coinbase caps each call at 300 candles —
/// callers (the paginator) are responsible for keeping `[start_iso, end_iso)`
/// within a 300-candle span at the given `granularity_secs`.
///
/// Pure function — no I/O. Used by tests and the paginator.
#[must_use]
pub fn build_coinbase_candles_url(
    product_id: &str,
    granularity_secs: u64,
    start_iso: &str,
    end_iso: &str,
) -> String {
    format!(
        "{COINBASE_EXCHANGE_BASE}/products/{product_id}/candles?start={start_iso}&end={end_iso}&granularity={granularity_secs}"
    )
}

/// Convert a Unix-millis timestamp to an RFC3339 UTC string
/// (`"2024-01-01T00:00:00Z"`) for the Coinbase `start`/`end` query params.
///
/// Truncates to whole seconds (Coinbase candle boundaries are second-aligned;
/// sub-second precision is never meaningful here).
#[must_use]
pub fn millis_to_rfc3339(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    OffsetDateTime::from_unix_timestamp(secs)
        .ok()
        .and_then(|dt| {
            dt.format(&time::format_description::well_known::Rfc3339)
                .ok()
        })
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_owned())
}

// ── Candle parser (the shim — positional array + seconds→millis + trade_count=0) ─

/// Parse one Coinbase candle JSON array element
/// (`[time, low, high, open, close, volume]`, `time` in **seconds**) into the
/// shared, venue-neutral [`Kline`].
///
/// The four ADR-0084 D2.a shim mappings, all confined to this function:
/// 1. Positional order `[time,low,high,open,close,volume]` (Coinbase) →
///    canonical `Kline{open_time,close_time,open,high,low,close,volume,trade_count}`.
/// 2. `time` (seconds) × 1000 → millis (`open_time`).
/// 3. `close_time = open_time + granularity_ms − 1` (Coinbase returns only the
///    candle's open instant; Binance-style klines carry both).
/// 4. `trade_count = 0` (Coinbase candles never report it — the same
///    sentinel `crate::coinbase::parse_candles_event` uses for the live feed).
///
/// Never panics — malformed input returns `Err`.
pub fn parse_coinbase_candle(row: &[serde_json::Value], granularity_secs: u64) -> Result<Kline> {
    if row.len() < 6 {
        return Err(anyhow!(
            "coinbase candle row too short: len={} (want 6)",
            row.len()
        ));
    }
    let time_secs = row[0]
        .as_i64()
        .ok_or_else(|| anyhow!("coinbase candle[0] (time) not i64: {:?}", row[0]))?;
    let low = row[1]
        .as_f64()
        .ok_or_else(|| anyhow!("coinbase candle[1] (low) not f64: {:?}", row[1]))?;
    let high = row[2]
        .as_f64()
        .ok_or_else(|| anyhow!("coinbase candle[2] (high) not f64: {:?}", row[2]))?;
    let open = row[3]
        .as_f64()
        .ok_or_else(|| anyhow!("coinbase candle[3] (open) not f64: {:?}", row[3]))?;
    let close = row[4]
        .as_f64()
        .ok_or_else(|| anyhow!("coinbase candle[4] (close) not f64: {:?}", row[4]))?;
    let volume = row[5]
        .as_f64()
        .ok_or_else(|| anyhow!("coinbase candle[5] (volume) not f64: {:?}", row[5]))?;

    let open_time = time_secs
        .checked_mul(1000)
        .ok_or_else(|| anyhow!("coinbase candle time overflow on ×1000: {time_secs}"))?;
    #[allow(clippy::cast_possible_wrap)]
    let granularity_ms = (granularity_secs * 1000) as i64;
    let close_time = open_time + granularity_ms - 1;

    Ok(Kline {
        open_time,
        close_time,
        // Coinbase returns raw f64 in the JSON array (not strings, unlike
        // Binance) — format to a decimal string so the shared `Kline`
        // struct + `write_parquet` stay byte-identical to the Binance path
        // (Utf8 price columns; ADR-0003 no-f64-in-money-math is preserved
        // because the string is re-parsed to `Decimal` downstream, exactly
        // like the Binance read path — this function only bridges JSON→Kline).
        open: format!("{open}"),
        high: format!("{high}"),
        low: format!("{low}"),
        close: format!("{close}"),
        volume: format!("{volume}"),
        trade_count: 0,
    })
}

// ── Fetcher trait + impls ─────────────────────────────────────────────────────

/// Trait so tests can inject a mock fetcher (the R3 "external I/O behind a
/// trait" seam — no test hits a live socket). Mirrors `binance_klines::KlineFetcher`.
#[async_trait::async_trait]
pub trait CoinbaseKlineFetcher: Send + Sync {
    async fn fetch(&self, url: &str) -> Result<Vec<Kline>>;
}

/// Real HTTP fetcher backed by `reqwest`.
pub struct HttpCoinbaseKlineFetcher {
    client: reqwest::Client,
    granularity_secs: u64,
}

impl HttpCoinbaseKlineFetcher {
    #[must_use]
    pub fn new(client: reqwest::Client, granularity_secs: u64) -> Self {
        Self {
            client,
            granularity_secs,
        }
    }
}

/// Coinbase Exchange rejects requests with no `User-Agent` header
/// (`HTTP 400 {"message":"User-Agent header is required."}` — discovered
/// during the T2 live-network dry-run, 2026-07-10; `reqwest::Client`'s
/// default has no `User-Agent` set). Binance's public REST does not enforce
/// this, which is why `binance_klines::HttpKlineFetcher` needs no equivalent.
const COINBASE_USER_AGENT: &str = "trading-agent-p2-corpus-expansion/0.1.0";

#[async_trait::async_trait]
impl CoinbaseKlineFetcher for HttpCoinbaseKlineFetcher {
    async fn fetch(&self, url: &str) -> Result<Vec<Kline>> {
        let resp = self
            .client
            .get(url)
            .header(reqwest::header::USER_AGENT, COINBASE_USER_AGENT)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Coinbase returned HTTP {status}: {body}"));
        }
        let raw: Vec<Vec<serde_json::Value>> = resp
            .json()
            .await
            .with_context(|| format!("JSON decode for {url}"))?;
        raw.iter()
            .map(|row| parse_coinbase_candle(row, self.granularity_secs))
            .collect::<Result<Vec<_>>>()
    }
}

// ── Paginator (300-candle window step, forward sub-windows within a month) ───

/// Paginate over Coinbase klines for a product + time window.
///
/// Unlike Binance's `paginate_klines` (single 1000-candle-per-call cursor
/// that pages backward across the WHOLE requested span), Coinbase caps each
/// call at 300 candles AND rejects a `[start,end)` span wider than that at
/// the given granularity. This paginator therefore walks **forward
/// sub-windows** of `300 * granularity_secs` seconds each within
/// `[start_ms, end_ms)`, concatenating results — the caller (the CLI bin's
/// month loop) supplies the outer month-by-month backward walk to full
/// listing history, matching Binance's deep-history property at the
/// month-iteration layer instead of within a single pagination call.
///
/// Returns all klines whose `open_time` falls within `[start_ms, end_ms)`.
/// Sleeps `sleep_ms` between requests to stay under rate-limit budget
/// (Coinbase public ~10 req/s; `sleep_ms >= 200` → ≤5 req/s, safe).
pub async fn paginate_coinbase_candles(
    fetcher: &dyn CoinbaseKlineFetcher,
    product_id: &str,
    granularity_secs: u64,
    start_ms: i64,
    end_ms: i64,
    sleep_ms: u64,
) -> Result<Vec<Kline>> {
    const MAX_CANDLES_PER_CALL: i64 = 300;
    let window_span_ms = MAX_CANDLES_PER_CALL
        .checked_mul(i64::try_from(granularity_secs).unwrap_or(3600))
        .and_then(|s| s.checked_mul(1000))
        .ok_or_else(|| anyhow!("window span overflow for granularity_secs={granularity_secs}"))?;

    let mut all: Vec<Kline> = Vec::new();
    let mut cursor = start_ms;
    let mut request_count: u32 = 0;

    while cursor < end_ms {
        let window_end = (cursor + window_span_ms).min(end_ms);
        let start_iso = millis_to_rfc3339(cursor);
        // Coinbase's `end` is inclusive of the candle whose `time == end`;
        // subtract one granularity unit so we do not double-fetch the
        // boundary candle on the next sub-window.
        #[allow(clippy::cast_possible_wrap)]
        let granularity_ms = (granularity_secs * 1000) as i64;
        let end_iso = millis_to_rfc3339((window_end - granularity_ms).max(cursor));

        let url = build_coinbase_candles_url(product_id, granularity_secs, &start_iso, &end_iso);
        let batch = fetcher.fetch(&url).await?;
        request_count += 1;

        // Filter to the requested window (Coinbase candle order is
        // undefined/descending in practice; do not assume ascending).
        let mut in_window: Vec<Kline> = batch
            .into_iter()
            .filter(|k| k.open_time >= cursor && k.open_time < window_end)
            .collect();
        all.append(&mut in_window);

        cursor = window_end;

        if cursor < end_ms && sleep_ms > 0 {
            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
        }
    }

    all.sort_by_key(|k| k.open_time);
    all.dedup_by_key(|k| k.open_time);

    info!(
        product_id,
        granularity_secs,
        requests = request_count,
        bars = all.len(),
        "paginated Coinbase candles"
    );
    Ok(all)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::uninlined_format_args,
    clippy::pedantic
)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // ── Test 1: URL builder ───────────────────────────────────────────────────

    #[test]
    fn test_build_coinbase_candles_url() {
        let url = build_coinbase_candles_url(
            "BTC-USD",
            3600,
            "2024-01-01T00:00:00Z",
            "2024-01-13T11:00:00Z",
        );
        assert_eq!(
            url,
            "https://api.exchange.coinbase.com/products/BTC-USD/candles\
?start=2024-01-01T00:00:00Z&end=2024-01-13T11:00:00Z&granularity=3600"
        );
    }

    #[test]
    fn test_build_coinbase_candles_url_eth() {
        let url = build_coinbase_candles_url("ETH-USD", 3600, "2020-01-01T00:00:00Z", "x");
        assert!(url.contains("products/ETH-USD/candles"));
        assert!(url.contains("granularity=3600"));
        assert!(url.contains("start=2020-01-01T00:00:00Z"));
    }

    // ── Test 2: millis_to_rfc3339 ─────────────────────────────────────────────

    #[test]
    fn test_millis_to_rfc3339_epoch() {
        assert_eq!(millis_to_rfc3339(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn test_millis_to_rfc3339_known_date() {
        // 2024-01-01T00:00:00Z = 1704067200 seconds.
        let s = millis_to_rfc3339(1_704_067_200_000);
        assert_eq!(s, "2024-01-01T00:00:00Z");
    }

    #[test]
    fn test_millis_to_rfc3339_truncates_subsecond() {
        // Sub-second component must be dropped (truncate, not round).
        let s = millis_to_rfc3339(1_704_067_200_999);
        assert_eq!(s, "2024-01-01T00:00:00Z");
    }

    // ── Test 3: parse_coinbase_candle — the positional + seconds→millis shim ──

    #[test]
    fn test_parse_coinbase_candle_positional_mapping() {
        // [time, low, high, open, close, volume], time in SECONDS.
        let row = vec![
            serde_json::json!(1_704_067_200_i64), // 2024-01-01T00:00:00Z
            serde_json::json!(41_800.0),          // low
            serde_json::json!(42_500.0),          // high
            serde_json::json!(42_000.0),          // open
            serde_json::json!(42_300.0),          // close
            serde_json::json!(123.456),           // volume
        ];
        let k = parse_coinbase_candle(&row, 3600).expect("parse ok");

        // seconds × 1000 → millis.
        assert_eq!(k.open_time, 1_704_067_200_000);
        // close_time = open_time + granularity_ms - 1.
        assert_eq!(k.close_time, 1_704_067_200_000 + 3_600_000 - 1);
        // Positional mapping is NOT swapped (low/high/open/close land correctly).
        assert_eq!(k.low, "41800");
        assert_eq!(k.high, "42500");
        assert_eq!(k.open, "42000");
        assert_eq!(k.close, "42300");
        assert_eq!(k.volume, "123.456");
        // No trade_count on Coinbase candles → sentinel 0.
        assert_eq!(k.trade_count, 0);
    }

    #[test]
    fn test_parse_coinbase_candle_different_granularity() {
        // 300s (5-min) granularity → close_time offset differs.
        let row = vec![
            serde_json::json!(1_000_i64),
            serde_json::json!(1.0),
            serde_json::json!(2.0),
            serde_json::json!(1.5),
            serde_json::json!(1.8),
            serde_json::json!(10.0),
        ];
        let k = parse_coinbase_candle(&row, 300).expect("parse ok");
        assert_eq!(k.open_time, 1_000_000);
        assert_eq!(k.close_time, 1_000_000 + 300_000 - 1);
    }

    #[test]
    fn test_parse_coinbase_candle_too_short_is_error_not_panic() {
        let row = vec![serde_json::json!(1000_i64), serde_json::json!(1.0)];
        let result = parse_coinbase_candle(&row, 3600);
        assert!(result.is_err(), "short row must error, not panic");
    }

    #[test]
    fn test_parse_coinbase_candle_non_numeric_time_is_error() {
        let row = vec![
            serde_json::json!("not_a_number"),
            serde_json::json!(1.0),
            serde_json::json!(2.0),
            serde_json::json!(1.5),
            serde_json::json!(1.8),
            serde_json::json!(10.0),
        ];
        let result = parse_coinbase_candle(&row, 3600);
        assert!(result.is_err(), "non-numeric time must error, not panic");
    }

    // ── Test 4: MockFetcher + paginator boundary logic ───────────────────────

    struct MockCoinbaseFetcher {
        batches: Mutex<Vec<Vec<Kline>>>,
        calls: Mutex<Vec<String>>,
    }

    impl MockCoinbaseFetcher {
        fn new(batches: Vec<Vec<Kline>>) -> Self {
            Self {
                batches: Mutex::new(batches),
                calls: Mutex::new(Vec::new()),
            }
        }
        fn recorded_calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl CoinbaseKlineFetcher for MockCoinbaseFetcher {
        async fn fetch(&self, url: &str) -> Result<Vec<Kline>> {
            self.calls.lock().unwrap().push(url.to_owned());
            let mut batches = self.batches.lock().unwrap();
            if batches.is_empty() {
                Ok(vec![])
            } else {
                Ok(batches.remove(0))
            }
        }
    }

    fn make_coinbase_kline(open_time: i64, granularity_ms: i64) -> Kline {
        Kline {
            open_time,
            close_time: open_time + granularity_ms - 1,
            open: "60000".to_owned(),
            high: "61000".to_owned(),
            low: "59000".to_owned(),
            close: "60500".to_owned(),
            volume: "10.0".to_owned(),
            trade_count: 0,
        }
    }

    fn make_coinbase_batch(start_ms: i64, step_ms: i64, n: usize) -> Vec<Kline> {
        (0..n)
            .map(|i| make_coinbase_kline(start_ms + i as i64 * step_ms, step_ms))
            .collect()
    }

    #[tokio::test]
    async fn test_paginator_walks_forward_sub_windows_of_300() {
        // 3600s granularity, window_span_ms = 300 * 3600 * 1000 = 1_080_000_000 ms (300h).
        let step = 3_600_000_i64; // 1h
        let start_ms = 0_i64;
        // Request a span of exactly 2 sub-windows (600 candles worth).
        let end_ms = start_ms + 2 * 300 * step;

        let batch1 = make_coinbase_batch(start_ms, step, 300);
        let batch2 = make_coinbase_batch(start_ms + 300 * step, step, 300);

        let fetcher = MockCoinbaseFetcher::new(vec![batch1, batch2]);
        let result = paginate_coinbase_candles(&fetcher, "BTC-USD", 3600, start_ms, end_ms, 0)
            .await
            .expect("pagination should succeed");

        assert_eq!(result.len(), 600, "two full 300-candle sub-windows");
        let calls = fetcher.recorded_calls();
        assert_eq!(calls.len(), 2, "exactly 2 sub-window requests");

        // Monotonic + deduped.
        for w in result.windows(2) {
            assert!(w[0].open_time < w[1].open_time);
        }
    }

    #[tokio::test]
    async fn test_paginator_stops_on_empty_response() {
        let fetcher = MockCoinbaseFetcher::new(vec![vec![]]);
        let result = paginate_coinbase_candles(&fetcher, "BTC-USD", 3600, 0, 3_600_000, 0)
            .await
            .expect("should not error on empty");
        assert!(result.is_empty());
        // A single sub-window covers [0, 3_600_000) fully (< 300-candle span),
        // so exactly one request is made even though the response is empty.
        assert_eq!(fetcher.recorded_calls().len(), 1);
    }

    #[tokio::test]
    async fn test_paginator_filters_out_of_window_candles() {
        let step = 3_600_000_i64;
        let start_ms = 0_i64;
        let end_ms = start_ms + 10 * step;

        // Batch includes candles both inside and outside [start_ms, end_ms).
        let mut batch = make_coinbase_batch(start_ms - 2 * step, step, 5); // 2 before, 3 inside
        batch.extend(make_coinbase_batch(end_ms, step, 3)); // 3 after (out of window)

        let fetcher = MockCoinbaseFetcher::new(vec![batch]);
        let result = paginate_coinbase_candles(&fetcher, "BTC-USD", 3600, start_ms, end_ms, 0)
            .await
            .expect("should succeed");

        assert_eq!(result.len(), 3, "only in-window candles retained");
        for k in &result {
            assert!(k.open_time >= start_ms && k.open_time < end_ms);
        }
    }

    #[tokio::test]
    async fn test_paginator_zero_span_makes_no_requests() {
        let fetcher = MockCoinbaseFetcher::new(vec![]);
        let result = paginate_coinbase_candles(&fetcher, "BTC-USD", 3600, 1000, 1000, 0)
            .await
            .expect("empty span should succeed trivially");
        assert!(result.is_empty());
        assert_eq!(fetcher.recorded_calls().len(), 0);
    }

    // ── Test 5: reuse of shared Kline / write_parquet (venue-neutral) ────────

    #[test]
    fn test_write_parquet_reused_from_binance_klines_is_venue_neutral() {
        let klines = vec![Kline {
            open_time: 1_704_067_200_000,
            close_time: 1_704_070_799_999,
            open: "42000".to_owned(),
            high: "42500".to_owned(),
            low: "41800".to_owned(),
            close: "42300".to_owned(),
            volume: "123.456".to_owned(),
            trade_count: 0,
        }];

        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("coinbase_test.parquet");
        write_parquet(&klines, &path).expect("write_parquet (reused fn) works for Coinbase klines");
        assert!(path.exists());
    }

    // ── Test 6: coinbase_product_id_for_symbol — the corrected D2.a seam ────
    //
    // Discovered during this session's testing: `crate::coinbase::
    // coinbase_symbol_map` returns "BTC-USDT" (not "BTC-USD") for a
    // "BTCUSDT" input, because it checks the USDC/USDT/USD suffixes in that
    // order and BTCUSDT matches "USDT" first. ADR-0084 is explicit the P2
    // corpus is BTC-USD (the deep, trust-blessed product). This module uses
    // its OWN mapping fn — `coinbase_product_id_for_symbol` — which strips
    // the on-disk symbol's own quote suffix and always re-appends a FIXED
    // "-USD", never propagating the on-disk USDT/USDC suffix.

    #[test]
    fn test_coinbase_product_id_for_symbol_btcusdt_maps_to_btc_usd() {
        let sym = trading_core::Symbol::new("BTCUSDT");
        assert_eq!(
            coinbase_product_id_for_symbol(&sym).expect("valid symbol"),
            "BTC-USD"
        );
    }

    #[test]
    fn test_coinbase_product_id_for_symbol_ethusdt_maps_to_eth_usd() {
        let sym = trading_core::Symbol::new("ETHUSDT");
        assert_eq!(
            coinbase_product_id_for_symbol(&sym).expect("valid symbol"),
            "ETH-USD"
        );
    }

    #[test]
    fn test_coinbase_product_id_for_symbol_lowercase_normalizes() {
        let sym = trading_core::Symbol::new("btcusdt");
        assert_eq!(
            coinbase_product_id_for_symbol(&sym).expect("valid symbol"),
            "BTC-USD"
        );
    }

    #[test]
    fn test_coinbase_product_id_for_symbol_never_returns_btc_usdt() {
        // The regression this test guards: NEVER propagate the on-disk
        // symbol's own USDT/USDC suffix into the Coinbase product-id.
        let sym = trading_core::Symbol::new("BTCUSDT");
        let product_id = coinbase_product_id_for_symbol(&sym).expect("valid symbol");
        assert_ne!(
            product_id, "BTC-USDT",
            "must map to the deep, trust-blessed BTC-USD product, not the thin BTC-USDT one"
        );
    }

    #[test]
    fn test_coinbase_product_id_for_symbol_no_quote_suffix_is_error() {
        let sym = trading_core::Symbol::new("NOTASYMBOL");
        assert!(coinbase_product_id_for_symbol(&sym).is_err());
    }
}
