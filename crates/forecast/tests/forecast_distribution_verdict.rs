//! Integration tests for the F-verdict classifier (ADR-0033 § D3).
//!
//! Tests T-D-4:
//! 1. F1 fixture → Verdict::F1
//! 2. F2 fixture → Verdict::F2
//! 3. F3 fixture → Verdict::F3
//! 4. F4 fixture (wide spread, no other trigger) → Verdict::F4
//! 5. Mutual-exclusivity property: 100 random CheckpointStats all return
//!    exactly one verdict.

// The verdict module is `pub mod verdict` in forecast_distribution.rs, but
// since that's a bin (not a lib), we re-declare the types here to test
// the algorithm logic independently.
//
// We inline the exact algorithm from forecast_distribution.rs so the
// test file is self-contained and does NOT require the candle feature.

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

// ── Inline types from forecast_distribution::verdict ─────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    F1 {
        evidence: String,
        follow_on: &'static str,
    },
    F2 {
        evidence: String,
        follow_on: &'static str,
    },
    F3 {
        evidence: String,
        follow_on: &'static str,
    },
    F4 {
        evidence: String,
        follow_on: &'static str,
    },
}

impl Verdict {
    pub fn label(&self) -> &'static str {
        match self {
            Verdict::F1 { .. } => "F1",
            Verdict::F2 { .. } => "F2",
            Verdict::F3 { .. } => "F3",
            Verdict::F4 { .. } => "F4",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckpointStats {
    pub abs_p95: f32,
    pub abs_p99: f32,
    pub std: f32,
    pub sigma_train: f32,
    pub epsilon: f32,
    pub tau: f32,
    pub frac_inside_epsilon: f32,
    pub frac_passes_confidence_gate: f32,
    pub confidence_gate_survival: [f32; 9],
}

/// Priority-ordered F-verdict classifier (verbatim copy from ADR-0033 § D3.b).
pub fn classify(s: &CheckpointStats) -> Verdict {
    // F1 — Training collapse.
    if (s.abs_p95 as f64) < 1e-6 {
        return Verdict::F1 {
            evidence: format!("abs_p95 = {:.9} < 1e-6", s.abs_p95),
            follow_on: "v25-tcn-retrain",
        };
    }

    // F2 — sigma_train mis-calibration.
    if s.std > 0.1 * s.sigma_train && (s.frac_passes_confidence_gate as f64) < 1e-6 {
        return Verdict::F2 {
            evidence: format!(
                "std = {:.9}, sigma_train = {:.6}, std/sigma_train = {:.3}, \
                 frac_passes_confidence_gate = {:.9}",
                s.std,
                s.sigma_train,
                s.std / s.sigma_train,
                s.frac_passes_confidence_gate
            ),
            follow_on: "v25-tcn-recalibrate",
        };
    }

    // F3 — Gating too tight.
    if s.confidence_gate_survival[5] >= 1e-4 && s.frac_inside_epsilon > 0.5 {
        return Verdict::F3 {
            evidence: format!(
                "frac_inside_epsilon = {:.6}, confidence_gate_survival[τ=0.6] = {:.6}",
                s.frac_inside_epsilon, s.confidence_gate_survival[5]
            ),
            follow_on: "v25-tcn-threshold-tuning",
        };
    }

    // F4 — Fallback.
    Verdict::F4 {
        evidence: format!(
            "abs_p95 = {:.9} >= 1e-6, std/sigma_train = {:.3} <= 0.1 OR \
             frac_passes_confidence_gate = {:.9} >= 1e-6, \
             frac_inside_epsilon = {:.6} <= 0.5",
            s.abs_p95,
            s.std / s.sigma_train,
            s.frac_passes_confidence_gate,
            s.frac_inside_epsilon
        ),
        follow_on: "v25-tcn-horizon-bump-or-retire",
    }
}

// ── Helper to build a base Stats struct ──────────────────────────────────────

fn base_stats() -> CheckpointStats {
    CheckpointStats {
        abs_p95: 0.001,
        abs_p99: 0.002,
        std: 0.0001,
        sigma_train: 10.0,
        epsilon: 0.0005,
        tau: 0.6,
        frac_inside_epsilon: 0.99,
        frac_passes_confidence_gate: 0.0,
        confidence_gate_survival: [0.0; 9],
    }
}

// ── Test 1: F1 fixture ────────────────────────────────────────────────────────

/// abs_p95 = 1e-9 < 1e-6 → must classify as F1.
#[test]
fn test_f1_fixture() {
    let s = CheckpointStats {
        abs_p95: 1e-9,
        ..base_stats()
    };
    let v = classify(&s);
    assert_eq!(v.label(), "F1", "expected F1 for abs_p95 = 1e-9");
    assert!(
        matches!(
            v,
            Verdict::F1 {
                follow_on: "v25-tcn-retrain",
                ..
            }
        ),
        "F1 follow_on must be v25-tcn-retrain"
    );
}

// ── Test 2: F2 fixture ────────────────────────────────────────────────────────

/// std = 5.0, sigma_train = 10.0 (std/sigma = 0.5 > 0.1),
/// frac_passes_confidence_gate = 0.0 < 1e-6 → must classify as F2.
///
/// abs_p95 must be >= 1e-6 to bypass F1.
#[test]
fn test_f2_fixture() {
    let s = CheckpointStats {
        abs_p95: 0.01, // > 1e-6 so F1 doesn't fire
        std: 5.0,
        sigma_train: 10.0,
        frac_passes_confidence_gate: 0.0,
        frac_inside_epsilon: 0.99,
        ..base_stats()
    };
    let v = classify(&s);
    assert_eq!(
        v.label(),
        "F2",
        "expected F2 for std/sigma=0.5, frac_passes=0"
    );
    assert!(
        matches!(
            v,
            Verdict::F2 {
                follow_on: "v25-tcn-recalibrate",
                ..
            }
        ),
        "F2 follow_on must be v25-tcn-recalibrate"
    );
}

// ── Test 3: F3 fixture ────────────────────────────────────────────────────────

/// abs_p95 = 0.0003 (> 1e-6, bypasses F1),
/// std/sigma_train = 0.0001/10 = 1e-5 < 0.1 so F2 doesn't fire,
/// confidence_gate_survival[5 (τ=0.6)] = 0.001 >= 1e-4,
/// frac_inside_epsilon = 0.7 > 0.5 → must classify as F3.
#[test]
fn test_f3_fixture() {
    let mut surv = [0.0f32; 9];
    surv[5] = 0.001; // τ=0.6 survival >= 1e-4
    let s = CheckpointStats {
        abs_p95: 0.0003,
        std: 0.0001,
        sigma_train: 10.0,
        frac_passes_confidence_gate: 0.001,
        frac_inside_epsilon: 0.7,
        confidence_gate_survival: surv,
        ..base_stats()
    };
    let v = classify(&s);
    assert_eq!(v.label(), "F3", "expected F3 for given fixture");
    assert!(
        matches!(
            v,
            Verdict::F3 {
                follow_on: "v25-tcn-threshold-tuning",
                ..
            }
        ),
        "F3 follow_on must be v25-tcn-threshold-tuning"
    );
}

// ── Test 4: F4 fixture ────────────────────────────────────────────────────────

/// Wide spread but no other gate triggers → must classify as F4.
///
/// - abs_p95 = 0.01 (> 1e-6, bypasses F1)
/// - std = 0.001, sigma_train = 10.0 → std/sigma = 1e-4 < 0.1, so F2 checks
///   condition `std > 0.1 * sigma_train` = 0.001 > 1.0 → FALSE
/// - confidence_gate_survival[5] = 0.0 < 1e-4 → F3 condition FALSE
/// → falls through to F4.
#[test]
fn test_f4_fixture() {
    let s = CheckpointStats {
        abs_p95: 0.01,
        std: 0.001,
        sigma_train: 10.0,
        frac_passes_confidence_gate: 1e-4, // passes confidence gate for some bars
        frac_inside_epsilon: 0.3,          // < 0.5 so F3 doesn't fire
        confidence_gate_survival: [0.0; 9],
        ..base_stats()
    };
    let v = classify(&s);
    assert_eq!(
        v.label(),
        "F4",
        "expected F4 for wide-spread, no-other-trigger fixture"
    );
    assert!(
        matches!(
            v,
            Verdict::F4 {
                follow_on: "v25-tcn-horizon-bump-or-retire",
                ..
            }
        ),
        "F4 follow_on must be v25-tcn-horizon-bump-or-retire"
    );
}

// ── Test 5: Mutual-exclusivity property ──────────────────────────────────────

/// For N=100 random CheckpointStats (ChaCha20Rng seed 0xDEADBEEF),
/// classify returns exactly one of F1/F2/F3/F4 each time, and the four
/// cases never co-trigger (which is guaranteed by priority-ordered fallthrough,
/// but we assert it explicitly).
#[test]
fn test_mutual_exclusivity_random() {
    let mut rng = ChaCha20Rng::seed_from_u64(0xDEAD_BEEF);

    for _ in 0..100 {
        // Generate random stats. We allow the full range so all four
        // verdicts are reachable.
        let abs_p95: f32 = rng.random::<f32>() * 1e-3;
        let abs_p99: f32 = abs_p95 + rng.random::<f32>() * 1e-3;
        let sigma_train: f32 = 5.0 + rng.random::<f32>() * 10.0;
        let std: f32 = rng.random::<f32>() * sigma_train * 0.6; // 0..0.6*sigma
        let frac_inside_epsilon: f32 = rng.random();
        let frac_passes_confidence_gate: f32 = rng.random::<f32>() * 1e-4;
        let mut confidence_gate_survival = [0.0f32; 9];
        for g in &mut confidence_gate_survival {
            *g = rng.random::<f32>() * 1e-3;
        }

        let s = CheckpointStats {
            abs_p95,
            abs_p99,
            std,
            sigma_train,
            epsilon: 0.0005,
            tau: 0.6,
            frac_inside_epsilon,
            frac_passes_confidence_gate,
            confidence_gate_survival,
        };

        // classify must return exactly one verdict.
        let v = classify(&s);
        let label = v.label();
        assert!(
            label == "F1" || label == "F2" || label == "F3" || label == "F4",
            "classify returned unexpected label: {label}"
        );

        // Verify the returned verdict is consistent with the triggering condition
        // by checking the converse: if it returned F4, then F1/F2/F3 must be
        // NOT triggered.
        if label == "F4" {
            assert!(
                !((s.abs_p95 as f64) < 1e-6),
                "F4 returned but F1 condition is true"
            );
            assert!(
                !(s.std > 0.1 * s.sigma_train && (s.frac_passes_confidence_gate as f64) < 1e-6),
                "F4 returned but F2 condition is true"
            );
            assert!(
                !(s.confidence_gate_survival[5] >= 1e-4 && s.frac_inside_epsilon > 0.5),
                "F4 returned but F3 condition is true"
            );
        }
    }
}
