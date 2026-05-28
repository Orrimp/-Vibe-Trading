//! Markov-switching 4-state regime classifier (Hamilton 1989 + ADR-0049 § D1).
//!
//! ## Design contract
//!
//! This module provides:
//!
//! - [`RegimeClassifier`] — the **frozen v0.1.0 trait seam** for regime
//!   classification.  v0.2.0+ alternate model classes (e.g. DL classifiers)
//!   MUST satisfy this trait without amendment.  Two methods:
//!   [`RegimeClassifier::fit`] and [`RegimeClassifier::forward_filter`].
//!
//! - [`MarkovSwitchingClassifier`] — the Hamilton 1989 Markov-switching
//!   regression impl.  4 states with operator-set semantic priors; Baum-Welch
//!   EM refines parameter *values* only (no post-hoc state-label reassignment).
//!
//! ## State semantics (ADR-0049 § D1)
//!
//! | Index | Regime   | μ_s prior            | σ²_s prior          |
//! |-------|----------|----------------------|---------------------|
//! | 0     | Bull     | +1e-4 (≈+0.01%/h)    | 25th-pctile var     |
//! | 1     | Bear     | −1e-4                | 25th-pctile var     |
//! | 2     | Volatile | 0                    | 90th-pctile var     |
//! | 3     | Calm     | 0                    | 10th-pctile var     |
//!
//! ## EM convergence contract (ADR-0049 § D1)
//!
//! Δ log-likelihood ≤ 1e-6 over 5 consecutive iterations; max 200 iterations.
//! Failure → [`RegimeError::ConvergenceFailed`] (V-REG-1 upstream signal).
//!
//! ## Determinism contract
//!
//! [`MarkovSwitchingClassifier::forward_filter`] is a pure function given
//! the fitted parameters.  Two calls on identical input produce byte-identical
//! output.  No `std::time`, no unseeded RNG, no global state.
//!
//! ## Performance contract (K3 — ADR-0049 § D1 footnote)
//!
//! The forward filter is O(K² × T) with K=4.  Expected latency < 1 µs/bar
//! on Apple Silicon at hourly cadence.  If a future K or T increase busts the
//! 1 ms/bar p99 budget, fall back to a **24-bar cached cadence**: call
//! `forward_filter` once per 24 bars and cache the last result.  The trait
//! does NOT enforce a cadence policy — that is the caller's responsibility.
//!
//! ## References
//!
//! - Hamilton, J.D. (1989). *A new approach to the economic analysis of
//!   nonstationary time series and the business cycle*. Econometrica 57(2).
//! - ADR-0049 § D1-D6 — authoritative source for all locked values.

use thiserror::Error;
use tracing::{debug, warn};

// ── Error type ────────────────────────────────────────────────────────────────

/// Errors from the Markov-switching classifier.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum RegimeError {
    /// Not enough data to fit (need ≥ [`MIN_FIT_BARS`] bars).
    #[error("insufficient returns: need ≥ {min}, got {got}")]
    InsufficientData { min: usize, got: usize },

    /// EM did not converge within `max_iters` iterations.
    ///
    /// This is the V-REG-1 upstream signal per ADR-0049 § D4.
    #[error(
        "EM convergence failed after {iters} iterations \
         (last Δ log-lik = {delta:.2e}); V-REG-1"
    )]
    ConvergenceFailed { iters: usize, delta: f64 },

    /// A numerical instability was encountered (e.g. NaN in filter).
    #[error("numerical instability: {detail}")]
    NumericalInstability { detail: String },

    /// The model has not been fitted yet.
    #[error("classifier is not fitted — call fit() first")]
    NotFitted,
}

// ── Constants ─────────────────────────────────────────────────────────────────

/// Number of regime states (fixed at 4 for v0.1.0).
pub const N_STATES: usize = 4;

/// Minimum number of bars required to fit the model.
pub const MIN_FIT_BARS: usize = 50;

/// EM convergence tolerance: Δ log-likelihood per iteration.
const EM_CONV_TOL: f64 = 1e-6;

/// Number of consecutive iterations below tolerance required to declare convergence.
///
/// Matching ADR-0049 § D1: "Δ log-likelihood ≤ 1e-6 over 5 consecutive iters".
const EM_CONV_WINDOW: usize = 5;

/// Minimum sample variance of log-returns required to attempt fitting.
///
/// If the data has near-zero variance (flat / constant series), the
/// Markov-switching model is unidentifiable (all state emissions are
/// identical for every bar) → immediate V-REG-1.
const MIN_SAMPLE_VARIANCE: f64 = 1e-14;

/// Maximum EM iterations.
const EM_MAX_ITERS: usize = 200;

/// Floor for variance parameters (avoids division by zero / log(0)).
const VAR_FLOOR: f64 = 1e-12;

/// Confidence threshold for the regime dispatcher gate (ADR-0049 § D6).
///
/// A regime switch is only applied when `max_p ≥ CONFIDENCE_THRESHOLD`.
/// Below this value, the previous regime's strategy keeps running.
pub const CONFIDENCE_THRESHOLD: f64 = 0.70;

// ── RegimeProbability ─────────────────────────────────────────────────────────

/// Per-bar posterior probability over the 4 regime states.
///
/// Produced by [`RegimeClassifier::forward_filter`] — one entry per input bar.
///
/// The probabilities sum to 1.0 (within floating-point precision).
///
/// ## State index mapping (ADR-0049 § D1)
///
/// ```text
/// p[0] = P(Bull  | history)
/// p[1] = P(Bear  | history)
/// p[2] = P(Volatile | history)
/// p[3] = P(Calm  | history)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RegimeProbability {
    /// Posterior probabilities `[P(Bull), P(Bear), P(Volatile), P(Calm)]`.
    pub p: [f64; N_STATES],
}

