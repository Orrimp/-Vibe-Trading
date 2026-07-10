//! advisor-handoff-export v0.1.0 (remediation-plan P5, ADR-0088) — the pure,
//! deterministic serialiser: [`serialize_plan_export`] + [`export_filename`].
//!
//! ## The immutable-wording contract
//!
//! The operator ratified the § Draft wording (Variant A + short-crowned
//! Variant B, `spec/v3/advisor-handoff-export/feature.md`) AS DRAFTED
//! (DECISION-P5-WORDING, 2026-07-10). This module emits that text and does
//! NOT rewrite a word: every line is either
//! - an existing `crate::strings` const (`FORWARD_PLAN_*` / `LEADERBOARD_*` /
//!   [`SHORT_UNBOUNDED_LOSS_DISCLAIMER`]) already vetted by the not-advice
//!   discipline, reused VERBATIM (never re-derived, never re-worded), or
//! - one of the 27 new `PLAN_EXPORT_*` consts carrying the ratified text
//!   VERBATIM (`crate::strings`, § "advisor-handoff-export P5" section).
//!
//! The box-drawing rule lines (`═`/`─`) are layout, not user copy —
//! serialiser-emitted here, never a `crate::strings` const (ADR-0088 §
//! Design "New strings").
//!
//! ## Structural fidelity (ADR-0088 § D3)
//!
//! This module walks the SAME predicate tree
//! `screens/forward_plan.rs::view` + `screens/leaderboard.rs::
//! recommendation_block` walk, over the SAME [`ForwardPlanView`] /
//! [`BakeoffReportMirror`] mirror types, reusing the SAME
//! [`crate::widgets::num`] formatters and the SAME
//! [`crate::leaderboard::state::crown_credibility`] resolver (moved
//! `pub(crate)` by this feature so screen and export share ONE source of
//! truth) — emitting text instead of widgets. The rule-clause / short-rule /
//! ensemble-rule helpers below are an INDEPENDENT re-implementation (not a
//! shared function with `screens/forward_plan.rs`) over the SAME consts, in
//! the SAME order — see the developer handoff summary for the scope
//! rationale (kept `screens/forward_plan.rs` untouched beyond what
//! ADR-0088 names).
//!
//! ## Purity (ADR-0088 § D6 / R-HE.1)
//!
//! No wall-clock, no RNG, no I/O, no panic. Same inputs ⇒ byte-identical
//! `String`. The single `std::fs::write` leaf lives at the ui-designer's
//! `Message::ExportPlan` handler (T6), not here.

use smol_str::SmolStr;
use trading_core::FxNote;

use crate::forward_plan::{
    ConfidenceSummaryView, ForwardPlanView, PlanRuleView, PlanSignalView, PlanStanceView,
    PlanVoteMethodView,
};
use crate::leaderboard::state::{CrownCredibility, crown_credibility};
use crate::leaderboard::{BakeoffReportMirror, DataQualityView, NarrationState, OutcomeKind};
use crate::strings::{
    FORWARD_PLAN_AS_OF_FMT, FORWARD_PLAN_BUDGET_LINE_FMT, FORWARD_PLAN_CADENCE_FMT,
    FORWARD_PLAN_CONFIDENCE_BEATS_HOLD_LABEL, FORWARD_PLAN_CONFIDENCE_BEATS_HOLD_NO,
    FORWARD_PLAN_CONFIDENCE_BEATS_HOLD_YES, FORWARD_PLAN_CONFIDENCE_CANDIDATES_LABEL,
    FORWARD_PLAN_CONFIDENCE_DSR_GLOSS, FORWARD_PLAN_CONFIDENCE_DSR_LABEL,
    FORWARD_PLAN_CONFIDENCE_MIN_BTL_FMT, FORWARD_PLAN_CONFIDENCE_MIN_BTL_LABEL,
    FORWARD_PLAN_CONFIDENCE_NOTE, FORWARD_PLAN_DISCLAIMER, FORWARD_PLAN_LATEST_SIGNAL_FMT,
    FORWARD_PLAN_NOT_A_PREDICTION, FORWARD_PLAN_RULE_ALWAYS_SHORT,
    FORWARD_PLAN_RULE_BBANDS_ENTRY_IF_FMT, FORWARD_PLAN_RULE_BBANDS_ENTRY_THEN,
    FORWARD_PLAN_RULE_BBANDS_EXIT_IF, FORWARD_PLAN_RULE_BBANDS_EXIT_THEN,
    FORWARD_PLAN_RULE_BUY_AND_HOLD, FORWARD_PLAN_RULE_COMPOUND_CAVEAT,
    FORWARD_PLAN_RULE_ENSEMBLE_CAVEAT, FORWARD_PLAN_RULE_ENSEMBLE_MAJORITY_NAMED_FMT,
    FORWARD_PLAN_RULE_ENSEMBLE_TALLY_FMT, FORWARD_PLAN_RULE_ENSEMBLE_UNANIMOUS_NAMED_FMT,
    FORWARD_PLAN_RULE_IF, FORWARD_PLAN_RULE_MACD_ENTRY_IF_FMT, FORWARD_PLAN_RULE_MACD_ENTRY_THEN,
    FORWARD_PLAN_RULE_MACD_EXIT_IF, FORWARD_PLAN_RULE_MACD_EXIT_THEN,
    FORWARD_PLAN_RULE_RSI_ENTRY_IF_FMT, FORWARD_PLAN_RULE_RSI_ENTRY_THEN,
    FORWARD_PLAN_RULE_RSI_EXIT_IF_FMT, FORWARD_PLAN_RULE_RSI_EXIT_THEN,
    FORWARD_PLAN_RULE_SHORT_COVER_IF, FORWARD_PLAN_RULE_SHORT_COVER_THEN,
    FORWARD_PLAN_RULE_SHORT_LIQUIDATION, FORWARD_PLAN_RULE_SHORT_OPEN_IF_GENERIC,
    FORWARD_PLAN_RULE_SHORT_OPEN_THEN, FORWARD_PLAN_RULE_SMA_ENTRY_IF_FMT,
    FORWARD_PLAN_RULE_SMA_ENTRY_THEN, FORWARD_PLAN_RULE_SMA_EXIT_IF_FMT,
    FORWARD_PLAN_RULE_SMA_EXIT_THEN, FORWARD_PLAN_RULE_THEN, FORWARD_PLAN_SHORT_RULES_HEADING,
    FORWARD_PLAN_SIGNAL_BUY, FORWARD_PLAN_SIGNAL_HOLD, FORWARD_PLAN_SIGNAL_SELL,
    FORWARD_PLAN_SIZING_BUY_AND_HOLD_FMT, FORWARD_PLAN_SIZING_CAPPED_NOTE,
    FORWARD_PLAN_SIZING_FLAT_FMT, FORWARD_PLAN_SIZING_LONG_FMT, FORWARD_PLAN_STANCE_FLAT,
    FORWARD_PLAN_STANCE_LONG, LEADERBOARD_CROWN_PASSES_DSR, LEADERBOARD_CROWN_WEAK_EVIDENCE,
    LEADERBOARD_CROWN_WEAK_EVIDENCE_HINT, LEADERBOARD_DATA_QUALITY_PROVENANCE_LABEL,
    LEADERBOARD_DATA_QUALITY_SURVIVAL_LABEL, LEADERBOARD_DATA_QUALITY_TRUST_LABEL,
    LEADERBOARD_DATA_QUALITY_VENUE_LABEL, LEADERBOARD_DATA_QUALITY_WARNINGS_LABEL,
    LEADERBOARD_EXPLAIN_LLM_LABEL, LEADERBOARD_HEADLINE_ACTIVE_WINS,
    LEADERBOARD_HEADLINE_ALL_FRAGILE, LEADERBOARD_HEADLINE_BENCHMARK_WINS,
    PLAN_EXPORT_BENCHMARK_WINS_BRIDGE, PLAN_EXPORT_ERA_QUALIFIED_THESIS, PLAN_EXPORT_FOOTER,
    PLAN_EXPORT_HANDOFF_FRAME, PLAN_EXPORT_HEADER_META_FMT, PLAN_EXPORT_NOT_ADVICE_BANNER,
    PLAN_EXPORT_NOT_BULLET_ADVICE, PLAN_EXPORT_NOT_BULLET_CHANCE, PLAN_EXPORT_NOT_BULLET_PAPER,
    PLAN_EXPORT_NOT_BULLET_PAST, PLAN_EXPORT_NOT_BULLET_PREDICTION, PLAN_EXPORT_ONE_IN_FIVE_NOTE,
    PLAN_EXPORT_PROVENANCE_COIN_FMT, PLAN_EXPORT_PROVENANCE_PICK_FMT,
    PLAN_EXPORT_PROVENANCE_SEED_FMT, PLAN_EXPORT_REPRODUCE_HINT, PLAN_EXPORT_SECTION_DATA_SOURCE,
    PLAN_EXPORT_SECTION_MEASURED_ANSWER, PLAN_EXPORT_SECTION_PROVENANCE,
    PLAN_EXPORT_SECTION_RIGHT_NOW, PLAN_EXPORT_SECTION_SHORT_RISK, PLAN_EXPORT_SECTION_SIZING,
    PLAN_EXPORT_SECTION_STANDING_RULES, PLAN_EXPORT_SECTION_TRUST,
    PLAN_EXPORT_SECTION_WHAT_THIS_IS_NOT, PLAN_EXPORT_TITLE, SHORT_UNBOUNDED_LOSS_DISCLAIMER,
};
use crate::widgets::num::{fmt_eur, fmt_eur_plain, fmt_price, fmt_qty, fmt_rate, fmt_usdt_plain};

