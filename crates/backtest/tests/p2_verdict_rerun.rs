//! P2 — multi-corpus ship-passive verdict re-run harness (ADR-0084 D4).
//!
//! ## The point
//!
//! The product's terminal thesis — "no active strategy robustly beats
//! buy-and-hold net of costs" — currently rests on a narrow evidence base:
//! one venue (Binance), one bar size (hourly), ~2 regimes. This harness
//! re-runs the SAME bake-off + rank + P0-1 overfitting scorecard pipeline
//! over an EXTENDED corpus set (5 Binance-hourly regimes + 1 Coinbase-BTC
//! venue cross-check) to test whether the verdict holds, wobbles, or breaks.
//! Both outcomes are product value — the gate decides, not the author.
//!
//! ## Why a dedicated harness, not a `--corpus` selector (ADR-0084 D4)
//!
//! `run_bakeoff` resolves real bars through `resolve_bakeoff_bars` →
//! `preload_bakeoff_binance_bars`, which **hardcodes**
//! `BINANCE_CORPUS_ROOT = "data/binance"` (`bakeoff/mod.rs:101`, a `const`;
//! the fn is `pub(crate)`). Pointing the shipped runner at
//! `data/binance-1718` would require adding a corpus-root parameter to the
//! public `BakeoffConfig`/`BakeoffRequest` — a larger, riskier change than
//! the honest multi-corpus loop warrants.
//!
//! The clean seam already exists and is proven twice:
//! `realdata_simple_strategy_bear_survey.rs:168` loads an arbitrary corpus
//! via `ReplayFeed::new(root.join("data/binance-2122"), true)
//! .subscribe_bars(...)` → `Vec<Bar>`; `null_data_no_crown.rs::
//! run_field_and_rank` reproduces `run_bakeoff`'s EXACT per-arm sequence
//! (`run_scenario` with `bars_override` → `derive_candidate_kpis` →
//! `derive_master_seed` + `compute_robustness_flag` → `rank_candidates` →
//! `compute_scorecard`) against caller-supplied bars. This harness
//! **composes these two proven pieces**: for each `(corpus_root, symbol,
//! supported_arm_field)`, load bars via `ReplayFeed::merge_symbols` and run
//! the null-CI's `run_field_and_rank` shape verbatim (generalized here to
//! thread the DVOL/macro overrides + an optional slippage-model override
//! for the S7/S8 era-cost annex). Every function called is the identical
//! production function `run_bakeoff` calls; only the bar *source* (and, for
//! the annex, the slippage model) differs.
//!
//! ## The R4 honest arm-availability matrix (feature.md § R4 / ADR-0084 D3)
//!
//! | Arm class | 1718 | 2020 | 2122 | 2324 (base) | 2526 | coinbase |
//! |-----------|:----:|:----:|:----:|:-----------:|:----:|:--------:|
//! | Price-only singles + ensembles + short/`_ls` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (BTC) |
//! | `v0.dvol_regime` (needs DVOL BTC/ETH ≥2021-04) | ❌ | ❌ | ✅ (back-filled) | ✅ (on disk) | ✅ (back-filled) | ❌ |
//! | `v0.macro_riskon` (needs yahoo-macro ≥2021) | ❌ | ❌ | ✅ (on disk) | ✅ (on disk) | ✅ (back-filled) | ❌ |
//! | Perp-basis / funding MN arms | ❌ | ❌ | ❌ | (N/A — separate sweep path, not in `default_field()`) | ❌ | ❌ |
//!
//! An absent arm is simply **not added to that corpus's field** — this
//! harness never runs a warm-up-only proxy arm and calls it "evaluated".
//! (Perp-basis/funding arms never appear in `default_field()` /
//! `default_ensemble_field()` / `default_macro_field()` at all — they live in
//! the separate `bakeoff::sweep` robustness-sweep path — so no exclusion
//! logic is needed for them here; R4's "❌ except 2324" note about them
//! describes a DIFFERENT harness family entirely.)
//!
//! ## SKIP-safe (D4 consequence)
//!
//! Each corpus's test fn returns early (`eprintln!` SKIP) when the
//! gitignored parquets are absent — mirrors the null-CI + `bear_survey`
//! SKIP guards, so CI without the (large, multi-hour-fetch) corpora stays
//! green. The `data/binance` smoke test (S4, the existing PINNED corpus)
//! runs **un-ignored** so the harness is exercised in CI-less reality even
//! before the new P2 corpora land; S1/S2/S3/S5/S6 (the new corpora, which
//! land via long-running background fetches per ADR-0084) are `#[ignore]`d
//! like the bear-survey / t14 precedent — run explicitly once fetched.
//!
//! ## Anchor safety (ADR-0084 D8 — non-negotiable, by construction)
//!
//! Every `scenario_cfg_for` call sets `write_report: false` — no anchored
//! CLI report body is EVER produced by this file. `verify_anchors.sh` stays
//! 119/119 unaffected; this file adds zero anchors and mutates none.
//!
//! ## Determinism
//!
//! Each corpus's field uses a FIXED seed base (`seed_bytes_from_u64`,
//! `ChaCha20Rng` under the hood via the existing bootstrap machinery) —
//! recorded per corpus below. No `thread_rng`, no `OsRng`, no wall-clock.
//!
//! ## FROZEN-gate contract
//!
//! This file only READS `rank_candidates` / `compute_robustness_flag` /
//! `classify_verdict` (via `RecommendationOutcome`) / `compute_scorecard`
//! via the existing public re-exports. It never modifies them.

