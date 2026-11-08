//! Live EUR/USD reference-rate fetcher (story 3-19 — ADR-0101).
//!
//! # What this is
//!
//! A single, keyless GET against the ECB daily reference rate, behind the
//! repo's standard "external I/O behind a trait" seam ([`RateSource`]) so no
//! test opens a socket. The fetcher returns a **validated**
//! [`trading_core::FxRate`] — the checked [`FxRate::new`] ctor is the only way
//! a rate gets out of here, so a non-positive quote can never reach the money
//! path (AD-9 / story AC3).
//!
//! # Mock seam
//!
//! [`RateSource`] is the trait every caller takes; the real HTTP impl is
//! [`HttpRateSource`] and the test/fixture fake is [`FakeRateSource`]
//! (the [`crate::binance_klines::KlineFetcher`] / `MockFetcher` shape,
//! ADR-0061 precedent).
//!
//! # The `f64` trap (ADR-0101 D6 — the reason this module parses by hand)
//!
//! `serde_json` parses **every** JSON float into an `f64` unless its
//! `arbitrary_precision` feature is on. So `#[derive(Deserialize)]` onto a
//! `Decimal` field, `serde_json::Value::as_f64`, and even
//! `Value::Number::to_string()` all route the rate through binary floating
//! point. At quote magnitudes (~1.1) the round-trip looks convincing, which is
//! exactly why this breaks AD-9 while every test still passes.
//!
//! The fix is to never let the number become a number until `Decimal` sees it:
//! the rate is deserialized as a [`serde_json::value::RawValue`] — the
//! untouched JSON **token** — and parsed with `Decimal::from_str`.
//! `crates/data/tests/fx_rate_decimal_boundary.rs` pins this with a token that
//! carries more precision than an `f64` can hold.
//!
//! # Failure policy (ADR-0101 D5)
//!
//! This module only ever reports what happened; it never substitutes a value.
//! One attempt, one [`LIVE_RATE_TIMEOUT`], no retry. Every error — offline,
//! DNS, timeout, non-2xx, unparseable body, missing `USD` key, non-positive
//! rate — is the caller's signal to use its own static fallback (the UI's
//! `ui::live_fx::resolve_fx_rate`).

use std::{collections::BTreeMap, str::FromStr, time::Duration};

use reqwest::Client;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::value::RawValue;
use trading_core::{FxRate, FxRateError};

// ── Pinned endpoint + labels (ADR-0101 D3) ───────────────────────────────────

/// The pinned Frankfurter endpoint for the ECB daily EUR→USD reference rate.
///
/// **`.dev`, not `.app`.** `api.frankfurter.app` — the host every older
/// document names — answers `301` to this host (measured 2026-11-08). Pinning
/// the redirect target means the client needs no redirect-following and a
/// future reader cannot "simplify" the URL back into a 301.
pub const FRANKFURTER_EUR_USD_URL: &str = "https://api.frankfurter.dev/v1/latest?from=EUR&to=USD";

/// Provenance label stamped on a successfully fetched rate (ADR-0101 D3).
///
/// Read by the display layer as the honest `source` in
/// "€200 ≈ $X (at R EUR/USD, ⟨source⟩)". It names the ECB because that is
/// whose number it is — Frankfurter is the transport, not the authority.
pub const LIVE_RATE_SOURCE: &str = "live:ecb";

/// The quote currency key this fetcher requires in the response.
const QUOTE_CURRENCY: &str = "USD";

/// One attempt, one short timeout, no retry (ADR-0101 D5).
///
/// A retry would double the worst-case stall on a label the static path can
/// already supply.
pub const LIVE_RATE_TIMEOUT: Duration = Duration::from_secs(2);

// ── Typed error ───────────────────────────────────────────────────────────────