// ── Layout primitives (serialiser-emitted, NOT `crate::strings` consts) ───────

/// Body-line indent — 2 spaces, matching the ratified § Draft wording.
const INDENT: &str = "  ";

/// "Fact" indent — 4 spaces, used ONLY inside the dense label/value blocks
/// of the TRUST and DATA-SOURCE sections (matches the ratified template's
/// literal indent for those two blocks; every other section's body lines use
/// [`INDENT`]).
const FACT_INDENT: &str = "    ";

/// Box-drawing rule width — matches the ratified § Draft wording template.
const RULE_WIDTH: usize = 76;

/// Push a heavy (`═`, header/footer) or light (`─`, section) box-drawing
/// rule line. Layout only — never a `crate::strings` const (ADR-0088 §
/// Design).
fn push_rule(lines: &mut Vec<String>, heavy: bool) {
    let ch = if heavy { '\u{2550}' } else { '\u{2500}' };
    lines.push(std::iter::repeat_n(ch, RULE_WIDTH).collect());
}

/// Push a blank separator line.
fn push_blank(lines: &mut Vec<String>) {
    lines.push(String::new());
}

/// Push a 2-space-indented body line.
fn push_body(lines: &mut Vec<String>, text: impl AsRef<str>) {
    lines.push(format!("{INDENT}{}", text.as_ref()));
}

/// Push a 4-space-indented "fact" line (TRUST / DATA-SOURCE dense blocks).
fn push_fact(lines: &mut Vec<String>, text: impl AsRef<str>) {
    lines.push(format!("{FACT_INDENT}{}", text.as_ref()));
}

/// Push a 2-space-indented `• ` bullet line ("What this is NOT").
fn push_bullet(lines: &mut Vec<String>, text: &str) {
    lines.push(format!("{INDENT}\u{2022} {text}"));
}

/// Push an `IF …` / `THEN …` clause pair — the `IF`/`THEN` keywords
/// left-padded to a shared width (5) so the clauses column-align, matching
/// the ratified template's literal `"IF   …"` / `"THEN …"` spacing.
fn push_if_then(lines: &mut Vec<String>, if_clause: &str, then_clause: &str) {
    lines.push(format!("{INDENT}{FORWARD_PLAN_RULE_IF:<5}{if_clause}"));
    lines.push(format!("{INDENT}{FORWARD_PLAN_RULE_THEN:<5}{then_clause}"));
}

/// Compose a dense "Label: value" fact line — `label` carries no trailing
/// punctuation of its own (e.g. `"Strategies tried"`).
fn label_colon(label: &str, value: &str) -> String {
    format!("{label}: {value}")
}

// ── The pure serialiser (ADR-0088 § D2) ────────────────────────────────────

