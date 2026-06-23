//! Ensemble signal-vote strategy (ADR-0063 § D1).
//!
//! `EnsembleStrategy` implements the frozen `Strategy` trait (ADR-0005) by
//! wrapping N member `Strategy` instances and reducing their per-bar signals
//! to a single composite signal via a pure deterministic arbiter.
//!
//! ## Warmup / abstention rule (load-bearing — DO NOT simplify)
//!
//! A member that has not yet warmed up (it has produced no edge-triggered
//! Buy/Sell — `last_rule_value == None`) is **NOT** counted as a FLAT vote.
//! It **ABSTAINS**: counted in neither `long_count` nor the effective
//! denominator.  The ensemble stays FLAT until its quorum is warm:
//!
//! - `Majority { k, n }` needs ≥ `k` warmed members before it can vote Long.
//! - `Unanimous { n }` needs all `n` warmed.
//!
//! Treating an un-warmed member as FLAT would manufacture false majorities
//! from early-warming members and hide (rather than measure) consensus.
//! This is the architect's load-bearing correctness decision (ADR-0063 § D1).
//!
//! ## Edge-triggered emission
//!
//! The ensemble emits Buy/Sell **only on its own stance transition**
//! (same semantics as `ComposedStrategy::on_bar`).  The ensemble's own
//! `last_stance` tracks Long/Flat independently of each member's stance.
//!
//! ## PlanDescribe
//!
//! `EnsembleStrategy` implements `PlanDescribe` via a non-mutating read of
//! each member's current stance + the arbiter (ADR-0063 § D2).

use smol_str::SmolStr;
use tracing::debug;
use trading_core::{Bar, Signal, SignalEvidence, SignalKind, StrategyId, Tick};

use crate::Strategy;
use crate::plan::{
    PlanContext, PlanDescribe, PlanRuleShape, PlanSignal, PlanStance, PlanVoteMethod,
    ProjectedSizing, StrategyPlan,
};

// ── Error type for ensemble construction ──────────────────────────────────────

/// Error returned by `build_member` and `build_ensemble`.
#[derive(Debug, thiserror::Error)]
pub enum EnsembleBuildError {
    #[error("unknown strategy id '{0}' — refusing to fall back (F5b anti-fake gate)")]
    UnknownId(String),
    #[error("TOML load failure for '{id}': {cause}")]
    TomlLoadFailure {
        id: String,
        #[source]
        cause: crate::composed::error::StrategyLoadError,
    },
    #[error("member build failure in ensemble '{ensemble_id}', member '{member_id}': {cause}")]
    MemberBuildFailure {
        ensemble_id: String,
        member_id: String,
        #[source]
        cause: Box<EnsembleBuildError>,
    },
}

// ── VoteMethod ────────────────────────────────────────────────────────────────

/// Vote arbitration method — frozen in code (two pre-registered choices).
///
/// Used internally by `EnsembleStrategy`. The `PlanVoteMethod` enum in `plan.rs`
/// is the structured plan-seam mirror of this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteMethod {
    /// Long iff `long_count >= k`. Ensemble needs ≥ k warmed members to vote.
    Majority { k: usize, n: usize },
    /// Long iff `long_count == n`. Ensemble needs all n warmed members.
    Unanimous { n: usize },
}

// ── MemberStance ─────────────────────────────────────────────────────────────

/// Per-member stance including the un-warmed sentinel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberStance {
    /// Member has not yet produced a Buy/Sell edge.  ABSTAINS — not a FLAT vote.
    Unwarmed,
    /// Member is currently Long (most recent edge was Buy).
    Long,
    /// Member is currently Flat (most recent edge was Sell, or warmed then no Buy).
    Flat,
}

// ── Pure arbiter ─────────────────────────────────────────────────────────────

/// Pure, deterministic arbitration function.
///
/// # Abstention rule (ADR-0063 § D1)
///
/// - `Unwarmed` members are IGNORED — counted in NEITHER `long_count` NOR
///   the denominator.
/// - The ensemble stays Flat (no Long vote) until the method's quorum of
///   warmed members is present:
///   - `Majority { k, n }`: needs ≥ k warmed members before Long is possible.
///   - `Unanimous { n }`: needs all n warmed members.
///
/// # Return
///
/// `true` iff the ensemble vote is Long; `false` iff Flat.
#[must_use]
pub fn arbitrate(method: VoteMethod, stances: &[MemberStance]) -> bool {
    let long_count = stances.iter().filter(|&&s| s == MemberStance::Long).count();
    let warmed_count = stances
        .iter()
        .filter(|&&s| s != MemberStance::Unwarmed)
        .count();

    match method {
        VoteMethod::Majority { k, n: _ } => {
            // Need at least k warmed members before any Long is possible.
            if warmed_count < k {
                return false;
            }
            long_count >= k
        }
        VoteMethod::Unanimous { n } => {
            // Need all n members warmed.
            if warmed_count < n {
                return false;
            }
            long_count == n
        }
    }
}

// ── EnsembleStrategy ─────────────────────────────────────────────────────────

