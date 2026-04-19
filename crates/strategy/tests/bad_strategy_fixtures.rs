//! T504 — Negative fixture tests for composed strategy parsing.
//!
//! Each fixture under `tests/fixtures/bad_strategies/` must produce a distinct
//! non-panic `StrategyLoadError` with the expected error code.

use std::path::PathBuf;
use strategy::composed::{ComposedStrategyConfig, StrategyLoadError};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/bad_strategies")
}

fn load(stem: &str) -> Result<ComposedStrategyConfig, StrategyLoadError> {
    let path = fixtures_dir().join(format!("{stem}.toml"));
    ComposedStrategyConfig::from_file(&path)
}

/// Load and expect failure with the given error code.
fn assert_error_code(stem: &str, expected_code: &str) {
    let result = load(stem);
    match result {
        Ok(_) => panic!("expected error for {stem}, but parse succeeded"),
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
fn t504_arity_mismatch() {
    assert_error_code("bad_arity_mismatch", "arity_mismatch");
}

#[test]
fn t504_unknown_indicator() {
    assert_error_code("bad_unknown_indicator", "unknown_indicator");
}

#[test]
fn t504_unknown_param() {
    assert_error_code("bad_unknown_param", "unknown_param");
}

#[test]
fn t504_invalid_range() {
    assert_error_code("bad_invalid_range", "invalid_range");
}

#[test]
fn t504_invalid_stage() {
    assert_error_code("bad_invalid_stage", "invalid_stage");
}

#[test]
fn t504_unsupported_sizing() {
    assert_error_code("bad_unsupported_sizing", "unsupported_sizing");
}

#[test]
fn t504_empty_signal() {
    assert_error_code("bad_empty_signal", "empty_signal");
}

#[test]
fn t504_id_filename_mismatch() {
    assert_error_code("bad_id_filename_mismatch", "id_filename_mismatch");
}

#[test]
fn t504_grammar_parse() {
    assert_error_code("bad_grammar_parse", "grammar_parse");
}

#[test]
fn t504_toml_parse() {
    assert_error_code("bad_toml_parse", "toml_parse");
}

/// Verify that all 10 error codes from the error-code table are distinct.
#[test]
fn t504_all_error_codes_distinct() {
    let codes: Vec<&str> = vec![
        "arity_mismatch",
        "unknown_indicator",
        "unknown_param",
        "invalid_range",
        "invalid_stage",
        "unsupported_sizing",
        "empty_signal",
        "id_filename_mismatch",
        "grammar_parse",
        "toml_parse",
    ];
    let stems = vec![
        "bad_arity_mismatch",
        "bad_unknown_indicator",
        "bad_unknown_param",
        "bad_invalid_range",
        "bad_invalid_stage",
        "bad_unsupported_sizing",
        "bad_empty_signal",
        "bad_id_filename_mismatch",
        "bad_grammar_parse",
        "bad_toml_parse",
    ];

    let mut observed_codes: Vec<String> = Vec::new();
    for (stem, expected) in stems.iter().zip(codes.iter()) {
        let result = load(stem);
        let actual_code = match result {
            Err(e) => {
                let code = e.error_code().to_string();
                assert_eq!(&code, expected, "fixture '{stem}' wrong code");
                code
            }
            Ok(_) => panic!("fixture '{stem}' should have failed"),
        };
        assert!(!observed_codes.contains(&actual_code),
            "duplicate error code '{actual_code}' from fixture '{stem}'");
        observed_codes.push(actual_code);
    }
    assert_eq!(observed_codes.len(), 10, "expected exactly 10 distinct error codes");
}
