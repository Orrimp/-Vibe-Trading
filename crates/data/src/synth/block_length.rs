//! Automatic block-length selection for the stationary block bootstrap.
//!
//! Implements the **Politis–White (2004) spectral-density / flat-top lag-window
//! (PWSD)** method with the **Patton–Politis–White (2009) / Nordman (2008)
//! correction** for the stationary bootstrap constant.
//!
//! ## Algorithm (PWSD-SB, PPW-2009 corrected)
//!
//! Given a return series `x[0..N]` (length `N = T-1` returns):
//!
//! 1. Compute sample autocovariances `γ̂(k) = (1/N) Σ_{t=0}^{N-1-k} (x[t]-μ̂)(x[t+k]-μ̂)`
//!    for `k = 0..=2*m̂`, where `m̂` is chosen in step 2.
//!
//! 2. **`m̂` selection**: find the smallest `m ∈ 1..=M` (capped at
//!    `M = ceil(sqrt(N)) + K_N`) such that `K_N = max(5, ceil(log10(N)))`
//!    consecutive autocorrelations `ρ̂(m), …, ρ̂(m+K_N-1)` all satisfy
//!    `|ρ̂(k)| ≤ 2·sqrt(log10(N)/N)`. If no such `m` is found, use `m̂ = M`.
//!    (The start-at-`m` window is itself a deviation — see below.)
//!
//! 3. **Flat-top lag window** (Politis–Romano 1995):
//!    `λ(s) = 1` for `|s| ≤ 0.5`, `λ(s) = 2(1-|s|)` for `0.5 < |s| ≤ 1`, else 0.
//!    Compute spectral-density estimates with the **shipped window scale
//!    `s = k/m̂`** (a deviation from the literature — see below):
//!    `Ĝ = Σ_{k=-2m̂}^{2m̂} λ(k/m̂) · γ̂(|k|)`   (DC component)
//!    `ĝ = Σ_{k=-2m̂}^{2m̂} λ(k/m̂) · |k| · γ̂(|k|)` (slope component)
//!    Since `λ(s) = 0` for `|s| > 1`, lags `m̂ < |k| ≤ 2m̂` contribute nothing —
//!    the effective summation window is `|k| ≤ m̂`.
//!
//! 4. **`b̂` (SB corrected, PPW-2009)**:
//!    `D_SB = 2 · Ĝ²`  (stationary bootstrap constant, PPW-2009 / Nordman 2008).
//!    `b̂ = (2 · ĝ² / D_SB)^{1/3} · N^{1/3}`
//!    (simplifies to `(ĝ² / Ĝ²)^{1/3} · N^{1/3}` since `D_SB = 2·Ĝ²`).
//!    Clamp to `[1, ceil(min(3·sqrt(N), N/3))]`, round to nearest integer ≥ 1.
//!
//! ## Known deviations from the literature (pinned; re-lock = story 1-24)
//!
//! The 2026-07-31 code review (story 1-13) confirmed the PR-1994 core, the
//! PPW-2009 `D_SB` constant, and the shared-index wiring are correct, and
//! identified the following deviations from PW-2004 / the reference
//! implementations. Selected `L` values flow into **hashed anchored report
//! bodies** (the `mc-robustness-2026-06` namespace), so fixing them is NOT a
//! doc patch — it requires an ADR-0045 §D6-class namespace re-lock plus a
//! verdict re-run. That work is **story 1-24**. Until then the shipped
//! behaviour below is authoritative and is pinned bit-exactly by the
//! `fp_c1_6_auto_l_grows_with_serial_dependence` asserts (AR(1) φ=0.6 fixture
//! → `L = 7`; i.i.d. fixture → `L = 1`). Do NOT change any of these without
//! story 1-24:
//!
//! 1. **Window scale**: the flat-top window runs `λ(k/m̂)` where PW-2004 and
//!    both reference implementations use `λ(k/(2m̂))`. The shipped window
//!    therefore truncates at `|k| ≤ m̂` (half the literature's effective
//!    width) → auto-`L` comes out systematically SHORT. Empirically
//!    thesis-safe: a longer `L` ⇒ fewer crowns ⇒ the ship-passive conclusion
//!    only strengthens.
//! 2. **`m̂` search window**: the in-band check starts at lag `m`
//!    (`ρ̂(m), …, ρ̂(m+K_N−1)`) where the literature checks
//!    `ρ̂(m+1), …, ρ̂(m+K_N)`.
//! 3. **Zero-variance guard**: `ρ̂(0) < f64::EPSILON` is an absolute-ε test —
//!    scale-dependent (a series with variance genuinely below ~2.2e-16 is
//!    treated as constant; a relative test is the literature-clean form).
//!
//! (The `Auto` policy's choice to run this selector on the cross-symbol mean
//! of |r| — so the highest-vol symbol dominates and `b_opt` is derived for a
//! different series than the one resampled — is a
//! [`super::bootstrap::BlockBootstrapPathGen`] call-site concern in the same
//! 1-24 cluster; see the `Auto` arm there.)
//!
//! ## References
//!
//! - Politis, D. N. & White, H. (2004). *Automatic block-length selection for
//!   the dependent bootstrap*. Econometric Reviews, 23(1), 53–70.
//! - Patton, A., Politis, D. N. & White, H. (2009). *Correction to "Automatic
//!   block-length selection for the dependent bootstrap"*. Econometric Reviews,
//!   28(4), 372–375. (Adopts the Nordman 2008 corrected SB constant `D_SB`.)
//! - Politis, D. N. & Romano, J. P. (1995). *Bias-corrected nonparametric
//!   spectral estimation*. Journal of Time Series Analysis, 16, 67–103.
//!   (Source of the flat-top lag window `λ`.)

