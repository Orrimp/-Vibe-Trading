//! V-verdict mutual exclusivity test (T-D-N13, R11.5, ADR-0038 § D1.b).
//!
//! Asserts:
//! 1. Five per-label fixtures: one hand-crafted stats set per V1..V5 — each
//!    fixture fires exactly the expected verdict.
//! 2. Property test: 100 random per-symbol stats sets always return exactly
//!    one verdict from {V1, V2, V3, V4, V5}.
//!
//! ## Algorithm
//!
//! The verdict algorithm is inlined here (verbatim from `vol_verdict.rs`) so
//! this test file is self-contained and does NOT require the `candle` feature.
//! This mirrors the `forecast_distribution_verdict.rs` precedent (ADR-0033 § D3.d).
//!
//! ## Threshold notes (from ADR-0038 § D1.b)
//!
//! - V1: CoV(σ̂) = std_sigma_hat / mean_sigma_hat < 1e-3 on EVERY symbol.
//! - V2: qlike_dispersion = qlike_max / qlike_min > 3.0.
//! - V3: mean(mean_sh / mean_sr) outside [0.7, 1.4].
//! - V4: n_symbols_with_≥10pct_improvement < 7/10.
//! - V5: fallback (all V1-V4 false).
//!
//! ## Cross-references
//!
//! - T-D-N13 — this test.
//! - R11.5 — V-verdict mutual exclusivity requirement.
//! - ADR-0038 § D1.b — V-verdict priority tree with evidence strings.
//! - `crates/forecast/tests/forecast_distribution_verdict.rs` — precedent.

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

// ── Inlined types (mirrors vol_verdict.rs) ───────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Verdict {
    V1 {
        evidence: String,
        follow_on: &'static str,
    },
    V2 {
        evidence: String,
        follow_on: &'static str,
    },
    V3 {
        evidence: String,
        follow_on: &'static str,
    },
    V4 {
        evidence: String,
        follow_on: &'static str,
    },
    V5 {
        evidence: String,
        follow_on: &'static str,
    },
}

impl Verdict {
    fn label(&self) -> &'static str {
        match self {
            Verdict::V1 { .. } => "V1",
            Verdict::V2 { .. } => "V2",
            Verdict::V3 { .. } => "V3",
            Verdict::V4 { .. } => "V4",
            Verdict::V5 { .. } => "V5",
        }
    }
}

#[derive(Debug, Clone)]
struct PerSymbolStats {
    symbol: String,
    qlike_garch: f64,
    qlike_constant: f64,
    mean_sigma_hat: f64,
    mean_sigma_realized: f64,
    std_sigma_hat: f64,
}

struct AggregateStats {
    qlike_garch_max: f64,
    qlike_garch_min: f64,
    qlike_dispersion: f64,
    mean_calibration_ratio: f64,
    n_symbols_improving: usize,
}

fn compute_aggregate(per_symbol: &[PerSymbolStats]) -> AggregateStats {
    let n = per_symbol.len() as f64;
    let qlike_garch_max = per_symbol
        .iter()
        .map(|s| s.qlike_garch)
        .fold(f64::NEG_INFINITY, f64::max);
    let qlike_garch_min = per_symbol
        .iter()
        .map(|s| s.qlike_garch)
        .fold(f64::INFINITY, f64::min);
    let qlike_dispersion = if qlike_garch_min > 1e-12 {
        qlike_garch_max / qlike_garch_min
    } else {
        f64::INFINITY
    };
    let mean_calibration_ratio = per_symbol
        .iter()
        .map(|s| s.mean_sigma_hat / s.mean_sigma_realized.max(1e-12))
        .sum::<f64>()
        / n;
    let n_symbols_improving = per_symbol
        .iter()
        .filter(|s| (s.qlike_constant - s.qlike_garch) / s.qlike_constant.max(1e-12) >= 0.10)
        .count();
    AggregateStats {
        qlike_garch_max,
        qlike_garch_min,
        qlike_dispersion,
        mean_calibration_ratio,
        n_symbols_improving,
    }
}