impl RegimeProbability {
    /// Return the index of the most probable state.
    #[must_use]
    pub fn argmax(&self) -> usize {
        self.p
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Return the maximum posterior probability (confidence).
    #[must_use]
    pub fn max_confidence(&self) -> f64 {
        self.p.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    }

    /// Return true if the max confidence exceeds the dispatcher gate threshold.
    ///
    /// Per ADR-0049 § D6: `max_p ≥ 0.70` required to trigger a strategy switch.
    #[must_use]
    pub fn above_confidence_threshold(&self) -> bool {
        self.max_confidence() >= CONFIDENCE_THRESHOLD
    }

    /// Returns the name of the most probable regime.
    ///
    /// Maps: 0 → "bull", 1 → "bear", 2 → "volatile", 3 → "calm".
    #[must_use]
    pub fn regime_name(&self) -> &'static str {
        match self.argmax() {
            0 => "bull",
            1 => "bear",
            2 => "volatile",
            3 => "calm",
            _ => "unknown",
        }
    }
}

// ── RegimeClassifier trait ─────────────────────────────────────────────────────

/// **Frozen v0.1.0 trait seam** for regime classifiers.
///
/// v0.2.0+ alternate implementations (deep-learning classifiers, etc.) MUST
/// implement this trait without amending it.  The contract is intentionally
/// small so that it remains forward-compatible.
///
/// ## Method contract
///
/// - [`fit`]: train the model on a slice of log-returns.  Must be called before
///   [`forward_filter`].  Re-fitting on the same input produces identical
///   results (determinism contract).
///
/// - [`forward_filter`]: run the Hamilton forward filter over a history of
///   log-returns, emitting per-bar posterior probabilities.  Pure function
///   given the fitted parameters — no side effects, no I/O.
///
/// ## Error semantics
///
/// Both methods return `Result<_, RegimeError>`.  [`RegimeError::NotFitted`]
/// is the sentinel for calling `forward_filter` before `fit`.
/// [`RegimeError::ConvergenceFailed`] is the V-REG-1 upstream signal.
pub trait RegimeClassifier {
    /// Fit the classifier on `log_returns`.
    ///
    /// After a successful call the classifier holds trained parameters and
    /// `forward_filter` may be called.
    ///
    /// # Arguments
    ///
    /// - `log_returns`: slice of hourly log-returns `r_t = ln(p_t / p_{t-1})`.
    ///   Must contain ≥ [`MIN_FIT_BARS`] values.
    ///
    /// # Errors
    ///
    /// - [`RegimeError::InsufficientData`] — too few bars.
    /// - [`RegimeError::ConvergenceFailed`] — EM hit `max_iters` (V-REG-1).
    /// - [`RegimeError::NumericalInstability`] — NaN/Inf during EM.
    fn fit(&mut self, log_returns: &[f64]) -> Result<(), RegimeError>;

    /// Run the forward filter over `history` and return per-bar posteriors.
    ///
    /// The length of the returned vector equals `history.len()`.  Each
    /// [`RegimeProbability`] sums to 1.0 within floating-point precision.
    ///
    /// This is a pure function given the fitted parameters: two calls with
    /// identical `history` on the same fitted model produce byte-identical
    /// output.
    ///
    /// # Errors
    ///
    /// - [`RegimeError::NotFitted`] — `fit()` has not been called yet.
    /// - [`RegimeError::NumericalInstability`] — NaN/Inf during filter.
    fn forward_filter(&self, history: &[f64]) -> Result<Vec<RegimeProbability>, RegimeError>;
}

// ── MarkovSwitchingParams ─────────────────────────────────────────────────────

/// Fitted parameters of the Markov-switching model.
///
/// All fields are in natural (not transformed) space.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkovSwitchingParams {
    /// State mean log-returns `μ_s` for s ∈ {Bull, Bear, Volatile, Calm}.
    pub mu: [f64; N_STATES],
    /// State variances `σ²_s` (not σ; variance).
    pub sigma2: [f64; N_STATES],
    /// Row-stochastic 4×4 transition matrix `P[i][j] = P(s_{t+1}=j | s_t=i)`.
    ///
    /// Stored row-major: `transition[i * N_STATES + j]`.
    pub transition: [f64; N_STATES * N_STATES],
    /// Stationary initial distribution `π_s`.
    pub initial: [f64; N_STATES],
    /// Log-likelihood at convergence (positive convention; higher is better).
    pub log_likelihood: f64,
    /// Number of EM iterations taken.
    pub n_iters: usize,
    /// Whether EM converged within [`EM_MAX_ITERS`].
    pub converged: bool,
}

// ── MarkovSwitchingClassifier ─────────────────────────────────────────────────

/// Hamilton (1989) 4-state Markov-switching regression classifier.
///
/// ## Prior specification (ADR-0049 § D1)
///
/// State identities are locked at construction via `operator-set semantic
/// priors`.  Baum-Welch refines the parameter *values* only; the ordering
/// Bull=0, Bear=1, Volatile=2, Calm=3 is NEVER reassigned post-hoc.
///
/// ## Construction
///
/// Use [`MarkovSwitchingClassifier::new`].  The prior variance percentiles
/// are estimated from the training data at `fit()` time.
///
/// ## Thread safety
///
/// `MarkovSwitchingClassifier` is `Send + Sync` — only plain numeric data.
pub struct MarkovSwitchingClassifier {
    /// Fitted parameters, or `None` if `fit()` has not been called.
    params: Option<MarkovSwitchingParams>,
}

impl MarkovSwitchingClassifier {
    /// Construct a new, unfitted classifier.
    ///
    /// Call [`RegimeClassifier::fit`] before calling
    /// [`RegimeClassifier::forward_filter`].
    #[must_use]
    pub fn new() -> Self {
        Self { params: None }
    }