/// Signal-vote ensemble: wraps N member strategies and arbitrates per bar.
///
/// Implements the frozen `Strategy` trait (ADR-0005 not modified).
/// The `RegimeDispatcher` precedent generalised to N homogeneous members +
/// a consensus arbiter (ADR-0063 § D1).
pub struct EnsembleStrategy {
    id: StrategyId,
    /// Member id strings — kept for `PlanDescribe` and diagnostics.
    member_ids: Vec<SmolStr>,
    /// Member strategies (order matches `member_stances`).
    members: Vec<Box<dyn Strategy>>,
    /// Current per-member stance.  Order matches `members`.
    member_stances: Vec<MemberStance>,
    /// Vote method.
    method: VoteMethod,
    /// The ensemble's own last stance (Long=true, Flat=false, None=pre-first-bar).
    /// `None` means no bar has been processed yet.
    last_stance: Option<bool>,
}

impl std::fmt::Debug for EnsembleStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnsembleStrategy")
            .field("id", &self.id)
            .field("method", &self.method)
            .field("member_ids", &self.member_ids)
            .finish()
    }
}

impl EnsembleStrategy {
    /// Build an `EnsembleStrategy` from pre-built members.
    ///
    /// `member_ids` must have the same length as `members`.
    #[must_use]
    pub fn new(
        id: &str,
        method: VoteMethod,
        member_ids: Vec<SmolStr>,
        members: Vec<Box<dyn Strategy>>,
    ) -> Self {
        let n = members.len();
        Self {
            id: StrategyId::new(id),
            member_ids,
            members,
            member_stances: vec![MemberStance::Unwarmed; n],
            method,
            last_stance: None,
        }
    }

    /// Returns the current per-member stances (for diagnostics / tests).
    #[must_use]
    pub fn member_stances(&self) -> &[MemberStance] {
        &self.member_stances
    }

    /// Returns the current ensemble vote stance (None before first bar).
    #[must_use]
    pub fn last_stance(&self) -> Option<bool> {
        self.last_stance
    }

    /// Non-mutating read: current ensemble Long/Flat from `last_stance`.
    ///
    /// Used by `PlanDescribe::describe_plan` — MUST NOT advance indicator state.
    #[must_use]
    fn current_plan_stance(&self) -> PlanStance {
        match self.last_stance {
            Some(true) => PlanStance::Long,
            _ => PlanStance::Flat,
        }
    }
}

impl Strategy for EnsembleStrategy {
    fn id(&self) -> StrategyId {
        self.id.clone()
    }

    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        // 1. Fan the bar to every member and update their stances.
        for (i, member) in self.members.iter_mut().enumerate() {
            let sigs = member.on_bar(bar);
            for sig in &sigs {
                match sig.kind {
                    SignalKind::Buy => {
                        self.member_stances[i] = MemberStance::Long;
                    }
                    SignalKind::Sell => {
                        // Member is now warmed AND Flat.
                        self.member_stances[i] = MemberStance::Flat;
                    }
                    _ => {
                        // Hold or other — if Unwarmed, keep Unwarmed (member hasn't
                        // produced its first edge yet — the warmup boundary case).
                        // If already Long or Flat, stance is unchanged (hold).
                    }
                }
            }
            // If the member has no signals yet (indicator not warmed), and no edge
            // ever fired, the stance stays Unwarmed — which is the correct abstention.
        }

        // 2. Arbitrate the current member stances.
        let now_long = arbitrate(self.method, &self.member_stances);

        // 3. Edge-triggered emission — only emit on own stance transition.
        let prev_long = self.last_stance;
        self.last_stance = Some(now_long);

        match (prev_long, now_long) {
            (Some(false), true) | (None, true) => {
                // Transition to Long — emit Buy.
                debug!(
                    ensemble_id = self.id.0.as_str(),
                    symbol = bar.symbol.0.as_str(),
                    "EnsembleStrategy: Buy emitted (stance → Long)"
                );
                vec![Signal {
                    strategy_id: self.id.clone(),
                    symbol: bar.symbol.clone(),
                    ts: bar.close_ts,
                    kind: SignalKind::Buy,
                    evidence: SignalEvidence::empty(),
                    pair_data: None,
                }]
            }
            (Some(true), false) => {
                // Transition to Flat — emit Sell.
                debug!(
                    ensemble_id = self.id.0.as_str(),
                    symbol = bar.symbol.0.as_str(),
                    "EnsembleStrategy: Sell emitted (stance → Flat)"
                );
                vec![Signal {
                    strategy_id: self.id.clone(),
                    symbol: bar.symbol.clone(),
                    ts: bar.close_ts,
                    kind: SignalKind::Sell,
                    evidence: SignalEvidence::empty(),
                    pair_data: None,
                }]
            }
            // None/true → already Long on first bar (treated as Buy above),
            // stable Long, or stable Flat — no emission.
            _ => vec![],
        }
    }

    fn on_tick(&mut self, _tick: &Tick) -> Vec<Signal> {
        // Ensembles are bar-driven (consistent with ComposedStrategy).
        vec![]
    }

    fn config_schema() -> serde_json::Value
    where
        Self: Sized,
    {
        serde_json::json!({
            "type": "object",
            "description": "EnsembleStrategy — signal-vote ensemble (ADR-0063)",
            "properties": {
                "id": { "type": "string" },
                "method": { "type": "string", "enum": ["majority", "unanimous"] }
            }
        })
    }
}

// ── PlanDescribe for EnsembleStrategy ────────────────────────────────────────

