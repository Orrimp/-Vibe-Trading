//! Which [`FxRate`] reaches the budget seam (story 3-19 — ADR-0101).
//!
//! F7 (ADR-0065) gave the cockpit ONE EUR→USDT conversion, built at the
//! forward-launch seam from the configured static rate. This module decides
//! **which** rate that seam uses: the live ECB reference rate when a fetch
//! succeeded, the configured static rate otherwise. Everything downstream of
//! [`trading_core::BudgetConversion`] is unchanged — the engine and the display
//! still read one converted value, so they cannot drift (story AC3).
//!
//! # The policy, in one line
//!
//! **Any failure is the static path** (ADR-0101 D5). Offline, DNS, timeout,
//! non-2xx, unparseable body, missing quote, non-positive rate — all of them
//! resolve to `AdvisorConfig.eur_usd_rate` → [`trading_core::DEFAULT_EUR_USD_RATE`]
//! with its `"config"` provenance. The static rate is a permanent fallback, not
//! a migration step: a dead network can never block or delay a plan beyond one
//! short fetch budget.
//!
//! # Why the policy lives in the library and not in the binary
//!
//! The two seams it serves are in `bin/cockpit_live.rs`, which no integration
//! test can import. Story AC4 requires the live-success and forced-failure
//! paths to be proven through a fake, so the decision lives here, in `ui`, and
//! the binary only supplies the source and the runtime handle.
//!
//! # Fetch timing (ADR-0101 D7 / story AC2)
//!
//! Nothing in this module polls or fetches on its own. The binary calls
//! [`fetch_live_fx_blocking`] at a plan render and on an explicit operator
//! refresh — no fetch at boot, no background loop, no subscription recipe.

use rust_decimal::Decimal;
use trading_core::FxRate;

/// Build the static (configured) rate with its `"config"` provenance.
///
/// This is the F7 block that was duplicated at both forward-launch seams:
/// `FxRate::config(rate)`, upgraded to carry the operator's `as_of` label when
/// one is configured. Non-positive config values cannot reach here — the
/// config boundary validates them — and `FxRate::config` keeps the
/// `debug_assert` for the programming-error case.
#[must_use]
pub fn static_fx_rate(rate: Decimal, as_of: &str) -> FxRate {
    let fallback = FxRate::config(rate);
    if as_of.is_empty() {
        fallback
    } else {
        FxRate::new(rate, "config", as_of).unwrap_or(fallback)
    }
}

/// ADR-0101 D5 — resolve the live fetch outcome against the static fallback.
///
/// `Ok(rate)` is used as-is (it came out of the checked [`FxRate::new`] ctor in
/// `data`, so it is already validated). Every `Err` — whatever it was — becomes
/// the static path, with one `debug`-level line naming the cause so an operator
/// chasing a `"config"` label in the UI can find out why.
#[must_use]
pub fn resolve_fx_rate<E: std::fmt::Display>(
    live: Result<FxRate, E>,
    static_rate: Decimal,
    static_as_of: &str,
) -> FxRate {
    match live {
        Ok(rate) => rate,
        Err(err) => {
            tracing::debug!(
                error = %err,
                "live EUR/USD fetch did not arrive; using the configured static rate"
            );
            static_fx_rate(static_rate, static_as_of)
        }
    }
}

// ── The blocking bridge (binary-facing) ───────────────────────────────────────
//
// Gated on `live-fx` AND `live`: `live-fx` brings the `data` dep (the trait),
// `live` brings `tokio` (the runtime handle). `live-fx` alone therefore still
// compiles — which is what keeps `cargo check -p ui --no-default-features
// --features fixtures` (ci.yml) honest.