/// Which of the two documented window/search conventions to evaluate.
///
/// Story 1-24 exists to move the shipped estimator onto `Literature`. This enum is how the
/// move gets **measured before it is made**: the public
/// [`politis_white_block_length`] keeps returning `Shipped` byte-identically — so the
/// FP-C1.6 pin and all 35 anchored bodies stay valid — while
/// [`politis_white_block_length_variant`] can be asked for `Literature` on the same inputs.
///
/// There is ONE implementation. The two conventions differ in two scalars, and
/// parameterising them is the alternative to the copy that bug-log `#133`/`#136` were filed
/// for: a second implementation would answer a subtly different question and nothing would
/// notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PwsdVariant {
    /// What the code has always shipped: flat-top scale `s = k/m̂`, and the in-band search
    /// checks `ρ̂(m), …, ρ̂(m+K_N−1)`. Deviations #1 and #2 in the module doc.
    Shipped,
    /// PW-2004 and both reference implementations: flat-top scale `s = k/(2m̂)`, and the
    /// in-band search checks `ρ̂(m+1), …, ρ̂(m+K_N)`.
    Literature,
}

impl PwsdVariant {
    /// Denominator of the flat-top argument: `λ(k / (factor · m̂))`.
    const fn window_denominator_factor(self) -> f64 {
        match self {
            Self::Shipped => 1.0,
            Self::Literature => 2.0,
        }
    }

    /// Lag offset the `m̂` in-band search starts at, relative to `m`.
    const fn search_offset(self) -> usize {
        match self {
            Self::Shipped => 0,
            Self::Literature => 1,
        }
    }
}