    /// Return the fitted parameters, if available.
    #[must_use]
    pub fn params(&self) -> Option<&MarkovSwitchingParams> {
        self.params.as_ref()
    }
}

impl Default for MarkovSwitchingClassifier {
    fn default() -> Self {
        Self::new()
    }
}

// ── RegimeClassifier impl ─────────────────────────────────────────────────────

impl RegimeClassifier for MarkovSwitchingClassifier {
    fn fit(&mut self, log_returns: &[f64]) -> Result<(), RegimeError> {
        let n = log_returns.len();
        if n < MIN_FIT_BARS {
            return Err(RegimeError::InsufficientData {
                min: MIN_FIT_BARS,
                got: n,
            });
        }

        // ── Step 1a: degenerate-input guard ──────────────────────────────────
        // If the data has near-zero variance the Markov-switching model is
        // unidentifiable: all state emissions are identical for every bar and
        // the EM cannot distinguish regimes.  Fail fast with V-REG-1.
        let sample_var = {
            let nf = n as f64;
            let mean = log_returns.iter().sum::<f64>() / nf;
            log_returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / nf
        };
        if sample_var < MIN_SAMPLE_VARIANCE {
            return Err(RegimeError::ConvergenceFailed {
                iters: 0,
                delta: f64::NAN,
            });
        }

        // ── Step 1b: compute percentile-based variance priors ─────────────────
        let var_priors = compute_variance_priors(log_returns, sample_var)?;

        // ── Step 2: initialise parameters from priors ─────────────────────────
        let mut mu = [1e-4_f64, -1e-4_f64, 0.0_f64, 0.0_f64];
        let mut sigma2 = var_priors; // [25th, 25th, 90th, 10th]

        // Row-stochastic transition matrix: initialise close to identity
        // (high persistence is a sensible default for financial regimes).
        let mut transition = [0.0_f64; N_STATES * N_STATES];
        for i in 0..N_STATES {
            for j in 0..N_STATES {
                if i == j {
                    transition[i * N_STATES + j] = 0.85;
                } else {
                    transition[i * N_STATES + j] = 0.05;
                }
            }
        }

        // Stationary distribution: uniform start.
        let mut initial = [0.25_f64; N_STATES];

        debug!(
            n_bars = n,
            mu = ?mu,
            sigma2 = ?sigma2,
            "MarkovSwitchingClassifier::fit starting EM"
        );

        // ── Step 3: Baum-Welch EM ──────────────────────────────────────────────
        let mut prev_ll = f64::NEG_INFINITY;
        let mut converged = false;
        let mut n_iters = 0usize;
        let mut consecutive_below_tol = 0usize;
        let mut last_delta = 0.0_f64;

        for iter in 0..EM_MAX_ITERS {
            n_iters = iter + 1;

            // E-step: run forward-backward algorithm.
            let (alpha_mat, c_vec) =
                forward_pass(log_returns, &mu, &sigma2, &transition, &initial)?;
            let beta_mat = backward_pass(log_returns, &mu, &sigma2, &transition, &c_vec)?;
            let (gamma, xi) = compute_posteriors(
                &alpha_mat,
                &beta_mat,
                log_returns,
                &mu,
                &sigma2,
                &transition,
            )?;

            // Log-likelihood: sum of log scaling factors from forward pass.
            let ll: f64 = c_vec.iter().map(|c| c.ln()).sum();

            let delta = (ll - prev_ll).abs();
            last_delta = delta;

            if delta < EM_CONV_TOL {
                consecutive_below_tol += 1;
                if consecutive_below_tol >= EM_CONV_WINDOW {
                    converged = true;
                    debug!(iter, ll, delta, "MarkovSwitchingClassifier: EM converged");
                    break;
                }
            } else {
                consecutive_below_tol = 0;
            }
            prev_ll = ll;

            // M-step: update parameters (NO state-label reassignment).
            m_step_update(
                log_returns,
                &gamma,
                &xi,
                &mut mu,
                &mut sigma2,
                &mut transition,
                &mut initial,
            );
        }

        if !converged {
            warn!(
                iters = n_iters,
                last_delta, "MarkovSwitchingClassifier: EM did not converge (V-REG-1)"
            );
            return Err(RegimeError::ConvergenceFailed {
                iters: n_iters,
                delta: last_delta,
            });
        }

        // Final log-likelihood (positive convention: higher is better).
        let (_, c_vec_final) = forward_pass(log_returns, &mu, &sigma2, &transition, &initial)?;
        let final_ll: f64 = c_vec_final.iter().map(|c| c.ln()).sum();

        self.params = Some(MarkovSwitchingParams {
            mu,
            sigma2,
            transition,
            initial,
            log_likelihood: final_ll,
            n_iters,
            converged,
        });

        debug!(
            n_iters,
            log_likelihood = final_ll,
            "MarkovSwitchingClassifier: fit complete"
        );
        Ok(())
    }

    fn forward_filter(&self, history: &[f64]) -> Result<Vec<RegimeProbability>, RegimeError> {
        let params = self.params.as_ref().ok_or(RegimeError::NotFitted)?;

        if history.is_empty() {
            return Ok(Vec::new());
        }

        let (alpha_mat, _) = forward_pass(
            history,
            &params.mu,
            &params.sigma2,
            &params.transition,
            &params.initial,
        )?;

        // alpha_mat rows are already normalised (each row sums to 1.0).
        let result = alpha_mat
            .into_iter()
            .map(|row| RegimeProbability { p: row })
            .collect();

        Ok(result)
    }
}

// ── Pure helper functions ─────────────────────────────────────────────────────

