//! advisor-forward-plan v0.1.0 — the `agent::config::ForwardPlan` →
//! [`ForwardPlanView`] adapter (ADR-0062 § D4 — the ONE place `ui` reads the
//! `agent` plan type).
//!
//! ## Why feature-gated on `live`
//!
//! `agent` is an **optional** `ui` dependency (only the `cockpit_live`
//! binary's `live` feature pulls it). So — unlike
//! `BakeoffReportMirror::from_report`, which reads `backtest` (a *hard* `ui`
//! dep) — this adapter, which names `agent::config::ForwardPlan`, compiles
//! only under `#[cfg(feature = "live")]`. The `ui` lib's default build never
//! sees `agent`, so `cargo tree -p ui` is unchanged (the layering invariant,
//! R8 / ADR-0062 § D8.3). Headless render tests use the
//! [`crate::fixtures`] fake plan instead, which constructs a
//! [`ForwardPlanView`] directly — no `agent` type involved.
//!
//! ## Reconciliation note (developer ‖ ui-designer parallel)
//!
//! This module is written against the **ADR-0062 § D4 field/enum names**
//! (`ForwardPlan` + the closed `agent`-owned `PlanStance{Flat,Long}` /
//! `PlanSignal{Buy,Sell,Hold}` / `PlanRuleKind`). The developer owns the
//! canonical `agent::config` definitions; the exact `PlanRuleKind` variant
//! set is reconciled at integration (see the feature handoff). If a name
//! drifts, this single adapter is the only edit site — the mirror discipline
//! keeps the blast radius to one function.

#![cfg(feature = "live")]

use smol_str::SmolStr;

use super::state::{
    ConfidenceSummaryView, ForwardPlanView, PlanRuleView, PlanSignalView, PlanStanceView,
    PlanVoteMethodView,
};

impl ForwardPlanView {
    /// Mirror the `core`-typed `agent::config::ForwardPlan` into the
    /// render-ready `ui` view (ADR-0062 § D4). This is the ONLY place the
    /// `agent` `ForwardPlan` is read on the `ui` side — pure + total (no I/O,
    /// no panic): the `Price`/`Quantity`/`Money` newtypes are unwrapped to
    /// their inner `Decimal`, the `Timestamp` is pre-formatted to the
    /// honest-staleness label + the "planned through" date, and the closed
    /// `agent` enums map one-for-one to the closed `ui` enums.
    #[must_use]
    pub fn from_plan(plan: &agent::config::ForwardPlan) -> Self {
        let as_of_label = SmolStr::new(format_ts_label(plan.last_bar_ts));
        let horizon_through_label =
            SmolStr::new(format_through_label(plan.last_bar_ts, plan.horizon_days));

        // P0-3: mirror the scorecard summary from the ForwardPlan into the
        // pure-`ui` ConfidenceSummaryView (plain f64/bool/usize — zero new dep edge).
        let confidence = plan.confidence.map(confidence_summary_view);

        Self {
            strategy: SmolStr::new(plan.strategy.0.as_str()),
            symbol: SmolStr::new(plan.symbol.0.as_str()),
            stance: stance_view(plan.stance),
            latest_signal: plan.latest_signal.map(signal_view),
            rule: rule_view(&plan.rule),
            last_close: plan.last_close.get(),
            as_of_label,
            budget: plan.budget.amount(),
            projected_units: plan.projected_units.get(),
            sizing_capped: plan.sizing_capped,
            horizon_days: plan.horizon_days,
            horizon_through_label,
            confidence,
        }
    }
}

/// Map the closed `agent::config::PlanStance` to the closed `ui` stance.
fn stance_view(stance: agent::config::PlanStance) -> PlanStanceView {
    match stance {
        agent::config::PlanStance::Flat => PlanStanceView::Flat,
        agent::config::PlanStance::Long => PlanStanceView::Long,
    }
}

/// Map the closed `agent::config::PlanSignal` to the closed `ui` signal.
fn signal_view(signal: agent::config::PlanSignal) -> PlanSignalView {
    match signal {
        agent::config::PlanSignal::Buy => PlanSignalView::Buy,
        agent::config::PlanSignal::Sell => PlanSignalView::Sell,
        agent::config::PlanSignal::Hold => PlanSignalView::Hold,
    }
}