/// Verbatim copy of the ADR-0038 § D1.b V-verdict priority tree.
fn classify_verdict(agg: &AggregateStats, per_symbol: &[PerSymbolStats]) -> Verdict {
    // V1 — Constant collapse.
    let max_cov = per_symbol
        .iter()
        .map(|s| s.std_sigma_hat / s.mean_sigma_hat.max(1e-12))
        .fold(0.0_f64, f64::max);
    if per_symbol
        .iter()
        .all(|s| s.std_sigma_hat / s.mean_sigma_hat.max(1e-12) < 1e-3)
    {
        let worst = per_symbol
            .iter()
            .max_by(|a, b| {
                (a.std_sigma_hat / a.mean_sigma_hat.max(1e-12))
                    .partial_cmp(&(b.std_sigma_hat / b.mean_sigma_hat.max(1e-12)))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| s.symbol.as_str())
            .unwrap_or("?");
        return Verdict::V1 {
            evidence: format!(
                "max CoV(σ̂) = {max_cov:.6} < 1e-3 across all 10 symbols (worst-symbol = {worst})"
            ),
            follow_on: "v3-garch-refit-diagnose",
        };
    }

    // V2 — Per-symbol mis-fit.
    if agg.qlike_dispersion > 3.0 {
        return Verdict::V2 {
            evidence: format!(
                "qlike_dispersion = {:.6} > 3.0 (max = {:.6}, min = {:.6})",
                agg.qlike_dispersion, agg.qlike_garch_max, agg.qlike_garch_min,
            ),
            follow_on: "v3-garch-per-symbol-hyperparam-search",
        };
    }

    // V3 — Calibration drift.
    if agg.mean_calibration_ratio < 0.7 || agg.mean_calibration_ratio > 1.4 {
        return Verdict::V3 {
            evidence: format!(
                "mean_calibration_ratio = {:.6} outside [0.7, 1.4]",
                agg.mean_calibration_ratio,
            ),
            follow_on: "v3-garch-calibration-tune",
        };
    }

    // V4 — No improvement over constant-σ baseline.
    if agg.n_symbols_improving < 7 {
        return Verdict::V4 {
            evidence: format!(
                "n_symbols_improving_≥10pct_over_constant_sigma = {} < 7 of 10",
                agg.n_symbols_improving,
            ),
            follow_on: "v3-data-vol-investigation",
        };
    }

    // V5 — Healthy fallback.
    Verdict::V5 {
        evidence: format!(
            "n_improving = {} ≥ 7; qlike_dispersion = {:.6} ≤ 3.0; \
             mean_calibration_ratio = {:.6} ∈ [0.7, 1.4]",
            agg.n_symbols_improving, agg.qlike_dispersion, agg.mean_calibration_ratio,
        ),
        follow_on: "v_alpha_strategy_gate",
    }
}

// ── Fixture helpers ───────────────────────────────────────────────────────────

fn make_sym(
    symbol: &str,
    qlike_garch: f64,
    qlike_constant: f64,
    mean_sh: f64,
    mean_sr: f64,
    std_sh: f64,
) -> PerSymbolStats {
    PerSymbolStats {
        symbol: symbol.to_string(),
        qlike_garch,
        qlike_constant,
        mean_sigma_hat: mean_sh,
        mean_sigma_realized: mean_sr,
        std_sigma_hat: std_sh,
    }
}

/// Healthy 10-symbol stats (should yield V5).
fn healthy_10() -> Vec<PerSymbolStats> {
    (0..10)
        .map(|i| {
            let base = 0.001 * (i as f64 + 1.0);
            make_sym(
                &format!("SYM{i}USDT"),
                0.10 + base,               // qlike_garch
                0.20 + base,               // qlike_constant (50% improvement → > 10%)
                0.010 + 0.0005 * i as f64, // mean_sigma_hat
                0.011 + 0.0005 * i as f64, // mean_sigma_realized  (calib ratio ≈ 0.9)
                0.003 + 0.0001 * i as f64, // std_sigma_hat (CoV ≈ 0.3 > 1e-3)
            )
        })
        .collect()
}

// ── Fixture tests: one per V-label ───────────────────────────────────────────

/// V1 fixture: all symbols have std_sigma_hat = 0 → CoV = 0 < 1e-3.
#[test]
fn v1_fixture_fires_v1() {
    let per_symbol: Vec<PerSymbolStats> = (0..10)
        .map(|i| {
            make_sym(
                &format!("SYM{i}"),
                0.10,  // qlike_garch
                0.20,  // qlike_constant
                0.010, // mean_sigma_hat
                0.011, // mean_sigma_realized
                0.0,   // std_sigma_hat = 0 → CoV = 0 < 1e-3 → V1
            )
        })
        .collect();
    let agg = compute_aggregate(&per_symbol);
    let v = classify_verdict(&agg, &per_symbol);
    assert_eq!(
        v.label(),
        "V1",
        "V1 fixture must fire V1, got {}",
        v.label()
    );
    println!(
        "[v1_fixture_fires_v1] PASS — evidence: {}",
        match &v {
            Verdict::V1 { evidence, .. } => evidence,
            _ => "N/A",
        }
    );
}

/// V2 fixture: qlike_dispersion > 3.0 (no V1, yes V2).
#[test]
fn v2_fixture_fires_v2() {
    let mut per_symbol: Vec<PerSymbolStats> = (0..10)
        .map(|i| {
            make_sym(
                &format!("SYM{i}"),
                0.10,
                0.20,
                0.010,
                0.011,
                0.005, // CoV > 1e-3 so V1 doesn't fire
            )
        })
        .collect();
    // Make qlike_dispersion > 3: first symbol has very high QLIKE.
    per_symbol[0].qlike_garch = 1.0;
    per_symbol[1].qlike_garch = 0.10; // min
    let agg = compute_aggregate(&per_symbol);
    assert!(
        agg.qlike_dispersion > 3.0,
        "dispersion should be > 3 for V2 fixture"
    );
    let v = classify_verdict(&agg, &per_symbol);
    assert_eq!(
        v.label(),
        "V2",
        "V2 fixture must fire V2, got {}",
        v.label()
    );
    println!(
        "[v2_fixture_fires_v2] PASS — dispersion={:.4}",
        agg.qlike_dispersion
    );
}

/// V3 fixture: mean_calibration_ratio > 1.4 (no V1/V2, yes V3).
#[test]
fn v3_fixture_fires_v3() {
    let per_symbol: Vec<PerSymbolStats> = (0..10)
        .map(|_| {
            make_sym(
                "SYMXUSDT", 0.10, 0.20,
                0.05, // mean_sigma_hat >> mean_sigma_realized → calib_ratio ≈ 5.0 > 1.4
                0.01, 0.005, // CoV > 1e-3 so V1 doesn't fire
            )
        })
        .collect();
    let agg = compute_aggregate(&per_symbol);
    assert!(
        agg.qlike_dispersion <= 3.0,
        "dispersion should be ≤3 for V3 fixture (no V2)"
    );
    assert!(
        agg.mean_calibration_ratio > 1.4,
        "calib ratio should > 1.4 for V3 fixture"
    );
    let v = classify_verdict(&agg, &per_symbol);
    assert_eq!(
        v.label(),
        "V3",
        "V3 fixture must fire V3, got {}",
        v.label()
    );
    println!(
        "[v3_fixture_fires_v3] PASS — calib_ratio={:.4}",
        agg.mean_calibration_ratio
    );
}

/// V4 fixture: n_symbols_improving < 7 (no V1/V2/V3, yes V4).
#[test]
fn v4_fixture_fires_v4() {
    let per_symbol: Vec<PerSymbolStats> = (0..10)
        .map(|_| {
            make_sym(
                "SYMXUSDT", 0.20, // qlike_garch == qlike_constant → 0% improvement → V4
                0.20, 0.010, 0.011, 0.005, // CoV > 1e-3 so V1 doesn't fire
            )
        })
        .collect();
    let agg = compute_aggregate(&per_symbol);
    assert_eq!(
        agg.n_symbols_improving, 0,
        "n_improving should be 0 for V4 fixture"
    );
    let v = classify_verdict(&agg, &per_symbol);
    assert_eq!(
        v.label(),
        "V4",
        "V4 fixture must fire V4, got {}",
        v.label()
    );
    println!(
        "[v4_fixture_fires_v4] PASS — n_improving={}",
        agg.n_symbols_improving
    );
}

/// V5 fixture: healthy stats (no V1..V4 fires).
#[test]
fn v5_fixture_fires_v5() {
    let per_symbol = healthy_10();
    let agg = compute_aggregate(&per_symbol);
    let v = classify_verdict(&agg, &per_symbol);
    assert_eq!(
        v.label(),
        "V5",
        "healthy fixture must fire V5, got {}",
        v.label()
    );
    println!(
        "[v5_fixture_fires_v5] PASS — n_improving={}, dispersion={:.4}, calib={:.4}",
        agg.n_symbols_improving, agg.qlike_dispersion, agg.mean_calibration_ratio
    );
}

// ── Mutual exclusivity ────────────────────────────────────────────────────────

/// Each fixture fires exactly one verdict (trivially true by construction,
/// but verified explicitly).
#[test]
fn v1_v2_v3_v4_v5_all_distinct() {
    let fixtures = [
        // V1 fixture
        {
            let per_symbol: Vec<PerSymbolStats> = (0..10)
                .map(|i| make_sym(&format!("SYM{i}"), 0.10, 0.20, 0.010, 0.011, 0.0))
                .collect();
            ("V1", per_symbol)
        },
        // V2 fixture
        {
            let mut per_symbol: Vec<PerSymbolStats> = (0..10)
                .map(|i| make_sym(&format!("SYM{i}"), 0.10, 0.20, 0.010, 0.011, 0.005))
                .collect();
            per_symbol[0].qlike_garch = 1.0;
            per_symbol[1].qlike_garch = 0.10;
            ("V2", per_symbol)
        },
        // V3 fixture
        {
            let per_symbol: Vec<PerSymbolStats> = (0..10)
                .map(|_| make_sym("SYMXUSDT", 0.10, 0.20, 0.05, 0.01, 0.005))
                .collect();
            ("V3", per_symbol)
        },
        // V4 fixture
        {
            let per_symbol: Vec<PerSymbolStats> = (0..10)
                .map(|_| make_sym("SYMXUSDT", 0.20, 0.20, 0.010, 0.011, 0.005))
                .collect();
            ("V4", per_symbol)
        },
        // V5 fixture
        { ("V5", healthy_10()) },
    ];

    for (expected, per_symbol) in &fixtures {
        let agg = compute_aggregate(per_symbol);
        let v = classify_verdict(&agg, per_symbol);
        assert_eq!(
            v.label(),
            *expected,
            "fixture[{expected}] returned {} instead of {expected}",
            v.label()
        );
        println!("[v1_v2_v3_v4_v5_all_distinct] {expected}: PASS");
    }
}

/// Property test: 100 random 10-symbol stats sets each return exactly one of {V1,V2,V3,V4,V5}.
#[test]
fn property_exactly_one_verdict_on_random_stats() {
    let mut rng = ChaCha20Rng::from_seed([42u8; 32]);
    let valid_labels = ["V1", "V2", "V3", "V4", "V5"];
    let mut counts = [0usize; 5];

    for trial in 0..100 {
        let per_symbol: Vec<PerSymbolStats> = (0..10)
            .map(|i| {
                let qlike_garch = rng.random_range(0.001_f64..2.0);
                let qlike_constant = rng.random_range(0.001_f64..2.0);
                let mean_sh = rng.random_range(1e-4_f64..0.1);
                let mean_sr = rng.random_range(1e-4_f64..0.1);
                let std_sh = rng.random_range(0.0_f64..0.01);
                make_sym(
                    &format!("SYM{i}"),
                    qlike_garch,
                    qlike_constant,
                    mean_sh,
                    mean_sr,
                    std_sh,
                )
            })
            .collect();

        let agg = compute_aggregate(&per_symbol);
        let v = classify_verdict(&agg, &per_symbol);
        let label = v.label();

        // Verify label is valid.
        assert!(
            valid_labels.contains(&label),
            "trial {trial}: got invalid verdict label '{label}'"
        );

        // Count verdicts.
        match label {
            "V1" => counts[0] += 1,
            "V2" => counts[1] += 1,
            "V3" => counts[2] += 1,
            "V4" => counts[3] += 1,
            "V5" => counts[4] += 1,
            _ => unreachable!(),
        }
    }

    println!(
        "[property_exactly_one_verdict_on_random_stats] PASS — 100 trials: \
         V1={}, V2={}, V3={}, V4={}, V5={}",
        counts[0], counts[1], counts[2], counts[3], counts[4]
    );

    // Sanity: all 100 trials returned exactly one verdict.
    let total: usize = counts.iter().sum();
    assert_eq!(total, 100, "expected 100 total, got {total}");
}