/// Compute the 4 variance priors from the training data (ADR-0049 § D1).
///
/// Returns `[σ²_Bull, σ²_Bear, σ²_Volatile, σ²_Calm]` where:
/// - Bull / Bear = 25th-percentile realised variance (low, trending)
/// - Volatile    = 90th-percentile realised variance (high)
/// - Calm        = 10th-percentile realised variance (very low)
///
/// The percentiles are computed from the squared demeaned returns.
/// To ensure the initial values are always in the right order of magnitude
/// for Baum-Welch, each percentile value is **also floored** at a fraction
/// of `sample_var` (the unconditional variance of the series):
///
/// - floor_low  = max(pct, sample_var × 0.005)  → avoids collapse to VAR_FLOOR
/// - floor_high = max(pct, sample_var × 0.5)    → for Volatile only
///
/// This ensures the initial σ² values bracket the sample variance so that
/// all four states have non-negligible emission probability on at least
/// some subset of training bars — necessary for Baum-Welch to bootstrap.
fn compute_variance_priors(
    log_returns: &[f64],
    sample_var: f64,
) -> Result<[f64; N_STATES], RegimeError> {
    // Squared demeaned returns as variance proxy.
    let n = log_returns.len() as f64;
    let mean = log_returns.iter().sum::<f64>() / n;
    let mut sq_demeaned: Vec<f64> = log_returns.iter().map(|r| (r - mean).powi(2)).collect();
    sq_demeaned.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let p10 = percentile_sorted(&sq_demeaned, 10.0);
    let p25 = percentile_sorted(&sq_demeaned, 25.0);
    let p90 = percentile_sorted(&sq_demeaned, 90.0);

    // Sanity: all must be finite and non-negative.
    for v in [p10, p25, p90] {
        if !v.is_finite() {
            return Err(RegimeError::NumericalInstability {
                detail: format!("variance prior is non-finite: {v}"),
            });
        }
    }

    // Floor each percentile at a fraction of sample_var so that the initial
    // σ² values are in the right neighbourhood for Baum-Welch to bootstrap.
    // Without this floor, p25 may be orders-of-magnitude below the actual
    // variance of trending bars, causing near-zero emission likelihoods.
    let p10 = p10.max(sample_var * 0.002).max(VAR_FLOOR);
    let p25 = p25.max(sample_var * 0.05).max(VAR_FLOOR);
    let p90 = p90.max(sample_var * 0.5).max(VAR_FLOOR);

    // [Bull=p25, Bear=p25, Volatile=p90, Calm=p10]
    Ok([p25, p25, p90, p10])
}

/// Interpolated percentile from a pre-sorted slice.
fn percentile_sorted(sorted: &[f64], pct: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if n == 1 {
        return sorted[0];
    }
    let idx_f = (pct / 100.0) * (n - 1) as f64;
    let lo = idx_f.floor() as usize;
    let hi = (lo + 1).min(n - 1);
    let frac = idx_f - lo as f64;
    sorted[lo] + frac * (sorted[hi] - sorted[lo])
}

/// Gaussian emission density N(r | μ_s, σ²_s).
///
/// Returns the pdf value (not log) — clipped to VAR_FLOOR to avoid
/// numerical zero.
#[inline]
fn gaussian_pdf(r: f64, mu: f64, sigma2: f64) -> f64 {
    let s2 = sigma2.max(VAR_FLOOR);
    let diff = r - mu;
    let exponent = -0.5 * diff * diff / s2;
    let coeff = (2.0 * std::f64::consts::PI * s2).sqrt();
    // exp underflow gives 0.0 which is fine — scaled forward probabilities
    // handle this via the c_t scaling factor.
    (exponent.exp() / coeff).max(0.0)
}

/// Forward pass (Hamilton 1989 eqs 22.4.5–22.4.7) with scaling.
///
/// Returns:
/// - `alpha_mat[t]` = normalised filtered probabilities `P(s_t = i | y_{1:t})`.
/// - `c_vec[t]` = scaling factor at step t (product = likelihood).
///
/// The scaling prevents underflow on long sequences.
fn forward_pass(
    returns: &[f64],
    mu: &[f64; N_STATES],
    sigma2: &[f64; N_STATES],
    transition: &[f64; N_STATES * N_STATES],
    initial: &[f64; N_STATES],
) -> Result<(Vec<[f64; N_STATES]>, Vec<f64>), RegimeError> {
    let t_len = returns.len();
    let mut alpha_mat = vec![[0.0_f64; N_STATES]; t_len];
    let mut c_vec = vec![0.0_f64; t_len];

    // t = 0: α_0(i) ∝ π_i × b_i(r_0)
    let mut alpha = [0.0_f64; N_STATES];
    for i in 0..N_STATES {
        alpha[i] = initial[i] * gaussian_pdf(returns[0], mu[i], sigma2[i]);
    }
    let c0: f64 = alpha.iter().sum();
    if c0 <= 0.0 || !c0.is_finite() {
        // Pathological: no state can explain the first observation.
        // Fall back to uniform to avoid numerical collapse.
        for a in alpha.iter_mut() {
            *a = 0.25;
        }
        c_vec[0] = 1.0_f64 / 4.0;
    } else {
        for a in alpha.iter_mut() {
            *a /= c0;
        }
        c_vec[0] = c0;
    }
    alpha_mat[0] = alpha;

    // t = 1..T-1
    for t in 1..t_len {
        let prev = alpha_mat[t - 1];
        let mut alpha_new = [0.0_f64; N_STATES];

        for j in 0..N_STATES {
            // Sum over previous states: Σ_i α_{t-1}(i) × P(i→j)
            let predicted: f64 = (0..N_STATES)
                .map(|i| prev[i] * transition[i * N_STATES + j])
                .sum();
            alpha_new[j] = predicted * gaussian_pdf(returns[t], mu[j], sigma2[j]);
        }

        let ct: f64 = alpha_new.iter().sum();
        if ct <= 0.0 || !ct.is_finite() {
            // Cannot explain this bar: fall back to prediction without update.
            let mut pred_sum = [0.0_f64; N_STATES];
            for j in 0..N_STATES {
                pred_sum[j] = (0..N_STATES)
                    .map(|i| prev[i] * transition[i * N_STATES + j])
                    .sum();
            }
            let ps: f64 = pred_sum.iter().sum();
            if ps <= 0.0 {
                for a in alpha_new.iter_mut() {
                    *a = 1.0 / N_STATES as f64;
                }
                c_vec[t] = 1.0 / N_STATES as f64;
            } else {
                for j in 0..N_STATES {
                    alpha_new[j] = pred_sum[j] / ps;
                }
                c_vec[t] = ps;
            }
        } else {
            for a in alpha_new.iter_mut() {
                *a /= ct;
            }
            c_vec[t] = ct;
        }

        alpha_mat[t] = alpha_new;
    }

    // Validate: no NaN in alpha_mat.
    for (t, row) in alpha_mat.iter().enumerate() {
        for &v in row.iter() {
            if !v.is_finite() {
                return Err(RegimeError::NumericalInstability {
                    detail: format!("NaN/Inf in forward pass at t={t}"),
                });
            }
        }
    }

    Ok((alpha_mat, c_vec))
}

