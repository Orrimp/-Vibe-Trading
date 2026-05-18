//! Observability wiring (T27).
//!
//! - `tracing` JSON layer to stdout + rolling file.
//! - `metrics-exporter-prometheus` on the configured port.
//! - All counters and gauges from R9.2 registered at startup.

use std::net::SocketAddr;

use anyhow::{Context, Result};
use metrics::{Unit, counter, describe_counter, describe_gauge, gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use tracing::info;

use crate::config::ObservabilityConfig;

/// Register all v0 metrics (R9.2).
///
/// Call once at agent startup before any data flows.
pub fn register_metrics() {
    // ── Counters ──────────────────────────────────────────────────────────────
    describe_counter!(
        "bars_in_total",
        Unit::Count,
        "Total 1m bars received from the feed"
    );
    describe_counter!(
        "ticks_in_total",
        Unit::Count,
        "Total raw trades received from the feed"
    );
    describe_counter!(
        "signals_total",
        Unit::Count,
        "Total strategy signals emitted"
    );
    describe_counter!(
        "orders_sent_total",
        Unit::Count,
        "Total orders sent to the matching engine"
    );
    describe_counter!(
        "fills_total",
        Unit::Count,
        "Total fills received from the matching engine"
    );
    describe_counter!(
        "kill_switch_trips_total",
        Unit::Count,
        "Number of times the kill switch has been tripped"
    );
    describe_counter!(
        "ledger_imbalance_total",
        Unit::Count,
        "Number of ledger reconciliation mismatches"
    );
    describe_counter!(
        "fees_usdt_total",
        Unit::Count, // we use Count as a proxy; actual is a decimal
        "Cumulative taker fees in USDT"
    );

    // ── Gauges ────────────────────────────────────────────────────────────────
    describe_gauge!(
        "clock_skew_ms",
        Unit::Milliseconds,
        "Current clock skew vs venue timestamp (ms)"
    );
    describe_gauge!(
        "position_qty",
        Unit::Count,
        "Current base-asset position quantity"
    );
    describe_gauge!("equity_usdt", Unit::Count, "Current total equity in USDT");
    describe_gauge!("cash_usdt", Unit::Count, "Current cash balance in USDT");

    // Initialise counters at zero so they appear in /metrics immediately
    counter!("bars_in_total", "symbol" => "BTCUSDT").absolute(0);
    counter!("ticks_in_total", "symbol" => "BTCUSDT").absolute(0);
    counter!("signals_total").absolute(0);
    counter!("orders_sent_total").absolute(0);
    counter!("fills_total").absolute(0);
    counter!("kill_switch_trips_total").absolute(0);
    counter!("ledger_imbalance_total").absolute(0);
    counter!("fees_usdt_total").absolute(0);
    gauge!("clock_skew_ms", "feed" => "binance").set(0.0);
    gauge!("position_qty", "symbol" => "BTCUSDT").set(0.0);
    gauge!("equity_usdt").set(0.0);
    gauge!("cash_usdt").set(0.0);

    info!("metrics registered");
}

/// Start the Prometheus exporter HTTP server.
///
/// When `cfg.prometheus_enabled` is `false` (T901 / live-cockpit-unified
/// Q4), the function returns `Ok(())` *without* binding any listener and
/// emits one `prometheus_listener_disabled` info line — the operator can
/// run the unified `cockpit_live` binary on a laptop without fighting for
/// port `:9100`.  When `true` (default), behavior is unchanged from the
/// pre-feature implementation.
///
/// # Errors
///
/// Returns an error if `cfg.prometheus_enabled = true` and the address
/// cannot be parsed or bound.
pub fn start_prometheus_exporter(cfg: &ObservabilityConfig) -> Result<()> {
    if !cfg.prometheus_enabled {
        info!(
            listen = %cfg.prometheus_listen,
            "prometheus_listener_disabled"
        );
        return Ok(());
    }
    let addr: SocketAddr = cfg
        .prometheus_listen
        .parse()
        .context("parse prometheus listen address")?;
    PrometheusBuilder::new()
        .with_http_listener(addr)
        .install()
        .context("install prometheus exporter")?;
    info!(addr = %cfg.prometheus_listen, "Prometheus exporter started");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T901 — when `prometheus_enabled = false`, the function does NOT
    /// attempt to bind a listener and returns `Ok(())`.  Under the
    /// pre-feature behavior this same call would fail loudly when port
    /// 9100 was already in use; with the toggle off it must succeed
    /// silently regardless of port availability.
    ///
    /// We assert the success path on a deliberately-bogus address — if
    /// the function tried to bind it, parsing or binding would error.
    #[test]
    fn t901_disabled_skips_listener() {
        let cfg = ObservabilityConfig {
            prometheus_listen: "this-is-not-an-address-and-would-fail-to-parse".into(),
            prometheus_enabled: false,
        };
        // Must succeed despite the malformed listen string — the
        // function short-circuits before parsing.
        start_prometheus_exporter(&cfg).expect("disabled exporter must skip parsing");
    }
}
