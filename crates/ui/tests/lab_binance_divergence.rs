//! No-op-source divergence guard + loader gate for the Binance Lab path
//! (simple-strategies-realdata T-C1 / T-A3 — AC3 / AC4).
//!
//! ## Why this file exists (the v3-vol-overlay-noop analog)
//!
//! CLAUDE.md's baseline-equity-divergence gate is scoped to strategy overlays /
//! sizing modifiers — N/A here (this feature adds NO overlay, NO sizing change,
//! NO new decision variable; only the *bars* differ). BUT the precise failure
//! mode that gate exists to catch has a direct analog: a `BinanceCache` toggle
//! that is WIRED but silently feeds synthetic bars. The operator would believe
//! they are testing real BTC while seeing a random walk — the exact
//! "computed-but-not-applied" class the v3 precedent burned us on.
//!
//! **The guard (AC4):** run `v0.sma × BTCUSDT × 2023` on Binance bars and on
//! synthetic bars with the SAME `(strategy, symbol, range, seed)`, and assert
//! the two equity curves DIVERGE by ≥ epsilon — proving the real parquet bytes
//! drove the result, not a silent synthetic fallback. The loader's
//! no-silent-fallback rule (T-A3 — `Err` on miss, never synthesize) is the
//! design-side half; THIS test is the behavioural half.
//!
//! ## Gating
//!
//! `#[cfg(all(feature = "live", feature = "binance"))]` — needs the engine
//! (`live`) + the Binance loader (`binance`), both default features.
//!
//! The pinned corpus (`data/binance/`, revision `3a8b96c4…`) is COMMITTED IN
//! FULL: `.gitignore` re-includes `!/data/binance/**` ("TRACKED IN FULL since
//! 2026-09-27", operator decision), and `git ls-files data/binance` lists the
//! BTCUSDT 2023 and 2024 months this crate's tests read. An absent corpus is
//! therefore a broken checkout, and it FAILS (bug-log `#66`): no skip path
//! remains in this file (story 2-63 Wave E3, AC3). Until Wave E3 every
//! corpus-reading body returned early behind a `[skip]` print when the months
//! were missing.
//!
//! The loader's corpus root is cwd-relative (`BINANCE_CORPUS_ROOT` in
//! `crates/ui/src/lab/runner.rs`), and cargo runs ui test binaries with
//! cwd = `crates/ui/`, so the real tests pin the process cwd to the workspace
//! root ([`pin_cwd_to_workspace_root`]). The absent-corpus twins never change
//! the cwd: they pass an empty temp dir as the root and fail at the M1
//! precondition ([`assert_binance_corpus`]) before any load.
//!
//! ## Shape (story 2-63 Wave E2 / E3)
//!
//! Every corpus-reading test is a root-parameterized body (`loader_gate_body`,
//! `divergence_body`) whose first statement loads through
//! [`load_binance_bars`], which asserts M1 first. The real test calls the body
//! at the workspace root; a `#[should_panic(expected = "absent means a broken
//! checkout")]` twin calls the SAME body at an empty temp dir. A twin that
//! called only the helper would not pin the body (story 2-63 E2 review,
//! OWN-1 / E2-P14), so there is one twin per body.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! Each probe is ONE exact-once replace in THIS file (the OLD text occurs
//! exactly once; `grep -c`), run with
//! `cargo test -p ui --test lab_binance_divergence`, then restored from a
//! `cp -p` snapshot (`cmp` to confirm). The skip prefix NEW inserts is
//! `if !corpus_data_present(root) { return; }`, the shape Wave E3 removed.
//!
//! | # | symbol | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | E3-BD-P1 | `assert_binance_corpus` | `assert!(` followed by `corpus_data_present(root),` | the skip prefix above the assert | both twins. libtest reports "panic did not contain expected string" when the load then fails at the twin's cwd (`crates/ui`), or "test did not panic as expected" when a sibling test has already pinned the cwd to the workspace root and the load succeeds; which one depends on test scheduling, and both are red |
//! | E3-BD-P2 | `loader_gate_body` | `let (bars, sha) = load_binance_bars(root);` | the skip prefix above it | `loader_gate_absent_corpus_is_a_failure_not_a_skip`: "test did not panic as expected" |
//! | E3-BD-P3 | `divergence_body` | `let (binance_bars, _sha) = load_binance_bars(root);` | the skip prefix above it | `divergence_absent_corpus_is_a_failure_not_a_skip`: "test did not panic as expected" |
//!
//! GREEN in every probe: the three real tests (they run on the committed
//! corpus, where the skip prefix never fires).
//!
//! ## Coverage
//!
//! | test | subject | pinned by |
//! |---|---|---|
//! | `loader_returns_nonempty_hourly_bars_with_revision_sha` | production `DefaultLabBinanceBarSource` on 2023 H1 | non-empty, pinned SHA, window, symbol, first gap 1 h |
//! | `binance_run_diverges_from_synthetic_baseline` | engine on Binance bars vs synthetic | final-equity delta ≥ 1 USDT and series not identical |
//! | `loader_missing_corpus_returns_typed_err_not_synthetic` | loader on a missing symbol | typed Err naming `ZZZUSDT` (the corpus is committed, so only the missing-symbol path is accepted) |
//! | `loader_gate_absent_corpus_is_a_failure_not_a_skip` | `loader_gate_body` at an empty root | panics at M1 |
//! | `divergence_absent_corpus_is_a_failure_not_a_skip` | `divergence_body` at an empty root | panics at M1 |