/// Backward pass (Hamilton 1989) with the same scaling sequence as the
/// forward pass.
///
/// Returns `beta_mat[t]` — scaled backward probabilities.
fn backward_pass(
    returns: &[f64],
    mu: &[f64; N_STATES],
    sigma2: &[f64; N_STATES],
    transition: &[f64; N_STATES * N_STATES],
    c_vec: &[f64],
) -> Result<Vec<[f64; N_STATES]>, RegimeError> {
    let t_len = returns.len();
    let mut beta_mat = vec![[0.0_f64; N_STATES]; t_len];

    // t = T-1: β_{T-1}(i) = 1 (unscaled) / c_{T-1}
    let init_beta = 1.0 / c_vec[t_len - 1].max(VAR_FLOOR);
    for b in beta_mat[t_len - 1].iter_mut() {
        *b = init_beta;
    }

    // t = T-2..0 (backward)
    if t_len > 1 {
        for t in (0..t_len - 1).rev() {
            let beta_next = beta_mat[t + 1];
            let mut beta_new = [0.0_f64; N_STATES];

            for i in 0..N_STATES {
                beta_new[i] = (0..N_STATES)
                    .map(|j| {
                        transition[i * N_STATES + j]
                            * gaussian_pdf(returns[t + 1], mu[j], sigma2[j])
                            * beta_next[j]
                    })
                    .sum();
            }

            // Scale by 1/c_t.
            let scale = 1.0 / c_vec[t].max(VAR_FLOOR);
            for b in beta_new.iter_mut() {
                *b *= scale;
            }

            beta_mat[t] = beta_new;
        }
    }

    // Validate.
    for (t, row) in beta_mat.iter().enumerate() {
        for &v in row.iter() {
            if !v.is_finite() {
                return Err(RegimeError::NumericalInstability {
                    detail: format!("NaN/Inf in backward pass at t={t}"),
                });
            }
        }
    }

    Ok(beta_mat)
}

/// Compute γ (state posteriors) and ξ (transition posteriors) from
/// α and β.
///
/// - `gamma[t][i]` = P(s_t = i | y_{1:T})
/// - `xi[t][i][j]` = P(s_t=i, s_{t+1}=j | y_{1:T})
///   stored as `xi[t * N_STATES * N_STATES + i * N_STATES + j]`
fn compute_posteriors(
    alpha_mat: &[[f64; N_STATES]],
    beta_mat: &[[f64; N_STATES]],
    returns: &[f64],
    mu: &[f64; N_STATES],
    sigma2: &[f64; N_STATES],
    transition: &[f64; N_STATES * N_STATES],
) -> Result<(Vec<[f64; N_STATES]>, Vec<f64>), RegimeError> {
    let t_len = returns.len();

    // γ[t][i] = α[t][i] × β[t][i] / Σ_i (α[t][i] × β[t][i])
    let mut gamma = vec![[0.0_f64; N_STATES]; t_len];
    for t in 0..t_len {
        let mut unnorm = [0.0_f64; N_STATES];
        for i in 0..N_STATES {
            unnorm[i] = alpha_mat[t][i] * beta_mat[t][i];
        }
        let sum: f64 = unnorm.iter().sum();
        if sum > 0.0 && sum.is_finite() {
            for i in 0..N_STATES {
                gamma[t][i] = unnorm[i] / sum;
            }
        } else {
            // Degenerate: uniform.
            let uniform = 1.0 / N_STATES as f64;
            for g in gamma[t].iter_mut() {
                *g = uniform;
            }
        }
    }

    // ξ[t][i][j] proportional to α[t][i] × P(i→j) × b_j(r_{t+1}) × β[t+1][j]
    let xi_len = (t_len.saturating_sub(1)) * N_STATES * N_STATES;
    let mut xi = vec![0.0_f64; xi_len];

    for t in 0..t_len.saturating_sub(1) {
        let mut unnorm = [0.0_f64; N_STATES * N_STATES];
        for i in 0..N_STATES {
            for j in 0..N_STATES {
                unnorm[i * N_STATES + j] = alpha_mat[t][i]
                    * transition[i * N_STATES + j]
                    * gaussian_pdf(returns[t + 1], mu[j], sigma2[j])
                    * beta_mat[t + 1][j];
            }
        }
        let sum: f64 = unnorm.iter().sum();
        let base = t * N_STATES * N_STATES;
        if sum > 0.0 && sum.is_finite() {
            for k in 0..N_STATES * N_STATES {
                xi[base + k] = unnorm[k] / sum;
            }
        } else {
            let uniform = 1.0 / (N_STATES * N_STATES) as f64;
            for k in 0..N_STATES * N_STATES {
                xi[base + k] = uniform;
            }
        }
    }

    Ok((gamma, xi))
}

