//! Harness for `spawn_lab_run`'s Binance preload arm
//! (simple-strategies-realdata review patch 5).
//!
//! ## What this file tests
//!
//! The production Binance branch glue — sentinel → `spawn_preload_on_rt` →
//! `classify_preload_result` → `data_source = BinanceCache` +
//! `bars_override = Some(bars)` — end-to-end with a MOCK source. The seam is
//! [`ui::lab::runner::run_binance_preload_arm`]: the SAME function
//! `spawn_lab_run`'s `#[cfg(feature = "binance")]` block calls with
//! `Box::new(DefaultLabBinanceBarSource)`, so these tests exercise the real
//! production glue, not a replica. There is NO `binance_source_override`
//! parameter on `spawn_lab_run` (the pre-review docs claimed one); injection
//! happens by calling this seam with any `LabBinanceBarSource`.
//!
//! Mirrors `spawn_lab_run_yahoo_harness.rs`'s real pattern: a mock
//! implementing `LabBarSource` + the marker trait, driven directly (the
//! `iced::Task` returned by `spawn_lab_run` cannot be polled without an iced
//! runtime).
//!
//! ## `#[cfg(feature = "live")]` gate
//!
//! `LabBinanceBarSource` and `run_binance_preload_arm` are compiled under
//! `live` only (NOT `binance`) — exactly so this harness can inject a fake
//! source without the real-corpus feature.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use backtest::progress::progress_pair;
use backtest::scenarios::sma_composed_run::{default_start_price, synthetic_bars_minute};
use smol_str::SmolStr;
use tokio::time::timeout;
use trading_core::Symbol;
use ui::lab::runner::{
    LabBarSource, LabBinanceBarSource, LabRunConfig, lab_config_to_scenario,
    preload_notice::{self, RunMessageKind},
    run_binance_preload_arm,
};
use ui::lab::state::LabDataSource;

// ── MockLabBinanceBarSource ───────────────────────────────────────────────────

/// What the mock preload returns.
enum MockBehavior {
    /// `Ok(bars, sha)` with the given number of deterministic synthetic bars.
    Bars(usize),
    /// `Ok(vec![], sha)` — the zero-bars defensive-arm trigger.
    Empty,
    /// `Err(msg)` — a hard preload failure.
    Fail(&'static str),
}

struct MockLabBinanceBarSource {
    behavior: MockBehavior,
}

impl LabBarSource for MockLabBinanceBarSource {
    fn preload<'a>(
        &'a self,
        cfg: &'a LabRunConfig,
        _range: &'a backtest::engine::DateRange,
    ) -> ui::lab::runner::PreloadFuture<'a> {
        Box::pin(async move {
            match self.behavior {
                MockBehavior::Bars(n) => {
                    let sym = Symbol::new(cfg.symbol.as_str());
                    let start_price = default_start_price(&sym);
                    let bars = synthetic_bars_minute(&sym, n, 0xB1AB, start_price, 2023);
                    Ok((bars, SmolStr::new("mock-binance-sha-0000000000000000")))
                }
                MockBehavior::Empty => Ok((
                    Vec::new(),
                    SmolStr::new("mock-binance-sha-0000000000000000"),
                )),
                MockBehavior::Fail(msg) => Err(SmolStr::new(msg)),
            }
        })
    }
}