#![cfg(all(feature = "live", feature = "binance"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use backtest::engine::{DateRange, ScenarioConfig, ScenarioDataSource, run_scenario};
use backtest::progress::ProgressSender;
use rust_decimal::Decimal;
use smol_str::SmolStr;
use trading_core::{StrategyId, Symbol, Venue};
use ui::lab::defaults::LAB_DEFAULT_SEED;
use ui::lab::runner::{DefaultLabBinanceBarSource, LabBarSource, LabRunConfig};

/// 2023 H1 as a `Custom` ms range: 2023-01-01 .. 2023-07-01 UTC. The pinned
/// corpus covers 2023-01..2024-12 at 1h, so this window resolves to on-disk
/// months for BTCUSDT.
const RANGE_2023_H1: DateRange = DateRange::Custom {
    start_ms: 1_672_531_200_000, // 2023-01-01T00:00:00Z
    end_ms: 1_688_169_600_000,   // 2023-07-01T00:00:00Z
};

/// Probe for the DATA this test loads — never for `REVISION.toml` (a probe on
/// the manifest alone was true while every month beneath it was absent, which
/// is what once turned the Linux and Windows UI legs red invisibly).
///
/// It backs the M1 assert ([`assert_binance_corpus`]), not a skip: the corpus
/// is committed (module doc § Gating).
///
/// The months named here are the ones `RANGE_2023_H1` actually loads. They are
/// NOT the same across these tests — this one reads 2023, its siblings read
/// another year — so this probe is deliberately per-file rather than a shared
/// helper that would be right for one caller and wrong for the next.
fn corpus_data_present(root: &std::path::Path) -> bool {
    (1..=6).all(|m| {
        root.join(format!("data/binance/BTCUSDT/2023/{m:02}.parquet"))
            .is_file()
    })
}

/// M1, the committed-corpus precondition (bug-log `#66`): PANIC when the six
/// 2023 H1 BTCUSDT parquets are missing under `root`.
fn assert_binance_corpus(root: &std::path::Path) {
    assert!(
        corpus_data_present(root),
        "the committed Binance corpus (data/binance/BTCUSDT/2023/01..06.parquet) must ship under {} — absent means a broken checkout, not a skip (bug-log #66)",
        root.display()
    );
}

/// Build the Lab run config the loader consumes (BTCUSDT, Binance source).
fn binance_cfg() -> LabRunConfig {
    LabRunConfig {
        strategy_id: SmolStr::new("v0.sma"),
        symbol: SmolStr::new("BTCUSDT"),
        venue: SmolStr::new("Binance"),
        // `range` is informational for the loader; the actual window comes from the
        // engine `DateRange` passed to preload (RANGE_2023_H1, the same ms). Written out
        // in full: this file imports `backtest::engine::DateRange` (bug-log #230).
        range: ui::lab::state::DateRange::Custom {
            start_raw: SmolStr::new("2023-01-01"),
            end_raw: SmolStr::new("2023-07-01"),
        },
        seed: LAB_DEFAULT_SEED,
        write_report: false,
        data_source: ui::lab::state::LabDataSource::BinanceCache,
        sma_fast_len: None,
        sma_slow_len: None,
    }
}

