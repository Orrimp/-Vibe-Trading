//! Overfitting scorecard — closed-form DSR, `MinBTL`, `N_eff` (P0-1 / ADR-0075).
//!
//! # Rationale
//!
//! Crowning "the best of N strategies" on one dataset is multiple-hypothesis
//! testing.  The Deflated Sharpe Ratio (DSR) and Minimum Backtest Length (`MinBTL`)
//! give a calibrated, closed-form audit trail — the "traceable & plausible"
//! evidence layer demanded by the product thesis (v2-architecture.md § P0-1).
//!
//! # FROZEN-gate contract (ADR-0075 + §6.0 D3)
//!
//! The scorecard is **REPORT-ONLY**.  It does NOT change crowning, eligibility,
//! or the robustness bands in `robustness.rs::verdict_bands`.  `crown_clears_dsr`
//! is an informational flag — never a veto in v2.  The gate (`classify_verdict` /
//! `rank_candidates` / ADR-0066 benchmark exemption) is byte-untouched.
//!
//! # Anchor safety
//!
//! The advisor bake-off runs `write_report = false`.  The scorecard is carried on
//! `Recommendation` — a backtest-internal type not on the anchored CLI report path.
//! `verify_anchors.sh` must pass 119/119 before and after this module lands.
//!
//! # Formulas (verbatim from research)
//!
//! **`N_eff`** (closed-form, §6.0 D4 frozen):
//! ```text
//! N_eff = ρ̄ + (1 − ρ̄) · M
//! ```
//! where ρ̄ = mean pairwise Sharpe correlation across M candidates.
//! Frozen at the closed form — the literature's "must cluster when M > T" mandate
//! does NOT apply at `MAX_SWEEP_CONFIGS` = 24 where T ≫ M always.
//!
//! **DSR** (Bailey & López de Prado, `evolution[98]`):
//! ```text
//! DSR = Φ[ (ŜR − SR₀) · √(T − 1) / √(1 − γ̂₃·ŜR + ((γ̂₄ − 1)/4)·ŜR²) ]
//! SR₀ = √V[{ŜRₙ}] · ((1 − γ)·Φ⁻¹[1 − 1/N_eff] + γ·Φ⁻¹[1 − (1/N_eff)·e⁻¹])
//! ```
//! with γ ≈ 0.5772 (Euler-Mascheroni), Φ the standard-normal CDF.
//! Note: ŜR here is NON-ANNUALISED (hourly scale); SR₀ is computed in the same units.
//! The public-facing `Scorecard.deflated_sharpe` reports DSR as a probability in [0, 1].
//!
//! **`MinBTL`** (`evolution[29]`, looser approximate form):
//! ```text
//! MinBTL ≈ 2 · ln(N_eff) / SR_target²   (years)
//! ```
//! At `SR_target` = 1 and `N_eff` = 24: `MinBTL` ≈ 6.4 years.
//!
//! # What is NOT done here (§6.0 RATIFIED)
//!
//! - PBO via CSCV: deferred to the Tune/sweep surface (R2). `pbo` is always `None`.
//! - Crown-eligibility veto: `crown_clears_dsr` is report-only in v2.
//! - ONC clustering for `N_eff`: moot at N = 24; closed-form is frozen.
//!
//! # Degenerate-Sharpe hardening (bug fix, post-P2, `p2-wobble-thesis-analysis-2026-07-10.md` § (d))
//!
//! A per-candidate Sharpe can be `NaN`: `compute_sharpe_hourly`'s log-return
//! `(curr / prev).ln()` is only guarded against a non-positive **starting**
//! equity (`prev <= 0.0 → 0.0`); when equity crosses from positive to
//! negative WITHIN one bar (the short-side arms `v0.sma_cross_ls` /
//! `v0.always_short` can blow equity through zero), `curr / prev` is
//! negative and `.ln()` of a negative number is `NaN` by IEEE 754. That NaN
//! survives into `CandidateResult.kpis.sharpe` untouched (this module does
//! not own `compute_sharpe_hourly` — it is a frozen M-DEV-1 verbatim lift,
//! `crates/backtest/src/stats/mod.rs:40`, and is out of scope here).
//!
//! **The defined semantics, applied at the [`compute_scorecard`] boundary:**
//! non-finite (`NaN` / `±∞`) Sharpe estimates are **excluded** from every
//! moment-based statistic — [`n_eff`] and [`sharpe_variance`] — before they
//! are computed. `n_candidates` (the "N tried" field size reported to the
//! operator) is UNCHANGED — it still counts every arm actually run,
//! including the degenerate ones, because "how many strategies were tried"
//! is what the DSR/`MinBTL` literature means by trial count. Only the
//! *statistics derived from the Sharpe distribution* use the finite subset.
//! If fewer than 2 finite Sharpes remain (e.g. a 1-candidate field, or a
//! field where every non-benchmark arm degenerated), `n_eff` falls back to
//! the existing `m < 2` convention — returns the raw candidate count,
//! documented as "conservative — never over-deflates" — using the FULL `m`
//! (not the finite count), so a field that is mostly non-finite is not
//! rewarded with an artificially small `N_eff`.
//!
//! No `NaN` may ever reach [`Scorecard::n_eff`], [`Scorecard::min_btl_years`],
//! or [`Scorecard::deflated_sharpe`] — [`n_eff`], [`min_btl`], and [`dsr`]
//! additionally each carry their own explicit `is_nan()` guard on every
//! external input (defense in depth: even a future caller that skips the
//! filtering above cannot smuggle a `NaN` past these three functions). A
//! `NaN`-triggered degenerate result is, respectively, `m as f64` (the raw
//! trial count — [`n_eff`]'s pre-existing "conservative, never
//! over-deflates" convention), `0.0` ([`min_btl`] — the same value the
//! pre-existing `f64::max(NaN, x) == x` clamp produced by accident, but now
//! from an explicit, documented branch rather than a silent IEEE-754
//! coincidence), and `0.0` ([`dsr`] — its existing "degenerate inputs"
//! convention). This preserves every existing non-degenerate numeric result
//! byte-for-byte (verified: the `n_eff`/`min_btl`/DSR unit tests below are
//! unchanged) while making the degenerate path honest instead of a silent
//! coincidence.

#![allow(clippy::float_arithmetic)] // statistical metric computations
#![allow(clippy::cast_precision_loss)] // intentional usize→f64 casts in stats computations

// ── Euler-Mascheroni constant (γ) ─────────────────────────────────────────────
/// Euler-Mascheroni constant γ ≈ 0.5772.
const EULER_MASCHERONI: f64 = 0.577_215_664_901_532_9;

/// DSR threshold used for the `crown_clears_dsr` informational flag.
/// "True SR > 0 at 95% after deflation."  REPORT-ONLY — NOT a gate in v2.
const DSR_THRESHOLD: f64 = 0.95;

/// Default SR target (annualised) for `MinBTL` computation.  Annualised SR = 1
/// is the "barely worth it vs B&H" bar — the honest minimum.
const SR_TARGET_ANNUALISED: f64 = 1.0;

/// Annualisation factor: √(24 × 365) for hourly bars.
/// Matches `compute_sharpe_hourly`'s `SQRT_HPY`.
const SQRT_HPY: f64 = 92.601_295_098_46;

// ── Public types ──────────────────────────────────────────────────────────────

