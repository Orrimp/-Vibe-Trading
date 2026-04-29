//! T605 — Negative fixture tests for `CrossSectionalMomentumConfig` parsing.
//!
//! Each fixture under `tests/fixtures/bad_v1_strategies/` must produce a
//! non-panic `CrossSectionalLoadError` with the expected error code.

use std::path::PathBuf;
use strategy::{CrossSectionalLoadError, CrossSectionalMomentumConfig};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bad_v1_strategies")
}

fn load(stem: &str) -> Result<CrossSectionalMomentumConfig, CrossSectionalLoadError> {
    let path = fixtures_dir().join(format!("{stem}.toml"));
    CrossSectionalMomentumConfig::from_file(&path)
}

fn assert_error_code(stem: &str, expected_code: &str) {
    let result = load(stem);
    match result {
        Ok(_) => panic!("expected error for fixture '{stem}', but parse succeeded"),
        Err(e) => {
            assert_eq!(
                e.error_code(),
                expected_code,
                "fixture '{stem}': expected error_code='{}', got='{}'",
                expected_code,
                e.error_code()
            );
        }
    }
}

#[test]
fn t605_bad_empty_universe_rejected() {
    assert_error_code("bad_empty_universe", "invalid_universe");
}

#[test]
fn t605_bad_k_short_nonzero_rejected() {
    assert_error_code("bad_k_short_nonzero", "unsupported_short_sizing");
}

#[test]
fn t605_bad_wrong_kind_rejected() {
    assert_error_code("bad_wrong_kind", "unsupported_kind");
}

#[test]
fn t605_bad_invalid_exposure_cap_rejected() {
    assert_error_code("bad_invalid_exposure_cap", "invalid_exposure_cap");
}

#[test]
fn t605_bad_zero_exposure_cap_rejected() {
    assert_error_code("bad_zero_exposure_cap", "invalid_exposure_cap");
}

#[test]
fn t605_bad_zero_k_long_rejected() {
    assert_error_code("bad_zero_k_long", "invalid_k_long");
}

#[test]
fn t605_bad_zero_lookback_rejected() {
    assert_error_code("bad_zero_lookback", "invalid_lookback");
}

#[test]
fn t605_bad_zero_rebalance_rejected() {
    assert_error_code("bad_zero_rebalance", "invalid_rebalance");
}

#[test]
fn t605_bad_unsupported_sizing_rejected() {
    assert_error_code("bad_unsupported_sizing", "unsupported_sizing");
}

#[test]
fn t605_bad_invalid_drift_threshold_rejected() {
    assert_error_code("bad_invalid_drift_threshold", "invalid_drift_threshold");
}

/// Verify all 10 v1 error codes are distinct.
#[test]
fn t605_all_error_codes_distinct() {
    let codes: Vec<&str> = vec![
        "invalid_universe",
        "unknown_symbol",
        "invalid_lookback",
        "invalid_rebalance",
        "invalid_k_long",
        "unsupported_short_sizing",
        "invalid_exposure_cap",
        "invalid_drift_threshold",
        "unsupported_sizing",
        "unsupported_kind",
    ];
    let mut unique = codes.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(
        unique.len(),
        codes.len(),
        "all v1 error codes must be distinct"
    );
}