impl LabBinanceBarSource for MockLabBinanceBarSource {}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a `LabRunConfig` pointing at the BinanceCache source (the state
/// `spawn_lab_run` gates the arm on; `classify_preload_result` also branches
/// on it for the source-appropriate notice copy — review patch 11).
fn binance_cache_cfg() -> LabRunConfig {
    LabRunConfig {
        strategy_id: SmolStr::new("v0.sma"),
        symbol: SmolStr::new("BTCUSDT"),
        venue: SmolStr::new("Binance"),
        range_label: SmolStr::new("H1_2024"),
        seed: ui::lab::defaults::LAB_DEFAULT_SEED,
        write_report: false,
        data_source: LabDataSource::BinanceCache,
        sma_fast_len: None,
        sma_slow_len: None,
    }
}

// ── Test 1 — success path wires classify + bars_override glue ─────────────────

/// The production glue end-to-end with a mock source: `run_binance_preload_arm`
/// (the exact function `spawn_lab_run` calls) must
///   (a) emit the `Progress { 0, 1, 0 }` sentinel BEFORE resolving,
///   (b) return `Ok(())`, and
///   (c) mutate `scenario_cfg` to `data_source = BinanceCache` +
///       `bars_override = Some(<the mock's bars>)`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn production_glue_sets_binance_source_and_bars_override() {
    let cfg = binance_cache_cfg();
    let mut scenario_cfg = lab_config_to_scenario(&cfg).expect("H1_2024 maps");
    assert_eq!(
        scenario_cfg.data_source,
        backtest::engine::ScenarioDataSource::Synthetic,
        "precondition: mapper leaves the engine default (Synthetic)"
    );
    assert!(
        scenario_cfg.bars_override.is_none(),
        "precondition: no bars"
    );

    let (progress_tx, mut progress_rx) = progress_pair();
    let mock = Box::new(MockLabBinanceBarSource {
        behavior: MockBehavior::Bars(120),
    });

    let rt = tokio::runtime::Handle::current();
    run_binance_preload_arm(&rt, mock, &cfg, &mut scenario_cfg, &progress_tx)
        .await
        .expect("mock preload with bars must succeed");

    // (a) sentinel arrived (first event, the shape the Yahoo harness pins).
    let first = timeout(Duration::from_millis(200), progress_rx.recv())
        .await
        .expect("sentinel must arrive")
        .expect("progress channel open");
    assert_eq!(first.current_bar, 0, "sentinel current_bar");
    assert_eq!(first.total_bars, 1, "sentinel total_bars placeholder");
    assert_eq!(first.elapsed_ms, 0, "sentinel elapsed_ms");

    // (c) the glue mutated the scenario config.
    assert_eq!(
        scenario_cfg.data_source,
        backtest::engine::ScenarioDataSource::BinanceCache,
        "glue must flip data_source to BinanceCache"
    );
    let bars = scenario_cfg
        .bars_override
        .as_ref()
        .expect("glue must set bars_override = Some(bars)");
    assert_eq!(bars.len(), 120, "the MOCK's bars must reach bars_override");
}

// ── Test 2 — zero-bars routes to the tagged, Binance-branded notice ───────────

/// `Ok(vec![])` from the source must short-circuit with a `NO_DATA_TAG`-tagged
/// notice (amber channel) carrying BINANCE copy — not Yahoo-branded copy
/// (review patch 11) — and must leave `scenario_cfg` untouched.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn empty_preload_routes_to_tagged_binance_notice() {
    let cfg = binance_cache_cfg();
    let mut scenario_cfg = lab_config_to_scenario(&cfg).expect("H1_2024 maps");
    let (progress_tx, _progress_rx) = progress_pair();
    let mock = Box::new(MockLabBinanceBarSource {
        behavior: MockBehavior::Empty,
    });

    let rt = tokio::runtime::Handle::current();
    let err = run_binance_preload_arm(&rt, mock, &cfg, &mut scenario_cfg, &progress_tx)
        .await
        .expect_err("empty preload must short-circuit");

    assert!(
        err.starts_with(preload_notice::NO_DATA_TAG),
        "zero-bars must be NO_DATA_TAG-tagged (amber notice channel); got: {err:?}"
    );
    match preload_notice::classify(err.as_str()) {
        RunMessageKind::Notice(body) => {
            assert!(
                body.contains("Binance"),
                "notice must carry Binance copy; got: {body}"
            );
            assert!(
                !body.contains("Yahoo"),
                "a Binance run must NOT ship Yahoo-branded copy (patch 11); got: {body}"
            );
            assert!(
                body.contains("BTCUSDT"),
                "notice must name the symbol; got: {body}"
            );
        }
        RunMessageKind::Error(e) => panic!("expected Notice, got Error({e})"),
    }

    // scenario_cfg untouched on the short-circuit path.
    assert_eq!(
        scenario_cfg.data_source,
        backtest::engine::ScenarioDataSource::Synthetic,
        "failed preload must not flip data_source"
    );
    assert!(
        scenario_cfg.bars_override.is_none(),
        "failed preload must not set bars_override"
    );
}

// ── Test 3 — hard errors pass through verbatim ────────────────────────────────

/// An `Err` from the source (loader failure) passes through unmodified —
/// untagged, so it renders on the red error channel, and `scenario_cfg` stays
/// untouched.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn preload_error_passes_through_verbatim() {
    let cfg = binance_cache_cfg();
    let mut scenario_cfg = lab_config_to_scenario(&cfg).expect("H1_2024 maps");
    let (progress_tx, _progress_rx) = progress_pair();
    let mock = Box::new(MockLabBinanceBarSource {
        behavior: MockBehavior::Fail("mock parquet io failure"),
    });

    let rt = tokio::runtime::Handle::current();
    let err = run_binance_preload_arm(&rt, mock, &cfg, &mut scenario_cfg, &progress_tx)
        .await
        .expect_err("failing preload must short-circuit");

    assert_eq!(
        err.as_str(),
        "mock parquet io failure",
        "hard preload errors must pass through verbatim"
    );
    assert!(
        !err.starts_with(preload_notice::NO_DATA_TAG),
        "hard errors are NOT tagged (they render red, not amber)"
    );
    assert!(
        scenario_cfg.bars_override.is_none(),
        "failed preload must not set bars_override"
    );
}