/// Serialise a crowned plan into the P5 hand-off export text (markdown,
/// Q-HE-1). Pure + total: same inputs ⇒ byte-identical output (R-HE.1); no
/// wall-clock, no RNG, no I/O, no panic, no network/model call (R-HE.2), no
/// re-derived number (R-HE.3), and no LLM call — `narration` is embedded
/// verbatim ONLY if it is already [`NarrationState::Ready`] for this run.
///
/// `plan` is the pure-`ui` [`ForwardPlanView`] mirror (NOT the
/// `#[cfg(feature = "live")]`-gated `agent::config::ForwardPlan` — see
/// ADR-0088 § D2 "refines the brief"); `report` carries the outcome,
/// scorecard, data-quality readout, coin, window label, and the run seed;
/// `narration` is the F9 lifecycle state; `fx` is the €→USDT budget note, as
/// the SUGGEST screen threads it (`None` falls back to the config default
/// rate, exactly as `screens/forward_plan.rs::sizing_block` does).
#[must_use]
pub fn serialize_plan_export(
    plan: &ForwardPlanView,
    report: &BakeoffReportMirror,
    narration: &NarrationState,
    fx: Option<&FxNote>,
) -> String {
    let mut lines: Vec<String> = Vec::new();

    push_header(&mut lines, plan, report);
    push_measured_answer(&mut lines, report);
    push_right_now(&mut lines, plan);
    push_standing_rules(&mut lines, plan);
    push_sizing(&mut lines, plan, fx);
    if let Some(confidence) = plan.confidence.as_ref() {
        push_trust(&mut lines, confidence);
    }
    push_data_source(&mut lines, &report.data_quality);
    if plan.is_short_capable() {
        push_short_risk(&mut lines);
    }
    push_what_this_is_not(&mut lines);
    push_provenance(&mut lines, plan, report, narration);
    push_footer(&mut lines);

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Deterministic export filename (ADR-0088 § D6):
/// `plan-{coin}-{window-slug}-{seed8}.md`, e.g.
/// `plan-BTCUSDT-2024-h1-a1b2c3d4.md`. `window-slug` slugifies
/// `report.range_label`; `seed8` is the first 8 lowercase-hex chars of
/// `report.run_seed`. No wall-clock — same run ⇒ same name (idempotent
/// overwrite); a different seed ⇒ a different suffix (no collision).
#[must_use]
pub fn export_filename(report: &BakeoffReportMirror) -> String {
    let coin = report.coin.as_str();
    let slug = window_slug(report.range_label.as_str());
    let seed8: String = hex_lower(&report.run_seed).chars().take(8).collect();
    format!("plan-{coin}-{slug}-{seed8}.md")
}

/// Slugify a human window label (e.g. `"2024 H1"` → `"2024-h1"`,
/// `"last 30 days"` → `"last-30-days"`) — lowercase alnum runs joined by a
/// single `-`, no leading/trailing `-`. Pure + total, no panic.
fn window_slug(range_label: &str) -> String {
    let mut slug = String::with_capacity(range_label.len());
    let mut last_was_dash = false;
    for ch in range_label.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash && !slug.is_empty() {
            slug.push('-');
            last_was_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    slug
}

/// Lowercase-hex encode a byte slice. Pure + total, no panic, no new dep
/// (`[u8; 32]` is `core`/`std` — no `hex` crate pulled in).
fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
            // `write!` to a `String` via `core::fmt::Write` is infallible (no
            // I/O) — the `Result` is discarded, not propagated.
            let _ = write!(out, "{b:02x}");
            out
        })
}

// ── Header (title + meta + not-advice banner + hand-off frame) ────────────────

fn push_header(lines: &mut Vec<String>, plan: &ForwardPlanView, report: &BakeoffReportMirror) {
    push_rule(lines, true);
    push_body(lines, PLAN_EXPORT_TITLE);
    push_body(lines, header_meta_line(plan, report));
    push_rule(lines, true);
    push_blank(lines);

    push_body(lines, PLAN_EXPORT_NOT_ADVICE_BANNER);
    push_body(lines, FORWARD_PLAN_DISCLAIMER);
    push_blank(lines);

    push_body(lines, PLAN_EXPORT_HANDOFF_FRAME);
    push_blank(lines);
}

fn header_meta_line(plan: &ForwardPlanView, report: &BakeoffReportMirror) -> String {
    PLAN_EXPORT_HEADER_META_FMT
        .replace("{coin}", report.coin.as_str())
        .replace("{budget}", &fmt_eur(plan.budget))
        .replace("{window}", report.range_label.as_str())
}

// ── THE MEASURED ANSWER FOR THIS WINDOW (R-HE.4 modal-case-first) ─────────────

fn push_measured_answer(lines: &mut Vec<String>, report: &BakeoffReportMirror) {
    push_rule(lines, false);
    push_body(lines, PLAN_EXPORT_SECTION_MEASURED_ANSWER);
    push_rule(lines, false);
    push_blank(lines);

    let rec = &report.recommendation;
    match rec.outcome {
        OutcomeKind::BenchmarkWins => {
            push_body(
                lines,
                LEADERBOARD_HEADLINE_BENCHMARK_WINS.replace("{coin}", report.coin.as_str()),
            );
            push_blank(lines);
            push_body(lines, PLAN_EXPORT_BENCHMARK_WINS_BRIDGE);
        }
        OutcomeKind::ActiveWins => {
            push_body(
                lines,
                LEADERBOARD_HEADLINE_ACTIVE_WINS.replace("{winner}", rec.winner.as_str()),
            );
            match crown_credibility(rec.outcome, report.scorecard.as_ref()) {
                CrownCredibility::WeakEvidence => {
                    push_blank(lines);
                    push_body(lines, LEADERBOARD_CROWN_WEAK_EVIDENCE);
                    push_body(lines, LEADERBOARD_CROWN_WEAK_EVIDENCE_HINT);
                }
                CrownCredibility::Passes => {
                    push_blank(lines);
                    push_body(lines, LEADERBOARD_CROWN_PASSES_DSR);
                }
                // ADR-0085 NotApplicable — no computed figure to present; the
                // banner (and the export) carry NO credibility line.
                CrownCredibility::NotApplicable => {}
            }
        }
        OutcomeKind::AllFragile => {
            push_body(lines, LEADERBOARD_HEADLINE_ALL_FRAGILE);
        }
    }
    push_blank(lines);
}

// ── RIGHT NOW (as of the last bar) ─────────────────────────────────────────────

fn push_right_now(lines: &mut Vec<String>, plan: &ForwardPlanView) {
    push_rule(lines, false);
    push_body(lines, PLAN_EXPORT_SECTION_RIGHT_NOW);
    push_rule(lines, false);
    push_blank(lines);

    let stance_word = match plan.stance {
        PlanStanceView::Flat => FORWARD_PLAN_STANCE_FLAT,
        PlanStanceView::Long => FORWARD_PLAN_STANCE_LONG,
    };
    push_body(lines, stance_word);

    let as_of = FORWARD_PLAN_AS_OF_FMT
        .replace("{close}", &fmt_price(plan.last_close))
        .replace("{as_of}", plan.as_of_label.as_str());
    push_body(lines, as_of);

    if let Some(signal) = plan.latest_signal {
        let word = match signal {
            PlanSignalView::Buy => FORWARD_PLAN_SIGNAL_BUY,
            PlanSignalView::Sell => FORWARD_PLAN_SIGNAL_SELL,
            PlanSignalView::Hold => FORWARD_PLAN_SIGNAL_HOLD,
        };
        push_body(
            lines,
            FORWARD_PLAN_LATEST_SIGNAL_FMT.replace("{signal}", word),
        );
    }
    push_blank(lines);
}

// ── THE STANDING RULES (mirrors `screens/forward_plan.rs::rules_block`) ───────