impl PlanDescribe for EnsembleStrategy {
    /// Non-mutating snapshot of the ensemble's current stance + rule structure.
    ///
    /// Reads `self.last_stance` and each member's own `describe_plan` for the
    /// members vector — NO indicator push, NO state advance (ADR-0062 § D2
    /// non-mutation contract).
    fn describe_plan(&self, ctx: &PlanContext) -> StrategyPlan {
        // Ensemble stance.
        let stance = self.current_plan_stance();

        // Latest signal: Buy if currently Long, Sell if Flat, None if not yet warmed.
        let latest_signal = match self.last_stance {
            Some(true) => Some(PlanSignal::Buy),
            Some(false) => Some(PlanSignal::Sell),
            None => None,
        };

        // PlanVoteMethod mirrors VoteMethod (no free-text string).
        let plan_method = match self.method {
            VoteMethod::Majority { k, n } => PlanVoteMethod::Majority {
                k: k as u32,
                n: n as u32,
            },
            VoteMethod::Unanimous { n } => PlanVoteMethod::Unanimous { n: n as u32 },
        };

        // Member rule shapes — each member implements PlanDescribe via its own
        // concrete type, but we only have &dyn Strategy here.  Instead, we
        // provide member id→shape mapping for the known pre-registered members,
        // mirroring what ComposedStrategy does via `id_str()`.
        // This keeps describe_plan non-mutating (no dyn-cast required).
        let members: Vec<PlanRuleShape> = self
            .member_ids
            .iter()
            .map(|id| member_id_to_rule_shape(id.as_str()))
            .collect();

        let rule = PlanRuleShape::Ensemble {
            method: plan_method,
            members,
        };

        let sizing = ProjectedSizing::compute(ctx.budget, ctx.budget_cap, ctx.last_close);

        StrategyPlan {
            stance,
            latest_signal,
            rule,
            sizing,
        }
    }
}

/// Map a member strategy id to its `PlanRuleShape`.
///
/// Mirrors the `ComposedStrategy::describe_plan` id→shape mapping so the
/// ensemble plan faithfully describes each member's rule.
#[must_use]
fn member_id_to_rule_shape(id: &str) -> PlanRuleShape {
    use rust_decimal_macros::dec;
    match id {
        "v0.sma" | "sma_cross" | "sma_crossover" | "sma_cross_h1" => PlanRuleShape::SmaCross {
            fast_len: 20,
            slow_len: 50,
        },
        "v0.5.macd" | "macd_trend" | "btc_macd_trend" | "macd_trend_h1" => {
            PlanRuleShape::MacdCross {
                fast: 12,
                slow: 26,
                signal: 9,
            }
        }
        "v0.5.rsi" | "rsi_reversion" | "btc_rsi_reversion" | "rsi_reversion_h1" => {
            PlanRuleShape::RsiReversion {
                len: 14,
                lower: dec!(30),
            }
        }
        "v0.5.bbands"
        | "bbands_mean_revert"
        | "btc_bbands_mean_revert"
        | "bbands_mean_revert_h1" => PlanRuleShape::BollingerReversion {
            len: 20,
            k: dec!(2),
        },
        "v0.buyhold" => PlanRuleShape::BuyAndHold,
        // Defensive fallback — unknown member id (should never occur with the
        // pre-registered frozen member sets, but avoids a panic if ever extended).
        _ => PlanRuleShape::SmaCross {
            fast_len: 20,
            slow_len: 50,
        },
    }
}

// ── Member builder (shared factory, single source of truth) ──────────────────

/// Build a single member strategy by id.
///
/// This is the shared `build_member` constructor used by `build_ensemble` to
/// construct each member through the **existing** per-id construction path —
/// the same TOMLs the bake-off and `build_registry_for` use (ADR-0063 § D1).
///
/// # Errors
///
/// Returns `Err` on unknown id or TOML load failure.  NO silent fallback —
/// the F5b anti-fake precedent applies to member construction too.
pub fn build_member(id: &str) -> Result<Box<dyn Strategy>, EnsembleBuildError> {
    match id {
        "v0.sma" | "v0.5.sma" | "sma_cross" | "sma_crossover" | "sma_cross_h1" => {
            // Default SMA parameters (fast=20, slow=50) — same defaults as
            // build_registry_for for the "v0.sma" arm.
            Ok(Box::new(crate::SmaCrossover::new(20, 50)))
        }
        "v0.5.macd" | "macd_trend" | "btc_macd_trend" | "macd_trend_h1" => {
            let strategy = load_composed_member("btc_macd_trend", id)?;
            Ok(Box::new(strategy))
        }
        "v0.5.rsi" | "rsi_reversion" | "btc_rsi_reversion" | "rsi_reversion_h1" => {
            let strategy = load_composed_member("btc_rsi_reversion", id)?;
            Ok(Box::new(strategy))
        }
        "v0.5.bbands"
        | "bbands_mean_revert"
        | "btc_bbands_mean_revert"
        | "bbands_mean_revert_h1" => {
            let strategy = load_composed_member("btc_bbands_mean_revert", id)?;
            Ok(Box::new(strategy))
        }
        unknown => Err(EnsembleBuildError::UnknownId(unknown.to_string())),
    }
}