#![allow(clippy::too_many_lines, clippy::too_many_arguments)]

use std::path::{Path, PathBuf};

use backtest::{
    ScenarioConfig,
    bakeoff::bootstrap::{compute_robustness_flag, derive_master_seed},
    bakeoff::scorecard::{Scorecard, compute_scorecard},
    bakeoff::{BakeoffConfig, CandidateResult, derive_candidate_kpis},
    bakeoff::{dvol_supported, resolve_dvol_override},
    cancel::cancellation_pair,
    cli_types::LatencySlippageSimConfig,
    engine::{DateRange, ScenarioDataSource},
    macro_regime::load_macro_regime_series,
    progress::ProgressSender,
    rank_candidates, run_scenario,
};
use cost::SlippageModel;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{StrategyId, Symbol, Timeframe, Venue};

// ─────────────────────────────────────────────────────────────────────────────
// Workspace-root + corpus paths
// ─────────────────────────────────────────────────────────────────────────────

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

const BUYHOLD_ID: &str = "v0.buyhold";

/// Bootstrap path count for this harness. Kept low (like `null_data_no_crown.rs`'s
/// `BOOTSTRAP_PATHS = 150`) purely for wall-clock — the qualitative gate
/// classification is path-count-invariant.
const BOOTSTRAP_PATHS: usize = 150;

// ─────────────────────────────────────────────────────────────────────────────
// Bar loading — the arbitrary-corpus seam (mirrors
// `realdata_simple_strategy_bear_survey.rs::load_year_bars`, generalized to
// an explicit corpus root + explicit symbol set + revision-verify like
// `preload_bakeoff_binance_bars`, minus the `data/binance`-only hardcode).
// ─────────────────────────────────────────────────────────────────────────────

/// Load hourly bars for one symbol from an arbitrary pinned corpus root.
///
/// Returns `None` when the corpus (or this symbol within it) is absent on
/// disk — the SKIP-safe contract. Verifies `REVISION.toml` when present
/// (loud error on tamper — same discipline `preload_bakeoff_binance_bars`
/// applies to `data/binance`); a MISSING manifest is itself a SKIP signal
/// (the corpus simply hasn't been fetched yet on this machine), not a hard
/// error, so CI-less environments stay green.
async fn load_corpus_symbol_bars(
    corpus_root: &Path,
    symbol: &Symbol,
) -> Option<Vec<trading_core::Bar>> {
    let sym_dir = corpus_root.join(symbol.0.as_str());
    if !sym_dir.exists() {
        return None;
    }

    // Revision-verify when a manifest is present (tamper-detection parity
    // with the production `data/binance` path); absence of REVISION.toml on
    // an otherwise-populated corpus dir is tolerated (some fetches may not
    // have run --emit-revision-manifest yet) rather than a hard SKIP.
    let manifest_path = corpus_root.join("REVISION.toml");
    if manifest_path.exists()
        && let Err(e) = data::revision::read_and_verify_revision_manifest(corpus_root)
    {
        panic!(
            "p2_verdict_rerun: REVISION.toml verification failed for {}: {e} \
             (tamper or corruption — refusing to run on unverified data)",
            corpus_root.display()
        );
    }

    let feed = data::ReplayFeed::new(corpus_root, true);
    let symbol_paths = [(symbol.clone(), corpus_root.to_path_buf())];
    match feed.merge_symbols(&symbol_paths, Timeframe::OneHour) {
        Ok(bars) if !bars.is_empty() => Some(bars),
        Ok(_) | Err(_) => None,
    }
}

/// `(start_ms, end_ms)` spanning the WHOLE bar vector (inclusive of the last
/// bar's open — the harness's `DateRange::Custom` window derivation, since
/// `resolve_dvol_override` / `load_macro_regime_series` both key their
/// exogenous-series load window off `range`, not off the bar vector itself).
fn bar_span_ms(bars: &[trading_core::Bar]) -> (i64, i64) {
    let start_ms = bars.first().map_or(0, |b| b.open_ts.unix_millis());
    let end_ms = bars
        .last()
        .map_or(start_ms, |b| b.open_ts.unix_millis() + 1);
    (start_ms, end_ms)
}