fn push_standing_rules(lines: &mut Vec<String>, plan: &ForwardPlanView) {
    push_rule(lines, false);
    push_body(lines, PLAN_EXPORT_SECTION_STANDING_RULES);
    push_rule(lines, false);
    push_blank(lines);

    push_body(lines, FORWARD_PLAN_NOT_A_PREDICTION);
    push_blank(lines);

    if plan.is_always_short() {
        // The always-short benchmark control has NO long-entry rule — the
        // down-half append below carries its single standing rule. Mirrors
        // `screens/forward_plan.rs::rules_block`'s `is_always_short` branch.
    } else if plan.is_buy_and_hold() {
        push_body(lines, FORWARD_PLAN_RULE_BUY_AND_HOLD);
        push_blank(lines);
    } else if let PlanRuleView::Ensemble { method, members } = &plan.rule {
        push_ensemble_rules(lines, plan, *method, members);
    } else {
        let (entry, exit, show_compound_caveat) = rule_clauses(&plan.rule);
        push_if_then(lines, &entry.0, entry.1);
        push_blank(lines);
        if let Some((exit_if, exit_then)) = exit {
            push_if_then(lines, &exit_if, exit_then);
            push_blank(lines);
        }
        if show_compound_caveat {
            push_body(lines, FORWARD_PLAN_RULE_COMPOUND_CAVEAT);
            push_blank(lines);
        }
        push_body(
            lines,
            FORWARD_PLAN_CADENCE_FMT.replace("{horizon}", &plan.horizon_days.to_string()),
        );
        push_blank(lines);
    }

    // advisor-short-selling (T-U3) — for a crowned short-capable arm, append
    // the honest down-half rules to the SAME standing-rules block, so a
    // short reads AS a directional strategy (R-HE.7).
    if plan.is_short_capable() {
        push_short_rules_append(lines, plan);
    }
}

/// Append the short-capable down-half rules (ADR-0088 § Draft wording
/// Variant B). Starts DIRECTLY with the heading (no leading blank — the
/// preceding long-rules block already ends with one, per the ratified
/// template's literal spacing).
fn push_short_rules_append(lines: &mut Vec<String>, plan: &ForwardPlanView) {
    push_body(lines, FORWARD_PLAN_SHORT_RULES_HEADING);
    if plan.is_always_short() {
        push_body(lines, FORWARD_PLAN_RULE_ALWAYS_SHORT);
    } else {
        let bearish_if = bearish_flip_clause(plan);
        push_if_then(lines, &bearish_if, FORWARD_PLAN_RULE_SHORT_OPEN_THEN);
        // NO blank between the two short IF/THEN pairs — the ratified
        // template's literal spacing here differs from the long entry/exit
        // pair above (which IS blank-separated); reproduced byte-for-byte.
        push_if_then(
            lines,
            FORWARD_PLAN_RULE_SHORT_COVER_IF,
            FORWARD_PLAN_RULE_SHORT_COVER_THEN,
        );
    }
    push_blank(lines);
}

/// The bearish-flip IF clause for a short-capable arm — mirrors
/// `screens/forward_plan.rs::bearish_flip_clause` exactly (SMA reuses its
/// own parameterised exit copy; every other family falls back to the
/// generic bearish-flip string).
fn bearish_flip_clause(plan: &ForwardPlanView) -> String {
    match &plan.rule {
        PlanRuleView::SmaCross { fast_len, slow_len } => FORWARD_PLAN_RULE_SMA_EXIT_IF_FMT
            .replace("{fast}", &fast_len.to_string())
            .replace("{slow}", &slow_len.to_string()),
        _ => FORWARD_PLAN_RULE_SHORT_OPEN_IF_GENERIC.to_string(),
    }
}

/// The `(entry, optional-exit, show_compound_caveat)` IF/THEN clause triple
/// for a rule family — mirrors `screens/forward_plan.rs::rule_clauses`
/// exactly (over `&PlanRuleView` here — the caller never needs an owned
/// clone). `BuyAndHold`/`Ensemble` never reach here (handled by the caller);
/// the fallback arm is defensive-total, not a live path.
fn rule_clauses(
    rule: &PlanRuleView,
) -> ((String, &'static str), Option<(String, &'static str)>, bool) {
    match rule {
        PlanRuleView::SmaCross { fast_len, slow_len } => {
            let entry_if = FORWARD_PLAN_RULE_SMA_ENTRY_IF_FMT
                .replace("{fast}", &fast_len.to_string())
                .replace("{slow}", &slow_len.to_string());
            let exit_if = FORWARD_PLAN_RULE_SMA_EXIT_IF_FMT
                .replace("{fast}", &fast_len.to_string())
                .replace("{slow}", &slow_len.to_string());
            (
                (entry_if, FORWARD_PLAN_RULE_SMA_ENTRY_THEN),
                Some((exit_if, FORWARD_PLAN_RULE_SMA_EXIT_THEN)),
                false,
            )
        }
        PlanRuleView::MacdCross { fast, slow, signal } => {
            let entry_if = FORWARD_PLAN_RULE_MACD_ENTRY_IF_FMT
                .replace("{fast}", &fast.to_string())
                .replace("{slow}", &slow.to_string())
                .replace("{signal}", &signal.to_string());
            (
                (entry_if, FORWARD_PLAN_RULE_MACD_ENTRY_THEN),
                Some((
                    FORWARD_PLAN_RULE_MACD_EXIT_IF.to_string(),
                    FORWARD_PLAN_RULE_MACD_EXIT_THEN,
                )),
                true,
            )
        }
        PlanRuleView::RsiReversion { len, lower } => {
            let entry_if = FORWARD_PLAN_RULE_RSI_ENTRY_IF_FMT
                .replace("{len}", &len.to_string())
                .replace("{lower}", &lower.to_string());
            let exit_if = FORWARD_PLAN_RULE_RSI_EXIT_IF_FMT.replace("{lower}", &lower.to_string());
            (
                (entry_if, FORWARD_PLAN_RULE_RSI_ENTRY_THEN),
                Some((exit_if, FORWARD_PLAN_RULE_RSI_EXIT_THEN)),
                true,
            )
        }
        PlanRuleView::BollingerReversion { len, k_tenths } => {
            let k = format!("{}.{}", k_tenths / 10, k_tenths % 10);
            let entry_if = FORWARD_PLAN_RULE_BBANDS_ENTRY_IF_FMT
                .replace("{len}", &len.to_string())
                .replace("{k}", &k);
            (
                (entry_if, FORWARD_PLAN_RULE_BBANDS_ENTRY_THEN),
                Some((
                    FORWARD_PLAN_RULE_BBANDS_EXIT_IF.to_string(),
                    FORWARD_PLAN_RULE_BBANDS_EXIT_THEN,
                )),
                true,
            )
        }
        // Defensive: `BuyAndHold` + `Ensemble` are handled by the caller's
        // `is_buy_and_hold` / `Ensemble` branches and never reach here.
        PlanRuleView::BuyAndHold | PlanRuleView::Ensemble { .. } => (
            (String::new(), FORWARD_PLAN_RULE_SMA_ENTRY_THEN),
            None,
            false,
        ),
    }
}