/// Load a `ComposedStrategy` from `config/strategies/<toml_name>.toml`.
///
/// Uses `backtest::paths::resolve_workspace_path` for CWD-independence
/// (same pattern as `build_registry_for::load_composed_strategy_from_toml`).
fn load_composed_member(
    toml_name: &str,
    id: &str,
) -> Result<crate::ComposedStrategy, EnsembleBuildError> {
    use std::path::PathBuf;

    let rel_path = PathBuf::from(format!("config/strategies/{toml_name}.toml"));

    // Walk up from CARGO_MANIFEST_DIR to find the workspace root.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    // crates/strategy → crates → workspace root
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let toml_path = workspace_root.join(&rel_path);

    let cfg = crate::ComposedStrategyConfig::from_file(&toml_path).map_err(|cause| {
        EnsembleBuildError::TomlLoadFailure {
            id: id.to_string(),
            cause,
        }
    })?;

    let source_path = SmolStr::new(rel_path.to_string_lossy());
    Ok(crate::ComposedStrategy::from_config(cfg, source_path))
}

// ── Ensemble factory (pre-registered ensembles — F8 + advisor-combination-search) ────

/// Build one of the eight pre-registered ensemble strategies by id.
///
/// Pre-registered ids (frozen in code, ADR-0063 § D1 + ADR-0067):
///
/// **F8 original arms:**
/// - `"v0.8.vote.majority"` → `Majority { k:2, n:3 }` over
///   `[v0.5.macd, v0.5.rsi, v0.5.bbands]`.
/// - `"v0.8.vote.unanimous"` → `Unanimous { n:4 }` over
///   `[v0.sma, v0.5.macd, v0.5.rsi, v0.5.bbands]`.
///
/// **advisor-combination-search new arms (ADR-0067, the FROZEN v1 slate):**
///
/// Decorrelation pairings:
/// - `"v0.8.vote.trend_pair"` → `Unanimous { n:2 }` over
///   `[v0.5.macd, v0.sma]` (predicted-null control — both trend).
/// - `"v0.8.vote.tr_mr_macd_rsi"` → `Unanimous { n:2 }` over
///   `[v0.5.macd, v0.5.rsi]` (trend ∧ mean-revert).
/// - `"v0.8.vote.tr_mr_sma_bb"` → `Unanimous { n:2 }` over
///   `[v0.sma, v0.5.bbands]` (trend ∧ band-reversion).
///
/// k-of-4 ladder (complete ladder k∈{1,2,3}; k=4 = unanimous above):
/// - `"v0.8.vote.any1of4"` → `Majority { k:1, n:4 }` over all 4.
/// - `"v0.8.vote.k2of4"` → `Majority { k:2, n:4 }` over all 4.
/// - `"v0.8.vote.k3of4"` → `Majority { k:3, n:4 }` over all 4.
///
/// # Errors
///
/// Returns `Err` on unknown id or member construction failure.
pub fn build_ensemble(id: &str) -> Result<EnsembleStrategy, EnsembleBuildError> {
    match id {
        // ── F8 original arms ──────────────────────────────────────────────────
        "v0.8.vote.majority" => {
            let member_ids = vec![
                SmolStr::new_static("v0.5.macd"),
                SmolStr::new_static("v0.5.rsi"),
                SmolStr::new_static("v0.5.bbands"),
            ];
            let members: Vec<Box<dyn Strategy>> = member_ids
                .iter()
                .map(|mid| {
                    build_member(mid.as_str()).map_err(|cause| {
                        EnsembleBuildError::MemberBuildFailure {
                            ensemble_id: id.to_string(),
                            member_id: mid.to_string(),
                            cause: Box::new(cause),
                        }
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(EnsembleStrategy::new(
                id,
                VoteMethod::Majority { k: 2, n: 3 },
                member_ids,
                members,
            ))
        }
        "v0.8.vote.unanimous" => {
            let member_ids = vec![
                SmolStr::new_static("v0.sma"),
                SmolStr::new_static("v0.5.macd"),
                SmolStr::new_static("v0.5.rsi"),
                SmolStr::new_static("v0.5.bbands"),
            ];
            let members: Vec<Box<dyn Strategy>> = member_ids
                .iter()
                .map(|mid| {
                    build_member(mid.as_str()).map_err(|cause| {
                        EnsembleBuildError::MemberBuildFailure {
                            ensemble_id: id.to_string(),
                            member_id: mid.to_string(),
                            cause: Box::new(cause),
                        }
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(EnsembleStrategy::new(
                id,
                VoteMethod::Unanimous { n: 4 },
                member_ids,
                members,
            ))
        }

        // ── advisor-combination-search: decorrelation pairings (ADR-0067) ─────
        //
        // These arms run write_report=false on the Bootstrap advisor path —
        // anchor-safe by construction (no anchored body collision).

        // Both-trend control — predicted little p5 lift (sanity check: correlated members).
        "v0.8.vote.trend_pair" => {
            let member_ids = vec![
                SmolStr::new_static("v0.5.macd"),
                SmolStr::new_static("v0.sma"),
            ];
            let members: Vec<Box<dyn Strategy>> = member_ids
                .iter()
                .map(|mid| {
                    build_member(mid.as_str()).map_err(|cause| {
                        EnsembleBuildError::MemberBuildFailure {
                            ensemble_id: id.to_string(),
                            member_id: mid.to_string(),
                            cause: Box::new(cause),
                        }
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(EnsembleStrategy::new(
                id,
                VoteMethod::Unanimous { n: 2 },
                member_ids,
                members,
            ))
        }

        // Trend ∧ mean-revert: real decorrelation lever.
        "v0.8.vote.tr_mr_macd_rsi" => {
            let member_ids = vec![
                SmolStr::new_static("v0.5.macd"),
                SmolStr::new_static("v0.5.rsi"),
            ];
            let members: Vec<Box<dyn Strategy>> = member_ids
                .iter()
                .map(|mid| {
                    build_member(mid.as_str()).map_err(|cause| {
                        EnsembleBuildError::MemberBuildFailure {
                            ensemble_id: id.to_string(),
                            member_id: mid.to_string(),
                            cause: Box::new(cause),
                        }
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(EnsembleStrategy::new(
                id,
                VoteMethod::Unanimous { n: 2 },
                member_ids,
                members,
            ))
        }

        // Trend ∧ band-reversion: second decorrelated pairing.
        "v0.8.vote.tr_mr_sma_bb" => {
            let member_ids = vec![
                SmolStr::new_static("v0.sma"),
                SmolStr::new_static("v0.5.bbands"),
            ];
            let members: Vec<Box<dyn Strategy>> = member_ids
                .iter()
                .map(|mid| {
                    build_member(mid.as_str()).map_err(|cause| {
                        EnsembleBuildError::MemberBuildFailure {
                            ensemble_id: id.to_string(),
                            member_id: mid.to_string(),
                            cause: Box::new(cause),
                        }
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(EnsembleStrategy::new(
                id,
                VoteMethod::Unanimous { n: 2 },
                member_ids,
                members,
            ))
        }

        // ── advisor-combination-search: k-of-4 ladder (ADR-0067) ─────────────
        //
        // Complete ladder k∈{1,2,3} over all 4 base signals.
        // k=4 is the existing `v0.8.vote.unanimous` arm above.
        // Reporting the WHOLE ladder ensures no per-arm k-selection cherry-picking.

        // k=1: loosest — long if ANY of the 4 fires.
        "v0.8.vote.any1of4" => {
            let member_ids = vec![
                SmolStr::new_static("v0.sma"),
                SmolStr::new_static("v0.5.macd"),
                SmolStr::new_static("v0.5.rsi"),
                SmolStr::new_static("v0.5.bbands"),
            ];
            let members: Vec<Box<dyn Strategy>> = member_ids
                .iter()
                .map(|mid| {
                    build_member(mid.as_str()).map_err(|cause| {
                        EnsembleBuildError::MemberBuildFailure {
                            ensemble_id: id.to_string(),
                            member_id: mid.to_string(),
                            cause: Box::new(cause),
                        }
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(EnsembleStrategy::new(
                id,
                VoteMethod::Majority { k: 1, n: 4 },
                member_ids,
                members,
            ))
        }

        // k=2: balanced quorum.
        "v0.8.vote.k2of4" => {
            let member_ids = vec![
                SmolStr::new_static("v0.sma"),
                SmolStr::new_static("v0.5.macd"),
                SmolStr::new_static("v0.5.rsi"),
                SmolStr::new_static("v0.5.bbands"),
            ];
            let members: Vec<Box<dyn Strategy>> = member_ids
                .iter()
                .map(|mid| {
                    build_member(mid.as_str()).map_err(|cause| {
                        EnsembleBuildError::MemberBuildFailure {
                            ensemble_id: id.to_string(),
                            member_id: mid.to_string(),
                            cause: Box::new(cause),
                        }
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(EnsembleStrategy::new(
                id,
                VoteMethod::Majority { k: 2, n: 4 },
                member_ids,
                members,
            ))
        }

        // k=3: strict — long only on broad agreement.
        "v0.8.vote.k3of4" => {
            let member_ids = vec![
                SmolStr::new_static("v0.sma"),
                SmolStr::new_static("v0.5.macd"),
                SmolStr::new_static("v0.5.rsi"),
                SmolStr::new_static("v0.5.bbands"),
            ];
            let members: Vec<Box<dyn Strategy>> = member_ids
                .iter()
                .map(|mid| {
                    build_member(mid.as_str()).map_err(|cause| {
                        EnsembleBuildError::MemberBuildFailure {
                            ensemble_id: id.to_string(),
                            member_id: mid.to_string(),
                            cause: Box::new(cause),
                        }
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(EnsembleStrategy::new(
                id,
                VoteMethod::Majority { k: 3, n: 4 },
                member_ids,
                members,
            ))
        }

        unknown => Err(EnsembleBuildError::UnknownId(unknown.to_string())),
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ── arbitrate tests ───────────────────────────────────────────────────────

    #[test]
    fn majority_requires_k_warmed_before_long() {
        // 0 warmed members — must return Flat.
        let stances = [MemberStance::Unwarmed; 3];
        assert!(
            !arbitrate(VoteMethod::Majority { k: 2, n: 3 }, &stances),
            "majority must be Flat when no members are warmed"
        );
    }

    #[test]
    fn majority_1_warmed_long_insufficient_quorum() {
        // 1 Long, 2 Unwarmed → only 1 warmed, need ≥ k=2 warmed → Flat.
        let stances = [
            MemberStance::Long,
            MemberStance::Unwarmed,
            MemberStance::Unwarmed,
        ];
        assert!(
            !arbitrate(VoteMethod::Majority { k: 2, n: 3 }, &stances),
            "majority must be Flat when only 1 of k=2 quorum is warmed"
        );
    }

    #[test]
    fn majority_2_warmed_both_long_fires() {
        // 2 Long, 1 Unwarmed → 2 warmed ≥ k=2 → 2 Long ≥ k=2 → Long.
        let stances = [
            MemberStance::Long,
            MemberStance::Long,
            MemberStance::Unwarmed,
        ];
        assert!(
            arbitrate(VoteMethod::Majority { k: 2, n: 3 }, &stances),
            "majority must fire when 2 of 3 are Long (even with 1 Unwarmed)"
        );
    }

    #[test]
    fn majority_2_warmed_1_long_1_flat_no_quorum_for_long() {
        // 1 Long, 1 Flat, 1 Unwarmed → 2 warmed ≥ k=2 but only 1 Long < k → Flat.
        let stances = [
            MemberStance::Long,
            MemberStance::Flat,
            MemberStance::Unwarmed,
        ];
        assert!(
            !arbitrate(VoteMethod::Majority { k: 2, n: 3 }, &stances),
            "majority must be Flat when only 1 of 3 is Long (< k=2)"
        );
    }

    #[test]
    fn majority_all_3_long_fires() {
        // All Long → Long.
        let stances = [MemberStance::Long; 3];
        assert!(
            arbitrate(VoteMethod::Majority { k: 2, n: 3 }, &stances),
            "majority fires when all 3 are Long"
        );
    }

    #[test]
    fn majority_2_long_1_flat_fires() {
        // 2 Long, 1 Flat → 2 ≥ k=2 → Long.
        let stances = [MemberStance::Long, MemberStance::Long, MemberStance::Flat];
        assert!(
            arbitrate(VoteMethod::Majority { k: 2, n: 3 }, &stances),
            "majority fires when 2 of 3 are Long"
        );
    }

    #[test]
    fn majority_1_long_2_flat_does_not_fire() {
        // 1 Long, 2 Flat → 1 < k=2 → Flat.
        let stances = [MemberStance::Long, MemberStance::Flat, MemberStance::Flat];
        assert!(
            !arbitrate(VoteMethod::Majority { k: 2, n: 3 }, &stances),
            "majority does not fire when only 1 of 3 is Long"
        );
    }

    #[test]
    fn unanimous_requires_all_n_warmed() {
        // 3 warmed, 1 Unwarmed → unanimous n=4 needs all 4 warmed → Flat.
        let stances = [
            MemberStance::Long,
            MemberStance::Long,
            MemberStance::Long,
            MemberStance::Unwarmed,
        ];
        assert!(
            !arbitrate(VoteMethod::Unanimous { n: 4 }, &stances),
            "unanimous must be Flat when not all n=4 members are warmed"
        );
    }

    #[test]
    fn unanimous_all_4_long_fires() {
        // All 4 Long → Long.
        let stances = [MemberStance::Long; 4];
        assert!(
            arbitrate(VoteMethod::Unanimous { n: 4 }, &stances),
            "unanimous fires when all 4 are Long"
        );
    }

    #[test]
    fn unanimous_3_long_1_flat_does_not_fire() {
        // 3 Long, 1 Flat → not all n=4 Long → Flat.
        let stances = [
            MemberStance::Long,
            MemberStance::Long,
            MemberStance::Long,
            MemberStance::Flat,
        ];
        assert!(
            !arbitrate(VoteMethod::Unanimous { n: 4 }, &stances),
            "unanimous does not fire when any member is Flat"
        );
    }

    // ── Unwarmed-as-abstention (warmup boundary) ──────────────────────────────

    /// Critical warmup boundary: 1 Long + 2 Unwarmed for k=2 → must be Flat.
    ///
    /// If `Unwarmed` were treated as FLAT, 1 Long + 2 "pseudo-Flat" → majority
    /// (1 of 3) would never fire — different failure mode but still wrong.
    /// If `Unwarmed` were counted as LONG (another naïve path), 1 Long + 2
    /// "pseudo-Long" = 3 Long ≥ k=2 → would fire — that's the manufactured
    /// majority we specifically guard against.
    #[test]
    fn warmup_boundary_abstention_not_false_majority() {
        // Only 1 member Long, 2 still Unwarmed.
        // Correct: warmed_count = 1 < k=2 → Flat (abstention quorum not met).
        // Wrong (if Unwarmed=FLAT): 1 Long of 3 → 1 < 2 → still Flat (coincidentally).
        // The real test is 2 Long + 1 Unwarmed vs 2 Long + 1 Flat (both → Long).
        let stances_abstain = [
            MemberStance::Long,
            MemberStance::Unwarmed,
            MemberStance::Unwarmed,
        ];
        let stances_with_flat = [
            MemberStance::Long,
            MemberStance::Flat,
            MemberStance::Unwarmed,
        ];
        // 2 Long + 1 Unwarmed: warmed=2 ≥ k=2, Long=2 ≥ k=2 → Long.
        let stances_2long_unwarmed = [
            MemberStance::Long,
            MemberStance::Long,
            MemberStance::Unwarmed,
        ];

        // 1 Long + 2 Unwarmed → Flat (quorum not met).
        assert!(
            !arbitrate(VoteMethod::Majority { k: 2, n: 3 }, &stances_abstain),
            "1 Long + 2 Unwarmed must be Flat — quorum k=2 requires 2 warmed"
        );
        // 1 Long + 1 Flat + 1 Unwarmed → Flat (Long=1 < k=2, even though warmed=2).
        assert!(
            !arbitrate(VoteMethod::Majority { k: 2, n: 3 }, &stances_with_flat),
            "1 Long + 1 Flat + 1 Unwarmed must be Flat — Long=1 < k=2"
        );
        // 2 Long + 1 Unwarmed → Long (warmed=2 ≥ k=2, Long=2 ≥ k=2).
        assert!(
            arbitrate(VoteMethod::Majority { k: 2, n: 3 }, &stances_2long_unwarmed),
            "2 Long + 1 Unwarmed must be Long — warmed=2 ≥ k=2, Long=2 ≥ k=2"
        );
    }

    // ── EnsembleStrategy edge-triggered emission ──────────────────────────────

    use time::OffsetDateTime;
    use trading_core::symbol::Symbol;
    use trading_core::{Bar, Price, Quantity, Timeframe, Timestamp, Venue};

    fn make_bar(symbol: &str, ts_secs: i64, close: rust_decimal::Decimal) -> Bar {
        let ts = Timestamp::new(
            OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1_700_000_000 + ts_secs),
        );
        Bar {
            symbol: Symbol::new(symbol),
            tf: Timeframe::OneHour,
            open_ts: ts,
            close_ts: ts,
            local_recv_ts: ts,
            venue: Venue::Binance,
            open: Price::new(close).unwrap(),
            high: Price::new(close).unwrap(),
            low: Price::new(close).unwrap(),
            close: Price::new(close).unwrap(),
            volume: Quantity::new(rust_decimal_macros::dec!(1.0)).unwrap(),
            trade_count: 1,
        }
    }

    /// Stub strategy that emits Buy on bar N and Sell on bar M.
    struct StubStrategy {
        id: StrategyId,
        buy_on: usize,
        sell_on: usize,
        bar_count: usize,
    }

    impl StubStrategy {
        fn new(id: &str, buy_on: usize, sell_on: usize) -> Self {
            Self {
                id: StrategyId::new(id),
                buy_on,
                sell_on,
                bar_count: 0,
            }
        }
    }

    impl Strategy for StubStrategy {
        fn id(&self) -> StrategyId {
            self.id.clone()
        }

        fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
            self.bar_count += 1;
            let n = self.bar_count;
            if n == self.buy_on {
                return vec![Signal {
                    strategy_id: self.id.clone(),
                    symbol: bar.symbol.clone(),
                    ts: bar.close_ts,
                    kind: SignalKind::Buy,
                    evidence: SignalEvidence::empty(),
                    pair_data: None,
                }];
            }
            if n == self.sell_on {
                return vec![Signal {
                    strategy_id: self.id.clone(),
                    symbol: bar.symbol.clone(),
                    ts: bar.close_ts,
                    kind: SignalKind::Sell,
                    evidence: SignalEvidence::empty(),
                    pair_data: None,
                }];
            }
            vec![]
        }

        fn on_tick(&mut self, _: &trading_core::Tick) -> Vec<Signal> {
            vec![]
        }

        fn config_schema() -> serde_json::Value
        where
            Self: Sized,
        {
            serde_json::json!({})
        }
    }

    #[test]
    fn ensemble_emits_buy_on_first_majority() {
        // 3 members: A buys at bar 1, B buys at bar 2, C buys at bar 3.
        // Majority (k=2, n=3): Long when ≥ 2 members are Long.
        // - Bar 1: A=Long, B=Unwarmed, C=Unwarmed → 1 warmed < k=2 → Flat (no emit).
        //   Wait: bar 1 — A emits Buy → A becomes Long.
        //   warmed=1 < k=2 → Flat. prev=None, now=Flat → no emit (we don't emit Buy for None→Flat).
        //
        // Actually: prev=None, now=Flat → `(None, false)` → no emit. Correct.
        //
        // - Bar 2: B emits Buy → B=Long. Now A=Long, B=Long, C=Unwarmed.
        //   warmed=2 ≥ k=2, long=2 ≥ k=2 → Long. prev=Some(false), now=true → Buy emitted.

        let members: Vec<Box<dyn Strategy>> = vec![
            Box::new(StubStrategy::new("a", 1, 10)),
            Box::new(StubStrategy::new("b", 2, 10)),
            Box::new(StubStrategy::new("c", 3, 10)),
        ];
        let member_ids = vec![
            SmolStr::new_static("a"),
            SmolStr::new_static("b"),
            SmolStr::new_static("c"),
        ];
        let mut ens = EnsembleStrategy::new(
            "test.majority",
            VoteMethod::Majority { k: 2, n: 3 },
            member_ids,
            members,
        );

        let bar1 = make_bar("BTCUSDT", 0, rust_decimal_macros::dec!(50_000));
        let bar2 = make_bar("BTCUSDT", 3600, rust_decimal_macros::dec!(50_100));
        let bar3 = make_bar("BTCUSDT", 7200, rust_decimal_macros::dec!(50_200));

        // Bar 1: A buys. A=Long, B=Unwarmed, C=Unwarmed.
        // warmed=1 < k=2 → Flat. prev=None → (None, false) → no emit.
        let s1 = ens.on_bar(&bar1);
        assert!(
            s1.is_empty(),
            "Bar 1: only A bought; warmed=1 < k=2 → no ensemble signal"
        );
        assert_eq!(ens.last_stance, Some(false));

        // Bar 2: B buys. A=Long, B=Long, C=Unwarmed.
        // warmed=2 ≥ k=2, long=2 ≥ k=2 → Long. prev=Some(false) → Buy.
        let s2 = ens.on_bar(&bar2);
        assert_eq!(s2.len(), 1, "Bar 2: majority achieved → Buy signal");
        assert_eq!(s2[0].kind, SignalKind::Buy);
        assert_eq!(ens.last_stance, Some(true));

        // Bar 3: C buys (3 Long now). Still Long. prev=Some(true), now=true → no emit.
        let s3 = ens.on_bar(&bar3);
        assert!(
            s3.is_empty(),
            "Bar 3: still majority Long → no additional signal"
        );
        assert_eq!(ens.last_stance, Some(true));
    }

    #[test]
    fn ensemble_emits_sell_on_majority_lost() {
        // 3 members, majority.
        // Bar 1: A buys.
        // Bar 2: B buys → ensemble Long (Buy emitted).
        // Bar 3: A sells → A=Flat, B=Long, C=Unwarmed → long=1 < k=2 → Flat (Sell emitted).

        let members: Vec<Box<dyn Strategy>> = vec![
            Box::new(StubStrategy::new("a", 1, 3)), // Buy at 1, Sell at 3
            Box::new(StubStrategy::new("b", 2, 10)),
            Box::new(StubStrategy::new("c", 20, 30)), // never warms in this window
        ];
        let member_ids = vec![
            SmolStr::new_static("a"),
            SmolStr::new_static("b"),
            SmolStr::new_static("c"),
        ];
        let mut ens = EnsembleStrategy::new(
            "test.majority",
            VoteMethod::Majority { k: 2, n: 3 },
            member_ids,
            members,
        );

        let b1 = make_bar("BTCUSDT", 0, rust_decimal_macros::dec!(100));
        let b2 = make_bar("BTCUSDT", 3600, rust_decimal_macros::dec!(101));
        let b3 = make_bar("BTCUSDT", 7200, rust_decimal_macros::dec!(102));

        ens.on_bar(&b1);
        let s2 = ens.on_bar(&b2);
        assert_eq!(s2.len(), 1);
        assert_eq!(s2[0].kind, SignalKind::Buy);

        // A sells at bar 3.
        let s3 = ens.on_bar(&b3);
        assert_eq!(s3.len(), 1, "Bar 3: majority lost → Sell");
        assert_eq!(s3[0].kind, SignalKind::Sell);
        assert_eq!(ens.last_stance, Some(false));
    }

    #[test]
    fn unanimous_stays_flat_until_all_4_warmed() {
        // 4 members, unanimous n=4.
        // Bar 1: A buys.
        // Bar 2: B buys.
        // Bar 3: C buys.
        // Still only 3 warmed — unanimous needs all 4. Still Flat.
        // Bar 4: D buys. All 4 Long → Long → Buy emitted.

        let members: Vec<Box<dyn Strategy>> = vec![
            Box::new(StubStrategy::new("a", 1, 20)),
            Box::new(StubStrategy::new("b", 2, 20)),
            Box::new(StubStrategy::new("c", 3, 20)),
            Box::new(StubStrategy::new("d", 4, 20)),
        ];
        let member_ids = vec![
            SmolStr::new_static("a"),
            SmolStr::new_static("b"),
            SmolStr::new_static("c"),
            SmolStr::new_static("d"),
        ];
        let mut ens = EnsembleStrategy::new(
            "test.unanimous",
            VoteMethod::Unanimous { n: 4 },
            member_ids,
            members,
        );

        let bars: Vec<Bar> = (0..5)
            .map(|i| make_bar("BTCUSDT", i * 3600, rust_decimal::Decimal::from(50_000 + i)))
            .collect();

        assert!(
            ens.on_bar(&bars[0]).is_empty(),
            "Bar 1: 1 warmed < 4 needed → Flat"
        );
        assert!(
            ens.on_bar(&bars[1]).is_empty(),
            "Bar 2: 2 warmed < 4 → Flat"
        );
        assert!(
            ens.on_bar(&bars[2]).is_empty(),
            "Bar 3: 3 warmed < 4 → Flat"
        );

        // Bar 4 (index 3): D buys. Now all 4 Long.
        let s4 = ens.on_bar(&bars[3]);
        assert_eq!(s4.len(), 1, "Bar 4: all 4 unanimous → Buy");
        assert_eq!(s4[0].kind, SignalKind::Buy);

        // Bar 5: stable Long → no emit.
        assert!(
            ens.on_bar(&bars[4]).is_empty(),
            "Bar 5: stable Long → no signal"
        );
    }
}