// ─────────────────────────────────────────────────────────────────────────────
// Field construction — the R4 matrix, encoded as data
// ─────────────────────────────────────────────────────────────────────────────

/// Which optional exogenous arms a corpus supports (R4 matrix). Price-only
/// singles + ensembles + short/`_ls` are ALWAYS included (every corpus is
/// OHLCV-complete by construction); this struct only toggles the two
/// exogenous-data arms.
#[derive(Debug, Clone, Copy)]
struct ArmSupport {
    dvol: bool,
    macro_riskon: bool,
}

/// Build the strategy-id field for one corpus per its `ArmSupport`.
///
/// Base: `default_field()` (10 arms, incl. `v0.dvol_regime` — filtered later
/// per-symbol/per-corpus) + `default_ensemble_field()` (8 vote-ensembles) +
/// `default_short_field()` (5 short/`_ls` arms). Then conditionally ADD
/// `default_macro_field()` (`v0.macro_riskon`) when `support.macro_riskon`.
/// `v0.dvol_regime` is part of `default_field()` already — when
/// `!support.dvol` it is explicitly REMOVED from the field (never run
/// warm-up-only and reported as "evaluated").
fn build_field(support: ArmSupport) -> Vec<StrategyId> {
    let mut field = BakeoffConfig::default_field();
    field.extend(BakeoffConfig::default_ensemble_field());
    field.extend(BakeoffConfig::default_short_field());
    if support.macro_riskon {
        field.extend(BakeoffConfig::default_macro_field());
    }
    if !support.dvol {
        field.retain(|id| id.0.as_str() != "v0.dvol_regime");
    }
    field
}

// ─────────────────────────────────────────────────────────────────────────────
// Bake-off + rank + scorecard harness — reproduces `run_bakeoff`'s exact
// per-arm sequence (mirrors `null_data_no_crown.rs::run_field_and_rank`,
// generalized for the DVOL/macro overrides + optional slippage override)
// ─────────────────────────────────────────────────────────────────────────────

/// Combined result of one field run: the FROZEN gate's `Ranking` plus the
/// P0-1 overfitting `Scorecard`, computed exactly as `run_bakeoff` computes
/// it — crown's Sharpe vector across ALL candidates + the crowned
/// candidate's equity curve + bar count.
struct FieldOutcome {
    ranking: backtest::Ranking,
    scorecard: Scorecard,
    candidates: Vec<CandidateResult>,
}

fn seed_bytes_from_u64(seed: u64) -> [u8; 32] {
    let mut s = [0u8; 32];
    s[0..8].copy_from_slice(&seed.to_le_bytes());
    s
}

/// Build the `ScenarioConfig` for one arm against a caller-supplied bar
/// series. Mirrors the field values `run_bakeoff` sets in its per-arm loop
/// (`bakeoff/mod.rs:1070-1105`), generalized to accept the resolved
/// `dvol_override` / `macro_regime_series` for THIS arm + an optional
/// `slippage_model` override (S7/S8 era-cost annex; `None` → the frozen
/// `LatencySlippageSimConfig::default()` noop, matching `run_bakeoff`'s own
/// per-arm config exactly — the frozen-default primary matrix).
fn scenario_cfg_for(
    strategy_id: &str,
    bars: Vec<trading_core::Bar>,
    symbol: &Symbol,
    seed: [u8; 32],
    start_ms: i64,
    end_ms: i64,
    short_enabled: bool,
    dvol_override: Option<Vec<Option<Decimal>>>,
    macro_regime_series: Option<trading_core::pit::PitSeries<bool>>,
    slippage_model: Option<SlippageModel>,
) -> ScenarioConfig {
    let latency_slippage_sim = match slippage_model {
        Some(model) => LatencySlippageSimConfig {
            slippage_model: model,
            ..LatencySlippageSimConfig::default()
        },
        None => LatencySlippageSimConfig::default(),
    };
    ScenarioConfig {
        strategy: StrategyId(smol_str::SmolStr::new(strategy_id)),
        pair: (Venue::Binance, symbol.clone()), // Venue is metadata only — bars come from bars_override; matches run_bakeoff's own per-arm hardcode of Venue::Binance regardless of actual data source (bakeoff/mod.rs:1072)
        range: DateRange::Custom { start_ms, end_ms },
        seed,
        write_report: false, // anchor-safe: no report body ever written (ADR-0084 D8)
        data_source: ScenarioDataSource::BinanceCache,
        bars_override: Some(bars),
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim,
        reports_dir: None,
        params: None,
        short_enabled,
        initial_capital: Some(dec!(100_000)),
        composed_toml_override: None,
        dvol_override,
        macro_regime_series,
    }
}