/// The ensemble (signal-vote) standing rules — named members + method +
/// live tally + caveat + cadence. Mirrors
/// `screens/forward_plan.rs::ensemble_rules` (not one of the T5 golden
/// variants, but kept structurally faithful — same consts, same order).
fn push_ensemble_rules(
    lines: &mut Vec<String>,
    plan: &ForwardPlanView,
    method: PlanVoteMethodView,
    members: &[SmolStr],
) {
    let names = member_brace_list(members);
    let if_clause = match method {
        PlanVoteMethodView::Majority { k, .. } => FORWARD_PLAN_RULE_ENSEMBLE_MAJORITY_NAMED_FMT
            .replace("{k}", &k.to_string())
            .replace("{members}", &names),
        PlanVoteMethodView::Unanimous { .. } => {
            FORWARD_PLAN_RULE_ENSEMBLE_UNANIMOUS_NAMED_FMT.replace("{members}", &names)
        }
    };
    push_if_then(lines, &if_clause, FORWARD_PLAN_RULE_SMA_ENTRY_THEN);
    push_blank(lines);

    let member_count = u32::try_from(members.len()).unwrap_or(u32::MAX);
    let n = member_count.max(method.member_count());
    let quorum = method.quorum();
    let long = match plan.stance {
        PlanStanceView::Long => quorum,
        PlanStanceView::Flat => quorum.saturating_sub(1),
    };
    let stance_word = match plan.stance {
        PlanStanceView::Long => FORWARD_PLAN_STANCE_LONG,
        PlanStanceView::Flat => FORWARD_PLAN_STANCE_FLAT,
    };
    let tally = FORWARD_PLAN_RULE_ENSEMBLE_TALLY_FMT
        .replace("{long}", &long.to_string())
        .replace("{n}", &n.to_string())
        .replace("{stance}", stance_word);
    push_body(lines, tally);
    push_blank(lines);

    push_body(lines, FORWARD_PLAN_RULE_ENSEMBLE_CAVEAT);
    push_blank(lines);

    push_body(
        lines,
        FORWARD_PLAN_CADENCE_FMT.replace("{horizon}", &plan.horizon_days.to_string()),
    );
    push_blank(lines);
}

/// Render a member-label list as a brace-list, mirroring
/// `screens/forward_plan.rs::member_brace_list`.
fn member_brace_list(members: &[SmolStr]) -> String {
    if members.is_empty() {
        return "the member strategies".to_string();
    }
    let joined = members
        .iter()
        .map(SmolStr::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{{joined}}}")
}

// ── SIZING AT YOUR €200 BUDGET ─────────────────────────────────────────────────

fn push_sizing(lines: &mut Vec<String>, plan: &ForwardPlanView, fx: Option<&FxNote>) {
    push_rule(lines, false);
    push_body(lines, PLAN_EXPORT_SECTION_SIZING);
    push_rule(lines, false);
    push_blank(lines);

    push_body(lines, budget_line_text(plan, fx));
    push_blank(lines);

    let units = fmt_qty(plan.projected_units);
    let close = fmt_price(plan.last_close);
    let sizing_line = if plan.is_buy_and_hold() {
        FORWARD_PLAN_SIZING_BUY_AND_HOLD_FMT
    } else {
        match plan.stance {
            PlanStanceView::Flat => FORWARD_PLAN_SIZING_FLAT_FMT,
            PlanStanceView::Long => FORWARD_PLAN_SIZING_LONG_FMT,
        }
    }
    .replace("{units}", &units)
    .replace("{close}", &close);
    push_body(lines, sizing_line);

    if plan.sizing_capped {
        push_blank(lines);
        push_body(lines, FORWARD_PLAN_SIZING_CAPPED_NOTE);
    }
    push_blank(lines);
}

/// The honest EUR→USDT budget + hard-cap line — mirrors
/// `screens/forward_plan.rs::sizing_block`'s `fx_note`/fallback logic
/// exactly (same formatters, same default-rate fallback when `fx` is
/// `None`).
fn budget_line_text(plan: &ForwardPlanView, fx: Option<&FxNote>) -> String {
    if let Some(note) = fx {
        FORWARD_PLAN_BUDGET_LINE_FMT
            .replace("{eur}", &fmt_eur_plain(note.eur))
            .replace("{usdt}", &fmt_usdt_plain(note.usdt))
            .replace("{rate}", &fmt_rate(note.rate))
            .replace("{source}", note.source.as_str())
    } else {
        use trading_core::{BudgetConversion, DEFAULT_EUR_USD_RATE, FxRate};
        let rate = FxRate::config(DEFAULT_EUR_USD_RATE);
        let conv = BudgetConversion::new(plan.budget, rate);
        FORWARD_PLAN_BUDGET_LINE_FMT
            .replace("{eur}", &fmt_eur_plain(conv.eur()))
            .replace("{usdt}", &fmt_usdt_plain(conv.usdt().amount()))
            .replace("{rate}", &fmt_rate(conv.rate().rate()))
            .replace("{source}", conv.rate().source())
    }
}

// ── HOW MUCH TO TRUST THIS (only when `plan.confidence` is `Some`) ────────────

fn push_trust(lines: &mut Vec<String>, confidence: &ConfidenceSummaryView) {
    push_rule(lines, false);
    push_body(lines, PLAN_EXPORT_SECTION_TRUST);
    push_rule(lines, false);
    push_blank(lines);

    push_fact(
        lines,
        label_colon(
            FORWARD_PLAN_CONFIDENCE_CANDIDATES_LABEL,
            &confidence.n_candidates.to_string(),
        ),
    );
    let dsr_pct = format!("{:.0}\u{202f}%", confidence.deflated_sharpe * 100.0);
    push_fact(
        lines,
        label_colon(FORWARD_PLAN_CONFIDENCE_DSR_LABEL, &dsr_pct),
    );
    push_fact(lines, FORWARD_PLAN_CONFIDENCE_DSR_GLOSS);
    let beats = if confidence.crown_clears_dsr {
        FORWARD_PLAN_CONFIDENCE_BEATS_HOLD_YES
    } else {
        FORWARD_PLAN_CONFIDENCE_BEATS_HOLD_NO
    };
    push_fact(
        lines,
        format!("{FORWARD_PLAN_CONFIDENCE_BEATS_HOLD_LABEL} {beats}"),
    );
    let min_btl = FORWARD_PLAN_CONFIDENCE_MIN_BTL_FMT
        .replace("{years}", &format!("{:.1}", confidence.min_btl_years));
    push_fact(
        lines,
        label_colon(FORWARD_PLAN_CONFIDENCE_MIN_BTL_LABEL, &min_btl),
    );
    push_fact(lines, FORWARD_PLAN_CONFIDENCE_NOTE);
    push_blank(lines);

    push_body(lines, PLAN_EXPORT_ONE_IN_FIVE_NOTE);
    push_blank(lines);
}