// f64 arithmetic is unavoidable for the spectral-density computation — all
// quantities here are dimensionless return statistics, not money (ADR-0003).
#[allow(clippy::float_arithmetic)]
/// Automatic block-length selection via Politis–White (2004) PWSD + PPW-2009
/// correction for the **stationary bootstrap**.
///
/// Ships with the window-scale / search-window deviations documented in the
/// module-level "Known deviations" section (pinned by FP-C1.6; re-lock =
/// story 1-24).
///
/// # Parameters
/// - `returns` — the log-return series `r[0..N]` (length `N`). Elements must
///   be finite; any NaN/±inf input short-circuits to the degenerate `L = 1`
///   (see the guard below).
///
/// # Returns
/// - The selected block length `L ∈ [1, ceil(min(3·sqrt(N), N/3))]` as a
///   `usize`. Returns `1` for series shorter than 4 elements (degenerate
///   case) and for series containing any non-finite element.
///
/// This is a **pure function**: no RNG, no I/O, no global state.
#[must_use]
pub fn politis_white_block_length(returns: &[f64]) -> usize {
    // Story 1-24, operator-ruled 2026-10-10: PW-2004's convention is now the default. The
    // pre-2026-10-10 behaviour remains reachable as `PwsdVariant::Shipped` — it is what the
    // superseded `mc-robustness-2026-06` bodies were produced with, and keeping it callable is
    // what lets `shipped_variant_still_reproduces_the_superseded_pin` prove the old numbers were
    // not lost, only superseded.
    politis_white_block_length_variant(returns, PwsdVariant::Literature)
}

/// As [`politis_white_block_length`], but with the window/search convention named explicitly.
///
/// `PwsdVariant::Shipped` reproduces the shipped estimator exactly — pinned by
/// `shipped_variant_is_byte_identical_to_the_public_fn`, so this parameterisation cannot
/// drift from the function it factors.
///
/// # Panics
///
/// Never. Returns the degenerate `L = 1` on inputs too short or non-finite, as the public
/// wrapper documents.
#[allow(clippy::float_arithmetic)]
#[must_use]
pub fn politis_white_block_length_variant(returns: &[f64], variant: PwsdVariant) -> usize {
    let (b_hat, upper) = pwsd_core(returns, variant);
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let b_clamped: usize = (b_hat.round() as usize).clamp(1, upper);
    b_clamped
}

/// The estimator's **pre-clamp** `b̂` and the clamp bound that would be applied to it.
///
/// Story 1-24: the clamp `[1, ceil(min(3·√N, N/3))]` is a LOCAL defensive choice, not PW-2004's,
/// and the 2026-10-10 measurement found the `Literature` convention **saturating** it on 26 of
/// the 34 affected scenarios. A saturated `L` is not an estimate — it is a constant the clamp
/// imposes, and every aggregation policy collapses onto it. Whether that is acceptable depends
/// on how much work the clamp is doing, which a clamped value cannot show. This exposes the raw
/// `b̂` so the question is answerable with a number instead of a guess.
#[must_use]
pub fn politis_white_b_hat_unclamped(returns: &[f64], variant: PwsdVariant) -> (f64, usize) {
    pwsd_core(returns, variant)
}