/// Run the full field (+ benchmark) on one corpus's bars and return the
/// FROZEN gate's `Ranking` AND the P0-1 `Scorecard`. Reproduces
/// `run_bakeoff`'s exact sequence: `run_scenario` → `derive_candidate_kpis`
/// → `derive_master_seed` + `compute_robustness_flag` → `rank_candidates` →
/// `compute_scorecard`.
///
/// `symbol` drives BOTH the `pair` metadata AND the ADR-0072 D8 per-symbol
/// `v0.dvol_regime` filter (`dvol_supported = {BTCUSDT, ETHUSDT}`) — mirroring
/// `run_bakeoff`'s exact `continue`-before-dispatch behaviour
/// (`bakeoff/mod.rs:1030-1039`): for a non-BTC/ETH symbol the arm is REMOVED
/// from the field entirely (never dispatched, never counted as an evaluated
/// candidate), not run-and-degraded via a `None` override.
///
/// `dvol_bar_ts` / `macro_range` are threaded so the DVOL/macro arms (when
/// present in `field`) resolve their real exogenous series exactly as
/// `run_bakeoff` does — via the SAME public `resolve_dvol_override` /
/// `load_macro_regime_series` fns, pointed at the (possibly back-filled)
/// `data/deribit-dvol` / `data/yahoo-macro` roots.
async fn run_field_and_rank(
    bars: &[trading_core::Bar],
    field: &[StrategyId],
    symbol: &Symbol,
    seed_u64: u64,
    slippage_model: Option<SlippageModel>,
) -> FieldOutcome {
    let seed_bytes = seed_bytes_from_u64(seed_u64);
    let (start_ms, end_ms) = bar_span_ms(bars);
    let range = DateRange::Custom { start_ms, end_ms };
    let symbol_str = symbol.0.as_str();
    // Review 3-15 MEDIUM: this used to be a third independent copy of the
    // BTC/ETH allowlist (`matches!(symbol_str, "BTCUSDT" | "ETHUSDT")`). It now
    // calls the same `dvol_supported()` predicate production calls, so the
    // harness cannot silently disagree with the bake-off about which coins the
    // arm is even defined for.
    let dvol_sym_ok = dvol_supported(symbol_str);

    let mut strategy_ids: Vec<(String, bool)> = field
        .iter()
        // ADR-0072 D8 parity: v0.dvol_regime is ABSENT (not degraded) for
        // non-BTC/ETH symbols — filtered out of the field BEFORE dispatch,
        // exactly like production's `continue`.
        .filter(|s| s.0.as_str() != "v0.dvol_regime" || dvol_sym_ok)
        .map(|s| (s.0.to_string(), false))
        .collect();
    strategy_ids.push((BUYHOLD_ID.to_string(), true));

    let bar_ts: Vec<i64> = bars.iter().map(|b| b.open_ts.unix_millis()).collect();

    let mut candidates: Vec<CandidateResult> = Vec::with_capacity(strategy_ids.len());

    for (idx, (strategy_id, is_benchmark)) in strategy_ids.iter().enumerate() {
        let short_enabled = BakeoffConfig::is_short_enabled(strategy_id);

        let is_dvol_arm = strategy_id == "v0.dvol_regime";
        let dvol_override = if is_dvol_arm {
            resolve_dvol_override(symbol_str, &range, &bar_ts, strategy_id)
        } else {
            None
        };
        // bug-log #78 parity with production: an unresolvable DVOL series means
        // the arm is ABSENT, never a 100%-cash stub carrying the probe's label.
        // This harness's whole premise is "never run a warm-up-only proxy arm and
        // call it evaluated" (see the R4 matrix in the module docs) — that promise
        // was previously enforced only for the SYMBOL, not for a failed load.
        if is_dvol_arm && dvol_override.is_none() {
            eprintln!(
                "  [p2_verdict_rerun] v0.dvol_regime DROPPED — the DVOL series could not \
                 be resolved for this corpus/window (the arm is reported as ABSENT, \
                 never as an evaluated candidate)"
            );
            continue;
        }

        let is_macro_arm = strategy_id == "v0.macro_riskon";
        let macro_regime_series = if is_macro_arm {
            let yahoo_macro_root = workspace_root().join("data/yahoo-macro");
            match load_macro_regime_series(&yahoo_macro_root, &range) {
                Ok(series) => Some(series),
                Err(e) => {
                    eprintln!(
                        "  [p2_verdict_rerun] v0.macro_riskon: macro load failed ({e}) — \
                         arm runs warm-up-only for this corpus"
                    );
                    None
                }
            }
        } else {
            None
        };

        let cfg = scenario_cfg_for(
            strategy_id,
            bars.to_vec(),
            symbol,
            seed_bytes,
            start_ms,
            end_ms,
            short_enabled,
            dvol_override,
            macro_regime_series,
            slippage_model,
        );
        let (_handle, cancel_rx) = cancellation_pair();
        let progress_tx = ProgressSender::disabled();

        let report = run_scenario(cfg, cancel_rx, progress_tx)
            .await
            .unwrap_or_else(|e| panic!("run_scenario('{strategy_id}') must succeed: {e}"));

        let kpis = derive_candidate_kpis(&report);

        let equity_decimals: Vec<Decimal> = report
            .equity_series
            .iter()
            .map(|(_, m)| m.amount())
            .collect();
        let master_seed = derive_master_seed(seed_u64, idx);
        let robustness = Some(compute_robustness_flag(
            &equity_decimals,
            BOOTSTRAP_PATHS,
            master_seed,
        ));

        candidates.push(CandidateResult {
            strategy: StrategyId(smol_str::SmolStr::new(strategy_id.as_str())),
            is_benchmark: *is_benchmark,
            kpis,
            equity_curve: report.equity_series,
            robustness,
        });
    }

    let ranking = rank_candidates(&candidates);

    // ── P0-1 scorecard — mirrors `bakeoff/mod.rs:1172-1180` exactly ─────────
    let all_sharpes: Vec<f64> = candidates.iter().map(|c| c.kpis.sharpe).collect();
    let crowned_idx = ranking.crowned.unwrap_or(0);
    let crown_equity_decimals: Vec<Decimal> = candidates[crowned_idx]
        .equity_curve
        .iter()
        .map(|(_, m)| m.amount())
        .collect();
    let t_bars = crown_equity_decimals.len().saturating_sub(1).max(1);
    let scorecard = compute_scorecard(&all_sharpes, &crown_equity_decimals, t_bars);

    FieldOutcome {
        ranking,
        scorecard,
        candidates,
    }
}