/// Build the engine `ScenarioConfig` for `v0.sma × BTCUSDT`, parameterized by
/// data source + optional pre-loaded bars. Same seed + range for both arms so
/// the ONLY difference is the bar source (the divergence isolation).
fn scenario_cfg(
    data_source: ScenarioDataSource,
    bars_override: Option<Vec<trading_core::Bar>>,
) -> ScenarioConfig {
    ScenarioConfig {
        strategy: StrategyId("v0.sma".into()),
        pair: (Venue::Binance, Symbol::new("BTCUSDT")),
        range: RANGE_2023_H1,
        params: None,
        seed: LAB_DEFAULT_SEED,
        write_report: false,
        data_source,
        bars_override,
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: None,
        short_enabled: false,
        initial_capital: None,
        composed_toml_override: None,
        dvol_override: None,
        macro_regime_series: None,
    }
}

/// Resolve the workspace root (`crates/ui` → `crates` → root) and pin the
/// process cwd there. The loader's corpus root `data/binance` is cwd-relative,
/// and cargo runs ui test binaries with cwd = `crates/ui/` — which made every
/// guard body in this file skip on EVERY machine (review patch 1, empirically
/// proven 2026-07-26 with the corpus present). Tests in one binary run
/// multi-threaded; per-test `set_current_dir` to the SAME directory is the
/// established benign pattern (`crates/backtest/tests/binance_cache_dispatch.rs`).
/// The absent-corpus twins never call it.
fn pin_cwd_to_workspace_root() -> std::path::PathBuf {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("locate workspace root from CARGO_MANIFEST_DIR")
        .to_path_buf();
    std::env::set_current_dir(&root).unwrap_or_else(|e| panic!("set_current_dir({root:?}): {e}"));
    root
}

/// Load real Binance bars via the PRODUCTION `DefaultLabBinanceBarSource`
/// (the trait seam), routed exactly as `spawn_lab_run` routes it.
///
/// M1 first: PANICS when the committed corpus is missing under `root` (a
/// broken checkout). The load itself reads the cwd the caller pinned; with the
/// corpus present, a loader error also PANICS — a regression, not a missing
/// corpus (review patch 1, bug-log `#66`).
fn load_binance_bars(root: &std::path::Path) -> (Vec<trading_core::Bar>, SmolStr) {
    assert_binance_corpus(root);
    let cfg = binance_cfg();
    let src = DefaultLabBinanceBarSource;
    // `LabBarSource::preload` is an async fn returning a boxed future; drive it
    // on a throwaway current-thread runtime (the loader is a pure parquet read,
    // no reactor-sensitive spawn_blocking — see preload_binance_bars docs).
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime builds");
    let result = rt.block_on(async { src.preload(&cfg, &RANGE_2023_H1).await });
    match result {
        Ok((bars, sha)) => (bars, sha),
        Err(e) => panic!(
            "the committed Binance corpus is present under {} but the loader failed: {e} — a regression, not a missing corpus (bug-log #66)",
            root.display()
        ),
    }
}

/// Final equity (USDT) of a `RunReport` — the headline number a divergence is
/// measured on. Empty series → 0 (the test asserts non-empty separately).
fn final_equity(report: &backtest::RunReport) -> Decimal {
    report
        .equity_series
        .last()
        .map_or(Decimal::ZERO, |(_, money)| money.amount())
}

