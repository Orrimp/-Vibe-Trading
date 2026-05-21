//! Unit tests for `TcnOverlayMomentumStrategy` tuned builders (T-D-N3).
//!
//! Asserts the invariance contract from D-AR-1.f / D-AR-1.g:
//!
//! 1. `with_tcn_bs1(base).confidence_threshold == dec!(0.6)` (unchanged default).
//! 2. `TcnSyncForecaster::load_bs1()?.direction_epsilon() == None` (default = CONST path).
//! 3. `with_tcn_bs1_tuned(τ, ε).confidence_threshold == τ` (explicit τ forwarded).
//! 4. `TcnSyncForecaster::load_bs1()?.with_direction_epsilon(ε).direction_epsilon() == Some(ε_f32)`.
//! 5. Ditto for BS-2 (mirrors tests 1-4 on the BS-2 checkpoint).
//!
//! These tests require `--features forecast` (candle backend) because the real
//! anchor checkpoint must be loaded from disk to exercise the builder chain.
//!
//! # Cross-references
//!
//! - `crates/strategy/src/tcn_overlay_momentum.rs:158-214` — `TcnSyncForecaster` struct + builders.
//! - `crates/strategy/src/tcn_overlay_momentum.rs:305-313` — `infer()` epsilon read.
//! - `spec/v25-tcn-threshold-tuning/decomp.md § D-AR-1.f` — explicit-arg contract.
//! - `spec/v25-tcn-threshold-tuning/decomp.md § D-AR-1.g` — `direction_epsilon` field.

use std::path::PathBuf;

use rust_decimal_macros::dec;
use strategy::{
    MomentumStrategy, TcnOverlayMomentumStrategy, tcn_overlay_momentum::TcnSyncForecaster,
};

/// Workspace root directory (2 levels above `crates/strategy`).
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must exist")
}

/// Load the canonical momentum config for test use.
///
/// Path resolved from workspace root.
fn load_base() -> MomentumStrategy {
    let root = workspace_root();
    let toml_path = root.join("config/strategies/top10_momentum_h1.toml");
    let cfg = strategy::CrossSectionalMomentumConfig::from_file(&toml_path)
        .expect("failed to load top10_momentum_h1.toml");
    MomentumStrategy::from_config(cfg, smol_str::SmolStr::new(toml_path.to_string_lossy()))
}

/// Change to workspace root so that `load_anchor` (which uses relative paths)
/// works from within the test binary.
fn set_cwd_to_workspace_root() {
    let root = workspace_root();
    std::env::set_current_dir(&root)
        .expect("must be able to set CWD to workspace root for checkpoint loading");
}

/// (1) Default `with_tcn_bs1` builder must produce `confidence_threshold == dec!(0.6)`.
///
/// Verifies the anchor-byte-safety invariant: the existing builder is
/// unmodified by the T-D-N1 field addition.
#[test]
fn test_default_bs1_confidence_threshold() {
    set_cwd_to_workspace_root();
    let base = load_base();
    let strategy = TcnOverlayMomentumStrategy::with_tcn_bs1(base)
        .expect("BS-1 checkpoint must be loadable in test environment");
    assert_eq!(
        strategy.confidence_threshold(),
        dec!(0.6),
        "with_tcn_bs1 must keep confidence_threshold = dec!(0.6); got {}",
        strategy.confidence_threshold()
    );
}

/// (2) Default `load_bs1` forecaster must have `direction_epsilon == None`.
///
/// Verifies that the `None` branch of `infer()` uses the const-fold-identical
/// `forecast::tcn::DIRECTION_EPSILON` path — 26 predecessor anchors stay byte-identical.
#[test]
fn test_default_bs1_direction_epsilon_is_none() {
    set_cwd_to_workspace_root();
    let f = TcnSyncForecaster::load_bs1()
        .expect("BS-1 checkpoint must be loadable in test environment");
    assert_eq!(
        f.direction_epsilon(),
        None,
        "load_bs1 must set direction_epsilon = None (const-fold path); got {:?}",
        f.direction_epsilon()
    );
}

/// (3) `with_tcn_bs1_tuned(τ, ε)` builder must forward the supplied τ as `confidence_threshold`.
///
/// Verifies the explicit-arg contract from D-AR-1.f: no default-arg overloading;
/// the supplied value is stored verbatim.
#[test]
fn test_tuned_bs1_confidence_threshold_forwarded() {
    set_cwd_to_workspace_root();
    let base = load_base();
    let tau = dec!(0.35);
    let eps = dec!(0.001);
    let strategy = TcnOverlayMomentumStrategy::with_tcn_bs1_tuned(base, tau, eps)
        .expect("BS-1 checkpoint must be loadable in test environment");
    assert_eq!(
        strategy.confidence_threshold(),
        tau,
        "with_tcn_bs1_tuned must set confidence_threshold = tau ({}); got {}",
        tau,
        strategy.confidence_threshold()
    );
}

/// (4) `load_bs1().with_direction_epsilon(ε)` must store `Some(ε_f32)`.
///
/// Verifies that the builder sets the field and that `to_f32()` conversion
/// is applied. Tests a non-default ε (not the baseline 0.0005).
#[test]
fn test_tuned_bs1_direction_epsilon_set() {
    set_cwd_to_workspace_root();
    use rust_decimal::prelude::ToPrimitive;

    let eps = dec!(0.002);
    let f = TcnSyncForecaster::load_bs1()
        .expect("BS-1 checkpoint must be loadable in test environment")
        .with_direction_epsilon(eps);

    let expected_f32 = eps.to_f32().expect("0.002 must convert to f32");
    assert_eq!(
        f.direction_epsilon(),
        Some(expected_f32),
        "with_direction_epsilon must set direction_epsilon = Some({expected_f32}); got {:?}",
        f.direction_epsilon()
    );
}

/// (5) Default `with_tcn_bs2` builder must keep `confidence_threshold == dec!(0.6)`.
///
/// Mirror of test (1) for the BS-2 checkpoint. Verifies that adding the
/// `direction_epsilon` field does NOT change the BS-2 default builder behaviour.
#[test]
fn test_default_bs2_confidence_threshold() {
    set_cwd_to_workspace_root();
    let base = load_base();
    let strategy = TcnOverlayMomentumStrategy::with_tcn_bs2(base)
        .expect("BS-2 checkpoint must be loadable in test environment");
    assert_eq!(
        strategy.confidence_threshold(),
        dec!(0.6),
        "with_tcn_bs2 must keep confidence_threshold = dec!(0.6); got {}",
        strategy.confidence_threshold()
    );
}