fn crowned_id(outcome: &FieldOutcome) -> &str {
    let idx = outcome
        .ranking
        .crowned
        .expect("field is non-empty; crowned must be Some");
    outcome.candidates[idx].strategy.0.as_str()
}

/// Print an AC1-shaped summary line per candidate for `--nocapture` operator
/// visibility (the tester assembles the full AC1-AC8 report from this
/// stdout + a second pass reading `FieldOutcome` fields directly).
fn print_corpus_summary(corpus_label: &str, outcome: &FieldOutcome) {
    println!("## {corpus_label}");
    println!(
        "outcome={:?} crowned={}",
        outcome.ranking.outcome,
        crowned_id(outcome)
    );
    println!(
        "scorecard: n_candidates={} n_eff={:.2} deflated_sharpe={:.4} min_btl_years={:.2} crown_clears_dsr={}",
        outcome.scorecard.n_candidates,
        outcome.scorecard.n_eff,
        outcome.scorecard.deflated_sharpe,
        outcome.scorecard.min_btl_years,
        outcome.scorecard.crown_clears_dsr,
    );
    for c in &outcome.candidates {
        println!(
            "  {} (benchmark={}) sharpe={:.4} total_return_pct={} robustness={:?}",
            c.strategy.0, c.is_benchmark, c.kpis.sharpe, c.kpis.total_return_pct, c.robustness
        );
    }
    println!();
}

// ─────────────────────────────────────────────────────────────────────────────
// Seed bases per corpus (fixed, non-zero, arbitrary distinct constants —
// recorded here per corpus for the re-run report's determinism section)
// ─────────────────────────────────────────────────────────────────────────────

const SEED_1718: u64 = 0x1718_0000_C0FF_EE01;
const SEED_2020: u64 = 0x2020_0000_C0FF_EE01;
const SEED_2122: u64 = 0x2122_0000_C0FF_EE01;
const SEED_2324: u64 = 0x2324_0000_C0FF_EE01;
const SEED_2526: u64 = 0x2526_0000_C0FF_EE01;
const SEED_COINBASE: u64 = 0xC01B_0000_C0FF_EE01;

