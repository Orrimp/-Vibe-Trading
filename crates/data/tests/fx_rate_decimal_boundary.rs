//! Story 3-19 / ADR-0101 D6 — the live FX rate must never pass through `f64`.
//!
//! ## Why this file exists
//!
//! This is the one place in the story where AD-9 ("money math is `Decimal`,
//! never `f64`") can be broken by accident and every other test still passes:
//! `serde_json` turns every JSON float into an `f64` unless told otherwise, and
//! at quote magnitudes (~1.1) the round-trip *looks* right. `1.1165_f64`
//! reproduces `1.1165` exactly when printed, so a test written with the
//! measured body cannot see the defect.
//!
//! So this file pins the parse in the `f64` DIMENSION, two-sidedly:
//!
//! - [`a_rate_token_finer_than_f64_survives_exactly`] — a token carrying more
//!   significant digits than an `f64` can hold parses to the EXACT decimal, and
//!   is NOT equal to what the `f64` route would have produced. This fails the
//!   moment an `f64` hop is reintroduced.
//! - [`the_measured_rate_cannot_detect_an_f64_hop`] — the control that makes
//!   the first test meaningful: with the *measured* `1.1165` body the `f64`
//!   route gives the same value, so a guard written against that fixture would
//!   pin nothing. (The house rule: a constant is pinned only if two fixtures
//!   differ in its dimension AND expect different outcomes.)
//! - [`the_rate_path_source_names_no_f64`] — the structural leg: the module's
//!   executable lines mention no `f64` at all, so the next reader cannot
//!   "simplify" the `RawValue` parse into `Value::as_f64` without turning this
//!   red.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::str::FromStr;

use data::fx_rate::{FRANKFURTER_EUR_USD_URL, parse_frankfurter_eur_usd};
use rust_decimal::{Decimal, prelude::FromPrimitive};

/// A rate token with 20 significant digits — more than an `f64`'s ~17, so the
/// exact decimal and the `f64` round-trip are DIFFERENT numbers.
const FINE_TOKEN: &str = "1.1234567890123456789";

/// The rate measured against the live endpoint on 2026-11-08 (ADR-0101 D3) —
/// short enough that `f64` reproduces it, which is the trap.
const MEASURED_TOKEN: &str = "1.1165";

/// A Frankfurter `/v1/latest` body carrying `token` as the USD rate.
fn body_with(token: &str) -> String {
    format!(r#"{{"amount":1.0,"base":"EUR","date":"2026-11-06","rates":{{"USD":{token}}}}}"#)
}

/// What the rate WOULD be if the token were routed through `f64` — the defect
/// this file exists to catch, computed the way the accidental implementation
/// would: `serde_json` hands the float to `rust_decimal`'s `visit_f64`, which
/// is `Decimal::from_f64` (the *lossy* conversion that keeps the shortest
/// round-tripping decimal — the reason the hop is invisible at 1.1165).
fn via_f64(token: &str) -> Decimal {
    let as_float: f64 = token.parse().expect("the token is a valid float");
    Decimal::from_f64(as_float).expect("representable")
}

/// **The guard.** A token finer than `f64` parses to the exact decimal.
#[test]
fn a_rate_token_finer_than_f64_survives_exactly() {
    let fx = parse_frankfurter_eur_usd(&body_with(FINE_TOKEN), FRANKFURTER_EUR_USD_URL)
        .expect("a well-formed body with a fine-grained rate must parse");

    let exact = Decimal::from_str(FINE_TOKEN).expect("exact decimal");
    assert_eq!(
        fx.rate(),
        exact,
        "the parsed rate must be the token's exact decimal. Got {}, expected {exact}. \
         If this fails, the rate is being routed through f64 (ADR-0101 D6).",
        fx.rate()
    );

    // And it is NOT the f64 value — proof the assertion above is load-bearing
    // rather than true by coincidence.
    let f64_route = via_f64(FINE_TOKEN);
    assert_ne!(
        fx.rate(),
        f64_route,
        "the f64 route must produce a DIFFERENT number for this token \
         (got {f64_route}); if it does not, this fixture cannot detect an f64 hop \
         and the guard is vacuous."
    );
}

/// **The control.** The measured `1.1165` body cannot detect an `f64` hop —
/// both routes agree. Documented as an executable fact so nobody replaces the
/// fine-grained fixture above with "the realistic one" and quietly turns the
/// guard into a tautology.
#[test]
fn the_measured_rate_cannot_detect_an_f64_hop() {
    let fx = parse_frankfurter_eur_usd(&body_with(MEASURED_TOKEN), FRANKFURTER_EUR_USD_URL)
        .expect("the measured body must parse");
    let exact = Decimal::from_str(MEASURED_TOKEN).expect("exact decimal");

    assert_eq!(fx.rate(), exact, "the measured rate parses exactly");
    assert_eq!(
        via_f64(MEASURED_TOKEN),
        exact,
        "at 1.1165 the f64 route is indistinguishable from the exact decimal — \
         which is why the real guard uses a 20-digit token. If this ever stops \
         holding, the trap it documents has changed shape; read ADR-0101 D6 before \
         editing either test."
    );
}

/// **The structural leg.** No executable line of the rate module mentions
/// `f64`. The module's prose discusses the trap at length, so comments and doc
/// comments are stripped before the scan.
#[test]
fn the_rate_path_source_names_no_f64() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fx_rate.rs");
    let src = std::fs::read_to_string(&path).expect("the fx_rate module is readable");

    let offenders: Vec<(usize, String)> = src
        .lines()
        .enumerate()
        .map(|(i, line)| {
            // Strip everything from the first `//` — doc comments included.
            let code = line.split("//").next().unwrap_or("");
            (i + 1, code.to_owned())
        })
        .filter(|(_, code)| code.contains("f64"))
        .collect();

    assert!(
        offenders.is_empty(),
        "no executable line in crates/data/src/fx_rate.rs may mention f64 \
         (ADR-0101 D6 — the rate reaches Decimal straight from the JSON token). \
         Offending lines: {offenders:?}"
    );
}
