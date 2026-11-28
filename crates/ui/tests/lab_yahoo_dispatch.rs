//! T-C3.7 — Integration test for the Yahoo dispatch path.
//!
//! Validates the dispatch boundary from `LabConfig` (YahooCache source)
//! through `lab_config_to_scenario` and the `YahooBarSource::load_cached`
//! fixture path that `preload_yahoo_bars` delegates to.
//!
//! ## Test strategy
//!
//! `preload_yahoo_bars` is `#[cfg(feature = "yahoo")]`-gated and private to the
//! `lab::runner` module.  We test it indirectly via its two observable seams:
//!
//! 1. **Ticker mapping** — `data::yahoo::binance_to_yahoo_ticker("BTCUSDT") == "BTC-USD"`
//!    This is the conversion boundary (Q6 = (a) / D7) called inside `preload_yahoo_bars`.
//!
//! 2. **`lab_config_to_scenario` ScenarioConfig shape** — verifies that
//!    `LabRunConfig { data_source: YahooCache, symbol: "BTCUSDT", range: Preset(H1_2024) }`
//!    maps to the expected `ScenarioConfig` (strategy, pair, range, seed, write_report).
//!    The `data_source` + `bars_override` fields are set by the Yahoo preload arm
//!    after the preload (the `ScenarioDataSource::YahooCache` assignment in
//!    `crates/ui/src/lab/runner.rs`); this test validates the pre-fill config shape.
//!
//! 3. **`YahooBarSource::load_cached` with H1_2024 fixture** — exercises the
//!    parquet read + revision-verify + coverage-check path that `preload_yahoo_bars`
//!    calls.  Reads from `crates/data/tests/fixtures/yahoo/` directly.
//!    Uses `DateRange::H1_2024` which spans 2024-01-01..2024-07-01 (182 days → `Days1`
//!    cadence); the fixture has January 2024 only.  The test asserts
//!    `bars.len() > 0` (a floor: the exact count of the checked-in parquet is not
//!    pinned here); coverage threshold is relaxed because only 01.parquet is
//!    available.
//!
//! ## Cache-root gap note (T-C3.7 scope caveat)
//!
//! `preload_yahoo_bars` resolves the cache root as `PathBuf::from("data/yahoo")` which
//! is relative to the process CWD at runtime.  In the fixture-test path the fixture
//! lives at `crates/data/tests/fixtures/yahoo/`.  The tests below construct
//! `YahooBarSource::new(<fixture_path>)` directly, bypassing the `data/yahoo` root —
//! this validates the dispatch *boundary* (ticker conversion + config shape) but not
//! the exact file-system resolution used by the running cockpit.  A follow-up task
//! (`lab-yahoo-realdata` v0.1.1) should add a fixture at `data/yahoo/` or make the
//! cache root configurable via the runner.
//!
//! ## The fixture is committed (no skip)
//!
//! `crates/data/tests/fixtures/yahoo/{REVISION.toml,BTC-USD/1d/2024/01.parquet}`
//! is tracked in git, and [`fixture_root`] derives its path from
//! `CARGO_MANIFEST_DIR` (no cwd dependence). A missing fixture is therefore a
//! broken checkout and FAILS (bug-log `#66`). Until story 2-63 Wave E3 both
//! fixture tests printed "fixture not found" and returned early instead.
//!
//! Shape (story 2-63 Wave E2 / E3): each fixture-reading test is a
//! fixture-parameterized body whose first statement is
//! `assert_yahoo_fixture(&fixture);`. The real test calls the body at
//! [`fixture_root`]; a `#[should_panic(expected = "absent means a broken
//! checkout")]` twin calls the SAME body at an empty temp dir. One twin per
//! body (a twin calling only the helper would not pin the body).
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! Each probe is ONE exact-once replace in THIS file (`grep -c` of the OLD
//! text is 1), run with `cargo test -p ui --test lab_yahoo_dispatch`, then
//! restored from a `cp -p` snapshot (`cmp`). The skip prefix NEW inserts is
//! `if !fixture.join("REVISION.toml").is_file() { return; }`, the shape
//! Wave E3 removed.
//!
//! | # | symbol | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | E3-YD-P1 | `assert_yahoo_fixture` | `assert!(` followed by `fixture.join("REVISION.toml").is_file(),` | the skip prefix above the assert | both twins: "panic did not contain expected string" (each body then panics at its `load_cached` expect, "load_cached with Jan-2024 fixture must succeed" / "first load must succeed") |
//! | E3-YD-P2 | `jan_2024_fixture_load_body` | `assert_yahoo_fixture(&fixture);` followed by `let src = YahooBarSource::new(fixture);` | the skip prefix above the pair | `jan_2024_fixture_absent_is_a_failure_not_a_skip`: "test did not panic as expected" |
//! | E3-YD-P3 | `revision_sha_determinism_body` | `assert_yahoo_fixture(&fixture);` followed by `let start_ms: i64 = 1_704_067_200_000; // 2024-01-01` | the skip prefix above the pair | `revision_fixture_absent_is_a_failure_not_a_skip`: "test did not panic as expected" |
//!
//! GREEN in every probe: the seven real tests.

#![cfg(feature = "yahoo")]

use std::path::PathBuf;

use smol_str::SmolStr;
use trading_core::{StrategyId, Symbol, Venue};
use ui::lab::defaults::LAB_DEFAULT_SEED;
use ui::lab::runner::{LabRunConfig, lab_config_to_scenario};
use ui::lab::state::LabDataSource;

// ── Test 1: ticker conversion boundary ───────────────────────────────────────

/// T-C3.7 — `binance_to_yahoo_ticker("BTCUSDT")` returns `"BTC-USD"`.
///
/// This asserts the Q6 = (a) / D7 conversion that `preload_yahoo_bars` calls
/// before constructing `YahooBarSource::load_cached`.
#[test]
fn btcusdt_maps_to_btc_usd() {
    let sym = Symbol::new("BTCUSDT");
    let result = data::yahoo::binance_to_yahoo_ticker(&sym)
        .expect("BTCUSDT must map to BTC-USD without error");
    assert_eq!(result.as_str(), "BTC-USD", "ticker mapping mismatch");
}

/// T-C3.7 — `binance_to_yahoo_ticker` covers all 10 crypto-mirror pairs.
///
/// Verifies the full conversion table (T-AR2 / Q2 = (a)).
#[test]
fn all_10_crypto_mirror_pairs_map() {
    let pairs = [
        ("BTCUSDT", "BTC-USD"),
        ("ETHUSDT", "ETH-USD"),
        ("BNBUSDT", "BNB-USD"),
        ("SOLUSDT", "SOL-USD"),
        ("XRPUSDT", "XRP-USD"),
        ("ADAUSDT", "ADA-USD"),
        ("DOGEUSDT", "DOGE-USD"),
        ("AVAXUSDT", "AVAX-USD"),
        ("DOTUSDT", "DOT-USD"),
        ("LINKUSDT", "LINK-USD"),
    ];
    for (binance, yahoo) in pairs {
        let sym = Symbol::new(binance);
        let mapped = data::yahoo::binance_to_yahoo_ticker(&sym)
            .unwrap_or_else(|e| panic!("ticker {binance} failed to map: {e}"));
        assert_eq!(
            mapped.as_str(),
            yahoo,
            "ticker mapping mismatch for {binance}"
        );
    }
}

/// T-C3.7 — unmapped ticker returns `UnmappedTicker` error.
#[test]
fn unmapped_ticker_returns_error() {
    let sym = Symbol::new("FOOUSDT");
    let result = data::yahoo::binance_to_yahoo_ticker(&sym);
    assert!(
        result.is_err(),
        "FOOUSDT must return UnmappedTicker error; got: {result:?}"
    );
}

// ── Test 2: lab_config_to_scenario config shape ───────────────────────────────

/// T-C3.7 — `lab_config_to_scenario` with `YahooCache + BTCUSDT + H1_2024`
/// produces a well-formed `ScenarioConfig`.
///
/// Asserts that the symbol, strategy, range, seed and write_report fields
/// are threaded through correctly.  `data_source` and `bars_override` are set
/// post-call by `spawn_lab_run` (not this function's responsibility).
#[test]
fn lab_config_to_scenario_yahoo_btcusdt_h1_2024() {
    let cfg = LabRunConfig {
        strategy_id: SmolStr::new("v0.sma"),
        symbol: SmolStr::new("BTCUSDT"),
        venue: SmolStr::new("Binance"),
        range: ui::lab::state::DateRange::Preset(ui::lab::state::Preset::H1_2024),
        seed: LAB_DEFAULT_SEED,
        write_report: false,
        data_source: LabDataSource::YahooCache,
        sma_fast_len: None,
        sma_slow_len: None,
    };

    let sc = lab_config_to_scenario(&cfg).expect("H1_2024 must be a known range");

    assert_eq!(
        sc.strategy,
        StrategyId("v0.sma".into()),
        "strategy id mismatch"
    );
    assert_eq!(
        sc.pair.1,
        Symbol::new("BTCUSDT"),
        "symbol must be BTCUSDT (Binance-style; Yahoo conversion at dispatch boundary)"
    );
    assert_eq!(
        sc.pair.0,
        Venue::Binance,
        "venue must be Binance in base config"
    );
    assert_eq!(sc.seed, LAB_DEFAULT_SEED, "seed must pass through");
    assert!(!sc.write_report, "write_report must be false");
    // data_source and bars_override are set by spawn_lab_run after lab_config_to_scenario.
    // Confirm the pre-fill defaults:
    assert_eq!(
        sc.data_source,
        backtest::engine::ScenarioDataSource::Synthetic,
        "data_source pre-fill must be Synthetic (set by caller post-return)"
    );
    assert!(
        sc.bars_override.is_none(),
        "bars_override pre-fill must be None (set by caller post-return)"
    );
}

/// T-C3.7 — `lab_config_to_scenario` accepts `Last30d` range label.
#[test]
fn lab_config_to_scenario_yahoo_last30d_is_ok() {
    let cfg = LabRunConfig {
        strategy_id: SmolStr::new("v0.sma"),
        symbol: SmolStr::new("BTCUSDT"),
        venue: SmolStr::new("Binance"),
        range: ui::lab::state::DateRange::Preset(ui::lab::state::Preset::Last30d),
        seed: LAB_DEFAULT_SEED,
        write_report: false,
        data_source: LabDataSource::YahooCache,
        sma_fast_len: None,
        sma_slow_len: None,
    };
    let result = lab_config_to_scenario(&cfg);
    assert!(
        result.is_ok(),
        "Last30d must map to a valid DateRange; got: {result:?}"
    );
}

// ── Test 3: YahooBarSource::load_cached with fixture ─────────────────────────

use data::yahoo::{Interval, YahooBarSource};

/// Absolute path to the shipped test fixture for Yahoo parquet data.
///
/// Layout: `crates/data/tests/fixtures/yahoo/BTC-USD/1d/2024/01.parquet`
/// Contains 31 daily bars for January 2024 (2024-01-01..2024-01-31).
fn fixture_root() -> PathBuf {
    // `env!("CARGO_MANIFEST_DIR")` is `crates/ui` for this test file.
    // The Yahoo fixture lives under `crates/data/tests/fixtures/yahoo/`.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("..") // up to crates/
        .join("data")
        .join("tests")
        .join("fixtures")
        .join("yahoo")
}

/// M1, the committed-fixture precondition (bug-log `#66`): PANIC when the
/// fixture's `REVISION.toml` is missing under `fixture`.
fn assert_yahoo_fixture(fixture: &std::path::Path) {
    assert!(
        fixture.join("REVISION.toml").is_file(),
        "the committed Yahoo fixture must ship at {} — absent means a broken checkout, not a skip (bug-log #66)",
        fixture.display()
    );
}

/// T-C3.7 — `YahooBarSource::load_cached` with the January 2024 fixture
/// returns `bars.len() > 0` for the H1_2024 range.
///
/// The H1_2024 range spans 2024-01-01..2024-07-01 (182 days → `Days1` cadence).
/// Only January 2024 (01.parquet) is present in the fixture; the
/// remaining months produce a `CacheMiss`.  This test uses a narrower
/// `Custom` range that exactly covers January so it passes with the
/// available fixture without triggering a CacheMiss.
///
/// Gap acknowledged: testing the full `Last30d` Yahoo dispatch (which uses
/// a rolling `now()` window) requires a live `data/yahoo/` cache populated
/// by `fetch_yahoo_klines`.  That path is exercised by the cockpit operator
/// manually (H3 hypothesis gate) and is out of scope for offline fixture tests.
#[test]
fn yahoo_bar_source_jan_2024_fixture_loads_bars() {
    jan_2024_fixture_load_body(fixture_root());
}

/// The twin: the SAME load body at an empty dir must FAIL at M1, never return
/// early (bug-log `#66`; T-T4 E3-YD-P2).
#[test]
#[should_panic(expected = "absent means a broken checkout")]
fn jan_2024_fixture_absent_is_a_failure_not_a_skip() {
    let empty = tempfile::tempdir().expect("a temp dir to stand in for a broken checkout");
    jan_2024_fixture_load_body(empty.path().to_path_buf());
}

/// The Jan-2024 load body, parameterized by the fixture root.
fn jan_2024_fixture_load_body(fixture: PathBuf) {
    assert_yahoo_fixture(&fixture);
    let src = YahooBarSource::new(fixture);

    // Use a Custom range that exactly covers the fixture: 2024-01-01..2024-01-31.
    // This avoids triggering CacheMiss on months not in the fixture (Feb..Jun).
    // start = 2024-01-01T00:00:00Z = 1704067200000 ms
    // end   = 2024-02-01T00:00:00Z = 1706745600000 ms  (exclusive)
    let start_ms: i64 = 1_704_067_200_000;
    let end_ms: i64 = 1_706_745_600_000;

    let interval = Interval::Days1;

    let loaded = src
        .load_cached("BTC-USD", interval, start_ms, end_ms)
        .expect("load_cached with Jan-2024 fixture must succeed");

    assert!(
        !loaded.bars.is_empty(),
        "fixture must yield at least one bar; got 0"
    );
    assert_eq!(
        loaded.interval,
        Interval::Days1,
        "interval must be Days1 for a 31-day range"
    );
    assert!(
        !loaded.revision_sha.is_empty(),
        "revision_sha must be populated"
    );

    // Verify all bars carry Venue::Yahoo.
    for bar in &loaded.bars {
        assert_eq!(
            bar.venue,
            trading_core::Venue::Yahoo,
            "all loaded bars must have Venue::Yahoo"
        );
    }
}

/// T-C3.7 — revision SHA is deterministic across two `load_cached` calls
/// on the same fixture (H4 hypothesis — parquet SHA deterministic).
#[test]
fn yahoo_bar_source_revision_sha_is_deterministic() {
    revision_sha_determinism_body(fixture_root());
}

/// The twin: the SAME determinism body at an empty dir must FAIL at M1, never
/// return early (bug-log `#66`; T-T4 E3-YD-P3).
#[test]
#[should_panic(expected = "absent means a broken checkout")]
fn revision_fixture_absent_is_a_failure_not_a_skip() {
    let empty = tempfile::tempdir().expect("a temp dir to stand in for a broken checkout");
    revision_sha_determinism_body(empty.path().to_path_buf());
}

/// The revision-SHA determinism body, parameterized by the fixture root.
fn revision_sha_determinism_body(fixture: PathBuf) {
    assert_yahoo_fixture(&fixture);
    let start_ms: i64 = 1_704_067_200_000; // 2024-01-01
    let end_ms: i64 = 1_706_745_600_000; // 2024-02-01

    // Two separate `YahooBarSource` instances to exercise `OnceCell` independence.
    let src1 = YahooBarSource::new(fixture.clone());
    let src2 = YahooBarSource::new(fixture);

    let r1 = src1
        .load_cached("BTC-USD", Interval::Days1, start_ms, end_ms)
        .expect("first load must succeed");
    let r2 = src2
        .load_cached("BTC-USD", Interval::Days1, start_ms, end_ms)
        .expect("second load must succeed");

    assert_eq!(
        r1.revision_sha, r2.revision_sha,
        "revision_sha must be identical across two independent loads (H4)"
    );
    assert_eq!(
        r1.bars.len(),
        r2.bars.len(),
        "bar count must be identical across two independent loads"
    );
}