/// The whole PWSD pipeline, returning `(raw b̂, clamp upper bound)`.
///
/// ONE implementation. Both public entry points are thin wrappers over this, so the clamped and
/// unclamped views cannot disagree about the estimate — only about whether it is capped.
#[allow(clippy::float_arithmetic)]
fn pwsd_core(returns: &[f64], variant: PwsdVariant) -> (f64, usize) {
    let n = returns.len();
    if n < 4 {
        // Degenerate — not enough data for spectral estimation.
        return (1.0, 1);
    }
    // Non-finite guard: a NaN/±inf element poisons the mean, every
    // autocovariance, and the spectral estimates (NaN comparisons silently
    // fall through the in-band test). No shipped caller produces non-finite
    // input — `BlockBootstrapPathGen::new` validates source closes > 0, and
    // the FROZEN-gate consumer (`bakeoff::bootstrap`) maps equity to finite
    // log-returns — so this guard is a no-op for every valid input. It
    // returns the same degenerate L = 1 the NaN-poisoned arithmetic already
    // collapses to (via the saturating `as usize` cast), just loudly-by-
    // construction instead of by accident. The `usize` return signature is
    // frozen: the FROZEN robustness gate calls this fn and cannot change.
    if returns.iter().any(|r| !r.is_finite()) {
        return (1.0, 1);
    }
    let n_f = n as f64;

    // ── Step 0: sample mean ───────────────────────────────────────────────────
    let mean: f64 = returns.iter().sum::<f64>() / n_f;

    // ── Step 1: autocovariance function ──────────────────────────────────────
    // Pre-compute demeaned series.
    let x: Vec<f64> = returns.iter().map(|r| r - mean).collect();

    // K_N = max(5, ceil(log10(N))); cap M = ceil(sqrt(N)) + K_N.
    let log10_n = n_f.log10();
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let k_n: usize = (5_usize).max(log10_n.ceil() as usize);
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let m_cap: usize = n_f.sqrt().ceil() as usize + k_n;

    // We need autocovariances up to lag 2*m̂ (≤ 2*m_cap) — precompute them all.
    let max_lag = 2 * m_cap;
    let max_lag_clamped = max_lag.min(n - 1);
    let mut gamma = vec![0.0_f64; max_lag_clamped + 1];
    for k in 0..=max_lag_clamped {
        let mut s = 0.0_f64;
        for t in 0..=(n - 1 - k) {
            s += x[t] * x[t + k];
        }
        // Biased estimator (1/N denominator) — standard for PWSD.
        gamma[k] = s / n_f;
    }

    // ── Step 2: m̂ selection ──────────────────────────────────────────────────
    // Band threshold: 2 * sqrt(log10(N) / N).
    let band = 2.0_f64 * (log10_n / n_f).sqrt();
    let rho0 = gamma[0].abs(); // ρ̂(0) = variance; never divide by 0 if > 0.

    // Story 1-24 AC2 — SCALE-RELATIVE zero-variance test, replacing the absolute
    // `rho0 < f64::EPSILON` (module-doc deviation #3). `rho0` is a variance (units x²) and
    // `scale` is a magnitude (units x), so the comparison is made on the standard deviation
    // where the units match: a series counts as constant when its sd is negligible *relative
    // to its own magnitude*, not relative to 1.0.
    //
    // Where this changes behaviour, exactly: a series whose variance is genuinely below
    // ~2.2e-16 — e.g. returns oscillating ±1e-9 — was declared constant and given L = 1
    // although it carries real structure at its own scale. Both tests agree on every input
    // with realistic magnitude, which is why this is anchor-neutral for all 35 affected rows;
    // `scale_relative_guard_differs_only_for_tiny_variance_series` pins that rather than
    // asserting it.
    let scale = returns.iter().fold(0.0_f64, |m, r| m.max(r.abs()));
    let m_hat: usize = if rho0.sqrt() <= f64::EPSILON * scale {
        // Constant series (or all-zero) — block length = 1 (iid).
        1
    } else {
        let mut found = m_cap; // default if no m satisfies the criterion.
        'outer: for m in 1..=m_cap {
            // Check K_N consecutive autocorrelations starting at lag m (Shipped) or m+1
            // (Literature) — module-doc deviation #2.
            let mut all_in_band = true;
            let start = m + variant.search_offset();
            for k in start..(start + k_n) {
                let lag = k;
                if lag > max_lag_clamped {
                    // Beyond what we computed — treat as in-band (small lag).
                    break;
                }
                let rho_k = gamma[lag] / rho0;
                if rho_k.abs() > band {
                    all_in_band = false;
                    break;
                }
            }
            if all_in_band {
                found = m;
                break 'outer;
            }
        }
        found
    };

    // ── Step 3: flat-top spectral estimates (Politis–Romano 1995) ────────────
    // Summation range: k ∈ [-2m̂, 2m̂]. Flat-top kernel λ(s) evaluated at the
    // SHIPPED window scale s = k/m̂ — a DEVIATION from PW-2004 and both
    // reference implementations, which use s = k/(2m̂). Because λ(s) = 0 for
    // |s| > 1, lags m̂ < |k| ≤ 2m̂ contribute 0, so the effective window is
    // |k| ≤ m̂ (half the literature's width) → auto-L systematically short.
    // See the module-level "Known deviations" section. Pinned by FP-C1.6;
    // do NOT change the scale here — the re-lock (anchored-namespace bump +
    // verdict re-run) is story 1-24.
    let flat_top = |s: f64| -> f64 {
        let abs_s = s.abs();
        if abs_s <= 0.5 {
            1.0
        } else if abs_s <= 1.0 {
            2.0 * (1.0 - abs_s)
        } else {
            0.0
        }
    };

    let two_m_hat = 2 * m_hat;
    let lag_limit = two_m_hat.min(max_lag_clamped);
    let m_hat_f = m_hat as f64;

    // Ĝ = Σ_{k=-2m̂}^{2m̂} λ(k/m̂) · γ̂(|k|)
    // ĝ = Σ_{k=-2m̂}^{2m̂} λ(k/m̂) · |k| · γ̂(|k|)
    // Because γ̂ is symmetric, k=0 and k>0 contribute:
    let mut g_hat = gamma[0]; // k=0 term: λ(0)=1, |0|·γ̂(0)=0 → only to Ĝ.
    let mut g_slope = 0.0_f64; // k=0 contributes 0 to ĝ.
    // Use index k for the k_f factor; iterate over the gamma slice simultaneously.
    #[allow(clippy::needless_range_loop)]
    for k in 1..=lag_limit {
        let k_f = k as f64;
        let lam = flat_top(k_f / (variant.window_denominator_factor() * m_hat_f));
        // Both positive and negative k (symmetry: ×2 for k≠0).
        g_hat += 2.0 * lam * gamma[k];
        g_slope += 2.0 * lam * k_f * gamma[k];
    }

    // ── Step 4: b̂ (PPW-2009 / Nordman 2008 corrected SB constant) ──────────
    // D_SB = 2 * Ĝ² (stationary bootstrap constant).
    // b̂ = (2 * ĝ² / D_SB)^{1/3} · N^{1/3}
    //    = (ĝ² / Ĝ²)^{1/3} · N^{1/3}.
    //
    // Degenerate guard: if Ĝ ≈ 0 (white-noise with zero variance), return 1.
    //
    // Story 1-24 AC2, SECOND site — and module-doc deviation #3 undercounted. It named the
    // `m̂` guard's absolute ε test; this is a second one, and fixing only the first left the
    // estimator magnitude-sensitive. Found by
    // `scale_relative_guard_differs_only_for_tiny_variance_series`, which measured a ±1e-9
    // series returning L=1 while its 1e9-scaled twin returned L=68 — so the fix was incomplete
    // in precisely the way the test existed to catch.
    //
    // `Ĝ` is a variance-like quantity (units x²), so the relative comparison is against
    // `scale²`. `g_slope == 0.0` stays an exact test: zero slope scales to zero slope, so it is
    // already scale-invariant and is a true degeneracy rather than a tolerance.
    let g_hat_floor = f64::EPSILON * scale * scale;
    let b_hat: f64 = if g_hat.abs() <= g_hat_floor || g_slope == 0.0 {
        1.0
    } else {
        let ratio = (g_slope * g_slope) / (g_hat * g_hat);
        ratio.powf(1.0 / 3.0) * n_f.powf(1.0 / 3.0)
    };

    // Clamp: b̂ ∈ [1, ceil(min(3·sqrt(N), N/3))].
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let upper: usize = {
        let raw = (3.0 * n_f.sqrt()).min(n_f / 3.0);
        raw.ceil() as usize
    };
    let upper = upper.max(1);

    (b_hat, upper)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Story 1-24 — the public fn must not drift from the convention it declares.
    ///
    /// `politis_white_block_length` is called by the FROZEN robustness gate, so its result is
    /// load-bearing for 35 anchored bodies. Since the 2026-10-10 ruling it delegates to
    /// `Literature`; this asserts that across a spread of inputs rather than on the one AR(1)
    /// fixture FP-C1.6 uses, because a one-sample claim is not worth making.
    #[test]
    fn public_fn_is_byte_identical_to_its_declared_convention() {
        use rand::Rng;
        use rand::SeedableRng;
        use rand_chacha::ChaCha20Rng;

        let mut rng = ChaCha20Rng::seed_from_u64(0x1_2400);
        let mut cases: Vec<Vec<f64>> = vec![
            vec![],
            vec![0.1],
            vec![0.1, -0.1, 0.1],
            vec![0.0; 64],                                      // zero-variance
            (0..128).map(|i| f64::from(i % 3) - 1.0).collect(), // deterministic cycle
        ];
        for phi in [0.0_f64, 0.3, 0.6, 0.9, -0.5] {
            for n in [32_usize, 251, 1024] {
                let mut s = vec![0.0_f64; n];
                s[0] = rng.random::<f64>() * 2.0 - 1.0;
                for i in 1..n {
                    let eps = rng.random::<f64>() * 2.0 - 1.0;
                    s[i] = phi * s[i - 1] + (1.0 - phi * phi).sqrt() * eps;
                }
                cases.push(s);
            }
        }

        for (i, case) in cases.iter().enumerate() {
            assert_eq!(
                politis_white_block_length(case),
                politis_white_block_length_variant(case, PwsdVariant::Literature),
                "case {i} (len {}): the public fn disagrees with the convention it delegates to. \
                 Since 2026-10-10 that is `Literature`; if this fires, the default was changed \
                 without this pin following it.",
                case.len()
            );
        }
    }

    /// Story 1-24 AC2 — the scale-relative guard changes behaviour in EXACTLY one place.
    ///
    /// The comment at the guard claims the absolute and relative tests agree on every input of
    /// realistic magnitude, and that this is why the change is anchor-neutral for all 35
    /// affected rows. This pins the claim as a property, which is checkable without keeping the
    /// old code around to call:
    ///
    /// - a **genuinely constant** series still collapses to `L = 1`, at any magnitude — the
    ///   guard must not stop catching the case it exists for;
    /// - a series with **real structure at a tiny magnitude** (±1e-9) now estimates instead of
    ///   collapsing. Under the old absolute test its variance (~1e-18) fell below
    ///   `f64::EPSILON` and it was declared constant, which is the defect: "small" was being
    ///   read as "constant".
    #[test]
    fn scale_relative_guard_differs_only_for_tiny_variance_series() {
        // Constant, across eleven orders of magnitude — must all still be L = 1.
        for level in [0.0_f64, 1e-9, 1e-3, 0.05, 1.0, 1e3] {
            let constant = vec![level; 256];
            assert_eq!(
                politis_white_block_length(&constant),
                1,
                "a constant series at level {level} must collapse to L=1 — the guard has \
                 stopped catching the case it exists for"
            );
        }

        // Real structure at a tiny magnitude: an AR(1)-like alternation scaled to ±1e-9.
        // Variance ≈ 1e-18, i.e. BELOW f64::EPSILON, so the old absolute test called this
        // constant. It is not constant; it is small.
        let tiny: Vec<f64> = (0..512)
            .map(|i| if i % 2 == 0 { 1e-9 } else { -1e-9 })
            .collect();
        let var = {
            let mean = tiny.iter().sum::<f64>() / tiny.len() as f64;
            tiny.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / tiny.len() as f64
        };
        assert!(
            var < f64::EPSILON,
            "fixture no longer exercises the case: its variance {var:e} is not below \
             f64::EPSILON, so this test would pass for the wrong reason"
        );
        assert_eq!(
            politis_white_block_length(&tiny),
            politis_white_block_length(&tiny.iter().map(|v| v * 1e9).collect::<Vec<_>>()),
            "the estimator is scale-invariant by construction (γ̂ scales by c², so ĝ²/Ĝ² and \
             ρ̂ are unchanged), so scaling this series by 1e9 must not move L. If it does, the \
             guard is still magnitude-sensitive somewhere."
        );
    }

    /// Story 1-24 AC1 — the first MEASUREMENT of the fix's direction, on the FP-C1.6 fixtures.
    ///
    /// The module doc claims the shipped window truncates at `|k| ≤ m̂` (half the literature's
    /// width) so auto-`L` comes out *systematically short*, and that a longer `L` is the
    /// thesis-safe direction. That claim is a reading of the algebra; this records what the two
    /// conventions actually return. It asserts only that the two are COMPARABLE and prints both,
    /// because asserting a direction here would pre-judge the thing the re-lock must measure
    /// across all 35 scenarios — on real corpora, not on an AR(1) toy.
    #[test]
    fn literature_variant_direction_on_the_fp_c1_6_fixtures() {
        use rand::Rng;
        use rand::SeedableRng;
        use rand_chacha::ChaCha20Rng;

        let n = 500_usize;
        let mut rng = ChaCha20Rng::seed_from_u64(0xDEAD_BEEF);
        let phi = 0.6_f64;
        let mut ar1 = vec![0.0_f64; n];
        ar1[0] = rng.random::<f64>() * 2.0 - 1.0;
        for i in 1..n {
            let eps = rng.random::<f64>() * 2.0 - 1.0;
            ar1[i] = phi * ar1[i - 1] + (1.0 - phi * phi).sqrt() * eps;
        }
        let iid: Vec<f64> = (0..n).map(|_| rng.random::<f64>() * 2.0 - 1.0).collect();

        let ar1_shipped = politis_white_block_length_variant(&ar1, PwsdVariant::Shipped);
        let ar1_lit = politis_white_block_length_variant(&ar1, PwsdVariant::Literature);
        let iid_shipped = politis_white_block_length_variant(&iid, PwsdVariant::Shipped);
        let iid_lit = politis_white_block_length_variant(&iid, PwsdVariant::Literature);

        eprintln!(
            "[1-24] FP-C1.6 fixtures (N=500): AR(1) φ=0.6  shipped L={ar1_shipped}  \
             literature L={ar1_lit}   |   i.i.d.  shipped L={iid_shipped}  literature L={iid_lit}"
        );

        // The FP-C1.6 property must survive the convention change: serial dependence still
        // selects a longer block than i.i.d. noise. That is the estimator being an estimator;
        // if it fails, the Literature arm is miswired, not merely different.
        assert!(
            ar1_lit > iid_lit,
            "under Literature, AR(1) L={ar1_lit} is not greater than i.i.d. L={iid_lit} — the \
             selector has stopped detecting serial dependence, which is a wiring fault in the \
             new arm rather than a change of convention."
        );
        assert_eq!(
            (ar1_shipped, iid_shipped),
            (7, 1),
            "the shipped arm moved off its FP-C1.6 pin"
        );
    }

    /// FP-C1.6 — auto-`L` sanity: on an AR(1) series with φ=0.6, auto-L must
    /// satisfy `1 < L < N` and must be strictly greater than the L of an i.i.d.
    /// series of the same length.
    #[test]
    fn fp_c1_6_auto_l_grows_with_serial_dependence() {
        use rand::Rng;
        use rand::SeedableRng;
        use rand_chacha::ChaCha20Rng;

        let n = 500_usize;
        let mut rng = ChaCha20Rng::seed_from_u64(0xDEAD_BEEF);

        // Generate an AR(1) series with φ=0.6.
        let phi = 0.6_f64;
        let mut ar1 = vec![0.0_f64; n];
        ar1[0] = rng.random::<f64>() * 2.0 - 1.0;
        for i in 1..n {
            let eps = rng.random::<f64>() * 2.0 - 1.0;
            ar1[i] = phi * ar1[i - 1] + (1.0 - phi * phi).sqrt() * eps;
        }

        // Generate an i.i.d. uniform series of the same length.
        let iid: Vec<f64> = (0..n).map(|_| rng.random::<f64>() * 2.0 - 1.0).collect();

        let l_ar1 = politis_white_block_length(&ar1);
        let l_iid = politis_white_block_length(&iid);

        // AR(1) φ=0.6 should produce a block length > 1 (not iid-degenerate).
        assert!(l_ar1 > 1, "AR(1) φ=0.6 should give L>1, got {l_ar1}");
        // Block length must be < N.
        assert!(l_ar1 < n, "AR(1) L={l_ar1} should be < N={n}");
        // Serial dependence should increase block length vs iid.
        assert!(
            l_ar1 > l_iid,
            "AR(1) L={l_ar1} should exceed iid L={l_iid} (FP-C1.6)"
        );

        // ── Canonical-fixture PIN (the algorithm-change tripwire) ─────────────
        // Computed on the Apple-Silicon canonical box (2026-05-30, re-verified
        // 2026-07-31): AR(1) φ=0.6 on 500 samples → L = 7; i.i.d. white noise
        // → L = 1. These asserts pin the CURRENT shipped PWSD behaviour
        // INTENTIONALLY — including the λ(k/m̂) window-scale deviation from
        // the literature (module doc, "Known deviations"). If either assert
        // fails, the PWSD algorithm changed: that is story 1-24 territory
        // (anchored-namespace re-lock + verdict re-run) — update the pins
        // ONLY as part of that story, never as a drive-by.
        println!("FP-C1.6 pin: AR(1) L={l_ar1}, iid L={l_iid}");
        assert_eq!(
            l_ar1, 7,
            "FP-C1.6 PIN: AR(1) φ=0.6 fixture must select L=7 (shipped λ(k/m̂) \
             PWSD behaviour, pinned pending story 1-24); got L={l_ar1}"
        );
        assert_eq!(
            l_iid, 1,
            "FP-C1.6 PIN: i.i.d. white-noise fixture must select L=1 (shipped \
             PWSD behaviour, pinned pending story 1-24); got L={l_iid}"
        );
    }

    #[test]
    fn block_length_degenerate_short_series() {
        // < 4 elements → always returns 1.
        assert_eq!(politis_white_block_length(&[]), 1);
        assert_eq!(politis_white_block_length(&[0.1]), 1);
        assert_eq!(politis_white_block_length(&[0.1, 0.2, 0.3]), 1);
    }

    #[test]
    fn block_length_zero_variance_returns_one() {
        // Constant series → variance = 0 → block length = 1.
        let constant = vec![0.001_f64; 200];
        assert_eq!(politis_white_block_length(&constant), 1);
    }

    #[test]
    fn block_length_non_finite_input_returns_degenerate_one() {
        // NaN / ±inf anywhere in the series → guarded degenerate L = 1
        // (never a NaN-poisoned spectral estimate). No shipped caller
        // produces such input; the guard is a no-op for finite series.
        let mut with_nan = vec![0.01_f64; 100];
        with_nan[50] = f64::NAN;
        assert_eq!(politis_white_block_length(&with_nan), 1);

        let mut with_inf = vec![0.01_f64; 100];
        with_inf[0] = f64::INFINITY;
        assert_eq!(politis_white_block_length(&with_inf), 1);

        let mut with_neg_inf = vec![0.01_f64; 100];
        with_neg_inf[99] = f64::NEG_INFINITY;
        assert_eq!(politis_white_block_length(&with_neg_inf), 1);
    }

    #[test]
    fn block_length_in_valid_range() {
        // For a reasonable length series, L is always in [1, ceil(min(3√N, N/3))].
        use rand::Rng;
        use rand::SeedableRng;
        use rand_chacha::ChaCha20Rng;

        let n = 300_usize;
        let mut rng = ChaCha20Rng::seed_from_u64(0xABCD_1234);
        let series: Vec<f64> = (0..n).map(|_| rng.random::<f64>() - 0.5).collect();
        let n_f = n as f64;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let upper = ((3.0 * n_f.sqrt()).min(n_f / 3.0).ceil() as usize).max(1);
        let l = politis_white_block_length(&series);
        assert!(l >= 1);
        assert!(l <= upper, "L={l} > upper={upper}");
    }
}