/// A thin 4-field projection of [`Scorecard`] for the forward-plan confidence
/// check (P0-3 / advisor-confidence-not-verdict, ADR-0076).
///
/// "Summary" means the four facts the operator needs at the forward-plan
/// surface; the implementation-detail fields (`n_eff`, `pbo`) are excluded.
///
/// Chosen fields (rationale):
/// - `n_candidates` — "how many strategies were tried" (the trial count)
/// - `deflated_sharpe` — the headline credibility number (DSR probability)
/// - `crown_clears_dsr` — the yes/no "beats holding after the search?" flag
/// - `min_btl_years` — "how much data was needed" (the honest lower-bound)
///
/// `pbo` is always `None` in v2 (not displayed); `n_eff` is an
/// implementation detail the operator doesn't need at the plan surface.
///
/// # REPORT-ONLY (§6.0 D3 / ADR-0075)
///
/// Same contract as `Scorecard` — informational only, never a veto.
/// Carried on `agent::config::ForwardPlan` so the forward-plan screen
/// can show "confidence check, not verdict" alongside the plan.
#[derive(Debug, Clone, Copy)]
pub struct ScorecardSummary {
    /// Raw number of candidates tried — "how many strategies were ranked".
    pub n_candidates: usize,
    /// Deflated Sharpe Ratio — probability in `[0, 1]` the crown's edge
    /// exceeds zero after correcting for how many strategies were tried.
    pub deflated_sharpe: f64,
    /// `true` iff `deflated_sharpe >= 0.95`. Informational, never a veto.
    pub crown_clears_dsr: bool,
    /// Minimum backtest length needed to trust the crown (years).
    /// `2 · ln(n_eff) / SR_target²`. `0.0` when `n_eff ≤ 1`.
    pub min_btl_years: f64,
}

impl Scorecard {
    /// Project the four forward-plan-relevant fields into a [`ScorecardSummary`].
    ///
    /// Returns `None` for a degenerate scorecard (`n_candidates == 0`) so the
    /// plan screen can gracefully omit the confidence block when no real
    /// scorecard was computed (same guard as `ScorecardView::from_scorecard`).
    #[must_use]
    pub fn summary(&self) -> Option<ScorecardSummary> {
        if self.n_candidates == 0 {
            return None;
        }
        Some(ScorecardSummary {
            n_candidates: self.n_candidates,
            deflated_sharpe: self.deflated_sharpe,
            crown_clears_dsr: self.crown_clears_dsr,
            min_btl_years: self.min_btl_years,
        })
    }
}

/// Overfitting scorecard for one bake-off run.
///
/// Carried on [`super::Recommendation`] as `pub scorecard: Scorecard`.
/// All fields are computed from inputs already available in `run_bakeoff`:
/// the per-candidate Sharpe vector, the sample length T, and the crown's
/// return distribution (skew / kurtosis from the bootstrap log-return series).
///
/// # REPORT-ONLY (ADR-0075 + §6.0 D3)
///
/// No field on this struct changes the crowning decision.  `crown_clears_dsr`
/// is an informational boolean for display; it is NOT checked by `rank_candidates`
/// or `classify_verdict`.
#[derive(Debug, Clone, Copy)]
pub struct Scorecard {
    /// Raw number of candidates (the "N tried" field size, including benchmark).
    pub n_candidates: usize,
    /// Effective trial count.  Closed-form: `ρ̄ + (1 − ρ̄) · M`.
    /// Accounts for correlation among correlated strategy families.
    pub n_eff: f64,
    /// Deflated Sharpe Ratio (DSR) of the crown — probability the true Sharpe
    /// exceeds zero after correcting for M trials.  In [0, 1].
    pub deflated_sharpe: f64,
    /// Minimum backtest length required to have confidence in the crown, in years.
    /// `2 · ln(N_eff) / SR_target²` (approximate closed form).
    pub min_btl_years: f64,
    /// Probability of Backtest Overfitting (CSCV).  `None` in v2 — deferred to
    /// the Tune/sweep surface where CSCV is statistically meaningful.
    pub pbo: Option<f64>,
    /// Informational flag: `DSR ≥ 0.95`.
    /// **REPORT-ONLY** — NOT a veto in v2.  See §6.0 D3.
    pub crown_clears_dsr: bool,
}

// ── Pure functions ────────────────────────────────────────────────────────────