// ── WHERE THIS DATA CAME FROM ───────────────────────────────────────────────────

fn push_data_source(lines: &mut Vec<String>, dq: &DataQualityView) {
    push_rule(lines, false);
    push_body(lines, PLAN_EXPORT_SECTION_DATA_SOURCE);
    push_rule(lines, false);
    push_blank(lines);

    push_fact(
        lines,
        label_colon(LEADERBOARD_DATA_QUALITY_VENUE_LABEL, &dq.venue),
    );
    push_fact(
        lines,
        label_colon(LEADERBOARD_DATA_QUALITY_PROVENANCE_LABEL, &dq.provenance),
    );
    push_fact(
        lines,
        label_colon(
            LEADERBOARD_DATA_QUALITY_TRUST_LABEL,
            dq.venue_trust.badge_label(),
        ),
    );
    push_fact(
        lines,
        label_colon(LEADERBOARD_DATA_QUALITY_SURVIVAL_LABEL, &dq.survival_note),
    );
    // Warnings — rendered ONLY when non-empty (the honest "nothing to flag"
    // case for the default deep-liquidity universe renders no row at all,
    // mirroring `screens/leaderboard.rs::data_quality_block`).
    if !dq.warnings.is_empty() {
        push_fact(lines, format!("{LEADERBOARD_DATA_QUALITY_WARNINGS_LABEL}:"));
        for w in &dq.warnings {
            push_fact(lines, w.copy());
        }
    }
    push_blank(lines);

    push_body(lines, PLAN_EXPORT_ERA_QUALIFIED_THESIS);
    push_blank(lines);
}

// ── SHORT RISK (short-only, R-HE.7 — mandatory, inserted before "What this is
// NOT") ─────────────────────────────────────────────────────────────────────────

fn push_short_risk(lines: &mut Vec<String>) {
    push_rule(lines, false);
    push_body(lines, PLAN_EXPORT_SECTION_SHORT_RISK);
    push_rule(lines, false);
    push_blank(lines);

    push_body(lines, SHORT_UNBOUNDED_LOSS_DISCLAIMER);
    push_body(lines, FORWARD_PLAN_RULE_SHORT_LIQUIDATION);
    push_blank(lines);
}

// ── WHAT THIS IS NOT ─────────────────────────────────────────────────────────────

fn push_what_this_is_not(lines: &mut Vec<String>) {
    push_rule(lines, false);
    push_body(lines, PLAN_EXPORT_SECTION_WHAT_THIS_IS_NOT);
    push_rule(lines, false);
    push_blank(lines);

    for bullet in [
        PLAN_EXPORT_NOT_BULLET_ADVICE,
        PLAN_EXPORT_NOT_BULLET_PREDICTION,
        PLAN_EXPORT_NOT_BULLET_PAST,
        PLAN_EXPORT_NOT_BULLET_CHANCE,
        PLAN_EXPORT_NOT_BULLET_PAPER,
    ] {
        push_bullet(lines, bullet);
    }
    push_blank(lines);
}

// ── PROVENANCE (so you can reproduce this) ───────────────────────────────────────

fn push_provenance(
    lines: &mut Vec<String>,
    plan: &ForwardPlanView,
    report: &BakeoffReportMirror,
    narration: &NarrationState,
) {
    push_rule(lines, false);
    push_body(lines, PLAN_EXPORT_SECTION_PROVENANCE);
    push_rule(lines, false);
    push_blank(lines);

    push_body(
        lines,
        PLAN_EXPORT_PROVENANCE_COIN_FMT
            .replace("{coin}", report.coin.as_str())
            .replace("{budget}", &fmt_eur(plan.budget))
            .replace("{window}", report.range_label.as_str()),
    );

    // Crowned pick: the raw strategy id, verbatim — traceable/reproducible
    // (searchable in `config/strategies/*.toml`), never a fabricated family
    // name; matches `report.recommendation.winner`, the SAME source
    // `headline_copy` fills `{winner}` from (byte-fidelity with the screen).
    push_body(
        lines,
        PLAN_EXPORT_PROVENANCE_PICK_FMT
            .replace("{pick}", report.recommendation.winner.as_str())
            .replace("{horizon}", &plan.horizon_days.to_string()),
    );

    push_body(
        lines,
        PLAN_EXPORT_PROVENANCE_SEED_FMT
            .replace("{seed}", &hex_lower(&report.run_seed))
            .replace("{last_bar}", plan.as_of_label.as_str()),
    );

    push_body(lines, PLAN_EXPORT_REPRODUCE_HINT);

    // F9 narration (ADR-0064) — embedded ONLY if already generated + passed
    // its faithfulness post-check for THIS run; never re-generated here
    // (R-HE.2 — no model call in the export path).
    if let NarrationState::Ready(prose) = narration {
        push_body(lines, LEADERBOARD_EXPLAIN_LLM_LABEL);
        push_body(lines, prose.as_str());
    }
    push_blank(lines);
}

// ── Footer ───────────────────────────────────────────────────────────────────────