/// The loader-gate body, parameterized by the corpus root (M1 runs at
/// `root`; the load reads the cwd the real test pinned).
fn loader_gate_body(root: &std::path::Path) {
    let (bars, sha) = load_binance_bars(root);

    assert!(
        !bars.is_empty(),
        "Binance loader must return non-empty bars for BTCUSDT × 2023 H1"
    );
    assert!(
        !sha.is_empty(),
        "loader must carry the pinned aggregate revision SHA (forensics)"
    );
    // Review patch 3 through the seam: the loader pin-asserts, so the carried
    // SHA IS the pin (mirrors the CLI literal in crates/backtest/src/main.rs).
    assert_eq!(
        sha.as_str(),
        "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7",
        "loader-carried SHA must equal the pinned corpus revision (AC3)"
    );
    // Every bar is in the requested window and on the Binance venue.
    for b in &bars {
        let ts_ms = b.open_ts.unix_millis();
        assert!(
            (1_672_531_200_000..1_688_169_600_000).contains(&ts_ms),
            "bar at {ts_ms} ms is outside the requested 2023-H1 window"
        );
        assert_eq!(b.symbol, Symbol::new("BTCUSDT"), "wrong symbol in bars");
    }
    // Hourly cadence sanity: the first two open timestamps differ by exactly
    // 1 h (3_600_000 ms). An exact bar count is NOT pinned: Binance has
    // maintenance gaps, and the count for this window is unverified here.
    assert!(
        bars.len() >= 2,
        "the 2023 H1 load must hold at least two hourly bars to check the cadence (got {})",
        bars.len()
    );
    let gap_ms = bars[1].open_ts.unix_millis() - bars[0].open_ts.unix_millis();
    assert_eq!(
        gap_ms, 3_600_000,
        "Binance Lab bars must be HOURLY (Q-tf): first gap was {gap_ms} ms"
    );
}

/// **AC3 — the loader gate (revision-asserted, non-empty hourly bars).**
///
/// The production Binance loader returns non-empty HOURLY bars for
/// `BTCUSDT × 2023 H1` AND carries the pinned aggregate revision SHA (which it
/// asserted on load). All `BTCUSDT` bars are in range and stamped on the
/// Binance venue.
#[test]
fn loader_returns_nonempty_hourly_bars_with_revision_sha() {
    loader_gate_body(&pin_cwd_to_workspace_root());
}

/// The twin of the loader gate: the SAME body at an empty root must FAIL at
/// M1, never return early (bug-log `#66`; T-T4 E3-BD-P2).
#[test]
#[should_panic(expected = "absent means a broken checkout")]
fn loader_gate_absent_corpus_is_a_failure_not_a_skip() {
    let empty = tempfile::tempdir().expect("a temp dir to stand in for a broken checkout");
    loader_gate_body(empty.path());
}

/// The divergence body, parameterized by the corpus root (M1 runs at `root`;
/// the load reads the cwd the real test pinned).
fn divergence_body(root: &std::path::Path) {
    let (binance_bars, _sha) = load_binance_bars(root);

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime builds");

    // Arm A — real Binance bars.
    let (cancel_a, recv_a) = ui::lab::runner::cancellation_pair();
    let report_binance = rt
        .block_on(run_scenario(
            scenario_cfg(ScenarioDataSource::BinanceCache, Some(binance_bars.clone())),
            recv_a,
            ProgressSender::disabled(),
        ))
        .expect("Binance-sourced v0.sma run succeeds");
    drop(cancel_a);

    // Arm B — synthetic baseline, same seed + range.
    let (cancel_b, recv_b) = ui::lab::runner::cancellation_pair();
    let report_synth = rt
        .block_on(run_scenario(
            scenario_cfg(ScenarioDataSource::Synthetic, None),
            recv_b,
            ProgressSender::disabled(),
        ))
        .expect("synthetic v0.sma baseline run succeeds");
    drop(cancel_b);

    // Both runs produced equity.
    assert!(
        !report_binance.equity_series.is_empty(),
        "Binance run must produce a non-empty equity series"
    );
    assert!(
        !report_synth.equity_series.is_empty(),
        "synthetic baseline must produce a non-empty equity series"
    );

    // (The report's "binance" data_source label is compile-enforced in the
    // engine's data_source_str match and verified at the body layer by the
    // persist/Compare round-trip test `lab_binance_persist_compare`.)

    // ── THE divergence assertion (AC4) ────────────────────────────────────────
    // The Binance arm runs on ~hourly 2023-H1 BTC bars; the synthetic arm on a
    // fixed-seed GBM series. Two independent signals must differ:
    //   (1) the equity SERIES are not element-by-element identical, and
    //   (2) the final equity differs by a non-trivial epsilon.
    // A silent synthetic fallback in the Binance arm would make BOTH hold with
    // equality → this test fails (the no-op-source trap is caught).

    let eq_binance = final_equity(&report_binance);
    let eq_synth = final_equity(&report_synth);
    let delta = (eq_binance - eq_synth).abs();

    // Epsilon: ≥ 1 USDT on a ~10_000 USDT book is ~1 bp — above
    // rounding-noise scale, and trivially satisfied by two genuinely
    // different bar sources. (Real BTC 2023-H1 vs a GBM random walk diverge by
    // orders of magnitude more; 1 USDT is a deliberately conservative floor.)
    let epsilon = Decimal::ONE;
    assert!(
        delta >= epsilon,
        "NO-OP-SOURCE GUARD FAILED: Binance final equity ({eq_binance}) and \
         synthetic final equity ({eq_synth}) differ by only {delta} (< {epsilon}). \
         The Binance toggle may be silently feeding synthetic bars — the operator \
         would see a random walk while believing they test real BTC (the \
         v3-vol-overlay-noop failure class)."
    );

    // Series-level proof: the two equity curves are not identical sequences.
    let identical = report_binance.equity_series.len() == report_synth.equity_series.len()
        && report_binance
            .equity_series
            .iter()
            .zip(report_synth.equity_series.iter())
            .all(|((_, a), (_, b))| a.amount() == b.amount());
    assert!(
        !identical,
        "NO-OP-SOURCE GUARD FAILED: the Binance and synthetic equity SERIES are \
         element-by-element identical — the real parquet bytes did not reach the \
         strategy (silent synthetic fallback)."
    );
}