/// M-step: update μ, σ², transition matrix, and initial distribution.
///
/// **Crucially, state ordering is NOT reassigned** (ADR-0049 § D1).
/// The update equations are the standard Baum-Welch M-step for a
/// Gaussian emission HMM.
fn m_step_update(
    returns: &[f64],
    gamma: &[[f64; N_STATES]],
    xi: &[f64],
    mu: &mut [f64; N_STATES],
    sigma2: &mut [f64; N_STATES],
    transition: &mut [f64; N_STATES * N_STATES],
    initial: &mut [f64; N_STATES],
) {
    let t_len = returns.len();

    // Update initial distribution π_i = γ[0][i].
    *initial = gamma[0];

    // Update μ_s and σ²_s.
    for s in 0..N_STATES {
        let denom: f64 = gamma.iter().map(|g| g[s]).sum();
        if denom > VAR_FLOOR {
            // μ_s = Σ_t γ[t][s] × r_t / Σ_t γ[t][s]
            let new_mu: f64 = gamma
                .iter()
                .zip(returns.iter())
                .map(|(g, &r)| g[s] * r)
                .sum::<f64>()
                / denom;

            // σ²_s = Σ_t γ[t][s] × (r_t - μ_s)² / Σ_t γ[t][s]
            let new_sigma2: f64 = gamma
                .iter()
                .zip(returns.iter())
                .map(|(g, &r)| {
                    let diff = r - new_mu;
                    g[s] * diff * diff
                })
                .sum::<f64>()
                / denom;

            mu[s] = new_mu;
            sigma2[s] = new_sigma2.max(VAR_FLOOR);
        }
        // If denom ≤ VAR_FLOOR, the state is unpopulated — keep prior.
    }

    // Update transition matrix.
    // P(i→j) = Σ_{t=0}^{T-2} ξ[t][i][j] / Σ_{t=0}^{T-2} γ[t][i]
    for i in 0..N_STATES {
        let row_denom: f64 = (0..t_len.saturating_sub(1))
            .map(|t| gamma[t][i])
            .sum::<f64>();

        if row_denom > VAR_FLOOR {
            for j in 0..N_STATES {
                let xi_sum: f64 = (0..t_len.saturating_sub(1))
                    .map(|t| xi[t * N_STATES * N_STATES + i * N_STATES + j])
                    .sum();
                transition[i * N_STATES + j] = (xi_sum / row_denom).max(VAR_FLOOR);
            }
            // Re-normalise row to sum = 1.
            let row_sum: f64 = (0..N_STATES).map(|j| transition[i * N_STATES + j]).sum();
            if row_sum > VAR_FLOOR {
                for j in 0..N_STATES {
                    transition[i * N_STATES + j] /= row_sum;
                }
            }
        }
        // If row_denom ≤ VAR_FLOOR, keep prior row.
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── synthetic fixture helpers ──────────────────────────────────────────────

    /// Generate a synthetic 4-regime series with known regime blocks.
    ///
    /// Regime schedule (repeating, `block_len` bars each):
    ///   Bull: μ=+5e-3, σ=5e-3
    ///   Bear: μ=−5e-3, σ=5e-3
    ///   Volatile: μ=0, σ=3e-2
    ///   Calm: μ=0, σ=5e-4
    ///
    /// Uses a deterministic linear-congruential generator so there is NO
    /// dependency on `rand` — stays pure, reproducible, zero heap beyond Vec.
    fn synthetic_4regime(block_len: usize, n_cycles: usize) -> Vec<f64> {
        let mut series = Vec::new();
        // LCG state (Knuth constants).
        let mut state: u64 = 12345678901234567_u64;
        let next = |s: &mut u64| -> f64 {
            *s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            // Box-Muller: use two consecutive calls.
            let u1 = (*s >> 11) as f64 / (1u64 << 53) as f64;
            *s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let u2 = (*s >> 11) as f64 / (1u64 << 53) as f64;
            let u1 = u1.max(1e-15); // guard log(0)
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        };

        let regimes: [(f64, f64); 4] = [
            (5e-3, 5e-3),  // Bull
            (-5e-3, 5e-3), // Bear
            (0.0, 3e-2),   // Volatile
            (0.0, 5e-4),   // Calm
        ];

        for _ in 0..n_cycles {
            for (mu, sigma) in regimes.iter() {
                for _ in 0..block_len {
                    series.push(mu + sigma * next(&mut state));
                }
            }
        }
        series
    }

    /// Generate a flat constant series (all zeros) — pathological input.
    fn flat_series(n: usize) -> Vec<f64> {
        vec![0.0_f64; n]
    }

    // ── test: priors lock regime identities ───────────────────────────────────

    /// T-D-A3 / ADR-0049 § D1: after fit, μ_Bull > 0, μ_Bear < 0,
    /// σ²_Volatile > σ²_Calm.
    ///
    /// K-reg "labels don't drift" gate.
    #[test]
    fn priors_lock_regime_identities() {
        let returns = synthetic_4regime(100, 3);
        let mut clf = MarkovSwitchingClassifier::new();
        clf.fit(&returns)
            .expect("fit should succeed on 4-regime fixture");

        let p = clf.params().expect("params must be set after fit");

        // Bull (index 0): μ must be positive.
        assert!(
            p.mu[0] > 0.0,
            "Bull μ must be positive after fit, got {}",
            p.mu[0]
        );
        // Bear (index 1): μ must be negative.
        assert!(
            p.mu[1] < 0.0,
            "Bear μ must be negative after fit, got {}",
            p.mu[1]
        );
        // Volatile (index 2) σ² > Calm (index 3) σ².
        assert!(
            p.sigma2[2] > p.sigma2[3],
            "Volatile σ² ({}) must exceed Calm σ² ({})",
            p.sigma2[2],
            p.sigma2[3]
        );

        // Transition matrix rows sum to 1.
        for i in 0..N_STATES {
            let row_sum: f64 = (0..N_STATES).map(|j| p.transition[i * N_STATES + j]).sum();
            assert!(
                (row_sum - 1.0).abs() < 1e-9,
                "transition row {i} sum = {row_sum} (expected 1.0)"
            );
        }
    }

    // ── test: EM converges on synthetic data ──────────────────────────────────

    /// T-D-A3 / ADR-0049 § D1: EM converges within 200 iters;
    /// Δ log-lik ≤ 1e-6 criterion met.
    #[test]
    fn em_converges_on_synthetic() {
        let returns = synthetic_4regime(100, 3);
        let mut clf = MarkovSwitchingClassifier::new();
        clf.fit(&returns)
            .expect("EM must converge on synthetic 4-regime series");

        let p = clf.params().unwrap();
        assert!(
            p.converged,
            "params.converged must be true after successful fit"
        );
        assert!(
            p.n_iters <= EM_MAX_ITERS,
            "n_iters {} exceeds EM_MAX_ITERS {}",
            p.n_iters,
            EM_MAX_ITERS
        );
        assert!(
            p.log_likelihood.is_finite(),
            "log_likelihood must be finite"
        );
    }

    // ── test: EM fails loudly on pathological data ────────────────────────────

    /// T-D-A3 / ADR-0049 § D4 V-REG-1: flat-constant series should return
    /// ConvergenceFailed (NOT panic, NOT silently-bad fit).
    ///
    /// A flat series has zero variance — the M-step variance update collapses
    /// to VAR_FLOOR for all states, which means Δ log-lik never falls below
    /// EM_CONV_TOL × EM_CONV_WINDOW in 200 iters.
    #[test]
    fn em_fails_loudly_on_pathological_data() {
        let returns = flat_series(300);
        let mut clf = MarkovSwitchingClassifier::new();
        let err = clf
            .fit(&returns)
            .expect_err("flat series should fail EM convergence");

        assert!(
            matches!(err, RegimeError::ConvergenceFailed { .. }),
            "expected ConvergenceFailed, got {err:?}"
        );
    }

    // ── test: forward_filter emits 4 probabilities summing to 1 ──────────────

    /// T-D-A3 / ADR-0049 § D1: per-bar posteriors sum to 1.0 within 1e-9.
    #[test]
    fn forward_filter_emits_4_probabilities() {
        let returns = synthetic_4regime(100, 3);
        let history = synthetic_4regime(50, 1);

        let mut clf = MarkovSwitchingClassifier::new();
        clf.fit(&returns).expect("fit must succeed");

        let posteriors = clf
            .forward_filter(&history)
            .expect("forward_filter must succeed");

        assert_eq!(
            posteriors.len(),
            history.len(),
            "output length must equal history length"
        );

        for (t, p) in posteriors.iter().enumerate() {
            assert_eq!(
                p.p.len(),
                N_STATES,
                "each RegimeProbability must have {} states",
                N_STATES
            );
            let sum: f64 = p.p.iter().sum();
            assert!(
                (sum - 1.0).abs() < 1e-9,
                "bar {t}: probabilities sum to {sum} (expected 1.0)"
            );
            for (s, &prob) in p.p.iter().enumerate() {
                assert!(
                    prob >= 0.0,
                    "bar {t}, state {s}: probability {prob} must be ≥ 0"
                );
            }
        }
    }

    // ── test: regime confidence threshold gates dispatcher ────────────────────

    /// T-D-A3 / ADR-0049 § D6: `above_confidence_threshold()` fires iff
    /// max_p ≥ 0.70.
    #[test]
    fn regime_confidence_threshold_70_gates_dispatcher() {
        // Construct RegimeProbability directly with known values.

        // Below threshold: max = 0.50.
        let low = RegimeProbability {
            p: [0.50, 0.30, 0.10, 0.10],
        };
        assert!(
            !low.above_confidence_threshold(),
            "max_p=0.50 should NOT exceed threshold 0.70"
        );

        // Exactly at threshold: max = 0.70.
        let at = RegimeProbability {
            p: [0.70, 0.15, 0.10, 0.05],
        };
        assert!(
            at.above_confidence_threshold(),
            "max_p=0.70 should be AT or ABOVE threshold 0.70"
        );

        // Above threshold: max = 0.85.
        let above = RegimeProbability {
            p: [0.85, 0.10, 0.03, 0.02],
        };
        assert!(
            above.above_confidence_threshold(),
            "max_p=0.85 should exceed threshold 0.70"
        );

        // Uniform (0.25 each): below threshold.
        let uniform = RegimeProbability {
            p: [0.25, 0.25, 0.25, 0.25],
        };
        assert!(
            !uniform.above_confidence_threshold(),
            "max_p=0.25 (uniform) should NOT exceed threshold 0.70"
        );
    }

    // ── test: trait object dispatch (T-D-A2) ──────────────────────────────────

    /// T-D-A2: trait-object dispatch — `Box<dyn RegimeClassifier>` works.
    #[test]
    fn regime_classifier_is_object_safe() {
        let returns = synthetic_4regime(100, 3);
        let history = synthetic_4regime(20, 1);

        let mut clf: Box<dyn RegimeClassifier> = Box::new(MarkovSwitchingClassifier::new());
        clf.fit(&returns)
            .expect("fit via trait object must succeed");
        let posteriors = clf
            .forward_filter(&history)
            .expect("forward_filter via trait object must succeed");
        assert_eq!(posteriors.len(), history.len());
    }

    // ── test: determinism (two-run byte-identical) ────────────────────────────

    /// T-D-A3: two `forward_filter` calls on identical input produce
    /// byte-identical output.  Pure-function contract.
    #[test]
    fn forward_filter_is_deterministic() {
        let returns = synthetic_4regime(100, 3);
        let history = synthetic_4regime(30, 1);

        let mut clf = MarkovSwitchingClassifier::new();
        clf.fit(&returns).expect("fit must succeed");

        let run1 = clf.forward_filter(&history).expect("forward_filter run 1");
        let run2 = clf.forward_filter(&history).expect("forward_filter run 2");

        assert_eq!(run1.len(), run2.len(), "lengths must match");
        for (t, (p1, p2)) in run1.iter().zip(run2.iter()).enumerate() {
            for s in 0..N_STATES {
                assert_eq!(
                    p1.p[s].to_bits(),
                    p2.p[s].to_bits(),
                    "bar {t}, state {s}: p1={} != p2={} (not byte-identical)",
                    p1.p[s],
                    p2.p[s]
                );
            }
        }
    }

    // ── test: dispatcher confidence gate (D6 falsifiers) ─────────────────────

    /// T-D-A3 / ADR-0049 § D6: dispatcher should NOT switch when
    /// max_p < 0.70.
    #[test]
    fn dispatcher_confidence_gate_zero_when_uncertain() {
        let uncertain = RegimeProbability {
            p: [0.30, 0.30, 0.20, 0.20],
        };
        assert!(
            !uncertain.above_confidence_threshold(),
            "uncertain posterior (max=0.30) must not gate dispatcher"
        );
    }

    /// T-D-A3 / ADR-0049 § D6: dispatcher SHOULD switch when max_p ≥ 0.70.
    #[test]
    fn dispatcher_switches_when_confident() {
        let confident = RegimeProbability {
            p: [0.05, 0.05, 0.80, 0.10],
        };
        assert!(
            confident.above_confidence_threshold(),
            "confident posterior (max=0.80) must gate dispatcher"
        );
        assert_eq!(
            confident.argmax(),
            2,
            "argmax must be 2 (Volatile) for this posterior"
        );
        assert_eq!(confident.regime_name(), "volatile");
    }

    // ── test: K2 switch-rate gate ─────────────────────────────────────────────

    /// T-D-A3 / K2 falsifier: regime switch rate on a well-separated
    /// synthetic series must be ≤ 20/week (≤ 20/168 bars ≈ 0.119/bar).
    ///
    /// We generate a 336-bar (2-week) series and require ≤ 40 switches.
    #[test]
    fn regime_switch_rate_under_threshold() {
        // 4 cycles of 100 bars each = 400 bars (well-separated, clear regimes).
        let returns = synthetic_4regime(100, 3); // 1200 bars training
        let history = synthetic_4regime(84, 1); // 336 bars = 2 weeks

        let mut clf = MarkovSwitchingClassifier::new();
        clf.fit(&returns).expect("fit must succeed");

        let posteriors = clf
            .forward_filter(&history)
            .expect("forward_filter must succeed");

        let mut switches = 0usize;
        let mut prev_regime = posteriors[0].argmax();
        for p in posteriors.iter().skip(1) {
            let cur = p.argmax();
            if cur != prev_regime {
                switches += 1;
            }
            prev_regime = cur;
        }

        // 336 bars = 2 weeks; ≤ 40 switches corresponds to ≤ 20/week.
        let max_switches = 40usize;
        assert!(
            switches <= max_switches,
            "switch rate too high: {switches} switches in 336 bars (limit {max_switches} = 20/week)"
        );
    }

    // ── test: not-fitted error ────────────────────────────────────────────────

    /// forward_filter before fit returns NotFitted.
    #[test]
    fn forward_filter_before_fit_returns_error() {
        let clf = MarkovSwitchingClassifier::new();
        let err = clf
            .forward_filter(&[0.01, 0.02])
            .expect_err("must fail before fit");
        assert!(
            matches!(err, RegimeError::NotFitted),
            "expected NotFitted, got {err:?}"
        );
    }

    // ── test: insufficient data ───────────────────────────────────────────────

    /// fit with too-short series returns InsufficientData.
    #[test]
    fn fit_insufficient_data() {
        let mut clf = MarkovSwitchingClassifier::new();
        let err = clf
            .fit(&[0.01; 10])
            .expect_err("must fail with short series");
        assert!(
            matches!(err, RegimeError::InsufficientData { .. }),
            "expected InsufficientData, got {err:?}"
        );
    }

    // ── test: empty history ───────────────────────────────────────────────────

    /// forward_filter on empty history returns empty Vec.
    #[test]
    fn forward_filter_empty_history() {
        let returns = synthetic_4regime(100, 3);
        let mut clf = MarkovSwitchingClassifier::new();
        clf.fit(&returns).expect("fit must succeed");
        let result = clf
            .forward_filter(&[])
            .expect("empty filter should succeed");
        assert!(result.is_empty());
    }

    // ── test: regime name mapping ─────────────────────────────────────────────

    /// regime_name() maps argmax index to correct string.
    #[test]
    fn regime_name_mapping() {
        let make = |p: [f64; 4]| RegimeProbability { p };
        assert_eq!(make([0.9, 0.05, 0.03, 0.02]).regime_name(), "bull");
        assert_eq!(make([0.05, 0.9, 0.03, 0.02]).regime_name(), "bear");
        assert_eq!(make([0.05, 0.03, 0.9, 0.02]).regime_name(), "volatile");
        assert_eq!(make([0.05, 0.03, 0.02, 0.9]).regime_name(), "calm");
    }
}