/// Why a live rate did not arrive. Every variant means the same thing to the
/// caller — use the static fallback (ADR-0101 D5) — but they are distinct so a
/// `tracing` line can say which one happened.
#[derive(Debug, thiserror::Error)]
pub enum FxRateFetchError {
    /// Transport failure: offline, DNS, refused, TLS, or the request exceeded
    /// [`LIVE_RATE_TIMEOUT`].
    #[error("network error fetching {url}: {source}")]
    Network {
        /// The endpoint that was attempted.
        url: String,
        /// The underlying `reqwest` failure.
        #[source]
        source: reqwest::Error,
    },
    /// The endpoint answered, but not with success.
    #[error("{url} returned HTTP {status}")]
    HttpStatus {
        /// The endpoint that was attempted.
        url: String,
        /// The non-2xx status code.
        status: u16,
    },
    /// The body could not be read, was not the expected JSON shape, or the
    /// rate token was not a decimal number.
    #[error("unusable rate body from {url}: {detail}")]
    Body {
        /// The endpoint that was attempted.
        url: String,
        /// What was wrong with the body.
        detail: String,
    },
    /// Valid JSON of the right shape, but it carries no `USD` quote.
    #[error("response from {url} carries no USD rate")]
    MissingQuote {
        /// The endpoint that was attempted.
        url: String,
    },
    /// The quote parsed but the checked ctor rejected it (rate ≤ 0).
    #[error("rejected rate from {url}: {source}")]
    Invalid {
        /// The endpoint that was attempted.
        url: String,
        /// The `trading_core` validation failure.
        #[source]
        source: FxRateError,
    },
}

// ── The seam ──────────────────────────────────────────────────────────────────

/// External I/O behind a trait (CLAUDE.md rule / story AC1) so tests inject a
/// fake and never open a socket.
#[async_trait::async_trait]
pub trait RateSource: Send + Sync {
    /// Resolve the current EUR/USD reference rate.
    ///
    /// # Errors
    /// Any [`FxRateFetchError`]. The caller's contract (ADR-0101 D5) is to fall
    /// back to its static rate on **every** error, without inspecting which.
    async fn fetch_eur_usd(&self) -> Result<FxRate, FxRateFetchError>;
}

// ── Wire shape ────────────────────────────────────────────────────────────────

/// The Frankfurter `/v1/latest` body, measured 2026-11-08:
///
/// ```json
/// {"amount":1.0,"base":"EUR","date":"2026-11-06","rates":{"USD":1.1165}}
/// ```
///
/// `rates` values are borrowed [`RawValue`]s — the untouched JSON tokens. That
/// is the whole point: see the module docs on the `f64` trap.
#[derive(Debug, Deserialize)]
struct FrankfurterLatest<'a> {
    /// The ECB publication date, already in the `YYYY-MM-DD` shape
    /// [`FxRate::as_of`] carries, so the as-of label is the provider's own.
    date: String,
    /// Quote currency → raw rate token.
    #[serde(borrow)]
    rates: BTreeMap<String, &'a RawValue>,
}

/// Parse a Frankfurter `/v1/latest` body into a validated [`FxRate`].
///
/// Pulled out of the HTTP impl so the whole decode path — including the
/// `f64`-free number parse (ADR-0101 D6) — is testable without a socket, and
/// so [`FakeRateSource`] can exercise the *real* parser rather than a
/// look-alike.
///
/// `url` is carried only for error messages.
///
/// # Errors
/// [`FxRateFetchError::Body`] when the body is not the expected JSON shape or
/// the rate token is not a decimal; [`FxRateFetchError::MissingQuote`] when
/// there is no `USD` key; [`FxRateFetchError::Invalid`] when the rate is ≤ 0.
pub fn parse_frankfurter_eur_usd(body: &str, url: &str) -> Result<FxRate, FxRateFetchError> {
    let parsed: FrankfurterLatest<'_> =
        serde_json::from_str(body).map_err(|e| FxRateFetchError::Body {
            url: url.to_owned(),
            detail: e.to_string(),
        })?;

    let token = parsed
        .rates
        .get(QUOTE_CURRENCY)
        .ok_or_else(|| FxRateFetchError::MissingQuote {
            url: url.to_owned(),
        })?
        .get()
        .trim();

    // The ONE conversion of the wire number. `Decimal::from_str` reads the
    // token's digits directly; `from_scientific` covers an exponent-notation
    // token (`1.1165e0`), which is legal JSON. Neither path touches `f64`.
    let rate = Decimal::from_str(token)
        .or_else(|_| Decimal::from_scientific(token))
        .map_err(|e| FxRateFetchError::Body {
            url: url.to_owned(),
            detail: format!("{QUOTE_CURRENCY} rate token {token:?} is not a decimal: {e}"),
        })?;

    FxRate::new(rate, LIVE_RATE_SOURCE, parsed.date.as_str()).map_err(|source| {
        FxRateFetchError::Invalid {
            url: url.to_owned(),
            source,
        }
    })
}

// ── Real HTTP implementation ──────────────────────────────────────────────────

/// The live [`RateSource`]: one GET against [`FRANKFURTER_EUR_USD_URL`].
pub struct HttpRateSource {
    client: Client,
    url: String,
}