fn push_footer(lines: &mut Vec<String>) {
    push_rule(lines, true);
    push_body(lines, PLAN_EXPORT_FOOTER);
    push_rule(lines, true);
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::leaderboard::NarrationState;
    use rust_decimal_macros::dec;

    fn no_fx() -> Option<FxNote> {
        None
    }

    // ── Golden text — variant 1: BenchmarkWins (negative control: NO
    // credibility badge) ─────────────────────────────────────────────────────

    #[test]
    fn benchmark_wins_carries_headline_and_bridge_but_no_credibility_badge() {
        let report = fixtures::fake_bakeoff_report_mirror_benchmark_wins();
        let plan = fixtures::fake_forward_plan_buy_and_hold();
        let out = serialize_plan_export(
            &plan,
            &report,
            &NarrationState::NotRequested,
            no_fx().as_ref(),
        );

        let expected_headline = LEADERBOARD_HEADLINE_BENCHMARK_WINS.replace("{coin}", "BTCUSDT");
        assert!(
            out.contains(&expected_headline),
            "must carry the BenchmarkWins headline verbatim:\n{out}"
        );
        assert!(
            out.contains(PLAN_EXPORT_BENCHMARK_WINS_BRIDGE),
            "must carry the benchmark-wins bridge line verbatim:\n{out}"
        );

        // Negative control — NO credibility badge on a hold pick (ADR-0085
        // `NotApplicable`): neither credibility const may appear anywhere.
        assert!(
            !out.contains(LEADERBOARD_CROWN_WEAK_EVIDENCE),
            "BenchmarkWins must carry NO weak-evidence credibility badge:\n{out}"
        );
        assert!(
            !out.contains(LEADERBOARD_CROWN_PASSES_DSR),
            "BenchmarkWins must carry NO passes-DSR credibility badge:\n{out}"
        );
        // Not short-capable — no short risk section, no unbounded-loss line.
        assert!(!out.contains(PLAN_EXPORT_SECTION_SHORT_RISK));
        assert!(!out.contains(SHORT_UNBOUNDED_LOSS_DISCLAIMER));

        // Every honesty section is present.
        assert!(out.contains(PLAN_EXPORT_SECTION_WHAT_THIS_IS_NOT));
        assert!(out.contains(FORWARD_PLAN_DISCLAIMER));
        assert!(out.contains(&dq_survival_note()));
    }

    // ── Golden text — variant 2: ActiveWins + WeakEvidence (the money shot)
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn active_wins_weak_evidence_carries_the_credibility_caveat_in_body() {
        let report = fixtures::fake_bakeoff_report_mirror_five_arm();
        assert!(
            report
                .scorecard
                .as_ref()
                .is_some_and(|sc| !sc.crown_clears_dsr),
            "the five-arm fixture's crown must FAIL DSR (the money shot)"
        );
        let plan = fixtures::fake_forward_plan();
        let out = serialize_plan_export(
            &plan,
            &report,
            &NarrationState::NotRequested,
            no_fx().as_ref(),
        );

        let expected_headline = LEADERBOARD_HEADLINE_ACTIVE_WINS.replace("{winner}", "v0.sma");
        assert!(
            out.contains(&expected_headline),
            "must carry the ActiveWins headline verbatim:\n{out}"
        );
        assert!(
            out.contains(LEADERBOARD_CROWN_WEAK_EVIDENCE),
            "a WeakEvidence crown must carry the weak-evidence caveat IN-BODY:\n{out}"
        );
        assert!(
            out.contains(LEADERBOARD_CROWN_WEAK_EVIDENCE_HINT),
            "the weak-evidence hint must accompany the caveat:\n{out}"
        );
        assert!(
            !out.contains(LEADERBOARD_CROWN_PASSES_DSR),
            "a WeakEvidence crown must NOT ALSO carry the Passes line:\n{out}"
        );

        // The €200 sizing + budget line (config-default FX fallback, `fx =
        // None`) is present.
        assert!(
            out.contains("\u{20ac}200"),
            "the €200 budget must appear:\n{out}"
        );
        // The always-present survivorship caveat.
        assert!(out.contains(&dq_survival_note()));
        // The disclaimers.
        assert!(out.contains(FORWARD_PLAN_DISCLAIMER));
        assert!(out.contains(FORWARD_PLAN_NOT_A_PREDICTION));
        // The ~1-in-5 honesty note (P2-2) — only present because
        // `fake_forward_plan()` carries no confidence block... wait, the
        // TRUST section (and this note) only fire when `plan.confidence` is
        // `Some`; `fake_forward_plan()` has `confidence: None`.
        assert!(
            !out.contains(PLAN_EXPORT_SECTION_TRUST),
            "fake_forward_plan() has confidence: None — no TRUST section:\n{out}"
        );
        // Not short-capable.
        assert!(!out.contains(PLAN_EXPORT_SECTION_SHORT_RISK));
    }

    /// Same money-shot report + a plan carrying a confidence block — proves
    /// the TRUST section + the ~1-in-5 note DO fire when `confidence` is
    /// `Some`.
    #[test]
    fn active_wins_with_confidence_carries_the_trust_section_and_one_in_five_note() {
        let report = fixtures::fake_bakeoff_report_mirror_five_arm();
        let plan = fixtures::fake_forward_plan_with_confidence();
        let out = serialize_plan_export(
            &plan,
            &report,
            &NarrationState::NotRequested,
            no_fx().as_ref(),
        );

        assert!(out.contains(PLAN_EXPORT_SECTION_TRUST));
        assert!(out.contains(FORWARD_PLAN_CONFIDENCE_CANDIDATES_LABEL));
        assert!(out.contains(FORWARD_PLAN_CONFIDENCE_DSR_GLOSS));
        assert!(out.contains(FORWARD_PLAN_CONFIDENCE_NOTE));
        assert!(
            out.contains(PLAN_EXPORT_ONE_IN_FIVE_NOTE),
            "the honest ~1-in-5 search note must accompany the TRUST section:\n{out}"
        );
    }

    // ── Golden text — variant 3: ActiveWins + Passes (positive control) ────

    #[test]
    fn active_wins_passes_dsr_carries_the_passes_line_not_weak_evidence() {
        let report = fixtures::fake_bakeoff_report_mirror_five_arm_passes_dsr();
        assert!(
            report
                .scorecard
                .as_ref()
                .is_some_and(|sc| sc.crown_clears_dsr),
            "the passes-DSR fixture's crown must CLEAR DSR"
        );
        let plan = fixtures::fake_forward_plan();
        let out = serialize_plan_export(
            &plan,
            &report,
            &NarrationState::NotRequested,
            no_fx().as_ref(),
        );

        assert!(
            out.contains(LEADERBOARD_CROWN_PASSES_DSR),
            "a Passes crown must carry the passes-DSR line:\n{out}"
        );
        assert!(
            !out.contains(LEADERBOARD_CROWN_WEAK_EVIDENCE),
            "a Passes crown must NOT carry the weak-evidence caveat:\n{out}"
        );
        assert!(!out.contains(LEADERBOARD_CROWN_WEAK_EVIDENCE_HINT));
    }

    // ── Golden text — variant 4: short-crowned (mandatory extra section) ───

    #[test]
    fn short_capable_plan_carries_the_unbounded_loss_disclaimer_and_liquidation_line() {
        let report = fixtures::fake_bakeoff_report_mirror_with_shorts();
        let plan = fixtures::fake_forward_plan_short();
        assert!(plan.is_short_capable(), "fixture must be short-capable");
        let out = serialize_plan_export(
            &plan,
            &report,
            &NarrationState::NotRequested,
            no_fx().as_ref(),
        );

        assert!(
            out.contains(PLAN_EXPORT_SECTION_SHORT_RISK),
            "a short-capable plan must carry the SHORT RISK section:\n{out}"
        );
        assert!(
            out.contains(SHORT_UNBOUNDED_LOSS_DISCLAIMER),
            "the mandatory unbounded-loss disclaimer (R-SS.4) must be verbatim present:\n{out}"
        );
        assert!(
            out.contains(FORWARD_PLAN_RULE_SHORT_LIQUIDATION),
            "the liquidation reality line must be present:\n{out}"
        );
        assert!(out.contains(FORWARD_PLAN_SHORT_RULES_HEADING));
        assert!(out.contains(FORWARD_PLAN_RULE_SHORT_COVER_IF));
        assert!(out.contains(FORWARD_PLAN_RULE_SHORT_COVER_THEN));

        // The short-crowned SmaCross fixture's bearish-flip clause reuses
        // its OWN parameterised exit copy (fast=12, slow=26) — never the
        // generic fallback (mirrors `bearish_flip_clause`).
        let expected_bearish_if = FORWARD_PLAN_RULE_SMA_EXIT_IF_FMT
            .replace("{fast}", "12")
            .replace("{slow}", "26");
        assert!(
            out.contains(&expected_bearish_if),
            "the SMA-specific bearish-flip clause must be reused, not the generic fallback:\n{out}"
        );
    }

    /// Negative control for the short-risk section: a NON-short-capable plan
    /// (buy-and-hold) never carries it, even when short-family consts exist
    /// elsewhere in `crate::strings` — proves the branch tracks
    /// `is_short_capable()`, not a tautology.
    #[test]
    fn non_short_capable_plan_never_carries_short_risk_section() {
        let report = fixtures::fake_bakeoff_report_mirror_benchmark_wins();
        let plan = fixtures::fake_forward_plan_buy_and_hold();
        assert!(!plan.is_short_capable());
        let out = serialize_plan_export(
            &plan,
            &report,
            &NarrationState::NotRequested,
            no_fx().as_ref(),
        );
        assert!(!out.contains(PLAN_EXPORT_SECTION_SHORT_RISK));
        assert!(!out.contains(SHORT_UNBOUNDED_LOSS_DISCLAIMER));
    }

    // ── F9 narration — embedded ONLY when Ready ─────────────────────────────

    #[test]
    fn narration_embeds_only_when_ready() {
        let report = fixtures::fake_bakeoff_report_mirror_five_arm();
        let plan = fixtures::fake_forward_plan();

        let not_requested = serialize_plan_export(
            &plan,
            &report,
            &NarrationState::NotRequested,
            no_fx().as_ref(),
        );
        assert!(!not_requested.contains(LEADERBOARD_EXPLAIN_LLM_LABEL));

        let fell_back =
            serialize_plan_export(&plan, &report, &NarrationState::FellBack, no_fx().as_ref());
        assert!(!fell_back.contains(LEADERBOARD_EXPLAIN_LLM_LABEL));

        let ready_prose = "SMA crossover came out on top here.";
        let ready = serialize_plan_export(
            &plan,
            &report,
            &NarrationState::Ready(smol_str::SmolStr::new(ready_prose)),
            no_fx().as_ref(),
        );
        assert!(ready.contains(LEADERBOARD_EXPLAIN_LLM_LABEL));
        assert!(ready.contains(ready_prose));
    }

    // ── Byte-determinism (R-HE.1) ────────────────────────────────────────────

    #[test]
    fn same_inputs_produce_byte_identical_output() {
        let report = fixtures::fake_bakeoff_report_mirror_five_arm();
        let plan = fixtures::fake_forward_plan();
        let narration = NarrationState::Ready(smol_str::SmolStr::new("A faithful summary."));
        let fx = FxNote {
            eur: dec!(200),
            usdt: dec!(216.00),
            rate: dec!(1.08),
            source: smol_str::SmolStr::new("config"),
            as_of: smol_str::SmolStr::new("2024-06-30"),
        };

        let a = serialize_plan_export(&plan, &report, &narration, Some(&fx));
        let b = serialize_plan_export(&plan, &report, &narration, Some(&fx));
        assert_eq!(a, b, "same inputs must produce byte-identical output");

        // A different run seed changes the provenance line (and NOTHING
        // else drifts non-deterministically) — proves the output is a pure
        // function of the inputs, not of wall-clock/RNG.
        let mut report2 = report.clone();
        report2.run_seed = [0xffu8; 32];
        let c = serialize_plan_export(&plan, &report2, &narration, Some(&fx));
        assert_ne!(a, c, "a different run seed must change the provenance line");
    }

    #[test]
    fn short_capable_variant_is_also_byte_deterministic() {
        let report = fixtures::fake_bakeoff_report_mirror_with_shorts();
        let plan = fixtures::fake_forward_plan_short();
        let a = serialize_plan_export(
            &plan,
            &report,
            &NarrationState::NotRequested,
            no_fx().as_ref(),
        );
        let b = serialize_plan_export(
            &plan,
            &report,
            &NarrationState::NotRequested,
            no_fx().as_ref(),
        );
        assert_eq!(a, b);
    }

    // ── Filename determinism (ADR-0088 § D6) ────────────────────────────────

    #[test]
    fn export_filename_matches_the_adr_illustrative_example() {
        // fake_bakeoff_report_mirror_five_arm(): coin=BTCUSDT,
        // range_label="2024 H1", run_seed = a1b2c3d4 + zeros → seed8 =
        // "a1b2c3d4" — the EXACT filename ADR-0088 § Design "Filename +
        // determinism" gives as its illustrative example.
        let report = fixtures::fake_bakeoff_report_mirror_five_arm();
        assert_eq!(export_filename(&report), "plan-BTCUSDT-2024-h1-a1b2c3d4.md");
    }

    #[test]
    fn export_filename_is_deterministic_slug_safe_and_wall_clock_free() {
        let report = fixtures::fake_bakeoff_report_mirror_benchmark_wins();
        let a = export_filename(&report);
        let b = export_filename(&report);
        assert_eq!(a, b, "same report ⇒ same filename (no wall-clock)");
        assert!(a.starts_with("plan-BTCUSDT-2024-h1-"));
        assert!(
            std::path::Path::new(&a)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        );
        // Slug-safe: no spaces, no uppercase in the window slug segment.
        assert!(!a.contains(' '));
    }

    #[test]
    fn export_filename_differs_when_run_seed_differs() {
        let mut a = fixtures::fake_bakeoff_report_mirror_five_arm();
        let mut b = a.clone();
        b.run_seed = [0xffu8; 32];
        assert_ne!(export_filename(&a), export_filename(&b));
        // Same coin/window ⇒ same filename when seeds happen to agree too.
        b.run_seed = a.run_seed;
        assert_eq!(export_filename(&a), export_filename(&b));
        let _ = &mut a; // keep `a` mutable-binding-shaped for symmetry; no mutation needed.
    }

    #[test]
    fn window_slug_handles_relative_lookback_labels() {
        assert_eq!(window_slug("2024 H1"), "2024-h1");
        assert_eq!(window_slug("last 30 days"), "last-30-days");
        assert_eq!(window_slug("custom window"), "custom-window");
    }

    // ── Small helper ──────────────────────────────────────────────────────

    fn dq_survival_note() -> String {
        crate::leaderboard::DataQualityView::for_symbol("BTCUSDT").survival_note
    }
}