/// **AC4 — THE no-op-source divergence guard (the purpose-built gate).**
///
/// Run `v0.sma × BTCUSDT × 2023 H1` twice with the SAME `(strategy, symbol,
/// range, seed)`:
///   - arm A: real Binance bars injected via `bars_override` + `BinanceCache`,
///   - arm B: synthetic GBM bars (`bars_override = None` + `Synthetic`).
///
/// Assert the two equity curves DIVERGE — proving the real parquet bytes
/// reached the strategy, not a silent synthetic fallback. If the Binance path
/// ever silently synthesized, the two arms would be byte-identical and this
/// test would FAIL (exactly the v3-vol-overlay-noop failure signature).
#[test]
fn binance_run_diverges_from_synthetic_baseline() {
    divergence_body(&pin_cwd_to_workspace_root());
}

/// The twin of the divergence guard: the SAME body at an empty root must FAIL
/// at M1, never return early (bug-log `#66`; T-T4 E3-BD-P3).
#[test]
#[should_panic(expected = "absent means a broken checkout")]
fn divergence_absent_corpus_is_a_failure_not_a_skip() {
    let empty = tempfile::tempdir().expect("a temp dir to stand in for a broken checkout");
    divergence_body(empty.path());
}

/// **AC4 design-side half — the loader NEVER synthesizes on miss.**
///
/// Point the SAME loader logic at a symbol that does not exist in the corpus
/// (`ZZZUSDT`) and assert it returns a typed `Err` naming that symbol — NOT a
/// synthetic-bars `Ok`. This is the no-silent-fallback contract that makes the
/// divergence guard above trustworthy: a miss is loud, never a random walk.
#[test]
fn loader_missing_corpus_returns_typed_err_not_synthetic() {
    // cwd-pin so the loader resolves the same workspace-root corpus the other
    // tests use. The corpus is committed (M1 below), so this exercises the
    // missing-SYMBOL path only; a revision error here would be a regression.
    let root = pin_cwd_to_workspace_root();
    assert_binance_corpus(&root);
    // A symbol guaranteed absent from the 10-symbol pinned corpus.
    let cfg = LabRunConfig {
        symbol: SmolStr::new("ZZZUSDT"),
        ..binance_cfg()
    };
    let src = DefaultLabBinanceBarSource;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime builds");
    let result = rt.block_on(async { src.preload(&cfg, &RANGE_2023_H1).await });

    match result {
        Ok((bars, _sha)) => {
            panic!(
                "NO-SILENT-FALLBACK VIOLATED: loader returned Ok with {} bars for a \
                 missing symbol (ZZZUSDT) — it must return a typed cache-miss Err, \
                 NEVER synthesize bars.",
                bars.len()
            );
        }
        Err(e) => {
            // The message must be the operator-friendly cache-miss notice and
            // must name the symbol — never an internal panic string.
            let msg = e.as_str();
            assert!(
                msg.contains("ZZZUSDT"),
                "cache-miss Err must name the missing symbol (the corpus is committed, so a revision error is a regression); got: {msg}"
            );
            assert!(
                !msg.is_empty(),
                "cache-miss Err must carry an operator-facing message"
            );
        }
    }
}