/// How long a plan render will wait for the live rate before giving up.
///
/// `data::LIVE_RATE_TIMEOUT` (2 s) is the HTTP client's own cap; this is the
/// outer backstop for the whole hand-off, including the hop onto the agent
/// runtime, so a wedged task cannot hold the frame open indefinitely. The
/// small margin exists so the inner timeout is normally the one that fires and
/// the error says `network`, not `deadline`.
#[cfg(all(feature = "live-fx", feature = "live"))]
pub const PLAN_FETCH_BUDGET: std::time::Duration = std::time::Duration::from_millis(2_500);

/// Why no live rate arrived, from the caller's side of the bridge.
#[cfg(all(feature = "live-fx", feature = "live"))]
#[derive(Debug, thiserror::Error)]
pub enum LiveFxError {
    /// The source itself reported a failure (offline, non-2xx, unparseable, …).
    #[error(transparent)]
    Fetch(#[from] data::fx_rate::FxRateFetchError),
    /// The fetch did not come back inside [`PLAN_FETCH_BUDGET`].
    #[error("live EUR/USD fetch exceeded the {0:?} render budget")]
    Deadline(std::time::Duration),
}

/// Run one live fetch to completion on the agent runtime, bounded by `budget`.
///
/// Called from iced's thread, which has no tokio runtime of its own: the future
/// is spawned onto the agent runtime's `Handle` and the result is awaited over
/// a `std::sync::mpsc` channel with a timeout. Deliberately **not**
/// `Handle::block_on`, which panics if it is ever reached from inside an async
/// context — on the render path a panic would take the cockpit down, and the
/// whole point of ADR-0101 D5 is that a bad fetch costs a label, not the app.
///
/// One attempt. No retry (ADR-0101 D5).
///
/// # Errors
/// [`LiveFxError::Fetch`] when the source failed, [`LiveFxError::Deadline`]
/// when it did not answer in time (a dropped sender is reported the same way —
/// the runtime went away, so no rate is coming).
#[cfg(all(feature = "live-fx", feature = "live"))]
pub fn fetch_live_fx_blocking(
    source: &std::sync::Arc<dyn data::fx_rate::RateSource>,
    rt: &tokio::runtime::Handle,
    budget: std::time::Duration,
) -> Result<FxRate, LiveFxError> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let source = std::sync::Arc::clone(source);
    rt.spawn(async move {
        // A send error means the render already gave up and dropped the
        // receiver; there is nothing to do about it and nothing to log.
        let _ = tx.send(source.fetch_eur_usd().await);
    });

    match rx.recv_timeout(budget) {
        Ok(Ok(rate)) => Ok(rate),
        Ok(Err(err)) => Err(LiveFxError::Fetch(err)),
        Err(_) => Err(LiveFxError::Deadline(budget)),
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn the_static_path_keeps_the_config_provenance() {
        let fx = static_fx_rate(dec!(1.08), "");
        assert_eq!(fx.rate(), dec!(1.08));
        assert_eq!(fx.source(), "config");
        assert_eq!(fx.as_of(), "");
    }

    #[test]
    fn a_configured_as_of_label_is_carried() {
        let fx = static_fx_rate(dec!(1.08), "2026-06-22");
        assert_eq!(fx.source(), "config");
        assert_eq!(fx.as_of(), "2026-06-22");
    }

    #[test]
    fn a_live_rate_is_used_as_is() {
        let live = FxRate::new(dec!(1.1165), "live:ecb", "2026-11-06").unwrap();
        let resolved = resolve_fx_rate::<std::io::Error>(Ok(live), dec!(1.08), "");
        assert_eq!(resolved.rate(), dec!(1.1165));
        assert_eq!(resolved.source(), "live:ecb");
        assert_eq!(resolved.as_of(), "2026-11-06");
    }

    #[test]
    fn every_failure_is_the_static_path() {
        let resolved = resolve_fx_rate(Err::<FxRate, &str>("offline"), dec!(1.08), "2026-06-22");
        assert_eq!(resolved.rate(), dec!(1.08));
        assert_eq!(resolved.source(), "config");
        assert_eq!(resolved.as_of(), "2026-06-22");
    }
}