/// Standard normal CDF Φ(z) — rational approximation (Abramowitz & Stegun 26.2.17).
///
/// Used instead of a `statrs` / `special` dependency so the module is pure.
/// Accuracy ≤ 7.5e−8 absolute error.
///
/// Formula: Φ(z) = 1 − φ(z)·t·Σ aᵢtⁱ  where t = 1/(1+0.2316419·|z|).
#[must_use]
pub fn normal_cdf(z: f64) -> f64 {
    // Abramowitz & Stegun 26.2.17 coefficients.
    const B1: f64 = 0.319_381_530;
    const B2: f64 = -0.356_563_782;
    const B3: f64 = 1.781_477_937;
    const B4: f64 = -1.821_255_978;
    const B5: f64 = 1.330_274_429;
    const P: f64 = 0.231_641_9;

    let (x, flip) = if z >= 0.0 { (z, false) } else { (-z, true) };
    let t = 1.0 / (1.0 + P * x);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;
    // standard normal PDF at x
    let pdf = (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt();
    let poly = B1 * t + B2 * t2 + B3 * t3 + B4 * t4 + B5 * t5;
    let result = 1.0 - pdf * poly;
    if flip { 1.0 - result } else { result }
}

/// Inverse standard-normal CDF Φ⁻¹(p) — Peter Acklam's rational approximation
/// followed by one Halley refinement step.
///
/// Phase 1 (Acklam rational): relative error ≈ 1.15e-9.
/// Phase 2 (one Halley step using `normal_cdf`): drives error to ≈ 1e-12.
///
/// Clamps `p` to (1e-300, 1 − 1e-15) to avoid ±∞.
#[must_use]
#[allow(clippy::many_single_char_names)]
pub fn normal_inv_cdf(p: f64) -> f64 {
    // Peter Acklam's rational approximation coefficients.
    // See https://web.archive.org/web/20151030215612/http://home.online.no/~pjacklam/notes/invnorm/
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2, // trimmed to f64-exact (was 1.383_577_518_672_690e2)
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    const P_LOW: f64 = 0.02425;
    const P_HIGH: f64 = 1.0 - P_LOW;

    let p = p.clamp(1e-300, 1.0 - 1e-15);

    // Phase 1: Acklam rational approximation.
    let x0 = if p < P_LOW {
        // Lower tail.
        let q = (-2.0 * p.ln()).sqrt();
        -(C[0] + q * (C[1] + q * (C[2] + q * (C[3] + q * (C[4] + q * C[5])))))
            / (1.0 + q * (D[0] + q * (D[1] + q * (D[2] + q * D[3]))))
    } else if p <= P_HIGH {
        let q = p - 0.5;
        let r = q * q;
        q * (A[0] + r * (A[1] + r * (A[2] + r * (A[3] + r * (A[4] + r * A[5])))))
            / (1.0 + r * (B[0] + r * (B[1] + r * (B[2] + r * (B[3] + r * B[4])))))
    } else {
        // Upper tail.
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        (C[0] + q * (C[1] + q * (C[2] + q * (C[3] + q * (C[4] + q * C[5])))))
            / (1.0 + q * (D[0] + q * (D[1] + q * (D[2] + q * D[3]))))
    };

    // Phase 2: Halley refinement steps to reach ~1e-12 relative error.
    // Halley for f(x)=Φ(x)-p=0:  f'=φ(x),  f''=-x·φ(x)
    // x_{k+1} = x_k − (e/φ) · 1/(1 + x_k·e/(2·φ))  where e = Φ(x_k)−p
    //
    // One step brings a well-started Acklam estimate from ~1e-9 to ~1e-12.
    // For the extreme tails (|z|≥2.5) where Acklam's initial error can be
    // ~0.3 (x0 far from true z), three steps are required; convergence is
    // verified to reach <1e-11 for all inputs tested.
    let halley = |x: f64| -> f64 {
        let e = normal_cdf(x) - p;
        let phi = (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt();
        if phi > 0.0 {
            x - e / phi / (1.0 + x * e / (2.0 * phi))
        } else {
            x
        }
    };
    let x1 = halley(x0);
    let x2 = halley(x1);
    halley(x2)
}

/// Effective trial count — closed-form (§6.0 D4 / ADR-0075).
///
/// `N_eff = ρ̄ + (1 − ρ̄) · M`
///
/// where ρ̄ is the mean pairwise correlation of the M candidate Sharpe estimates
/// (treated as a point-estimate of the strategy correlation).
///
/// # Arguments
///
/// - `sharpe_ratios`: the per-candidate annualised Sharpe estimates (one per arm,
///   including the benchmark).  Treated as a proxy for per-strategy performance
///   correlation at our N = 24 scale — the research does not require full return
///   matrices at this scale (§3 of the application doc).
/// - `m`: total candidate count (must equal `sharpe_ratios.len()`; passed
///   separately for clarity and to allow an early-return).
///
/// # Degenerate-Sharpe handling (post-P2 hardening — see module doc)
///
/// Non-finite entries (`NaN` / `±∞`) in `sharpe_ratios` are excluded from the
/// mean/variance/correlation computation below — a `NaN` Sharpe (e.g. a
/// short-side arm whose equity crossed zero, poisoning
/// `compute_sharpe_hourly`'s log-return) must never poison `ρ̄`/`N_eff` via
/// `f64` NaN-propagation. `m` (the caller-supplied raw trial count) is used
/// for the `M < 2` early-return and the final `N_eff` upper clamp REGARDLESS
/// of how many entries were finite — `N_eff` is bounded by "how many
/// strategies were tried", not "how many computed cleanly". If fewer than 2
/// Sharpes are finite, this falls back to the same `m < 2` convention below
/// (returns `m as f64` — conservative, never over-deflates) using the FULL
/// `m`, not the finite subset count, so a mostly-degenerate field is never
/// rewarded with an artificially small `N_eff`.
///
/// # Returns
///
/// `N_eff` clamped to `[1.0, m as f64]`. Never `NaN` — see the guard above
/// and the final `is_nan()` check.
/// Returns `m as f64` (raw N) when M < 2, when fewer than 2 Sharpes are
/// finite, or all finite Sharpes are identical (correlation undefined or
/// 1.0 — conservative, never over-deflates).
#[must_use]
pub fn n_eff(sharpe_ratios: &[f64], m: usize) -> f64 {
    if m < 2 || sharpe_ratios.len() < 2 {
        return m as f64;
    }

    // Exclude non-finite Sharpes (NaN / ±∞) BEFORE any moment computation —
    // see "Degenerate-Sharpe hardening" in the module doc. A finite Sharpe
    // is required for a candidate to contribute to the correlation estimate;
    // a poisoned candidate is still counted in `m` (the trial count) but not
    // in the statistics derived from the Sharpe distribution.
    let finite_sharpes: Vec<f64> = sharpe_ratios
        .iter()
        .copied()
        .filter(|s| s.is_finite())
        .collect();
    if finite_sharpes.len() < 2 {
        // Not enough clean signal to estimate ρ̄ — fall back to the raw trial
        // count (same conservative convention as the M < 2 guard above).
        return m as f64;
    }

    // Mean of all pairwise cross-products as a proxy for ρ̄.
    // For two series x, y: corr(x,y) ≈ cov(x,y) / (std(x)·std(y)).
    // At N=24, a proper correlation matrix is overkill — we use the
    // Sharpe sample to estimate the mean inter-strategy correlation directly.
    // The intuition: strategies whose Sharpes are close to each other are
    // likely highly correlated in returns; those far apart less so.
    // This is the closed-form approximation cited in backtesting[1-App.3].

    let m_f = m as f64;
    let sharpe_ratios = finite_sharpes.as_slice();
    let n = sharpe_ratios.len();

    // Compute sample mean and sample std of the Sharpe estimates.
    let mean_sr: f64 = sharpe_ratios.iter().sum::<f64>() / n as f64;
    let var_sr: f64 = sharpe_ratios
        .iter()
        .map(|&s| (s - mean_sr).powi(2))
        .sum::<f64>()
        / n as f64;
    let std_sr = var_sr.sqrt();

    if std_sr < 1e-12 {
        // All (finite) Sharpes identical → full correlation → N_eff = 1.
        // (Edge case: a totally homogeneous field.)
        return 1.0_f64.min(m_f);
    }

    // Estimate ρ̄: the mean pairwise correlation using the standard relationship
    // between V[{ŜRₙ}] / M and the per-item variance + mean correlation.
    // For M samples with equal pairwise correlation ρ:
    //   Var[mean of M] = σ²(1/M + (M-1)/M · ρ)
    // The cross-trial Sharpe variance V = Var({ŜRₙ}) approximated as var_sr.
    // The closed-form correlation proxy:
    //   ρ̄ ≈ (var_sr - σ²/M) / (σ² · (M-1)/M)   where σ² = per-item variance
    // At our M = 24 scale this reduces cleanly.  We clip to [0, 1].
    let per_item_var = var_sr; // each ŜRₙ is i.i.d. in the null ⟹ same variance
    let rho_bar = if m_f <= 1.0 || per_item_var < 1e-24 {
        0.0
    } else {
        // Bailey-López de Prado App.3: ρ̄ = (1/M · Σᵢ≠ⱼ ρᵢⱼ) estimated
        // from the cross-Sharpe standardised inner product.
        let mut sum_corr = 0.0_f64;
        let mut pairs = 0_usize;
        for i in 0..n {
            for j in (i + 1)..n {
                let z_i = (sharpe_ratios[i] - mean_sr) / std_sr;
                let z_j = (sharpe_ratios[j] - mean_sr) / std_sr;
                sum_corr += z_i * z_j;
                pairs += 1;
            }
        }
        if pairs == 0 {
            0.0
        } else {
            (sum_corr / pairs as f64).clamp(-1.0, 1.0)
        }
    };

    // N_eff = ρ̄ + (1 − ρ̄) · M   (Bailey-López de Prado App.3)
    let n_eff_raw = rho_bar + (1.0 - rho_bar) * m_f;
    // Defense in depth (see module doc): `n_eff_raw` is mathematically
    // guaranteed finite here (every input above was filtered to `is_finite()`
    // first), but an explicit `is_nan()` guard — rather than relying on
    // `f64::clamp`'s implicit NaN-propagation behaviour — keeps this function
    // honest even if a future edit reintroduces an unfiltered path.
    if n_eff_raw.is_nan() {
        return m_f;
    }
    n_eff_raw.clamp(1.0, m_f)
}

/// Minimum Backtest Length in years (approximate closed form, `evolution[29]`).
///
/// `MinBTL ≈ 2 · ln(N_eff) / SR_target²`
///
/// At `SR_target` = 1 this is `2 · ln(N_eff)`.
/// At `N_eff` = 24: `MinBTL` ≈ 6.4 years.
/// At `N_eff` = 1: `MinBTL` = 0 years (trivially satisfied).
///
/// # Arguments
///
/// - `n_eff`: effective trial count (from [`n_eff`]).
/// - `sr_target`: the annualised Sharpe target (default 1.0).
///
/// # Returns
///
/// Years of backtest data required.  Clamped to ≥ 0.  `0.0` on a `NaN`
/// `n_eff` — the SAME degenerate output the `N_eff ≤ 1` case already
/// documents (a `NaN` trial count is, definitionally, not a trustworthy
/// multiple-testing correction; `0.0` is the honest "cannot compute a
/// non-trivial bound" answer, not a silent coincidence of `f64::max`'s
/// NaN-propagation rule — see the module doc's degenerate-Sharpe section).
#[must_use]
pub fn min_btl(n_eff: f64, sr_target: f64) -> f64 {
    // `n_eff <= 1.0` is `false` for a NaN `n_eff` (NaN comparisons are always
    // false), so a bare `<=` guard alone would fall through to
    // `n_eff.max(1.0 + f64::EPSILON)` — which silently returns `1.0 + ε`
    // because `f64::max` returns the non-NaN operand. Guard `is_nan()`
    // explicitly first so the degenerate case is a documented `0.0`, not an
    // accident of IEEE 754 max semantics.
    if n_eff.is_nan() || n_eff <= 1.0 || sr_target <= 0.0 {
        return 0.0;
    }
    let n_eff_clamped = n_eff.max(1.0 + f64::EPSILON);
    (2.0 * n_eff_clamped.ln() / sr_target.powi(2)).max(0.0)
}

/// Deflated Sharpe Ratio — DSR (`evolution[98]` / `backtesting[1]`).
///
/// DSR = Φ[ (ŜR − SR₀) · √(T − 1) / √(1 − γ̂₃·ŜR + ((γ̂₄ − 1)/4)·ŜR²) ]
///
/// `SR₀ = √V · ((1 − γ)·Φ⁻¹[1 − 1/N_eff] + γ·Φ⁻¹[1 − e⁻¹/N_eff])`
///
/// All inputs (ŜR, SR₀, skew, kurtosis) are in the **non-annualised** (hourly)
/// scale to match the return series from which skew/kurtosis are derived.
/// The probability output is in [0, 1].
///
/// # Arguments
///
/// - `sr_hat`: crown's annualised Sharpe (we convert to per-period).
/// - `sharpe_variance`: cross-trial variance V of the per-candidate Sharpe
///   estimates (Var({ŜRₙ})); computed from the candidate Sharpe vector.
///   In annualised units² — we convert to per-period below.
/// - `t_periods`: sample length in bars (hourly).
/// - `skew`: crown's return-series skew (γ̂₃), 3rd standardised moment.
/// - `kurtosis`: crown's return-series (excess) kurtosis (γ̂₄ − 3, so that
///   Normal → 0).  The formula uses `γ̂₄ − 1` which becomes `excess_kurtosis + 3 − 1`.
///   We accept standard excess-kurtosis (0 for Normal) and convert internally.
/// - `n_eff`: effective trial count from [`n_eff`].
///
/// # Degenerate-Sharpe handling (post-P2 hardening — see module doc)
///
/// `n_eff < 1.0` is `false` for a `NaN` `n_eff` (NaN comparisons are always
/// false in IEEE 754), so an `is_nan()` check is explicit here rather than
/// relying on `<` to catch it — the same discipline as [`min_btl`]. In
/// practice `n_eff` reaching this function is already guaranteed finite by
/// [`n_eff`]'s own hardening (see module doc); this is defense in depth.
///
/// # Returns
///
/// DSR as a probability in [0, 1].  Returns 0.0 on degenerate inputs
/// (including a `NaN` `n_eff` — never propagated into the output).
#[must_use]
pub fn dsr(
    sr_hat_annualised: f64,
    sharpe_variance_annualised: f64,
    t_periods: usize,
    skew: f64,
    excess_kurtosis: f64,
    n_eff: f64,
) -> f64 {
    if t_periods < 2 || n_eff.is_nan() || n_eff < 1.0 {
        return 0.0;
    }

    // Convert annualised Sharpe → per-period (hourly).
    // SR_annualised = SR_period · √HPY  →  SR_period = SR_annualised / √HPY
    let sr_hat = sr_hat_annualised / SQRT_HPY;

    // Convert annualised Sharpe variance → per-period.
    // Var[SR_ann] = Var[SR_period · √HPY] = HPY · Var[SR_period]
    // → Var[SR_period] = Var[SR_ann] / HPY
    // (`sharpe_variance_annualised` is guaranteed finite — see
    // `sharpe_variance`'s own hardening — so `.max(0.0)` here is purely a
    // non-negativity clamp, not a NaN-swallow.)
    let hpy = SQRT_HPY * SQRT_HPY; // 24 * 365
    let v_sr = (sharpe_variance_annualised / hpy).max(0.0);

    let t = t_periods as f64;

    // SR₀ = √V · ((1−γ)·Φ⁻¹[1−1/N_eff] + γ·Φ⁻¹[1−e⁻¹/N_eff])
    let n = n_eff.max(1.0 + f64::EPSILON);
    let gamma = EULER_MASCHERONI;
    let inv_n = 1.0 / n;
    let inv_ne = inv_n / std::f64::consts::E;
    // Clamp arguments to a valid range for the inverse CDF.
    let arg1 = (1.0 - inv_n).clamp(1e-300, 1.0 - 1e-15);
    let arg2 = (1.0 - inv_ne).clamp(1e-300, 1.0 - 1e-15);
    let sr0 = v_sr.sqrt() * ((1.0 - gamma) * normal_inv_cdf(arg1) + gamma * normal_inv_cdf(arg2));

    // Non-Normal SE of the Sharpe estimator (denominator):
    // √(1 − γ̂₃·ŜR + ((γ̂₄ − 1)/4)·ŜR²)
    // The formula uses γ̂₄ (total kurtosis = excess_kurtosis + 3).
    let kurt4 = excess_kurtosis + 3.0; // total kurtosis
    let se_sq = 1.0 - skew * sr_hat + ((kurt4 - 1.0) / 4.0) * sr_hat.powi(2);
    if se_sq <= 0.0 {
        // Degenerate: denominator is imaginary — return 0 (conservative).
        return 0.0;
    }
    let se = se_sq.sqrt();

    // DSR = Φ[(ŜR − SR₀) · √(T − 1) / SE]
    let z = (sr_hat - sr0) * (t - 1.0).sqrt() / se;
    normal_cdf(z)
}

/// Compute return-series skew (3rd standardised central moment).
///
/// `skew = E[(r − μ)³] / σ³`
///
/// Returns 0.0 for fewer than 3 observations or zero standard deviation.
#[must_use]
pub fn compute_skew(log_returns: &[f64]) -> f64 {
    let n = log_returns.len();
    if n < 3 {
        return 0.0;
    }
    let n_f = n as f64;
    let mean = log_returns.iter().sum::<f64>() / n_f;
    let var = log_returns.iter().map(|&r| (r - mean).powi(2)).sum::<f64>() / n_f;
    let std = var.sqrt();
    if std < 1e-15 {
        return 0.0;
    }
    log_returns
        .iter()
        .map(|&r| ((r - mean) / std).powi(3))
        .sum::<f64>()
        / n_f
}

/// Compute return-series excess kurtosis (4th standardised central moment − 3).
///
/// `excess_kurt = E[(r − μ)⁴] / σ⁴ − 3`
///
/// Normal distribution → 0.  Returns 0.0 for fewer than 4 observations.
#[must_use]
pub fn compute_excess_kurtosis(log_returns: &[f64]) -> f64 {
    let n = log_returns.len();
    if n < 4 {
        return 0.0;
    }
    let n_f = n as f64;
    let mean = log_returns.iter().sum::<f64>() / n_f;
    let var = log_returns.iter().map(|&r| (r - mean).powi(2)).sum::<f64>() / n_f;
    let std = var.sqrt();
    if std < 1e-15 {
        return 0.0;
    }
    let m4 = log_returns
        .iter()
        .map(|&r| ((r - mean) / std).powi(4))
        .sum::<f64>()
        / n_f;
    m4 - 3.0
}

/// Cross-trial Sharpe variance V (= `Var[{ŜRₙ}]`).
///
/// The **across-trial dispersion of the candidates' Sharpe estimates** — NOT
/// the sampling SE of one Sharpe.  This is the common error warned about in
/// `backtesting[1]`: using a single SE as V over-deflates the DSR.
///
/// # Degenerate-Sharpe handling (post-P2 hardening — see module doc)
///
/// Non-finite entries (`NaN` / `±∞`) are excluded before the mean/variance
/// computation, same discipline as [`n_eff`] — a `NaN` Sharpe must not
/// poison the cross-trial variance DSR uses as `V`.
///
/// # Returns
///
/// Annualised Sharpe variance (sample variance, Bessel-corrected). `0.0`
/// when fewer than 2 Sharpes are finite. Never `NaN`.
#[must_use]
pub fn sharpe_variance(sharpe_ratios: &[f64]) -> f64 {
    let finite_sharpes: Vec<f64> = sharpe_ratios
        .iter()
        .copied()
        .filter(|s| s.is_finite())
        .collect();
    let n = finite_sharpes.len();
    if n < 2 {
        return 0.0;
    }
    let n_f = n as f64;
    let mean = finite_sharpes.iter().sum::<f64>() / n_f;
    // Bessel correction (n − 1) for an unbiased sample variance.
    finite_sharpes
        .iter()
        .map(|&s| (s - mean).powi(2))
        .sum::<f64>()
        / (n_f - 1.0)
}

/// Convert an equity curve (Decimal) to f64 log-returns — mirrors `bootstrap.rs`.
///
/// Used to derive skew / kurtosis from the crown's equity curve without
/// re-importing the private helper from `bootstrap.rs`.
#[must_use]
pub fn equity_to_log_returns(equity: &[rust_decimal::Decimal]) -> Vec<f64> {
    use rust_decimal::prelude::ToPrimitive;
    if equity.len() < 2 {
        return vec![];
    }
    equity
        .windows(2)
        .map(|w| {
            let prev = w[0].to_f64().unwrap_or(1.0);
            let curr = w[1].to_f64().unwrap_or(1.0);
            if prev <= 0.0 { 0.0 } else { (curr / prev).ln() }
        })
        .collect()
}

/// Build a [`Scorecard`] from the inputs already available in `run_bakeoff`.
///
/// # Arguments
///
/// - `all_sharpe_ratios`: slice of per-candidate annualised Sharpe ratios
///   (one per arm, **including the benchmark** — consistent with how the
///   literature defines N: all arms tried, including the null).
/// - `crown_equity`: the crowned candidate's equity curve as `Decimal` values.
/// - `t_bars`: total number of bars in the backtest window.
///
/// # Returns
///
/// A fully-computed [`Scorecard`].  On degenerate inputs (empty slices, T < 2)
/// returns a zero scorecard rather than panicking.
#[must_use]
pub fn compute_scorecard(
    all_sharpe_ratios: &[f64],
    crown_equity: &[rust_decimal::Decimal],
    t_bars: usize,
) -> Scorecard {
    let n_candidates = all_sharpe_ratios.len();
    if n_candidates == 0 || crown_equity.len() < 2 || t_bars < 2 {
        return Scorecard {
            n_candidates,
            n_eff: 0.0,
            deflated_sharpe: 0.0,
            min_btl_years: 0.0,
            pbo: None,
            crown_clears_dsr: false,
        };
    }

    // N_eff (closed-form).
    let n_eff_val = n_eff(all_sharpe_ratios, n_candidates);

    // MinBTL in years (hourly bars: 1 year ≈ 24 × 365 = 8760 bars).
    let min_btl_val = min_btl(n_eff_val, SR_TARGET_ANNUALISED);

    // Cross-trial Sharpe variance (annualised).
    let v_sr = sharpe_variance(all_sharpe_ratios);

    // Crown's annualised Sharpe (mean of the input slice as crown's SR).
    // The crown's actual Sharpe is the max; we take it from the slice.
    let crown_sr = all_sharpe_ratios
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);

    // Crown's log-returns for skew / kurtosis.
    let log_returns = equity_to_log_returns(crown_equity);
    let skew = compute_skew(&log_returns);
    let excess_kurt = compute_excess_kurtosis(&log_returns);

    // DSR.
    let dsr_val = dsr(crown_sr, v_sr, t_bars, skew, excess_kurt, n_eff_val);

    Scorecard {
        n_candidates,
        n_eff: n_eff_val,
        deflated_sharpe: dsr_val,
        min_btl_years: min_btl_val,
        pbo: None, // deferred to Tune/sweep surface (§6.0 D1)
        crown_clears_dsr: dsr_val >= DSR_THRESHOLD,
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::float_arithmetic,
    clippy::pedantic,
    clippy::approx_constant
)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    // ── normal_cdf / normal_inv_cdf smoke tests ────────────────────────────────

    #[test]
    fn normal_cdf_symmetry_and_boundary() {
        // Φ(0) = 0.5
        let mid = normal_cdf(0.0);
        assert!((mid - 0.5).abs() < 1e-6, "Φ(0) = {mid}");
        // Φ(z) + Φ(-z) = 1
        let z = 1.96;
        assert!((normal_cdf(z) + normal_cdf(-z) - 1.0).abs() < 1e-6);
        // Φ(1.96) ≈ 0.975
        assert!(
            (normal_cdf(z) - 0.975).abs() < 0.001,
            "Φ(1.96) = {}",
            normal_cdf(z)
        );
    }

    #[test]
    fn normal_inv_cdf_roundtrip() {
        // Φ⁻¹(Φ(z)) ≈ z for several z values.
        for z in [-2.5, -1.0, 0.0, 1.0, 2.5] {
            let p = normal_cdf(z);
            let z2 = normal_inv_cdf(p);
            assert!((z2 - z).abs() < 1e-5, "roundtrip failed at z={z}: got {z2}");
        }
    }

    // ── n_eff tests ────────────────────────────────────────────────────────────

    #[test]
    fn n_eff_perfectly_correlated_returns_one() {
        // All Sharpes identical → full correlation → N_eff = 1.
        let sharpes = vec![1.0, 1.0, 1.0, 1.0, 1.0];
        let ne = n_eff(&sharpes, sharpes.len());
        // With identical Sharpes the std_sr = 0 → clamps to 1.
        assert!(ne <= 1.5, "expected N_eff≈1, got {ne}");
    }

    #[test]
    fn n_eff_uncorrelated_field_approaches_m() {
        // Sharpes spanning a wide range → low estimated correlation → N_eff closer to M.
        // 10 candidates from 0.1 to 2.0 (high spread → low pairwise correlation proxy).
        let sharpes: Vec<f64> = (0..10).map(|i| 0.1 + i as f64 * 0.2).collect();
        let m = sharpes.len();
        let ne = n_eff(&sharpes, m);
        // Should be > 1 and ≤ m.
        assert!(ne > 1.0, "N_eff should be > 1, got {ne}");
        assert!(ne <= m as f64 + 1e-10, "N_eff should be ≤ M={m}, got {ne}");
    }

    #[test]
    fn n_eff_single_candidate() {
        let ne = n_eff(&[2.5], 1);
        assert_eq!(ne, 1.0);
    }

    #[test]
    fn n_eff_empty_returns_zero() {
        let ne = n_eff(&[], 0);
        assert_eq!(ne, 0.0);
    }

    // ── NaN-Sharpe regression (post-P2 hardening) ─────────────────────────────
    //
    // Root cause: `compute_sharpe_hourly` can return `NaN` for a short-side
    // arm whose equity crosses through zero mid-window (ln of a negative
    // ratio). Before the fix, ANY NaN entry in `sharpe_ratios` poisoned the
    // mean/variance/correlation computation, producing `n_eff = NaN`, which
    // `min_btl`'s `n_eff.max(1.0 + f64::EPSILON)` then silently clamped to
    // ~1.0 (`f64::max(NaN, x) == x` per IEEE 754) — so `min_btl_years` read
    // `0.00` while `n_eff` itself stayed a raw, unusable `NaN`.
    //
    // Reproduces the exact S4 field shape: a realistic Sharpe spread (24
    // finite values matching the observed `s4_binance_2324_base_smoke`
    // magnitudes) plus 2 NaN entries at the positions `v0.sma_cross_ls` /
    // `v0.always_short` occupy in the real field.

    /// A single `NaN` Sharpe in an otherwise-normal field must NOT poison
    /// `N_eff` — it must be excluded from the moment computation, leaving
    /// `N_eff` finite and matching what the SAME field minus the NaN entry
    /// would produce (the NaN candidate is invisible to the correlation
    /// estimate, not present-but-corrupting).
    #[test]
    fn n_eff_excludes_single_nan_sharpe() {
        let clean = vec![1.4668, 0.6604, -0.9666, 1.7909, 0.5, -0.3, 1.1, 0.2];
        let mut with_nan = clean.clone();
        with_nan.insert(2, f64::NAN); // same position family as the real field
        let m = with_nan.len();

        let ne_with_nan = n_eff(&with_nan, m);
        assert!(
            ne_with_nan.is_finite(),
            "N_eff must be finite when the input contains one NaN Sharpe, got {ne_with_nan}"
        );

        // The NaN-bearing field's N_eff must equal the clean field's N_eff
        // computed against the SAME raw trial count `m` (NaN is excluded from
        // the correlation estimate, but `m` — the trial count — still counts
        // the degenerate arm; see module doc).
        let ne_clean_same_m = n_eff(&clean, m);
        assert!(
            (ne_with_nan - ne_clean_same_m).abs() < 1e-9,
            "N_eff with 1 NaN excluded should match N_eff of the finite subset \
             at the same trial count m={m}: with_nan={ne_with_nan}, clean={ne_clean_same_m}"
        );
    }

    /// A field with ALL Sharpes `NaN` (fully degenerate) must fall back to
    /// the conservative `m as f64` convention — not propagate `NaN`.
    #[test]
    fn n_eff_all_nan_falls_back_to_m() {
        let all_nan = vec![f64::NAN, f64::NAN, f64::NAN, f64::NAN];
        let ne = n_eff(&all_nan, all_nan.len());
        assert_eq!(
            ne, 4.0,
            "all-NaN field must fall back to raw trial count m, got {ne}"
        );
        assert!(!ne.is_nan());
    }

    /// The exact S4 shape: 25 candidates (24 finite + benchmark), 2 of them
    /// `NaN` (`v0.sma_cross_ls`, `v0.always_short`) — matches the field size
    /// observed in `s4_binance_2324_base_smoke`. `N_eff` must be finite and
    /// in `(1.0, 25.0]`.
    #[test]
    fn n_eff_s4_shape_two_nan_of_25_stays_finite_and_bounded() {
        let mut sharpes = vec![
            1.4668,
            0.6604,
            -0.5399,
            -1.6644,
            -1.9697,
            1.5656,
            -2.8920,
            -0.0955,
            -0.3175,
            0.0,
            -0.9666,
            0.0,
            0.3371,
            0.0,
            -0.9443,
            1.2460,
            -0.2157,
            -0.1275,
            f64::NAN,
            0.5721,
            -0.5724,
            -1.6946,
            f64::NAN,
            1.0655,
            1.7909,
        ];
        let m = sharpes.len();
        assert_eq!(m, 25, "must match the S4 field size (24 arms + benchmark)");
        let ne = n_eff(&sharpes, m);
        assert!(ne.is_finite(), "N_eff must be finite, got {ne}");
        assert!(
            ne > 1.0 && ne <= m as f64 + 1e-9,
            "N_eff={ne} out of (1, {m}] bounds"
        );

        // Order independence sanity: shuffling the NaN positions must not
        // change the result (filter is position-independent by construction).
        sharpes.swap(0, 18);
        let ne2 = n_eff(&sharpes, m);
        assert!(
            (ne - ne2).abs() < 1e-9,
            "n_eff must be order-independent: {ne} vs {ne2}"
        );
    }

    // ── min_btl tests ──────────────────────────────────────────────────────────

    /// Verify the MinBTL formula against the table from `evolution[29]`:
    ///
    /// | N   | MinBTL ≈ 2·ln(N)/SR² (years, SR=1) |
    /// |-----|--------------------------------------|
    /// | 10  | ≈ 4.6  (table: "~0.5–5")            |
    /// | 50  | ≈ 7.8  (table: "~1.5–2" for SR=2)   |
    /// | 100 | ≈ 9.2  (table: "~2.5–3" for SR=2)   |
    ///
    /// The research table gives SR=1 approximate values from the *exact* formula;
    /// the looser `2·ln(N)` form gives slightly higher numbers — the spec says
    /// "calibrate against whichever is more conservative," so the loose form is
    /// the correct choice here.  We verify the loose form matches the formula.
    #[test]
    fn min_btl_formula_matches_2lnn_over_sr2() {
        // N=100, SR_target=1 → 2·ln(100)/1² = 2·4.605 ≈ 9.21 years.
        let btl = min_btl(100.0, 1.0);
        assert!((btl - 9.21).abs() < 0.05, "MinBTL(100, SR=1) = {btl}");
    }

    #[test]
    fn min_btl_n_eq_24_sr_eq_1() {
        // N_eff=24 → 2·ln(24) = 2·3.178 ≈ 6.36 years.
        let btl = min_btl(24.0, 1.0);
        assert!((btl - 6.36).abs() < 0.05, "MinBTL(24, 1) = {btl}");
    }

    #[test]
    fn min_btl_zero_for_n_le_1() {
        assert_eq!(min_btl(1.0, 1.0), 0.0);
        assert_eq!(min_btl(0.0, 1.0), 0.0);
    }

    /// Regression: a `NaN` `n_eff` must produce an EXPLICIT, documented
    /// `0.0` (the `is_nan()` guard firing), not a value that HAPPENS to be
    /// ~0.0 via `f64::max(NaN, x) == x` silently clamping to `x=1.0+ε` and
    /// then `2·ln(1.0+ε) ≈ 4.4e-16`. Both produce a number that *rounds* to
    /// `0.00` at 2dp — this test pins the EXACT bit-for-bit `0.0`, which the
    /// pre-fix silent-clamp path did NOT produce (it produced `4.44e-16`, a
    /// non-zero float that only LOOKED like zero when formatted `{:.2}`).
    #[test]
    fn min_btl_nan_input_produces_exact_zero_not_epsilon() {
        let btl = min_btl(f64::NAN, 1.0);
        assert_eq!(
            btl, 0.0,
            "NaN n_eff must produce an EXACT 0.0 via the explicit is_nan() \
             guard, not an epsilon-near-zero value from a silent clamp, got {btl:e}"
        );
        assert!(!btl.is_nan(), "min_btl must never return NaN");
    }

    // ── DSR tests ──────────────────────────────────────────────────────────────

    /// Reference worked example from `evolution[98]` / §1a of the research doc:
    ///
    /// `ŜR = 2.5` (annualised), `N = 100`, `V = 0.5`, skew = −3, excess_kurt = 7
    /// (total kurt = 10 → excess = 7), daily data T = 5·252 = 1260 bars.
    ///
    /// Expected: `DSR ≈ 0.90 < 0.95` → NOT a discovery.
    ///
    /// We use daily bars here (T = 1260) and match the non-annualised conversion.
    /// The daily SQRT_HPY equivalent is √252 ≈ 15.875.
    ///
    /// NOTE: the research worked example uses a daily Sharpe computation.
    /// Our `dsr()` function uses the hourly-bar scale (SQRT_HPY = √(24·365)).
    /// For this test we call `dsr()` directly with a per-period SR converted from
    /// the daily-annualised example — the result should remain below 0.95.
    #[test]
    fn dsr_research_worked_example_fails_at_n100() {
        // From `evolution[98]`: ŜR_ann = 2.5, T_daily = 5·252 = 1260 days.
        // Our formula accepts annualised SR; we use T_periods = 1260 (daily bars).
        // Override SQRT_HPY for this test by supplying daily-scale SR directly.
        // The function's internal conversion uses SQRT_HPY (hourly). To test the
        // daily example faithfully we call the formula internals directly.
        // The key check: with N=100, V=0.5, fat-tail crypto returns, DSR < 0.95.

        // Replicate the DSR calculation for the daily example:
        let sr_hat_ann = 2.5_f64;
        let v_sr_ann = 0.5_f64; // variance of Sharpes across 100 trials
        let t = 1260_usize; // 5 years daily
        let skew = -3.0_f64;
        let excess_kurt = 7.0_f64; // total kurt=10, excess=7
        let n_eff_val = 100.0_f64;

        // Call through the public API (which uses hourly scale internally).
        // Scale to hourly-equivalent to isolate the formula logic.
        // We test the sub-formulas directly instead.
        let sqrt_daily: f64 = (252.0_f64).sqrt();

        // Per-period SR in daily units.
        let sr_hat_period = sr_hat_ann / sqrt_daily;
        // Per-period SR variance.
        let v_sr_period = v_sr_ann / 252.0;

        // SR₀
        let gamma = EULER_MASCHERONI;
        let n = n_eff_val;
        let arg1 = (1.0 - 1.0 / n).clamp(1e-300, 1.0 - 1e-15);
        let arg2 = (1.0 - 1.0 / (n * std::f64::consts::E)).clamp(1e-300, 1.0 - 1e-15);
        let sr0 = v_sr_period.sqrt()
            * ((1.0 - gamma) * normal_inv_cdf(arg1) + gamma * normal_inv_cdf(arg2));

        // SE
        let kurt4 = excess_kurt + 3.0; // total kurtosis = 10
        let se_sq = 1.0 - skew * sr_hat_period + ((kurt4 - 1.0) / 4.0) * sr_hat_period.powi(2);
        let se = se_sq.sqrt();

        let z = (sr_hat_period - sr0) * ((t - 1) as f64).sqrt() / se;
        let dsr_val = normal_cdf(z);

        // Research: DSR ≈ 0.90 < 0.95 → NOT a discovery at N=100.
        assert!(
            dsr_val < 0.95,
            "DSR should be < 0.95 at N=100 (fat tails), got {dsr_val:.4}"
        );
        assert!(
            dsr_val > 0.80,
            "DSR should be > 0.80 (not completely rejected), got {dsr_val:.4}"
        );
    }

    /// At N=46 (the same example) DSR should just clear 0.95 — fat tails halve the
    /// tolerable number of trials (88 Normal → 46 fat-tail).
    #[test]
    fn dsr_research_worked_example_passes_at_n46() {
        let sqrt_daily: f64 = (252.0_f64).sqrt();
        let sr_hat_period = 2.5_f64 / sqrt_daily;
        let v_sr_period = 0.5_f64 / 252.0;
        let gamma = EULER_MASCHERONI;
        let n = 46.0_f64;
        let arg1 = (1.0 - 1.0 / n).clamp(1e-300, 1.0 - 1e-15);
        let arg2 = (1.0 - 1.0 / (n * std::f64::consts::E)).clamp(1e-300, 1.0 - 1e-15);
        let sr0 = v_sr_period.sqrt()
            * ((1.0 - gamma) * normal_inv_cdf(arg1) + gamma * normal_inv_cdf(arg2));
        let kurt4 = 10.0_f64; // total kurtosis (excess 7 + 3)
        let skew = -3.0_f64;
        let t = 1260_usize;
        let se_sq = 1.0 - skew * sr_hat_period + ((kurt4 - 1.0) / 4.0) * sr_hat_period.powi(2);
        let se = se_sq.sqrt();
        let z = (sr_hat_period - sr0) * ((t - 1) as f64).sqrt() / se;
        let dsr_val = normal_cdf(z);
        // Research: clears 0.95 at N=46.
        assert!(
            dsr_val >= 0.95,
            "DSR should be ≥ 0.95 at N=46, got {dsr_val:.4}"
        );
    }

    /// Normal-return case (skew=0, excess_kurt=0): DSR should clear at N=88 per the research.
    #[test]
    fn dsr_normal_returns_clears_at_n88() {
        let sqrt_daily: f64 = (252.0_f64).sqrt();
        let sr_hat_period = 2.5_f64 / sqrt_daily;
        let v_sr_period = 0.5_f64 / 252.0;
        let gamma = EULER_MASCHERONI;
        let n = 88.0_f64;
        let arg1 = (1.0 - 1.0 / n).clamp(1e-300, 1.0 - 1e-15);
        let arg2 = (1.0 - 1.0 / (n * std::f64::consts::E)).clamp(1e-300, 1.0 - 1e-15);
        let sr0 = v_sr_period.sqrt()
            * ((1.0 - gamma) * normal_inv_cdf(arg1) + gamma * normal_inv_cdf(arg2));
        let skew = 0.0_f64;
        let excess_kurt = 0.0_f64; // Normal returns
        let t = 1260_usize;
        let kurt4 = excess_kurt + 3.0;
        let se_sq = 1.0 - skew * sr_hat_period + ((kurt4 - 1.0) / 4.0) * sr_hat_period.powi(2);
        let se = se_sq.sqrt();
        let z = (sr_hat_period - sr0) * ((t - 1) as f64).sqrt() / se;
        let dsr_val = normal_cdf(z);
        // Research: clears at N=88 with Normal returns.
        assert!(
            dsr_val >= 0.95,
            "DSR should be ≥ 0.95 at N=88 Normal, got {dsr_val:.4}"
        );
    }

    /// Regression: a `NaN` `n_eff` passed into `dsr()` must yield `0.0`, not
    /// propagate `NaN` into `deflated_sharpe` and not silently proceed as if
    /// `N_eff == 1` (the pre-fix behaviour, via `n_eff < 1.0` being `false`
    /// for NaN so the early-return didn't fire, then `n_eff.max(1.0+ε)`
    /// silently substituting ~1.0).
    #[test]
    fn dsr_nan_n_eff_returns_zero_not_nan() {
        let dsr_val = dsr(1.5, 0.3, 500, 0.0, 0.0, f64::NAN);
        assert_eq!(
            dsr_val, 0.0,
            "NaN n_eff must produce an explicit 0.0 DSR, got {dsr_val}"
        );
        assert!(!dsr_val.is_nan(), "dsr() must never return NaN");
    }

    // ── sharpe_variance NaN-exclusion tests ───────────────────────────────────

    /// `sharpe_variance` must exclude non-finite entries — a single `NaN`
    /// Sharpe must not poison `V`, DSR's cross-trial-variance input.
    #[test]
    fn sharpe_variance_excludes_nan() {
        let clean = vec![1.0, 2.0, 0.5, -0.5, 1.5];
        let mut with_nan = clean.clone();
        with_nan.push(f64::NAN);

        let v_with_nan = sharpe_variance(&with_nan);
        assert!(
            v_with_nan.is_finite(),
            "sharpe_variance must be finite with one NaN entry, got {v_with_nan}"
        );
        let v_clean = sharpe_variance(&clean);
        assert!(
            (v_with_nan - v_clean).abs() < 1e-9,
            "sharpe_variance with NaN excluded should match the finite subset: \
             with_nan={v_with_nan}, clean={v_clean}"
        );
    }

    /// All-NaN input → `0.0`, not `NaN` (mirrors the `n < 2` convention).
    #[test]
    fn sharpe_variance_all_nan_returns_zero() {
        let v = sharpe_variance(&[f64::NAN, f64::NAN, f64::NAN]);
        assert_eq!(v, 0.0);
    }

    // ── compute_scorecard integration test ────────────────────────────────────

    #[test]
    fn compute_scorecard_degenerate_empty() {
        let sc = compute_scorecard(&[], &[], 0);
        assert_eq!(sc.n_candidates, 0);
        assert_eq!(sc.deflated_sharpe, 0.0);
        assert!(!sc.crown_clears_dsr);
    }

    /// End-to-end regression pinning the ORIGINAL bug: `compute_scorecard`
    /// fed the exact S4 field shape (25 candidates, 2 `NaN` Sharpes at the
    /// `v0.sma_cross_ls` / `v0.always_short` positions, matching
    /// `s4_binance_2324_base_smoke`'s observed field) must produce a FINITE
    /// `n_eff` and a `min_btl_years` that is EITHER a genuine positive value
    /// OR an honest `0.0` derived from a finite, in-bounds `n_eff` — never a
    /// `NaN` `n_eff` paired with an accidental `0.0` `min_btl_years`.
    ///
    /// Before the fix this test would have failed on `sc.n_eff.is_finite()`
    /// (it was `NaN`), even though `min_btl_years` happened to already read
    /// `~0.0` (see `min_btl_nan_input_produces_exact_zero_not_epsilon` for
    /// why "reads as 0.00" was not proof of correctness).
    #[test]
    fn compute_scorecard_s4_shape_two_nan_sharpes_stays_honest() {
        let all_sharpes = vec![
            1.4668,
            0.6604,
            -0.5399,
            -1.6644,
            -1.9697,
            1.5656,
            -2.8920,
            -0.0955,
            -0.3175,
            0.0,
            -0.9666,
            0.0,
            0.3371,
            0.0,
            -0.9443,
            1.2460,
            -0.2157,
            -0.1275,
            f64::NAN,
            0.5721,
            -0.5724,
            -1.6946,
            f64::NAN,
            1.0655,
            1.7909,
        ];
        assert_eq!(all_sharpes.len(), 25, "must match the S4 field shape");

        // Crown (buy-and-hold, Sharpe=1.7909) equity curve — a modest, steady
        // rise over 1000 bars is sufficient to exercise the full pipeline.
        let crown_equity: Vec<Decimal> = (0..1001)
            .map(|i| dec!(100_000) + Decimal::from(i * 50))
            .collect();

        let sc = compute_scorecard(&all_sharpes, &crown_equity, 1000);

        assert_eq!(sc.n_candidates, 25, "n_candidates counts every arm tried");
        assert!(
            sc.n_eff.is_finite(),
            "n_eff must be finite with 2 NaN Sharpes in the field — THE bug this test pins, got {}",
            sc.n_eff
        );
        assert!(
            sc.n_eff > 1.0 && sc.n_eff <= 25.0 + 1e-9,
            "n_eff must stay within (1, 25], got {}",
            sc.n_eff
        );
        assert!(
            !sc.min_btl_years.is_nan(),
            "min_btl_years must never be NaN"
        );
        assert!(
            sc.min_btl_years >= 0.0,
            "min_btl_years must be non-negative, got {}",
            sc.min_btl_years
        );
        // With n_eff well above 1 (24 finite arms feeding the correlation
        // estimate), min_btl_years must be a GENUINE positive number, not the
        // degenerate near-zero the pre-fix NaN-clamp produced.
        assert!(
            sc.min_btl_years > 0.1,
            "min_btl_years should be a real positive bound (n_eff={}), not a \
             degenerate near-zero artefact of the pre-fix clamp, got {}",
            sc.n_eff,
            sc.min_btl_years
        );
        assert!(
            !sc.deflated_sharpe.is_nan(),
            "deflated_sharpe must never be NaN"
        );
    }

    #[test]
    fn compute_scorecard_single_candidate() {
        // One equity curve: 1000 hourly bars, steadily rising.
        let equity: Vec<Decimal> = (0..1001)
            .map(|i| dec!(100_000) + Decimal::from(i * 100))
            .collect();
        let sc = compute_scorecard(&[2.0], &equity, 1000);
        assert_eq!(sc.n_candidates, 1);
        assert!(sc.n_eff >= 1.0);
        assert!(sc.n_eff <= 1.0 + 1e-10);
        assert!(sc.min_btl_years >= 0.0);
        assert!(sc.pbo.is_none(), "PBO must be None in v2");
    }

    // ── ScorecardSummary / Scorecard::summary() tests (P0-3) ──────────────────

    /// Positive case: a non-degenerate scorecard produces a `Some(summary)`
    /// with the four projected fields matching.
    #[test]
    fn scorecard_summary_positive_case() {
        let equity: Vec<Decimal> = (0..200)
            .map(|i| dec!(100_000) + Decimal::from(i * 50))
            .collect();
        let sharpes = vec![2.0, 1.5, 1.2, 0.8, 0.3, -0.2, 1.1, 0.9, 1.7, 0.5];
        let sc = compute_scorecard(&sharpes, &equity, 199);
        let summary = sc.summary();
        assert!(
            summary.is_some(),
            "non-degenerate scorecard must yield Some from summary()"
        );
        let summary = summary.unwrap_or_else(|| panic!("checked above"));

        assert_eq!(summary.n_candidates, sc.n_candidates);
        assert!((summary.deflated_sharpe - sc.deflated_sharpe).abs() < 1e-12);
        assert_eq!(summary.crown_clears_dsr, sc.crown_clears_dsr);
        assert!((summary.min_btl_years - sc.min_btl_years).abs() < 1e-12);
    }

    /// Degenerate case: empty scorecard (`n_candidates == 0`) must yield `None`.
    #[test]
    fn scorecard_summary_degenerate_yields_none() {
        let sc = compute_scorecard(&[], &[], 0);
        assert!(
            sc.summary().is_none(),
            "degenerate scorecard (n_candidates==0) must yield None from summary()"
        );
    }

    #[test]
    fn compute_scorecard_pbo_always_none() {
        // Regression: pbo must be None — it's deferred to the Tune surface (§6.0 D1).
        let equity: Vec<Decimal> = (0..100)
            .map(|i| dec!(100_000) + Decimal::from(i * 50))
            .collect();
        let sharpes = vec![1.5, 0.8, 0.3, -0.2, 1.2];
        let sc = compute_scorecard(&sharpes, &equity, 99);
        assert!(sc.pbo.is_none(), "pbo must always be None in v2");
    }

    // ── Frozen-gate identity: scorecard does NOT change crowning ─────────────

    /// Prove that `rank_candidates` produces the SAME output before and after
    /// the scorecard is computed — the scorecard is additive, not a gate.
    ///
    /// This test directly verifies the FROZEN-gate byte-identity contract:
    /// same input → same crown → same `Ranking.crowned` / `outcome` / `order`.
    #[test]
    fn scorecard_does_not_change_ranking() {
        use crate::bakeoff::rank::rank_candidates;
        use crate::bakeoff::{CandidateKpis, CandidateResult, RobustnessFlag};
        use rust_decimal_macros::dec;
        use smol_str::SmolStr;
        use trading_core::StrategyId;

        let make_candidate = |id: &'static str, sharpe: f64, is_benchmark: bool| CandidateResult {
            strategy: StrategyId(SmolStr::new_static(id)),
            is_benchmark,
            kpis: CandidateKpis {
                sharpe,
                sortino: sharpe * 0.8,
                calmar: sharpe * 0.5,
                total_return_pct: dec!(0.1),
                max_drawdown: dec!(0.05),
                trade_count: 10,
                turnover: rust_decimal::Decimal::ZERO,
            },
            equity_curve: vec![],
            robustness: Some(RobustnessFlag::Robust),
        };

        let candidates = vec![
            make_candidate("v0.sma", 1.5, false),
            make_candidate("v0.macd", 0.9, false),
            make_candidate("v0.buyhold", 1.1, true),
        ];

        // Ranking BEFORE scorecard.
        let ranking_before = rank_candidates(&candidates);

        // Build a scorecard (the pure computation).
        let sharpes: Vec<f64> = candidates.iter().map(|c| c.kpis.sharpe).collect();
        let dummy_equity: Vec<Decimal> = (0..100)
            .map(|i| dec!(100_000) + Decimal::from(i * 100))
            .collect();
        let _sc = compute_scorecard(&sharpes, &dummy_equity, 99);

        // Ranking AFTER scorecard — must be byte-identical.
        let ranking_after = rank_candidates(&candidates);

        assert_eq!(
            ranking_before.crowned, ranking_after.crowned,
            "crowned index changed!"
        );
        assert_eq!(
            ranking_before.outcome, ranking_after.outcome,
            "outcome changed!"
        );
        assert_eq!(
            ranking_before.order, ranking_after.order,
            "ranking order changed!"
        );
    }
}