impl HttpRateSource {
    /// Build a source with its own client carrying [`LIVE_RATE_TIMEOUT`] and
    /// no retry (ADR-0101 D5).
    ///
    /// # Errors
    /// Returns the `reqwest` error if the TLS/client stack cannot be built.
    pub fn new() -> Result<Self, reqwest::Error> {
        let client = Client::builder().timeout(LIVE_RATE_TIMEOUT).build()?;
        Ok(Self {
            client,
            url: FRANKFURTER_EUR_USD_URL.to_owned(),
        })
    }

    /// Build a source over a caller-supplied client and endpoint.
    ///
    /// The endpoint is a parameter so a future provider swap (or a local
    /// `wiremock` server in a dev-dependency test) needs no new type; the
    /// shipped path always passes [`FRANKFURTER_EUR_USD_URL`]. The caller owns
    /// the timeout when it owns the client.
    #[must_use]
    pub fn with_client(client: Client, url: impl Into<String>) -> Self {
        Self {
            client,
            url: url.into(),
        }
    }

    /// The endpoint this source quotes.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }
}

#[async_trait::async_trait]
impl RateSource for HttpRateSource {
    async fn fetch_eur_usd(&self) -> Result<FxRate, FxRateFetchError> {
        let resp = self.client.get(&self.url).send().await.map_err(|source| {
            FxRateFetchError::Network {
                url: self.url.clone(),
                source,
            }
        })?;

        let status = resp.status();
        if !status.is_success() {
            return Err(FxRateFetchError::HttpStatus {
                url: self.url.clone(),
                status: status.as_u16(),
            });
        }

        let body = resp
            .text()
            .await
            .map_err(|source| FxRateFetchError::Network {
                url: self.url.clone(),
                source,
            })?;

        parse_frankfurter_eur_usd(&body, &self.url)
    }
}

// ── Test / fixture fake ───────────────────────────────────────────────────────

/// What a [`FakeRateSource`] does when asked.
///
/// `Body` goes through the **real** [`parse_frankfurter_eur_usd`], so a test
/// that uses the fake still exercises the decode path (including the
/// `f64`-free number parse) — the fake stands in for the socket, not for the
/// parser.
#[cfg(any(test, feature = "fixtures"))]
#[derive(Debug, Clone)]
pub enum FakeRateOutcome {
    /// Serve this response body.
    Body(String),
    /// Fail as if the host were unreachable.
    Unreachable,
}

/// Fake [`RateSource`] for tests and other crates' fixtures — no socket.
///
/// Exposed under `#[cfg(any(test, feature = "fixtures"))]` like
/// [`crate::MockFeed`] and `binance_klines::MockFetcher`, so `crates/ui`'s
/// tests (which dev-depend on `data` with `features = ["fixtures"]`) can drive
/// the live-rate path without a network.
#[cfg(any(test, feature = "fixtures"))]
pub struct FakeRateSource {
    outcome: FakeRateOutcome,
    calls: std::sync::atomic::AtomicUsize,
}