/// Map the closed `agent::config::PlanRuleKind` to the closed `ui` rule
/// family, carrying the parameters the IF/THEN copy reads. Exhaustive — a
/// new `agent` variant fails to compile here until it is mapped (the closed
/// enum can never silently fall through to the wrong copy).
fn rule_view(rule: &agent::config::PlanRuleKind) -> PlanRuleView {
    match rule {
        agent::config::PlanRuleKind::SmaCross { fast_len, slow_len } => PlanRuleView::SmaCross {
            fast_len: *fast_len,
            slow_len: *slow_len,
        },
        agent::config::PlanRuleKind::MacdCross { fast, slow, signal } => PlanRuleView::MacdCross {
            fast: *fast,
            slow: *slow,
            signal: *signal,
        },
        agent::config::PlanRuleKind::RsiReversion { len, lower } => PlanRuleView::RsiReversion {
            len: *len,
            lower: *lower,
        },
        agent::config::PlanRuleKind::BollingerReversion { len, k_tenths } => {
            PlanRuleView::BollingerReversion {
                len: *len,
                k_tenths: *k_tenths,
            }
        }
        agent::config::PlanRuleKind::BuyAndHold => PlanRuleView::BuyAndHold,
        // F8 / ADR-0063 + F6 member-name enrichment — the ensemble (signal-vote)
        // rule shape. RECONCILED to the shipped `agent::config::PlanRuleKind::
        // Ensemble { method, members: Vec<SmolStr> }` (the developer enriched it
        // from the old scalar `member_count: u32` to carry each member's display
        // label). The `ui` mirror carries the SAME `members: Vec<SmolStr>`
        // field-for-field, so the plan can NAME the members ("≥ k of {MACD trend,
        // RSI reversion, Bollinger reversion} agree…"); `members.len()` is the
        // authoritative member count.
        agent::config::PlanRuleKind::Ensemble { method, members } => PlanRuleView::Ensemble {
            method: vote_method_view(method),
            members: members.iter().map(|m| SmolStr::new(m.as_str())).collect(),
        },
    }
}

/// Map the closed `agent::config::PlanVoteMethod` to the closed `ui` vote
/// method, field-for-field. Exhaustive — a new `agent` method fails to compile
/// here until it is mapped.
fn vote_method_view(method: &agent::config::PlanVoteMethod) -> PlanVoteMethodView {
    match method {
        agent::config::PlanVoteMethod::Majority { k, n } => {
            PlanVoteMethodView::Majority { k: *k, n: *n }
        }
        agent::config::PlanVoteMethod::Unanimous { n } => PlanVoteMethodView::Unanimous { n: *n },
    }
}

/// Map the `backtest::bakeoff::ScorecardSummary` into the pure-`ui`
/// `ConfidenceSummaryView` (plain scalars — zero new `ui` dep edge; `backtest`
/// is already a `ui` dep). This is the ONLY place a `ScorecardSummary` is read
/// on the `ui` side; it is called exclusively from `ForwardPlanView::from_plan`.
fn confidence_summary_view(sc: backtest::bakeoff::ScorecardSummary) -> ConfidenceSummaryView {
    ConfidenceSummaryView {
        n_candidates: sc.n_candidates,
        deflated_sharpe: sc.deflated_sharpe,
        crown_clears_dsr: sc.crown_clears_dsr,
        min_btl_years: sc.min_btl_years,
    }
}

/// Format a `Timestamp` into the short "as of" label (e.g. `"Jun 19 14:00"`)
/// using the SAME month-abbreviation table + UTC-for-tests gate the chart
/// time axis uses, so the render layer carries no time-zone logic and the
/// render tests are deterministic.
fn format_ts_label(ts: trading_core::Timestamp) -> String {
    let dt = ts
        .inner()
        .to_offset(crate::widgets::chart::local_offset_or_utc());
    let mon = crate::strings::month_abbrev(dt.month() as u8);
    let (day, h, m) = (dt.day(), dt.hour(), dt.minute());
    format!("{mon} {day:02} {h:02}:{m:02}")
}

/// Format the "planned through" date = `last_bar_ts + horizon_days` (e.g.
/// `"Jun 26"`). Date-only (no time) — the horizon is a day-grained planning
/// frame, not a precise instant.
fn format_through_label(ts: trading_core::Timestamp, horizon_days: u16) -> String {
    let through = ts.inner() + time::Duration::days(i64::from(horizon_days));
    let dt = through.to_offset(crate::widgets::chart::local_offset_or_utc());
    let mon = crate::strings::month_abbrev(dt.month() as u8);
    let day = dt.day();
    format!("{mon} {day:02}")
}
