//! Regression test: Prometheus `/metrics` endpoint must return all R9.2 metric
//! names after the recorder is installed.
//!
//! This test verifies HF-2: `start_prometheus_exporter` (which calls
//! `PrometheusBuilder::install()`) must be called **before** `register_metrics()`
//! so that the registered counters/gauges are captured by the Prometheus
//! recorder rather than the no-op default recorder.
//!
//! Port 19100 is used instead of the default 9100 to avoid conflicts with a
//! running agent in development.  The port is hard-coded; if it is already in
//! use the test will fail with a bind error rather than a false pass.

#![allow(clippy::unwrap_used)]

const METRICS_ADDR: &str = "127.0.0.1:19100";
const METRICS_URL: &str = "http://127.0.0.1:19100/metrics";

/// All metric names required by R9.2 (as registered in `observability::register_metrics`).
const REQUIRED_METRICS: &[&str] = &[
    "bars_in_total",
    "fills_total",
    "clock_skew_ms",
    "ledger_imbalance_total",
    "ticks_in_total",
    "signals_total",
    "orders_sent_total",
    "kill_switch_trips_total",
    "fees_usdt_total",
    "position_qty",
    "equity_usdt",
    "cash_usdt",
];

#[tokio::test]
async fn t27_metrics_endpoint_returns_all_r9_2_names() {
    // Install the Prometheus recorder first, then register metrics.
    // This is the fixed ordering introduced in HF-2.
    let cfg = agent::config::ObservabilityConfig {
        prometheus_listen: METRICS_ADDR.into(),
        prometheus_enabled: true,
    };
    agent::observability::start_prometheus_exporter(&cfg)
        .expect("failed to start prometheus exporter on test port");
    agent::observability::register_metrics();

    // Give the HTTP server a moment to bind.
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    let client = reqwest::Client::new();
    let body = client
        .get(METRICS_URL)
        .send()
        .await
        .expect("GET /metrics failed")
        .text()
        .await
        .expect("read /metrics body");

    assert!(
        !body.is_empty(),
        "/metrics returned an empty body — recorder ordering bug (HF-2) may not be fixed"
    );

    for name in REQUIRED_METRICS {
        assert!(
            body.contains(name),
            "/metrics body is missing metric '{name}'.\nFull body:\n{body}"
        );
    }
}