// ─────────────────────────────────────────────────────────────────────────────
// S1 — data/binance-1718 (2017 mania blow-off + 2018 bear, BTC/ETH/BNB)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires data/binance-1718 on disk — run after the long-running P2 fetch"]
async fn s1_binance_1718_btc_eth_bnb() {
    let root = workspace_root().join("data/binance-1718");
    let symbols = ["BTCUSDT", "ETHUSDT", "BNBUSDT"];
    let support = ArmSupport {
        dvol: false,
        macro_riskon: false,
    }; // R4: 1718 predates both DVOL (2021-04+) and macro (2021+)
    let field = build_field(support);

    let mut any_ran = false;
    for sym_s in symbols {
        let sym = Symbol::new(sym_s);
        let Some(bars) = load_corpus_symbol_bars(&root, &sym).await else {
            eprintln!(
                "SKIP s1_binance_1718: {sym_s} absent under {}",
                root.display()
            );
            continue;
        };
        any_ran = true;
        let outcome = run_field_and_rank(&bars, &field, &sym, SEED_1718, None).await;
        print_corpus_summary(&format!("S1 data/binance-1718 · {sym_s}"), &outcome);
    }
    if !any_ran {
        eprintln!("SKIP s1_binance_1718: corpus entirely absent — nothing to run");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// S2 — data/binance-2020 (COVID crash + recovery, 7 pre-2020 listers)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires data/binance-2020 on disk — run after the long-running P2 fetch"]
async fn s2_binance_2020_seven_listers() {
    let root = workspace_root().join("data/binance-2020");
    let symbols = [
        "BTCUSDT", "ETHUSDT", "BNBUSDT", "XRPUSDT", "ADAUSDT", "LINKUSDT", "DOGEUSDT",
    ];
    let support = ArmSupport {
        dvol: false,
        macro_riskon: false,
    }; // R4: 2020 predates both DVOL and macro
    let field = build_field(support);

    let mut any_ran = false;
    for sym_s in symbols {
        let sym = Symbol::new(sym_s);
        let Some(bars) = load_corpus_symbol_bars(&root, &sym).await else {
            eprintln!(
                "SKIP s2_binance_2020: {sym_s} absent under {}",
                root.display()
            );
            continue;
        };
        any_ran = true;
        let outcome = run_field_and_rank(&bars, &field, &sym, SEED_2020, None).await;
        print_corpus_summary(&format!("S2 data/binance-2020 · {sym_s}"), &outcome);
    }
    if !any_ran {
        eprintln!("SKIP s2_binance_2020: corpus entirely absent — nothing to run");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// S3 — data/binance-2122 (bear regime, all 10 — DVOL+macro back-filled)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires data/binance-2122 on disk — run after the long-running P2 fetch"]
async fn s3_binance_2122_all_ten() {
    let root = workspace_root().join("data/binance-2122");
    let symbols = [
        "BTCUSDT", "ETHUSDT", "BNBUSDT", "XRPUSDT", "ADAUSDT", "LINKUSDT", "DOGEUSDT", "DOTUSDT",
        "SOLUSDT", "AVAXUSDT",
    ];
    // R4: DVOL supported for BTC/ETH only (per-symbol filter in build_field's
    // downstream resolve_dvol_override is symbol-gated already); macro is
    // symbol-independent (a market-wide regime flag) so it applies to all 10.
    let support = ArmSupport {
        dvol: true,
        macro_riskon: true,
    };
    let field = build_field(support);

    let mut any_ran = false;
    for sym_s in symbols {
        let sym = Symbol::new(sym_s);
        let Some(bars) = load_corpus_symbol_bars(&root, &sym).await else {
            eprintln!(
                "SKIP s3_binance_2122: {sym_s} absent under {}",
                root.display()
            );
            continue;
        };
        any_ran = true;
        let outcome = run_field_and_rank(&bars, &field, &sym, SEED_2122, None).await;
        print_corpus_summary(&format!("S3 data/binance-2122 · {sym_s}"), &outcome);
    }
    if !any_ran {
        eprintln!("SKIP s3_binance_2122: corpus entirely absent — nothing to run");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// S4 — data/binance (the 2023-24 EXISTING PINNED base corpus) — SMOKE,
// UN-IGNORED so the harness is exercised in CI-less reality today.
// ─────────────────────────────────────────────────────────────────────────────

/// Un-ignored smoke of the harness against the ALREADY-PINNED `data/binance`
/// corpus (byte-immutable per ADR-0084 D8 — this test only READS it, never
/// writes). Proves the harness composes correctly (bar load → field build →
/// `run_field_and_rank` → gate + scorecard) even on machines without the new
/// P2 corpora, and doubles as the S4 reference/baseline row in the
/// per-corpus verdict table. SKIP-guards on absence exactly like the other
/// scenarios, so a fresh checkout without the gitignored parquets stays green.
#[tokio::test]
async fn s4_binance_2324_base_smoke() {
    let root = workspace_root().join("data/binance");
    let sym = Symbol::new("BTCUSDT");
    let Some(bars) = load_corpus_symbol_bars(&root, &sym).await else {
        eprintln!(
            "SKIP s4_binance_2324_base_smoke: data/binance/BTCUSDT absent under {} \
             (gitignored parquets not fetched on this machine)",
            root.display()
        );
        return;
    };

    let support = ArmSupport {
        dvol: true,
        macro_riskon: true,
    }; // R4: 2324 base has DVOL + macro on disk already
    let field = build_field(support);

    let outcome = run_field_and_rank(&bars, &field, &sym, SEED_2324, None).await;
    print_corpus_summary("S4 data/binance (2324 base) · BTCUSDT — SMOKE", &outcome);

    // Structural assertions (the harness itself must be sound, independent
    // of which arm happens to crown on this particular corpus/seed):
    assert!(
        outcome.ranking.crowned.is_some(),
        "a non-empty field must always produce a crowned candidate"
    );
    assert!(
        !outcome.candidates.is_empty(),
        "candidates must be non-empty (field + buy-and-hold benchmark)"
    );
    assert!(
        outcome.candidates.iter().any(|c| c.is_benchmark),
        "the buy-and-hold benchmark arm must always be present"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// S5 — data/binance-2526 (recent regime, all 10 — DVOL+macro back-filled)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires data/binance-2526 on disk — run after the long-running P2 fetch"]
async fn s5_binance_2526_all_ten() {
    let root = workspace_root().join("data/binance-2526");
    let symbols = [
        "BTCUSDT", "ETHUSDT", "BNBUSDT", "XRPUSDT", "ADAUSDT", "LINKUSDT", "DOGEUSDT", "DOTUSDT",
        "SOLUSDT", "AVAXUSDT",
    ];
    let support = ArmSupport {
        dvol: true,
        macro_riskon: true,
    }; // R4: DVOL + macro both back-filled for 2025-26
    let field = build_field(support);

    let mut any_ran = false;
    for sym_s in symbols {
        let sym = Symbol::new(sym_s);
        let Some(bars) = load_corpus_symbol_bars(&root, &sym).await else {
            eprintln!(
                "SKIP s5_binance_2526: {sym_s} absent under {}",
                root.display()
            );
            continue;
        };
        any_ran = true;
        let outcome = run_field_and_rank(&bars, &field, &sym, SEED_2526, None).await;
        print_corpus_summary(&format!("S5 data/binance-2526 · {sym_s}"), &outcome);
    }
    if !any_ran {
        eprintln!("SKIP s5_binance_2526: corpus entirely absent — nothing to run");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// S6 — data/coinbase (venue cross-check, BTC only, price-only field)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires data/coinbase on disk — run after the long-running P2 fetch"]
async fn s6_coinbase_btc_venue_crosscheck() {
    let root = workspace_root().join("data/coinbase");
    let sym = Symbol::new("BTCUSDT"); // on-disk canonical (D2.a normalization)
    let Some(bars) = load_corpus_symbol_bars(&root, &sym).await else {
        eprintln!(
            "SKIP s6_coinbase_btc_venue_crosscheck: data/coinbase/BTCUSDT absent under {}",
            root.display()
        );
        return;
    };
    let support = ArmSupport {
        dvol: false,
        macro_riskon: false,
    }; // R4: venue cross-check is price-only by design (ADR-0084 D2.b)
    let field = build_field(support);

    let outcome = run_field_and_rank(&bars, &field, &sym, SEED_COINBASE, None).await;
    print_corpus_summary("S6 data/coinbase (venue cross-check) · BTCUSDT", &outcome);
}

// ─────────────────────────────────────────────────────────────────────────────
// S7/S8 — era-cost sensitivity annex (opt-in VolScaledSpread, ADR-0081 /
// ADR-0084 D7). Re-runs S1/S2's field once under the opt-in slippage model;
// SUPPLEMENTARY, not the primary verdict (which stays on the frozen default).
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires data/binance-1718 on disk — era-cost annex (ADR-0084 D7), run after the long-running P2 fetch"]
async fn s7_binance_1718_era_cost_annex_vol_scaled_spread() {
    let root = workspace_root().join("data/binance-1718");
    let symbols = ["BTCUSDT", "ETHUSDT", "BNBUSDT"];
    let support = ArmSupport {
        dvol: false,
        macro_riskon: false,
    };
    let field = build_field(support);
    let model = Some(cost::DEFAULT_VOL_SCALED_SPREAD);

    let mut any_ran = false;
    for sym_s in symbols {
        let sym = Symbol::new(sym_s);
        let Some(bars) = load_corpus_symbol_bars(&root, &sym).await else {
            eprintln!("SKIP s7 annex: {sym_s} absent under {}", root.display());
            continue;
        };
        any_ran = true;
        let outcome = run_field_and_rank(&bars, &field, &sym, SEED_1718, model).await;
        print_corpus_summary(
            &format!("S7 data/binance-1718 · {sym_s} — VolScaledSpread annex (E-2)"),
            &outcome,
        );
    }
    if !any_ran {
        eprintln!("SKIP s7_binance_1718_era_cost_annex: corpus entirely absent");
    }
}

#[tokio::test]
#[ignore = "requires data/binance-2020 on disk — era-cost annex (ADR-0084 D7), run after the long-running P2 fetch"]
async fn s8_binance_2020_era_cost_annex_vol_scaled_spread() {
    let root = workspace_root().join("data/binance-2020");
    let symbols = [
        "BTCUSDT", "ETHUSDT", "BNBUSDT", "XRPUSDT", "ADAUSDT", "LINKUSDT", "DOGEUSDT",
    ];
    let support = ArmSupport {
        dvol: false,
        macro_riskon: false,
    };
    let field = build_field(support);
    let model = Some(cost::DEFAULT_VOL_SCALED_SPREAD);

    let mut any_ran = false;
    for sym_s in symbols {
        let sym = Symbol::new(sym_s);
        let Some(bars) = load_corpus_symbol_bars(&root, &sym).await else {
            eprintln!("SKIP s8 annex: {sym_s} absent under {}", root.display());
            continue;
        };
        any_ran = true;
        let outcome = run_field_and_rank(&bars, &field, &sym, SEED_2020, model).await;
        print_corpus_summary(
            &format!("S8 data/binance-2020 · {sym_s} — VolScaledSpread annex (E-2)"),
            &outcome,
        );
    }
    if !any_ran {
        eprintln!("SKIP s8_binance_2020_era_cost_annex: corpus entirely absent");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Structural unit tests (no I/O — pure field-construction / span logic)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn build_field_excludes_dvol_when_unsupported() {
        let field = build_field(ArmSupport {
            dvol: false,
            macro_riskon: false,
        });
        assert!(
            !field.iter().any(|id| id.0.as_str() == "v0.dvol_regime"),
            "v0.dvol_regime must be removed when the corpus lacks DVOL history"
        );
        assert!(
            !field.iter().any(|id| id.0.as_str() == "v0.macro_riskon"),
            "v0.macro_riskon must be absent when macro is unsupported"
        );
    }

    #[test]
    fn build_field_includes_dvol_and_macro_when_supported() {
        let field = build_field(ArmSupport {
            dvol: true,
            macro_riskon: true,
        });
        assert!(field.iter().any(|id| id.0.as_str() == "v0.dvol_regime"));
        assert!(field.iter().any(|id| id.0.as_str() == "v0.macro_riskon"));
    }

    #[test]
    fn build_field_always_includes_price_only_singles_and_ensembles_and_shorts() {
        let field = build_field(ArmSupport {
            dvol: false,
            macro_riskon: false,
        });
        assert!(field.iter().any(|id| id.0.as_str() == "v0.sma"));
        assert!(field.iter().any(|id| id.0.as_str() == "v0.8.vote.majority"));
        assert!(field.iter().any(|id| id.0.as_str() == "v0.sma_cross_ls"));
    }

    #[test]
    fn build_field_never_includes_basis_or_funding_arms() {
        // R4: perp-basis/funding MN arms never appear in default_field() /
        // default_ensemble_field() / default_macro_field() — confirmed here
        // as a structural guard against accidental future inclusion.
        let field = build_field(ArmSupport {
            dvol: true,
            macro_riskon: true,
        });
        assert!(
            !field
                .iter()
                .any(|id| id.0.as_str().contains("basis") || id.0.as_str().contains("funding")),
            "basis/funding arms must never appear in the p2_verdict_rerun field"
        );
    }

    #[test]
    fn bar_span_ms_empty_is_zero_zero() {
        let (start, end) = bar_span_ms(&[]);
        assert_eq!(start, 0);
        assert_eq!(end, 0);
    }

    #[test]
    fn seed_bytes_from_u64_is_nonzero_for_nonzero_input() {
        let bytes = seed_bytes_from_u64(SEED_1718);
        assert_ne!(bytes, [0u8; 32], "ScenarioConfig rejects the all-zero seed");
    }

    #[test]
    fn all_corpus_seed_bases_are_distinct() {
        let seeds = [
            SEED_1718,
            SEED_2020,
            SEED_2122,
            SEED_2324,
            SEED_2526,
            SEED_COINBASE,
        ];
        for i in 0..seeds.len() {
            for j in (i + 1)..seeds.len() {
                assert_ne!(
                    seeds[i], seeds[j],
                    "corpus seed bases must be pairwise distinct (orthogonality)"
                );
            }
        }
    }
}