#[cfg(any(test, feature = "fixtures"))]
impl FakeRateSource {
    /// A fake that serves `body` verbatim through the real parser.
    #[must_use]
    pub fn with_body(body: impl Into<String>) -> Self {
        Self {
            outcome: FakeRateOutcome::Body(body.into()),
            calls: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// A fake that serves a well-formed Frankfurter body for `rate` on `date`.
    ///
    /// `rate` is rendered with `Display`, so the body carries the decimal's own
    /// digits — a test can hand this a value no `f64` can represent.
    #[must_use]
    pub fn live(rate: Decimal, date: &str) -> Self {
        Self::with_body(format!(
            r#"{{"amount":1.0,"base":"EUR","date":"{date}","rates":{{"{QUOTE_CURRENCY}":{rate}}}}}"#
        ))
    }

    /// A fake that always fails, as if the host were unreachable — the
    /// forced-failure arm of story AC4(b).
    #[must_use]
    pub fn unreachable() -> Self {
        Self {
            outcome: FakeRateOutcome::Unreachable,
            calls: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// How many times [`RateSource::fetch_eur_usd`] has been called.
    ///
    /// Lets a test assert the fetch timing contract of story AC2 — that a
    /// render or an explicit refresh fetches, and nothing else does.
    #[must_use]
    pub fn calls(&self) -> usize {
        self.calls.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// The endpoint label the fake reports in its errors.
    pub const URL: &'static str = "fake://rate-source";
}

#[cfg(any(test, feature = "fixtures"))]
#[async_trait::async_trait]
impl RateSource for FakeRateSource {
    async fn fetch_eur_usd(&self) -> Result<FxRate, FxRateFetchError> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        match &self.outcome {
            FakeRateOutcome::Body(body) => parse_frankfurter_eur_usd(body, Self::URL),
            FakeRateOutcome::Unreachable => Err(FxRateFetchError::Body {
                url: Self::URL.to_owned(),
                detail: "forced failure (fake source)".to_owned(),
            }),
        }
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// The body measured against the live endpoint on 2026-11-08 (ADR-0101 D3).
    const MEASURED_BODY: &str =
        r#"{"amount":1.0,"base":"EUR","date":"2026-11-06","rates":{"USD":1.1165}}"#;

    #[test]
    fn parses_the_measured_body_into_rate_source_and_as_of() {
        let fx = parse_frankfurter_eur_usd(MEASURED_BODY, FRANKFURTER_EUR_USD_URL).unwrap();
        assert_eq!(fx.rate(), dec!(1.1165));
        assert_eq!(fx.source(), "live:ecb");
        // The as-of is the PROVIDER's publication date, not a clock read —
        // asked on Sunday 2026-11-08 the ECB's latest is Friday's (D4).
        assert_eq!(fx.as_of(), "2026-11-06");
    }

    #[test]
    fn a_missing_usd_key_is_an_error_not_a_zero() {
        let body = r#"{"amount":1.0,"base":"EUR","date":"2026-11-06","rates":{"CHF":0.93}}"#;
        assert!(matches!(
            parse_frankfurter_eur_usd(body, "u"),
            Err(FxRateFetchError::MissingQuote { .. })
        ));
    }

    #[test]
    fn a_non_positive_rate_is_rejected_by_the_checked_ctor() {
        for token in ["0", "0.0", "-1.1165"] {
            let body = format!(
                r#"{{"amount":1.0,"base":"EUR","date":"2026-11-06","rates":{{"USD":{token}}}}}"#
            );
            assert!(
                matches!(
                    parse_frankfurter_eur_usd(&body, "u"),
                    Err(FxRateFetchError::Invalid { .. })
                ),
                "rate token {token} must be refused by FxRate::new"
            );
        }
    }

    #[test]
    fn a_non_numeric_token_and_a_truncated_body_are_errors() {
        let not_a_number =
            r#"{"amount":1.0,"base":"EUR","date":"2026-11-06","rates":{"USD":"1.1165x"}}"#;
        assert!(matches!(
            parse_frankfurter_eur_usd(not_a_number, "u"),
            Err(FxRateFetchError::Body { .. })
        ));
        assert!(matches!(
            parse_frankfurter_eur_usd(r#"{"rates":{"USD":1.1"#, "u"),
            Err(FxRateFetchError::Body { .. })
        ));
        assert!(matches!(
            parse_frankfurter_eur_usd("", "u"),
            Err(FxRateFetchError::Body { .. })
        ));
    }

    #[test]
    fn exponent_notation_is_legal_json_and_parses() {
        let body = r#"{"amount":1.0,"base":"EUR","date":"2026-11-06","rates":{"USD":1.1165e0}}"#;
        let fx = parse_frankfurter_eur_usd(body, "u").unwrap();
        assert_eq!(fx.rate(), dec!(1.1165));
    }

    #[tokio::test]
    async fn the_fake_serves_a_live_rate_and_counts_calls() {
        let fake = FakeRateSource::live(dec!(1.1165), "2026-11-06");
        assert_eq!(fake.calls(), 0, "constructing the fake fetches nothing");
        let fx = fake.fetch_eur_usd().await.unwrap();
        assert_eq!(fx.rate(), dec!(1.1165));
        assert_eq!(fx.source(), LIVE_RATE_SOURCE);
        assert_eq!(fake.calls(), 1);
    }

    #[tokio::test]
    async fn the_unreachable_fake_always_fails() {
        let fake = FakeRateSource::unreachable();
        assert!(fake.fetch_eur_usd().await.is_err());
        assert_eq!(fake.calls(), 1);
    }

    #[test]
    fn the_shipped_url_is_the_dev_host() {
        // `.app` answers 301 (ADR-0101 D3) — a "simplification" back to it
        // would need a redirect-following client this one deliberately is not.
        assert!(FRANKFURTER_EUR_USD_URL.starts_with("https://api.frankfurter.dev/"));
        assert!(FRANKFURTER_EUR_USD_URL.contains("from=EUR"));
        assert!(FRANKFURTER_EUR_USD_URL.contains("to=USD"));
    }
}
