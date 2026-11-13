//! F9 LLM narration generator — the `agent::plan` twin (ADR-0064 § D1).
//!
//! This module owns four pieces, as prescribed by ADR-0064 § D1:
//!
//! 1. [`NarrationFacts`] — the exact machine values the prompt may speak about.
//!    Built from `backtest::BakeoffReport` at this one boundary.
//! 2. The role-locked, cache-marked prompt builder (layer 1).
//! 3. [`check_faithful`] — the FROZEN deterministic faithfulness post-check
//!    (layer 2 — the load-bearing guard).  ADR-0064 § D2.
//! 4. [`generate_narration`] — the async orchestrator that builds the request,
//!    calls the provider, runs the post-check, and returns a `core`-clean
//!    [`NarrationOutcome`].
//!
//! ## Layering invariant
//!
//! `NarrationOutcome { Ready(SmolStr) | FellBack }` carries NO `llm` type and
//! NO `ChatResponse`.  It crosses the agent→iced seam the same way
//! `ForwardPlan` does.  The `ui` crate names `agent::NarrationOutcome` ONLY in
//! the `#[cfg(feature = "live")]` recipe/adapter — the single edit site if a
//! name drifts.
//!
//! ## Frozen predicate set
//!
//! The predicate set + banned-phrase list in `check_faithful` are FROZEN by
//! ADR-0064 § D2.  A change requires an ADR-0064 amendment — NOT an ad-hoc
//! edit.
//!
//! ## P2-1 faithfulness hardening (ADR-0064 amendment 2026-07-01)
//!
//! Two additive layers on top of the D2 predicate set, still `llm`-free and
//! deterministic:
//!
//! - **Verbatim-number match (P3 hardening)** — [`NarrationFacts::allowed_numbers`]
//!   now returns an owned `HashSet<String>` (was `Vec<String>`, converted to a
//!   `HashSet` inline at the `check_faithful` call site on every invocation),
//!   so every numeric token the LLM used must be a byte-exact member of the
//!   *exact* set of numbers the LLM was told. This was already the P3
//!   mechanism (exact-string, never float-tolerant); the change is
//!   representational (dedup + O(1) lookup, no redundant `Vec`→`HashSet`
//!   round-trip) plus a widened banned-phrase list (below) — no weakening of
//!   the match.
//! - **Prediction/causation banned-phrase list** — [`BANNED_PHRASES`] gains
//!   prediction verbs (`"expected to"`, `"forecast"`, `"predict"`, …),
//!   causation clauses (`"because of"`, `"driven by"`, …), and
//!   recommendation phrases (`"you should"`, `"we recommend"`, …). Still a
//!   case-insensitive substring match; still FROZEN by the same ADR-0064 § D2
//!   discipline — a further change requires another amendment.

use std::sync::Arc;

use async_trait::async_trait;
use smol_str::SmolStr;

use cost::{AgentRole, LlmTier};
use llm::{
    CachedSystemPromptBuilder, ChatMessage, ChatRequest, ContentBlock, LlmProvider, MessageRole,
    ModelId, SystemBlock,
};

// ── NarrationOutcome ─────────────────────────────────────────────────────────

/// The `core`-clean result that crosses the agent→iced seam (ADR-0064 § D1).
///
/// `Clone + Debug` — no `llm` type, no `ChatResponse`.  This is what the
/// `#[cfg(feature = "live")]` recipe/adapter in `ui` reads to produce
/// `Message::BakeoffNarrationCompleted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NarrationOutcome {
    /// A faithful narration passed the post-check — the prose can be shown.
    Ready(SmolStr),
    /// The narration was unavailable, errored, over-budget, or failed the
    /// post-check.  The templated copy is the honest fallback.  Silent.
    FellBack,
}

// ── NarrationRequest (the iced→agent channel payload) ────────────────────────

/// The request the iced thread sends over the narration channel to the agent
/// (ADR-0064 § D3).  `core`-typed — no `BakeoffReport`, no engine type.
#[derive(Debug, Clone)]
pub struct NarrationRequest {
    /// The structured facts the generator will render into prose.
    pub facts: NarrationFacts,
}

// ── NarrationFacts ───────────────────────────────────────────────────────────

/// The exact machine values the prompt is allowed to speak about (ADR-0064 § D1).
///
/// Built once from `backtest::BakeoffReport` at the
/// `BakeoffReportMirror::from_report` boundary.  All KPIs are pre-rendered to
/// their canonical `String` representations (the `ui::widgets::num` formatters)
/// so `check_faithful` P3 can do an exact-string match without any `ui` dep.
#[derive(Debug, Clone)]
pub struct NarrationFacts {
    /// The `RecommendationOutcome` (drives P2 contradiction check).
    pub outcome: NarrationOutcome_,
    /// The crowned strategy's display id (the string the LLM may name as winner).
    pub winner_id: SmolStr,
    /// All candidate ids (winner + runners-up + benchmark).
    pub candidate_ids: Vec<SmolStr>,
    /// Per-candidate KPI strings in canonical format (the allowed-number set for P3).
    /// Each inner vec is `[sharpe, sortino, calmar, total_return_pct, max_drawdown, trade_count]`.
    pub candidate_kpi_strings: Vec<CandidateKpiStrings>,
    /// The robustness label for the winner (drives P2 AllFragile check).
    pub winner_robustness_label: Option<SmolStr>,
    /// Human-readable reason codes (for the prompt's dynamic section).
    pub reason_codes: Vec<SmolStr>,
}

/// The `RecommendationOutcome` mirrored as a plain agent-owned enum so
/// `NarrationFacts` carries no `backtest` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NarrationOutcome_ {
    ActiveWins,
    BenchmarkWins,
    AllFragile,
}

/// Pre-rendered canonical KPI strings for one candidate (the P3 allowed set).
#[derive(Debug, Clone)]
pub struct CandidateKpiStrings {
    pub strategy_id: SmolStr,
    pub sharpe: String,
    pub sortino: String,
    pub calmar: String,
    pub total_return_pct: String,
    pub max_drawdown: String,
    pub trade_count: String,
}

impl NarrationFacts {
    /// Build `NarrationFacts` from a `backtest::BakeoffReport`.
    ///
    /// This is the ONE boundary where the engine `BakeoffReport` is read —
    /// the `BakeoffReportMirror::from_report` precedent (ADR-0064 § D1).
    #[must_use]
    pub fn from_report(report: &backtest::BakeoffReport) -> Self {
        let r = &report.rationale;

        let outcome = match r.outcome {
            backtest::RecommendationOutcome::ActiveWins => NarrationOutcome_::ActiveWins,
            backtest::RecommendationOutcome::BenchmarkWins => NarrationOutcome_::BenchmarkWins,
            backtest::RecommendationOutcome::AllFragile => NarrationOutcome_::AllFragile,
        };

        let winner_id = SmolStr::new(r.winner.0.as_str());

        let candidate_ids: Vec<SmolStr> = report
            .candidates
            .iter()
            .map(|c| SmolStr::new(c.strategy.0.as_str()))
            .collect();

        let candidate_kpi_strings: Vec<CandidateKpiStrings> = report
            .candidates
            .iter()
            .map(|c| render_kpi_strings(c.strategy.0.as_str(), &c.kpis))
            .collect();

        let winner_robustness_label = r.winner_robustness.map(|flag| match flag {
            backtest::RobustnessFlag::Robust => SmolStr::new("robust"),
            backtest::RobustnessFlag::Marginal => SmolStr::new("marginal"),
            backtest::RobustnessFlag::Fragile => SmolStr::new("fragile"),
            backtest::RobustnessFlag::Skipped => SmolStr::new("not checked"),
        });

        let reason_codes: Vec<SmolStr> = r
            .reasons
            .iter()
            .map(|code| match code {
                backtest::ReasonCode::HighestRobustSharpe => {
                    SmolStr::new("highest Sharpe among robust candidates")
                }
                backtest::ReasonCode::BeatBenchmarkSharpe => {
                    SmolStr::new("Sharpe beat the benchmark")
                }
                backtest::ReasonCode::BenchmarkUndefeated => {
                    SmolStr::new("no active strategy beat buy-and-hold")
                }
                backtest::ReasonCode::AllCandidatesFragile => {
                    SmolStr::new("all candidates flagged fragile under resampling")
                }
                backtest::ReasonCode::TieBrokenByReturn => {
                    SmolStr::new("Sharpe tie broken by total return")
                }
                backtest::ReasonCode::TieBrokenByDrawdown => {
                    SmolStr::new("Sharpe and return tie broken by lower drawdown")
                }
            })
            .collect();

        Self {
            outcome,
            winner_id,
            candidate_ids,
            candidate_kpi_strings,
            winner_robustness_label,
            reason_codes,
        }
    }

    /// Collect the full set of allowed numeric token strings for P3
    /// (P2-1 hardening — ADR-0064 amendment 2026-07-01).
    ///
    /// Returns every canonical KPI string across all candidates, plus
    /// the `trade_count` for each, as a deduplicated `HashSet` — the exact
    /// set of numbers the LLM was told, in the exact display format the
    /// `render_kpi_strings` formatters use (which mirror
    /// `crates/ui/src/widgets/num.rs`). This is the allowed-token set: a
    /// verbatim-number-match failure means a narration numeric token is
    /// NOT a byte-exact member of this set, i.e. it is either a rounding /
    /// rephrasing of a real number OR wholly invented — both are rejected
    /// identically (P3 does not distinguish "close" from "wrong").
    ///
    /// `HashSet` (not `Vec`) is deliberate: P3 does membership checks only,
    /// never iteration order, so the O(1) lookup is both faster and the
    /// honest representation of "the allowed set", matching the task intent
    /// ("every number the LLM was told").
    #[must_use]
    pub fn allowed_numbers(&self) -> std::collections::HashSet<String> {
        let mut set = std::collections::HashSet::new();
        for kpi in &self.candidate_kpi_strings {
            set.insert(kpi.sharpe.clone());
            set.insert(kpi.sortino.clone());
            set.insert(kpi.calmar.clone());
            set.insert(kpi.total_return_pct.clone());
            set.insert(kpi.max_drawdown.clone());
            set.insert(kpi.trade_count.clone());
        }
        set
    }
}

/// Render a `backtest::CandidateKpis` into canonical string representations.
///
/// Uses the SAME digits as `crates/ui/src/widgets/num.rs` — the exact decimal
/// places / `%` suffix must match so P3 exact-string-match works; signs: ADR-0064 D12.
/// This keeps the `ui` dep out of `agent` while reproducing the formatters here.
///
/// Format contracts (FROZEN — ADR-0064 § D2.P3 + D12; the digits change only with `num.rs`):
/// - Sharpe: 4 decimal places (mirrors `format_sharpe`)
/// - Sortino: 4 decimal places (same rule as Sharpe — same magnitude range)
/// - Calmar: 4 decimal places (same)
/// - total_return_pct: [`kpi_return_pct`], the FRACTION × 100 at 2 dp: the digits
///   `num::fmt_pct_signed` puts in the leaderboard's return cell. (Bug-log `#198`:
///   this was the raw fraction plus `%`, so `0.1837` read `"0.18%"` beside `+18.37%`.)
/// - max_drawdown: [`kpi_drawdown_pct`], the digits of `num::format_pct_max_dd` fed
///   `max_drawdown × 100` (as the leaderboard feeds it), unsigned
/// - Glyphs follow P3's frozen tokenizer, not the screen: no `+` on a gain (P3 strips
///   leading `+`), ASCII `-` on a loss (U+2212 never joins a token), no sign on a
///   drawdown. The two fns hold the full matrix (ADR-0064 D12).
/// - trade_count: plain decimal integer (mirrors `format_count`)
fn render_kpi_strings(strategy_id: &str, kpis: &backtest::CandidateKpis) -> CandidateKpiStrings {
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;

    fn fmt_ratio4(v: f64) -> String {
        // 4 dp, no thousands separator (Sharpe/Sortino/Calmar never need it in practice).
        let d = Decimal::from_f64(v).unwrap_or(Decimal::ZERO);
        let rounded = d.round_dp(4);
        let raw = rounded.to_string();
        pad_fractional_kpi(&raw, 4)
    }

    fn fmt_count(n: usize) -> String {
        // Plain integer — mirrors `format_count` (no thousands sep needed for P3 match).
        n.to_string()
    }

    CandidateKpiStrings {
        strategy_id: SmolStr::new(strategy_id),
        sharpe: fmt_ratio4(kpis.sharpe),
        sortino: fmt_ratio4(kpis.sortino),
        calmar: fmt_ratio4(kpis.calmar),
        total_return_pct: kpi_return_pct(kpis.total_return_pct),
        max_drawdown: kpi_drawdown_pct(kpis.max_drawdown),
        trade_count: fmt_count(kpis.trade_count),
    }
}

/// Pad a decimal string to exactly `places` fractional digits.
fn pad_fractional_kpi(raw: &str, places: usize) -> String {
    match raw.split_once('.') {
        Some((int, frac)) => {
            if frac.len() >= places {
                format!("{int}.{}", &frac[..places])
            } else {
                format!("{int}.{frac}{}", "0".repeat(places - frac.len()))
            }
        }
        None => format!("{raw}.{}", "0".repeat(places)),
    }
}

// ── Faithfulness post-check (layer 2 — THE LOAD-BEARING GUARD) ───────────────

/// The verdict returned by [`check_faithful`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaithfulnessVerdict {
    /// The narration passed all predicates — it may be shown.
    Pass,
    /// A predicate fired — discard the narration and fall back to templated copy.
    Reject(RejectReason),
}

/// The reason a narration was rejected (for `tracing::warn` audit; never reaches `ui`).
///
/// P2-1 hardening (ADR-0064 amendment 2026-07-01): `FabricatedNumber` and
/// `BannedPhrase` now carry the offending token/phrase (were unit variants).
/// This is an additive extension of the rejection cases per the amendment —
/// `WrongCrown` / `ContradictedOutcome` are unchanged, and the `Pass` arm of
/// `FaithfulnessVerdict` is semantically identical to before the hardening.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectReason {
    /// P1 — a non-winner strategy was crowned, or the winner was not named
    /// when an active strategy won.
    WrongCrown,
    /// P2 — the narration's outcome contradicts `facts.outcome`.
    ContradictedOutcome,
    /// P3 — a numeric token in the prose does not match any canonical KPI
    /// string in `NarrationFacts::allowed_numbers()`. Carries the offending
    /// token (P2-1: verbatim-number-match hardening) — the token is either a
    /// rounded/rephrased real number or a wholly invented one; P3 does not
    /// distinguish the two, both are rejected.
    FabricatedNumber(String),
    /// P4 — a predict/advise/causation banned phrase was found. Carries the
    /// offending phrase (P2-1: the list was extended with prediction verbs,
    /// causation clauses, and recommendation phrases).
    BannedPhrase(String),
}

/// FROZEN BANNED-PHRASE LIST (ADR-0064 § D2.P4, extended by the P2-1
/// amendment 2026-07-01 — see the ADR-0064 "Amendment 2026-07-01" section).
///
/// Change requires an ADR-0064 amendment — NOT an ad-hoc edit.
const BANNED_PHRASES: &[&str] = &[
    // ── original ADR-0064 § D2.P4 list ──────────────────────────────────
    "will rise",
    "will fall",
    "will go up",
    "will go down",
    "will increase",
    "will decrease",
    "will return",
    "will keep",
    "will continue",
    "will outperform",
    "will beat",
    "is going to",
    "expected return",
    "expected to return",
    "projected return",
    "future return",
    "guaranteed",
    "guarantee",
    "risk-free",
    "sure thing",
    "you should buy",
    "you should sell",
    "you should invest",
    "should buy",
    "should sell",
    "recommend buying",
    "recommend selling",
    "we recommend you",
    "buy now",
    "sell now",
    "invest now",
    "financial advice",
    "i advise",
    "my advice",
    "price target",
    "going to rise",
    "going to climb",
    "set to rise",
    "poised to",
    "likely to rise",
    "likely to climb",
    "next week will",
    "going forward it will",
    // ── P2-1 amendment 2026-07-01 — prediction verbs ────────────────────
    // ("will rise" / "will fall" already frozen above — not duplicated.)
    "expected to",
    "forecast",
    "predict",
    "probably",
    "likely to",
    "anticipates",
    "projected",
    // ── P2-1 amendment 2026-07-01 — causation clauses ───────────────────
    "because of",
    "driven by",
    "caused by",
    "due to",
    // ── P2-1 amendment 2026-07-01 — advice/recommendation phrases ───────
    // ("buy now" / "sell now" already frozen above — not duplicated.)
    "you should",
    "we recommend",
    "invest in",
    "stay away from",
];

/// FROZEN CROWN LEXEME SET (ADR-0064 § D2.P1).
const CROWN_LEXEMES: &[&str] = &[
    "won",
    "winner",
    "wins",
    "crowned",
    "best",
    "top",
    "recommended",
    "the pick",
    "picked",
    "came out on top",
];

/// FROZEN CONTRADICTION LEXEMES for P2 `BenchmarkWins` (active-beat-bah claims).
const ACTIVE_BEAT_BAH_LEXEMES: &[&str] = &[
    "beat",
    "outperformed",
    "better than holding",
    "better than buy",
];

/// FROZEN CONTRADICTION LEXEMES for P2 `ActiveWins` (nothing-beat-bah claims).
const NOTHING_BEAT_BAH_LEXEMES: &[&str] = &[
    "nothing beat",
    "buy and hold won",
    "holding won",
    "just holding was best",
];

/// FROZEN ROBUSTNESS ASSERTION LEXEMES for P2 `AllFragile` (robust-without-caveat).
const ROBUST_ASSERTION_LEXEMES: &[&str] = &[
    "robust",
    "held up",
    "reliable",
    "survived resampling",
    "passed the robustness",
];

/// The frozen fragility caveat terms — if the text says "all strategies were fragile"
/// or similar, P2 AllFragile should not fire.
const FRAGILITY_CAVEAT_LEXEMES: &[&str] = &[
    "fragile",
    "all fragile",
    "all strategies were fragile",
    "all candidates were fragile",
];

/// Numeric-token extraction: standalone signed integers, decimals and percentages.
///
/// A token is "standalone" if it is NOT immediately preceded or followed by an ASCII
/// alphanumeric character, an underscore, or a dot, so "v0.5.macd" and "v0.buyhold"
/// are identifier components, not KPI numbers. Excludes ordinals (1st, 2nd, 3rd, 4th, …).
fn extract_numeric_tokens(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let n = chars.len();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < n {
        let c = chars[i];
        // Check for a sign character that starts a numeric token.
        let sign_start = (c == '-' || c == '+') && i + 1 < n && chars[i + 1].is_ascii_digit();

        if c.is_ascii_digit() || sign_start {
            // Reject if the character immediately BEFORE the token start is an
            // alphanumeric character, underscore, or `.` — it's an identifier.
            let pre_char = if i > 0 { chars[i - 1] } else { ' ' };
            let pre_is_ident = pre_char.is_alphanumeric() || pre_char == '_' || pre_char == '.';

            // An embedded digit inside an identifier: skip to the next non-digit. A sign
            // there is never part of the identifier: step over it alone and re-scan (`#199`).
            if pre_is_ident && sign_start {
                i += 1;
                continue;
            }
            if pre_is_ident {
                while i < n && (chars[i].is_alphanumeric() || chars[i] == '.' || chars[i] == '_') {
                    i += 1;
                }
                continue;
            }

            let start = i;
            if sign_start {
                i += 1; // skip the sign
            }
            // Consume digits.
            while i < n && chars[i].is_ascii_digit() {
                i += 1;
            }
            // Consume optional `.digits` (decimal part).
            if i < n && chars[i] == '.' && i + 1 < n && chars[i + 1].is_ascii_digit() {
                i += 1; // consume `.`
                while i < n && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            // Consume optional `%`.
            let has_pct = i < n && chars[i] == '%';
            if has_pct {
                i += 1;
            }

            // Reject if the character immediately AFTER the token is an
            // alphanumeric character or underscore — it's an identifier (e.g.
            // "v0.5macd"). A trailing `.` is treated as an identifier connector
            // ONLY when it is followed by another alphanumeric character (e.g.
            // "v0.5.macd") — a sentence-ending `.` (followed by space or end)
            // does NOT make the token part of an identifier.
            let post_char = if i < n { chars[i] } else { ' ' };
            let post_next = if i + 1 < n { chars[i + 1] } else { ' ' };
            let post_is_ident = post_char.is_alphanumeric()
                || post_char == '_'
                || (post_char == '.' && (post_next.is_alphanumeric() || post_next == '_'));
            if post_is_ident {
                // Skip the rest of this identifier token.
                while i < n
                    && (chars[i].is_alphanumeric()
                        || chars[i] == '_'
                        || (chars[i] == '.'
                            && i + 1 < n
                            && (chars[i + 1].is_alphanumeric() || chars[i + 1] == '_')))
                {
                    i += 1;
                }
                continue;
            }

            // Check for ordinal suffixes (st, nd, rd, th).
            let is_ordinal = if i + 1 < n {
                matches!(
                    (chars[i], chars[i + 1]),
                    ('s', 't') | ('n', 'd') | ('r', 'd') | ('t', 'h')
                )
            } else {
                false
            };
            if is_ordinal {
                i += 2; // skip ordinal suffix
                continue;
            }

            let token: String = chars[start..i].iter().collect();
            // Require at least one actual digit in the token.
            if token.chars().any(|ch| ch.is_ascii_digit()) {
                tokens.push(token);
            }
        } else {
            i += 1;
        }
    }
    tokens
}

/// `check_faithful(text, facts) -> FaithfulnessVerdict`
///
/// The FROZEN deterministic faithfulness post-check (ADR-0064 § D2).
///
/// REJECTS (→ `FellBack`) iff ANY predicate fires:
///
/// - **P1 wrong crown** — a non-winner id co-occurs with a crown lexeme near it,
///   OR the winner is not named at all when an active strategy was crowned.
/// - **P2 contradicted outcome** — the prose contradicts `facts.outcome`.
/// - **P3 fabricated number** — a numeric token absent from the allowed KPI set.
/// - **P4 banned phrase** — a predict/advise phrase from the frozen list.
///
/// Pure: no I/O, no `llm` dep, deterministic.
#[must_use]
pub fn check_faithful(text: &str, facts: &NarrationFacts) -> FaithfulnessVerdict {
    let lower = text.to_lowercase();

    // ── P4 — Banned phrase (predict / advise / causation) ────────────────────
    for phrase in BANNED_PHRASES {
        if lower.contains(phrase) {
            tracing::warn!(
                phrase,
                winner = %facts.winner_id,
                "narration rejected: P4 banned phrase found"
            );
            return FaithfulnessVerdict::Reject(RejectReason::BannedPhrase((*phrase).to_string()));
        }
    }

    // ── P1 — Wrong crown ─────────────────────────────────────────────────────
    //
    // For each candidate that is NOT the winner, check if a crown lexeme
    // appears "near" the non-winner id (within a 120-char window).
    let winner_lower = facts.winner_id.to_lowercase();
    for lexeme in CROWN_LEXEMES {
        // Find all positions of this crown lexeme.
        let mut search_start = 0;
        while let Some(lex_pos) = lower[search_start..].find(lexeme) {
            let abs_lex = search_start + lex_pos;
            // Check all non-winner ids.
            for cid in &facts.candidate_ids {
                let cid_lower = cid.to_lowercase();
                if cid_lower == winner_lower {
                    continue;
                }
                // Is this candidate id present in a ±120 char window?
                let window_start = abs_lex.saturating_sub(120);
                let window_end = (abs_lex + lexeme.len() + 120).min(lower.len());
                let window = &lower[window_start..window_end];
                if window.contains(cid_lower.as_str()) {
                    tracing::warn!(
                        non_winner = %cid,
                        crown_lexeme = lexeme,
                        winner = %facts.winner_id,
                        "narration rejected: P1 non-winner id co-occurs with crown lexeme"
                    );
                    return FaithfulnessVerdict::Reject(RejectReason::WrongCrown);
                }
            }
            search_start = abs_lex + lexeme.len();
        }
    }

    // P1 — also check that the winner is named at all when an active strategy won.
    if facts.outcome == NarrationOutcome_::ActiveWins && !lower.contains(winner_lower.as_str()) {
        tracing::warn!(
            winner = %facts.winner_id,
            "narration rejected: P1 winner not named when ActiveWins"
        );
        return FaithfulnessVerdict::Reject(RejectReason::WrongCrown);
    }

    // ── P2 — Contradicted outcome ─────────────────────────────────────────────
    match facts.outcome {
        NarrationOutcome_::BenchmarkWins => {
            // If the text claims an ACTIVE strategy beat buy-and-hold, reject.
            for lexeme in ACTIVE_BEAT_BAH_LEXEMES {
                // Find the lexeme, then check if a non-benchmark active id is near it.
                let mut search_start = 0;
                while let Some(lex_pos) = lower[search_start..].find(lexeme) {
                    let abs_lex = search_start + lex_pos;
                    // Check all non-benchmark candidate ids.
                    for cid in &facts.candidate_ids {
                        // Heuristic: if any active strategy id is within ±150 chars of the
                        // contradiction lexeme, it's a contradiction.
                        let cid_lower = cid.to_lowercase();
                        // Skip "v0.buyhold" and "buy and hold" — they are the benchmark.
                        if cid_lower.contains("buyhold") || cid_lower.contains("buy and hold") {
                            continue;
                        }
                        let window_start = abs_lex.saturating_sub(150);
                        let window_end = (abs_lex + lexeme.len() + 150).min(lower.len());
                        let window = &lower[window_start..window_end];
                        if window.contains(cid_lower.as_str()) {
                            tracing::warn!(
                                outcome = "BenchmarkWins",
                                lexeme,
                                active_strategy = %cid,
                                "narration rejected: P2 active strategy claims to beat benchmark"
                            );
                            return FaithfulnessVerdict::Reject(RejectReason::ContradictedOutcome);
                        }
                    }
                    search_start = abs_lex + lexeme.len();
                }
            }
        }
        NarrationOutcome_::ActiveWins => {
            // If the text claims nothing beat buy-and-hold, reject.
            for lexeme in NOTHING_BEAT_BAH_LEXEMES {
                if lower.contains(lexeme) {
                    tracing::warn!(
                        outcome = "ActiveWins",
                        lexeme,
                        "narration rejected: P2 claims nothing beat benchmark when ActiveWins"
                    );
                    return FaithfulnessVerdict::Reject(RejectReason::ContradictedOutcome);
                }
            }
        }
        NarrationOutcome_::AllFragile => {
            // If the text asserts robust/reliable WITHOUT a fragility caveat, reject.
            let has_fragility_caveat = FRAGILITY_CAVEAT_LEXEMES
                .iter()
                .any(|lex| lower.contains(lex));

            if !has_fragility_caveat {
                for lexeme in ROBUST_ASSERTION_LEXEMES {
                    if lower.contains(lexeme) {
                        tracing::warn!(
                            outcome = "AllFragile",
                            lexeme,
                            "narration rejected: P2 claims robust without fragility caveat"
                        );
                        return FaithfulnessVerdict::Reject(RejectReason::ContradictedOutcome);
                    }
                }
            }
        }
    }

    // ── P3 — Fabricated / invented number (verbatim-number match, P2-1) ──────
    //
    // Every numeric token in the narration MUST be a byte-exact member of
    // `allowed_numbers()` — the exact set of numbers the LLM was told, in the
    // exact display format `render_kpi_strings` produced. No rounding, no
    // rephrasing: "0.5678" passes only if "0.5678" (not "0.57" or "0.568")
    // is a KPI string somewhere in `facts`.
    let allowed_set = facts.allowed_numbers();

    let numeric_tokens = extract_numeric_tokens(text);
    for token in &numeric_tokens {
        // Normalise: strip leading `+` if present (the LLM might prefix positives).
        let norm = token.trim_start_matches('+').to_string();
        // Allow token if it matches any allowed KPI string (exact-string).
        if !allowed_set.contains(token) && !allowed_set.contains(&norm) {
            tracing::warn!(
                token,
                "narration rejected: P3 numeric token not in allowed KPI set"
            );
            return FaithfulnessVerdict::Reject(RejectReason::FabricatedNumber(token.clone()));
        }
    }

    FaithfulnessVerdict::Pass
}

// ── Prompt builder ────────────────────────────────────────────────────────────

/// The static system prompt (role lock + faithfulness constraints + NON-goals).
///
/// This is the STATIC portion — cache-marked via `CacheBreakpoint::Ephemeral`.
const SYSTEM_PROMPT_PROJECT: &str = "\
You are a financial data summarizer for a simulated paper-trading advisor tool. \
Your ONLY job is to take a structured bake-off result and render it as a plain-language \
explanation of WHY the crowned strategy was selected by the automated ranking engine. \
You are NOT a financial advisor. You do NOT predict future prices or returns. \
You do NOT recommend any real-money action. \
The system uses SIMULATED paper money (€200 equivalent); no real assets are ever traded. \
NEVER use the phrases: will rise, will fall, expected return, guaranteed, \
you should buy, price target, financial advice, or any other prediction or advice language.";

const SYSTEM_PROMPT_ROLE: &str = "\
You explain bake-off results. The bake-off compared strategies on historical data only. \
It does NOT predict the future. \
You may ONLY speak about the numbers you are given. \
Do NOT invent any number. Do NOT claim any strategy will perform well in the future. \
Do NOT name any strategy as a winner UNLESS it is the one explicitly provided as the winner. \
Your explanation should be 2-4 short paragraphs explaining: \
(1) which strategy was crowned and why, citing the exact KPIs provided; \
(2) what the benchmark (buy-and-hold) did over the same window; \
(3) why the runners-up did not win (citing KPIs or robustness flags). \
Do NOT add analysis, opinions, or forward-looking statements beyond what the data says.";

/// Build the `ChatRequest` for the narration (layer 1).
///
/// The static system prompt is cache-marked via `CacheBreakpoint::Ephemeral`
/// (per ADR-0064 § D3).  The variable user turn carries only `NarrationFacts`.
fn build_narration_request(
    facts: &NarrationFacts,
    provider_kind: &llm::ProviderKind,
) -> ChatRequest {
    // Compose the system prompt with cache markers.
    let system_blocks: Vec<SystemBlock> = CachedSystemPromptBuilder::default()
        .project(SYSTEM_PROMPT_PROJECT)
        .role(SYSTEM_PROMPT_ROLE)
        .dynamic("") // dynamic section is empty — facts go in the user turn
        .build_for(provider_kind);

    // Build the variable user turn (the structured facts).
    let user_content = build_facts_user_turn(facts);

    let mut req = ChatRequest::new(
        ModelId::new("claude-opus-4-7"), // Anthropic default model per operator lock
        LlmTier::QuickThink,             // narration is a quick-think task
        AgentRole::Other("narration".to_string()),
    );
    req.system = system_blocks;
    req.messages = vec![ChatMessage {
        role: MessageRole::User,
        content: vec![ContentBlock::Text(user_content)],
    }];
    req.max_tokens = 800; // narration is short (2-4 paragraphs)
    req
}

/// Render the structured `NarrationFacts` as the user turn text.
fn build_facts_user_turn(facts: &NarrationFacts) -> String {
    let outcome_str = match facts.outcome {
        NarrationOutcome_::ActiveWins => "ACTIVE_STRATEGY_WINS",
        NarrationOutcome_::BenchmarkWins => "BENCHMARK_WINS",
        NarrationOutcome_::AllFragile => "ALL_FRAGILE",
    };

    let mut s = format!(
        "BAKE-OFF RESULT\nOutcome: {outcome_str}\nWinner: {}\n",
        facts.winner_id
    );

    if let Some(ref rob) = facts.winner_robustness_label {
        s.push_str(&format!("Winner robustness: {rob}\n"));
    }

    s.push_str("\nREASONS:\n");
    for reason in &facts.reason_codes {
        s.push_str(&format!("- {reason}\n"));
    }

    s.push_str("\nCANDIDATE KPIs:\n");
    for kpi in &facts.candidate_kpi_strings {
        s.push_str(&format!(
            "  {} | Sharpe: {} | Sortino: {} | Calmar: {} | Total Return: {} | Max Drawdown: {} | Trades: {}\n",
            kpi.strategy_id,
            kpi.sharpe,
            kpi.sortino,
            kpi.calmar,
            kpi.total_return_pct,
            kpi.max_drawdown,
            kpi.trade_count,
        ));
    }

    s.push_str(
        "\nINSTRUCTION: Explain the above result in plain language. \
         Use ONLY the numbers listed above. \
         Do NOT predict future performance. \
         Do NOT recommend any real-money action.",
    );
    s
}

// ── generate_narration ────────────────────────────────────────────────────────

/// The async orchestrator (ADR-0064 § D1 item 4).
///
/// Build the request → call the provider → run `check_faithful` → return.
/// Every failure mode returns `FellBack` — the fallback is the honest floor.
///
/// # Parameters
///
/// - `provider` — injected `Arc<dyn LlmProvider>` (the `BudgetedProvider`
///   stack at boot, or a fake in tests — no network in tests).
/// - `facts` — the structured bake-off facts to narrate.
pub async fn generate_narration(
    provider: &Arc<dyn LlmProvider>,
    facts: &NarrationFacts,
) -> NarrationOutcome {
    let provider_kind = provider.provider_kind();
    let req = build_narration_request(facts, &provider_kind);

    let response = match provider.complete(req).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::warn!(
                error = %e,
                winner = %facts.winner_id,
                "narration fell back: provider error"
            );
            return NarrationOutcome::FellBack;
        }
    };

    // Extract the text from the response.
    let text = match extract_text_from_response(&response) {
        Some(t) if !t.is_empty() => t,
        _ => {
            tracing::warn!(
                winner = %facts.winner_id,
                "narration fell back: empty or non-text response"
            );
            return NarrationOutcome::FellBack;
        }
    };

    // Run the faithfulness post-check (layer 2).
    match check_faithful(&text, facts) {
        FaithfulnessVerdict::Pass => {
            tracing::debug!(
                winner = %facts.winner_id,
                len = text.len(),
                "narration passed faithfulness check"
            );
            NarrationOutcome::Ready(SmolStr::new(text))
        }
        FaithfulnessVerdict::Reject(reason) => {
            tracing::warn!(
                ?reason,
                winner = %facts.winner_id,
                "narration fell back: post-check rejected the LLM text"
            );
            NarrationOutcome::FellBack
        }
    }
}

/// Extract the first `ContentBlock::Text` from a `ChatResponse`.
fn extract_text_from_response(response: &llm::ChatResponse) -> Option<String> {
    response.content.iter().find_map(|block| {
        if let ContentBlock::Text(t) = block {
            Some(t.clone())
        } else {
            None
        }
    })
}

// ── Fake providers (for tests and render harness — NO network) ────────────────

/// A fake `LlmProvider` that returns a FAITHFUL narration for the given facts.
///
/// Used in unit tests + the render harness to exercise the `Ready` path without
/// a real network call (ADR-0064 § D5).
pub struct FaithfulFakeProvider {
    /// The facts the narration should describe (so the fake can produce a
    /// factually-correct response that passes `check_faithful`).
    pub facts: NarrationFacts,
}

#[async_trait]
impl LlmProvider for FaithfulFakeProvider {
    fn name(&self) -> &str {
        "faithful_fake"
    }

    fn provider_kind(&self) -> llm::ProviderKind {
        llm::ProviderKind::Anthropic
    }

    async fn complete(&self, _request: ChatRequest) -> Result<llm::ChatResponse, llm::LlmError> {
        // Build a faithful narration: names the winner, states the outcome
        // correctly, uses only numbers from facts, trips no banned phrase.
        let text = build_faithful_text(&self.facts);
        Ok(make_text_response(text))
    }
}

/// A fake `LlmProvider` that returns a targeted UNFAITHFUL narration.
///
/// Parameterised by which predicate to violate so each anti-hallucination
/// test can assert the corresponding predicate fires (ADR-0064 § D5).
pub struct UnfaithfulFakeProvider {
    /// Which predicate to violate.
    pub violation: UnfaithfulViolation,
    /// The facts the honest part of the narration should respect (so only
    /// the targeted predicate fires).
    pub facts: NarrationFacts,
}

/// Which faithfulness predicate the `UnfaithfulFakeProvider` will violate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnfaithfulViolation {
    /// P1 — crowns a non-winner strategy.
    WrongCrown,
    /// P2 — contradicts the outcome (e.g. claims active beat benchmark when `BenchmarkWins`).
    ContradictedOutcome,
    /// P3 — emits a fabricated numeric token (9999.9999).
    FabricatedNumber,
    /// P4 — uses a banned phrase ("will rise").
    BannedPhrase,
}

#[async_trait]
impl LlmProvider for UnfaithfulFakeProvider {
    fn name(&self) -> &str {
        "unfaithful_fake"
    }

    fn provider_kind(&self) -> llm::ProviderKind {
        llm::ProviderKind::Anthropic
    }

    async fn complete(&self, _request: ChatRequest) -> Result<llm::ChatResponse, llm::LlmError> {
        let text = build_unfaithful_text(&self.facts, self.violation);
        Ok(make_text_response(text))
    }
}

/// A fake provider that returns `LlmError::BudgetExceeded` (tests the FellBack path).
pub struct BudgetExceededFakeProvider;

#[async_trait]
impl LlmProvider for BudgetExceededFakeProvider {
    fn name(&self) -> &str {
        "budget_exceeded_fake"
    }

    fn provider_kind(&self) -> llm::ProviderKind {
        llm::ProviderKind::Anthropic
    }

    async fn complete(&self, _request: ChatRequest) -> Result<llm::ChatResponse, llm::LlmError> {
        use rust_decimal_macros::dec;
        Err(llm::LlmError::BudgetExceeded {
            spent_usd: dec!(180.00),
            ceiling_usd: dec!(200.00),
        })
    }
}

/// Build faithful narration text for a given set of facts.
///
/// Named the winner, states the correct outcome, uses only canonical KPI strings,
/// and includes no banned phrase.
pub fn build_faithful_text(facts: &NarrationFacts) -> String {
    let outcome_sentence = match facts.outcome {
        NarrationOutcome_::ActiveWins => format!(
            "The strategy {} was crowned the winner of this bake-off.",
            facts.winner_id
        ),
        NarrationOutcome_::BenchmarkWins => {
            "Buy-and-hold was crowned the winner — no active strategy topped it.".to_string()
        }
        NarrationOutcome_::AllFragile => format!(
            "The strategy {} had the highest Sharpe, but all candidates — \
             including it — were fragile under resampling.",
            facts.winner_id
        ),
    };

    let reasons_text: Vec<String> = facts
        .reason_codes
        .iter()
        .map(|r| format!("  - {r}"))
        .collect();

    let reasons_block = if reasons_text.is_empty() {
        String::new()
    } else {
        format!("The reasons were:\n{}", reasons_text.join("\n"))
    };

    // Include at least the winner's KPIs.
    let winner_kpis = facts
        .candidate_kpi_strings
        .iter()
        .find(|k| k.strategy_id == facts.winner_id)
        .map(|k| {
            format!(
                "The winner's KPIs were: Sharpe {}, Sortino {}, Calmar {}, \
                 total return {}, max drawdown {}, {} trades.",
                k.sharpe, k.sortino, k.calmar, k.total_return_pct, k.max_drawdown, k.trade_count
            )
        })
        .unwrap_or_default();

    format!(
        "{outcome_sentence}\n\n{reasons_block}\n\n{winner_kpis}\n\n\
         This is a summary of a simulated paper-trading bake-off. \
         Past performance does not indicate future results. \
         No real money was used."
    )
}

/// Build unfaithful narration text that violates a targeted predicate.
fn build_unfaithful_text(facts: &NarrationFacts, violation: UnfaithfulViolation) -> String {
    match violation {
        UnfaithfulViolation::WrongCrown => {
            // Crown a non-winner — find any candidate that isn't the winner.
            let non_winner = facts
                .candidate_ids
                .iter()
                .find(|id| **id != facts.winner_id)
                .cloned()
                .unwrap_or_else(|| SmolStr::new("v0.buyhold"));
            format!(
                "The strategy {} was crowned the winner and is the top pick for this period.",
                non_winner
            )
        }
        UnfaithfulViolation::ContradictedOutcome => {
            match facts.outcome {
                NarrationOutcome_::BenchmarkWins => {
                    // BenchmarkWins but claim an active strategy beat buy-and-hold.
                    let active = facts
                        .candidate_ids
                        .iter()
                        .find(|id| !id.contains("buyhold"))
                        .cloned()
                        .unwrap_or_else(|| SmolStr::new("v0.sma"));
                    format!(
                        "The strategy {} beat buy-and-hold and outperformed the benchmark.",
                        active
                    )
                }
                NarrationOutcome_::ActiveWins => {
                    "Nothing beat buy-and-hold. Just holding was best.".to_string()
                }
                NarrationOutcome_::AllFragile => {
                    format!(
                        "The strategy {} was robust and reliable — it held up well under resampling.",
                        facts.winner_id
                    )
                }
            }
        }
        UnfaithfulViolation::FabricatedNumber => {
            // Emit a fabricated number (9999.9999) that is not in any KPI.
            format!(
                "The strategy {} achieved a Sharpe ratio of 9999.9999, \
                 which is extraordinary.",
                facts.winner_id
            )
        }
        UnfaithfulViolation::BannedPhrase => {
            // Emit a banned phrase.
            format!(
                "The strategy {} looks promising. Based on its track record it will rise \
                 and continue to outperform in the future.",
                facts.winner_id
            )
        }
    }
}

/// Build a minimal `ChatResponse` wrapping a text string.
fn make_text_response(text: String) -> llm::ChatResponse {
    llm::ChatResponse {
        content: vec![ContentBlock::Text(text)],
        stop_reason: llm::StopReason::EndTurn,
        usage: llm::TokenUsage {
            tokens_in: 100,
            tokens_out: 200,
            tokens_cached_in: 50,
        },
        model: ModelId::new("claude-opus-4-7"),
        correlation_id: uuid::Uuid::nil(),
    }
}

// ── KPI percent strings, one rendering per fraction (bug-log `#198`) ──────────

/// A total-return FRACTION (`0.1837` = +18.37 %) as the narration's canonical
/// percent string: `"18.37%"`, `"-4.57%"`, `"0.00%"`.
///
/// The digits are the leaderboard return cell's. `ui::widgets::num::fmt_pct_signed`
/// computes `(fraction × 100).round_dp(2)` and reads the sign off the UNROUNDED
/// fraction, and so does this: `-0.00001` gives `"-0.00%"`, as on screen. Before
/// bug-log `#198` both builders printed the raw fraction (`"0.18%"` for +18.37 %),
/// so P3 accepted the wrong unit and rejected the true one.
///
/// Two glyphs differ from the screen on purpose. P3 ([`check_faithful`]) is an
/// exact-string match on the tokens `extract_numeric_tokens` yields, and that
/// tokenizer is frozen (ADR-0064 § D2; this convention is its amendment D12):
///
/// - **No `+` on a gain.** P3 tries each prose token as written and again with
///   its leading `+` stripped (`trim_start_matches('+')`); it never adds one. A
///   stored `"18.37%"` therefore accepts both `18.37%` and the screen's `+18.37%`,
///   while a stored `"+18.37%"` would reject the plain `18.37%`.
/// - **ASCII `-` on a loss, never U+2212.** A signed token starts only at ASCII
///   `-` or `+`, so `−4.57%` tokenizes as `4.57%`. A stored `"-4.57%"` accepts the
///   faithful copy and rejects `4.57%`, `+4.57%` and `−4.57%`: a narration cannot
///   drop or flip the sign of a loss. The screen's return cell uses ASCII `-` too,
///   so a loss is byte-identical to it.
///
/// The accepted cost: a loss stated as a magnitude (`lost 4.57%`) or with an en
/// dash (`–4.57%`) falls back to the templated copy. P3 reads no verbs, so storing
/// the loss unsigned to admit `lost 4.57%` would admit `gained 4.57%` as well.
#[must_use]
pub fn kpi_return_pct(fraction: rust_decimal::Decimal) -> String {
    let digits = pct_digits(fraction);
    if fraction.is_sign_negative() && !fraction.is_zero() {
        format!("-{digits}%")
    } else {
        format!("{digits}%")
    }
}

/// A max-drawdown FRACTION (`0.0612` = 6.12 %, non-negative by construction) as
/// the narration's canonical percent string: the unsigned magnitude, `"6.12%"`.
///
/// The digits are those of `ui::widgets::num::format_pct_max_dd(fraction × 100)`,
/// which takes `abs()` as this does. The screen prefixes U+2212, which the
/// tokenizer drops, so a narration that copies the prompt (`6.12%`) or the screen
/// (`−6.12%`) passes P3. A drawdown has no sign to protect; a stored `"-6.12%"`
/// would reject both faithful copies and accept only a string nobody shows. A
/// hyphenated range (`6.12%-18.72%`) tokenizes its second figure as `-18.72%` and
/// falls back. (The hand-written ADR-0064 D11 corpus stores signed drawdowns such
/// as `"-5.00%"`; no producer has ever emitted that form.)
#[must_use]
pub fn kpi_drawdown_pct(fraction: rust_decimal::Decimal) -> String {
    format!("{}%", pct_digits(fraction))
}

/// `|fraction × 100|`, rounded to 2 dp and padded to exactly two fractional
/// digits. `Decimal` only (AD-9): no `f64` anywhere in the conversion.
///
/// `round_dp` is banker's rounding (`MidpointNearestEven`), the very call
/// `num::fmt_pct_signed` and `num::format_pct_max_dd` make, so a midpoint rounds
/// the same way in the prompt and on screen: `0.18365` gives `18.36`, `0.18375`
/// gives `18.38`. The rounding is symmetric about zero, so taking `abs()` after
/// it equals the screen's `abs()` before it.
fn pct_digits(fraction: rust_decimal::Decimal) -> String {
    let pct = (fraction * rust_decimal::Decimal::ONE_HUNDRED).round_dp(2);
    pad_fractional_kpi(&pct.abs().to_string(), 2)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use llm::CacheBreakpoint;

    // ── Fixture helpers ───────────────────────────────────────────────────────

    fn make_facts_active_wins() -> NarrationFacts {
        NarrationFacts {
            outcome: NarrationOutcome_::ActiveWins,
            winner_id: SmolStr::new("v0.5.macd"),
            candidate_ids: vec![
                SmolStr::new("v0.sma"),
                SmolStr::new("v0.5.macd"),
                SmolStr::new("v0.buyhold"),
            ],
            candidate_kpi_strings: vec![
                CandidateKpiStrings {
                    strategy_id: SmolStr::new("v0.sma"),
                    sharpe: "0.1234".to_string(),
                    sortino: "0.2345".to_string(),
                    calmar: "0.3456".to_string(),
                    total_return_pct: "5.00%".to_string(),
                    max_drawdown: "-10.00%".to_string(),
                    trade_count: "42".to_string(),
                },
                CandidateKpiStrings {
                    strategy_id: SmolStr::new("v0.5.macd"),
                    sharpe: "0.5678".to_string(),
                    sortino: "0.6789".to_string(),
                    calmar: "0.7890".to_string(),
                    total_return_pct: "15.00%".to_string(),
                    max_drawdown: "-5.00%".to_string(),
                    trade_count: "88".to_string(),
                },
                CandidateKpiStrings {
                    strategy_id: SmolStr::new("v0.buyhold"),
                    sharpe: "0.2500".to_string(),
                    sortino: "0.3500".to_string(),
                    calmar: "0.1500".to_string(),
                    total_return_pct: "8.00%".to_string(),
                    max_drawdown: "-12.00%".to_string(),
                    trade_count: "1".to_string(),
                },
            ],
            winner_robustness_label: Some(SmolStr::new("robust")),
            reason_codes: vec![
                SmolStr::new("highest Sharpe among robust candidates"),
                SmolStr::new("Sharpe beat the benchmark"),
            ],
        }
    }

    fn make_facts_benchmark_wins() -> NarrationFacts {
        NarrationFacts {
            outcome: NarrationOutcome_::BenchmarkWins,
            winner_id: SmolStr::new("v0.buyhold"),
            candidate_ids: vec![
                SmolStr::new("v0.sma"),
                SmolStr::new("v0.5.macd"),
                SmolStr::new("v0.buyhold"),
            ],
            candidate_kpi_strings: vec![
                CandidateKpiStrings {
                    strategy_id: SmolStr::new("v0.sma"),
                    sharpe: "0.1000".to_string(),
                    sortino: "0.1500".to_string(),
                    calmar: "0.0800".to_string(),
                    total_return_pct: "3.00%".to_string(),
                    max_drawdown: "-15.00%".to_string(),
                    trade_count: "25".to_string(),
                },
                CandidateKpiStrings {
                    strategy_id: SmolStr::new("v0.5.macd"),
                    sharpe: "0.0500".to_string(),
                    sortino: "0.0700".to_string(),
                    calmar: "0.0300".to_string(),
                    total_return_pct: "1.50%".to_string(),
                    max_drawdown: "-20.00%".to_string(),
                    trade_count: "60".to_string(),
                },
                CandidateKpiStrings {
                    strategy_id: SmolStr::new("v0.buyhold"),
                    sharpe: "0.4000".to_string(),
                    sortino: "0.5000".to_string(),
                    calmar: "0.3000".to_string(),
                    total_return_pct: "20.00%".to_string(),
                    max_drawdown: "-8.00%".to_string(),
                    trade_count: "1".to_string(),
                },
            ],
            winner_robustness_label: None,
            reason_codes: vec![SmolStr::new("no active strategy beat buy-and-hold")],
        }
    }

    fn make_facts_all_fragile() -> NarrationFacts {
        NarrationFacts {
            outcome: NarrationOutcome_::AllFragile,
            winner_id: SmolStr::new("v0.5.rsi"),
            candidate_ids: vec![
                SmolStr::new("v0.sma"),
                SmolStr::new("v0.5.rsi"),
                SmolStr::new("v0.buyhold"),
            ],
            candidate_kpi_strings: vec![
                CandidateKpiStrings {
                    strategy_id: SmolStr::new("v0.sma"),
                    sharpe: "0.0200".to_string(),
                    sortino: "0.0300".to_string(),
                    calmar: "0.0100".to_string(),
                    total_return_pct: "1.00%".to_string(),
                    max_drawdown: "-25.00%".to_string(),
                    trade_count: "10".to_string(),
                },
                CandidateKpiStrings {
                    strategy_id: SmolStr::new("v0.5.rsi"),
                    sharpe: "0.0800".to_string(),
                    sortino: "0.0900".to_string(),
                    calmar: "0.0600".to_string(),
                    total_return_pct: "2.00%".to_string(),
                    max_drawdown: "-22.00%".to_string(),
                    trade_count: "15".to_string(),
                },
                CandidateKpiStrings {
                    strategy_id: SmolStr::new("v0.buyhold"),
                    sharpe: "0.0600".to_string(),
                    sortino: "0.0700".to_string(),
                    calmar: "0.0400".to_string(),
                    total_return_pct: "1.50%".to_string(),
                    max_drawdown: "-18.00%".to_string(),
                    trade_count: "1".to_string(),
                },
            ],
            winner_robustness_label: Some(SmolStr::new("fragile")),
            reason_codes: vec![SmolStr::new(
                "all candidates flagged fragile under resampling",
            )],
        }
    }

    // ── D3 — Prompt structure test ────────────────────────────────────────────

    #[test]
    fn d3_request_has_ephemeral_cache_block() {
        let facts = make_facts_active_wins();
        let provider_kind = llm::ProviderKind::Anthropic;
        let req = build_narration_request(&facts, &provider_kind);

        // The system prompt must have at least one Cached block with Ephemeral breakpoint.
        let has_ephemeral = req
            .system
            .iter()
            .any(|block| matches!(block, SystemBlock::Cached(_, CacheBreakpoint::Ephemeral)));
        assert!(
            has_ephemeral,
            "system prompt must carry a CacheBreakpoint::Ephemeral block"
        );

        // The user turn must carry the winner id.
        let user_text = req.messages.first().and_then(|m| {
            m.content.iter().find_map(|b| {
                if let ContentBlock::Text(t) = b {
                    Some(t.as_str())
                } else {
                    None
                }
            })
        });
        assert!(
            user_text.map(|t| t.contains("v0.5.macd")).unwrap_or(false),
            "user turn must contain the winner id"
        );
    }

    // ── D4 P1 — Wrong crown ───────────────────────────────────────────────────

    #[test]
    fn d4_p1_wrong_crown_rejects() {
        let facts = make_facts_active_wins();
        // Crown the non-winner "v0.sma" with a crown lexeme.
        let text = "The strategy v0.sma was crowned the best in the bake-off.";
        assert_eq!(
            check_faithful(text, &facts),
            FaithfulnessVerdict::Reject(RejectReason::WrongCrown),
            "P1: non-winner crowned should be rejected"
        );
    }

    #[test]
    fn d4_p1_winner_not_named_active_wins_rejects() {
        let facts = make_facts_active_wins();
        // ActiveWins but winner "v0.5.macd" not named anywhere.
        let text = "Buy-and-hold won the bake-off with excellent performance.";
        assert_eq!(
            check_faithful(text, &facts),
            FaithfulnessVerdict::Reject(RejectReason::WrongCrown),
            "P1: winner not named when ActiveWins should be rejected"
        );
    }

    // ── D4 P2 — Contradicted outcome ─────────────────────────────────────────

    #[test]
    fn d4_p2_benchmark_wins_but_active_claims_win_rejects() {
        let facts = make_facts_benchmark_wins();
        // BenchmarkWins, but text claims v0.sma beat buy-and-hold.
        let text = "The strategy v0.sma beat buy-and-hold and outperformed the benchmark.";
        assert_eq!(
            check_faithful(text, &facts),
            FaithfulnessVerdict::Reject(RejectReason::ContradictedOutcome),
            "P2: active strategy claims to beat benchmark when BenchmarkWins"
        );
    }

    #[test]
    fn d4_p2_active_wins_but_nothing_beat_claim_rejects() {
        let facts = make_facts_active_wins();
        // ActiveWins but text claims nothing beat buy-and-hold.
        let text =
            "The strategy v0.5.macd had the highest Sharpe. Nothing beat buy-and-hold this period.";
        assert_eq!(
            check_faithful(text, &facts),
            FaithfulnessVerdict::Reject(RejectReason::ContradictedOutcome),
            "P2: claims nothing beat benchmark when ActiveWins"
        );
    }

    #[test]
    fn d4_p2_all_fragile_robust_without_caveat_rejects() {
        let facts = make_facts_all_fragile();
        // AllFragile but text asserts robust without mentioning fragility.
        let text = "The strategy v0.5.rsi was robust and reliable based on the data.";
        assert_eq!(
            check_faithful(text, &facts),
            FaithfulnessVerdict::Reject(RejectReason::ContradictedOutcome),
            "P2: AllFragile but robust assertion without caveat"
        );
    }

    // ── D4 P3 — Fabricated number ─────────────────────────────────────────────

    #[test]
    fn d4_p3_fabricated_number_rejects() {
        let facts = make_facts_active_wins();
        // Text contains 9999.9999 which is not in any KPI.
        let text = format!(
            "The strategy {} achieved a Sharpe ratio of 9999.9999.",
            facts.winner_id
        );
        assert_eq!(
            check_faithful(&text, &facts),
            FaithfulnessVerdict::Reject(RejectReason::FabricatedNumber("9999.9999".to_string())),
            "P3: fabricated number should be rejected"
        );
    }

    // ── D4 P4 — Banned phrase ─────────────────────────────────────────────────

    #[test]
    fn d4_p4_banned_phrase_will_rise_rejects() {
        let facts = make_facts_active_wins();
        let text = format!(
            "The strategy {} was crowned the best. It will rise going forward.",
            facts.winner_id
        );
        assert_eq!(
            check_faithful(&text, &facts),
            FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("will rise".to_string())),
            "P4: 'will rise' should be rejected"
        );
    }

    #[test]
    fn d4_p4_banned_phrase_expected_return_rejects() {
        let facts = make_facts_active_wins();
        let text = format!(
            "The strategy {} won. The expected return is excellent.",
            facts.winner_id
        );
        assert_eq!(
            check_faithful(&text, &facts),
            FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("expected return".to_string())),
            "P4: 'expected return' should be rejected"
        );
    }

    // ── D4 faithful narration passes ─────────────────────────────────────────

    #[test]
    fn d4_faithful_narration_passes() {
        let facts = make_facts_active_wins();
        let text = build_faithful_text(&facts);
        assert_eq!(
            check_faithful(&text, &facts),
            FaithfulnessVerdict::Pass,
            "a faithful narration built from facts should pass"
        );
    }

    // ── D10 — additional P4 banned phrase coverage ───────────────────────────

    #[test]
    fn d4_p4_guaranteed_rejects() {
        let facts = make_facts_active_wins();
        let text = format!(
            "The strategy {} won. Its returns are guaranteed.",
            facts.winner_id
        );
        assert_eq!(
            check_faithful(&text, &facts),
            FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("guaranteed".to_string())),
        );
    }

    #[test]
    fn d4_p4_you_should_buy_rejects() {
        let facts = make_facts_active_wins();
        let text = format!(
            "The strategy {} was the best. You should buy this asset now.",
            facts.winner_id
        );
        assert_eq!(
            check_faithful(&text, &facts),
            FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("you should buy".to_string())),
        );
    }

    // ── D5 generate_narration — async tests ──────────────────────────────────

    #[tokio::test]
    async fn d5_faithful_fake_produces_ready() {
        let facts = make_facts_active_wins();
        let provider: Arc<dyn LlmProvider> = Arc::new(FaithfulFakeProvider {
            facts: facts.clone(),
        });
        let outcome = generate_narration(&provider, &facts).await;
        assert!(
            matches!(outcome, NarrationOutcome::Ready(_)),
            "FaithfulFakeProvider should produce Ready; got {outcome:?}"
        );
    }

    #[tokio::test]
    async fn d11_anti_hallucination_wrong_crown_produces_fellback() {
        let facts = make_facts_active_wins();
        let provider: Arc<dyn LlmProvider> = Arc::new(UnfaithfulFakeProvider {
            violation: UnfaithfulViolation::WrongCrown,
            facts: facts.clone(),
        });
        let outcome = generate_narration(&provider, &facts).await;
        assert_eq!(
            outcome,
            NarrationOutcome::FellBack,
            "UnfaithfulFakeProvider(WrongCrown) must produce FellBack"
        );
    }

    #[tokio::test]
    async fn d11_anti_hallucination_contradicted_outcome_produces_fellback() {
        let facts = make_facts_benchmark_wins();
        let provider: Arc<dyn LlmProvider> = Arc::new(UnfaithfulFakeProvider {
            violation: UnfaithfulViolation::ContradictedOutcome,
            facts: facts.clone(),
        });
        let outcome = generate_narration(&provider, &facts).await;
        assert_eq!(
            outcome,
            NarrationOutcome::FellBack,
            "UnfaithfulFakeProvider(ContradictedOutcome) must produce FellBack"
        );
    }

    #[tokio::test]
    async fn d11_anti_hallucination_fabricated_number_produces_fellback() {
        let facts = make_facts_active_wins();
        let provider: Arc<dyn LlmProvider> = Arc::new(UnfaithfulFakeProvider {
            violation: UnfaithfulViolation::FabricatedNumber,
            facts: facts.clone(),
        });
        let outcome = generate_narration(&provider, &facts).await;
        assert_eq!(
            outcome,
            NarrationOutcome::FellBack,
            "UnfaithfulFakeProvider(FabricatedNumber) must produce FellBack"
        );
    }

    #[tokio::test]
    async fn d11_anti_hallucination_banned_phrase_produces_fellback() {
        let facts = make_facts_active_wins();
        let provider: Arc<dyn LlmProvider> = Arc::new(UnfaithfulFakeProvider {
            violation: UnfaithfulViolation::BannedPhrase,
            facts: facts.clone(),
        });
        let outcome = generate_narration(&provider, &facts).await;
        assert_eq!(
            outcome,
            NarrationOutcome::FellBack,
            "UnfaithfulFakeProvider(BannedPhrase) must produce FellBack"
        );
    }

    #[tokio::test]
    async fn d11_budget_exceeded_produces_fellback() {
        let facts = make_facts_active_wins();
        let provider: Arc<dyn LlmProvider> = Arc::new(BudgetExceededFakeProvider);
        let outcome = generate_narration(&provider, &facts).await;
        assert_eq!(
            outcome,
            NarrationOutcome::FellBack,
            "BudgetExceeded must produce FellBack"
        );
    }

    // ── extract_numeric_tokens tests ─────────────────────────────────────────

    #[test]
    fn numeric_token_extraction_basic() {
        let tokens = extract_numeric_tokens("The Sharpe was 0.5678 and total return 15.00%.");
        assert!(tokens.contains(&"0.5678".to_string()));
        assert!(tokens.contains(&"15.00%".to_string()));
    }

    #[test]
    fn numeric_token_extraction_ordinals_excluded() {
        let tokens = extract_numeric_tokens("The 1st strategy came in 2nd place.");
        // ordinals "1st" and "2nd" should NOT appear.
        assert!(
            !tokens.iter().any(|t| t == "1" || t == "2"),
            "ordinals should be excluded; got: {tokens:?}"
        );
    }

    #[test]
    fn numeric_token_extraction_negative() {
        let tokens = extract_numeric_tokens("Max drawdown was -5.00%.");
        assert!(tokens.contains(&"-5.00%".to_string()));
    }

    /// Bug-log `#199`: run `f` on its own thread under a deadline, so a tokenizer that
    /// never returns fails the test instead of hanging the suite. (A spinning thread
    /// dies with the test process.) A panic inside `f` is re-raised as itself, not
    /// reported as a hang.
    fn within_deadline<T: Send + 'static>(what: &str, f: impl FnOnce() -> T + Send + 'static) -> T {
        use std::sync::mpsc::RecvTimeoutError;
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let _ = tx.send(f());
        });
        match rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(value) => value,
            Err(RecvTimeoutError::Timeout) => {
                panic!("{what} did not return within 5 s (bug-log #199)")
            }
            // The only sender dropped without sending, so `f` panicked: re-raise it.
            Err(RecvTimeoutError::Disconnected) => match handle.join() {
                Err(payload) => std::panic::resume_unwind(payload),
                Ok(()) => unreachable!("{what}: the sender dropped without a send or a panic"),
            },
        }
    }

    /// The deadline helper reports a panic inside `f` as that panic. Before this, a
    /// crash read "did not return within 5 s", the `#199` liveness message.
    #[test]
    #[should_panic(expected = "the original panic")]
    fn within_deadline_re_raises_a_panic_as_itself() {
        within_deadline::<()>("a panicking closure", || panic!("the original panic"));
    }

    /// `#199`: a sign after an identifier character no longer stalls the tokenizer,
    /// and the digits after it are still a token. Where HEAD already returned (a sign
    /// after a space or a `%`), the tokens are unchanged.
    #[test]
    fn numeric_token_extraction_returns_on_a_sign_after_an_identifier_character() {
        let tok = |text: &'static str| within_deadline(text, move || extract_numeric_tokens(text));
        assert_eq!(tok("the top-3 strategies"), vec!["3"]);
        assert_eq!(tok("the COVID-19 era"), vec!["19"]);
        assert_eq!(tok("over 2023-2024"), vec!["2023", "2024"]);
        assert_eq!(
            tok("Sharpe ranged 0.1234-0.5678."),
            vec!["0.1234", "0.5678"]
        );
        assert_eq!(tok("v0.sma+5%"), vec!["5%"]);
        assert_eq!(tok("ranged 6.12%-18.72%."), vec!["6.12%", "-18.72%"]);
        assert_eq!(tok("Max drawdown was -5.00%."), vec!["-5.00%"]);
    }

    /// `#199`, the bypass the obvious fix would open (step over the sign, then skip the
    /// "identifier"): the figure after the sign must still face P3, so a fabricated
    /// upper bound of a range is rejected.
    #[test]
    fn p3_rejects_a_fabricated_figure_after_a_sign_that_follows_a_digit() {
        let facts = make_facts_active_wins();
        let text = "The strategy v0.5.macd was crowned the winner of this bake-off. \
                    Its Sharpe ratio was in the range 0.1234-9.9999 across the field.";
        let verdict = within_deadline("check_faithful", move || check_faithful(text, &facts));
        assert_eq!(
            verdict,
            FaithfulnessVerdict::Reject(RejectReason::FabricatedNumber("9.9999".to_string()))
        );
    }

    // ── F9 Sortino/Calmar round-trip (the previously-barred metrics) ──────────

    /// Prove that `allowed_numbers()` includes non-empty Sortino and Calmar
    /// strings when the `CandidateKpiStrings` are properly populated (the fix
    /// for the gap where the mirror builder emitted empty strings → those
    /// metrics were excluded from the P3 allowed-set → any LLM Sortino/Calmar
    /// citation read as fabricated → `FellBack`).
    #[test]
    fn allowed_numbers_includes_sortino_and_calmar() {
        let facts = make_facts_active_wins();
        let allowed = facts.allowed_numbers();

        // The fixture has sortino "0.6789" and calmar "0.7890" for v0.5.macd (the winner).
        // `HashSet<String>: Borrow<str>` lets `.contains()` take a `&str` directly —
        // no owned-`String` allocation needed (P2-1: allowed_numbers() now returns
        // `HashSet<String>`, was `Vec<String>`).
        assert!(
            allowed.contains("0.6789"),
            "sortino for v0.5.macd must be in the allowed set (got: {allowed:?})"
        );
        assert!(
            allowed.contains("0.7890"),
            "calmar for v0.5.macd must be in the allowed set (got: {allowed:?})"
        );
        // No empty strings — the gap that caused FellBack.
        assert!(
            !allowed.contains(""),
            "the allowed set must NOT contain empty strings (each was a sortino/calmar gap)"
        );
    }

    /// A faithful narration that cites the winner's SORTINO (a real value from
    /// the allowed set) PASSES `check_faithful`.  Before the fix this path was
    /// structurally barred: sortino was always an empty string → not in the
    /// allowed set → P3 read it as FABRICATED → `FellBack`.
    #[test]
    fn faithful_sortino_citation_passes_check() {
        let facts = make_facts_active_wins();
        // The fixture winner is v0.5.macd; its sortino is "0.6789".
        let sortino_str = &facts
            .candidate_kpi_strings
            .iter()
            .find(|k| k.strategy_id == "v0.5.macd")
            .expect("winner kpi must be present")
            .sortino;
        assert!(
            !sortino_str.is_empty(),
            "sortino must be non-empty (the structural gap being fixed)"
        );

        // Build a faithful narration that cites the winner + the sortino value.
        let text = format!(
            "The strategy v0.5.macd was crowned the winner of this bake-off. \
             It had a Sortino ratio of {sortino_str}, which was the best risk-adjusted \
             performance among the field. This is a summary of a simulated paper-trading \
             bake-off. No real money was used."
        );

        assert_eq!(
            check_faithful(&text, &facts),
            FaithfulnessVerdict::Pass,
            "a faithful sortino citation must PASS check_faithful — \
             before the fix this was structurally barred (always FellBack)"
        );
    }

    /// An UNFAITHFUL sortino (a fabricated number not in the allowed set) STILL
    /// rejects — the fix must not weaken P3, only un-block the valid values.
    #[test]
    fn unfaithful_sortino_still_rejects() {
        let facts = make_facts_active_wins();
        // Use a number guaranteed absent from all KPIs.
        let text = format!(
            "The strategy {} was crowned the winner. \
             It had a Sortino ratio of 9999.9999, which was extraordinary.",
            facts.winner_id
        );
        assert_eq!(
            check_faithful(&text, &facts),
            FaithfulnessVerdict::Reject(RejectReason::FabricatedNumber("9999.9999".to_string())),
            "an unfaithful sortino (fabricated) must STILL be rejected by P3"
        );
    }

    /// A faithful narration that cites the winner's CALMAR (a real value from
    /// the allowed set) PASSES `check_faithful` — symmetric with the sortino test.
    #[test]
    fn faithful_calmar_citation_passes_check() {
        let facts = make_facts_active_wins();
        // The fixture winner is v0.5.macd; its calmar is "0.7890".
        let calmar_str = &facts
            .candidate_kpi_strings
            .iter()
            .find(|k| k.strategy_id == "v0.5.macd")
            .expect("winner kpi must be present")
            .calmar;
        assert!(
            !calmar_str.is_empty(),
            "calmar must be non-empty (the structural gap being fixed)"
        );

        let text = format!(
            "The strategy v0.5.macd was crowned the winner of this bake-off. \
             Its Calmar ratio was {calmar_str}, reflecting strong return-to-drawdown \
             efficiency. This is a summary of a simulated paper-trading bake-off. \
             No real money was used."
        );

        assert_eq!(
            check_faithful(&text, &facts),
            FaithfulnessVerdict::Pass,
            "a faithful calmar citation must PASS check_faithful"
        );
    }

    // ── bug-log `#198`: the percent KPIs carry the leaderboard's digits ───────
    //
    // A KPI fraction (`0.1837` = +18.37 %, per `backtest::CandidateKpis`' own
    // docs) must reach the prompt and the P3 set as `(fraction × 100).round_dp(2)`,
    // the digits the leaderboard renders. Every expected string below is that
    // documented arithmetic written out; none is read off the subject. Parity
    // with the screen's real formatters (`ui::widgets::num`) is pinned in
    // `crates/ui/tests/narration_kpi_units.rs`, the one target that links both
    // crates (`agent` cannot depend on `ui`).

    /// The string both builders printed before `#198`: the FRACTION rounded to
    /// 2 dp with a `%` glued on. The negative control every pin below differs from.
    fn pre_198_unit(fraction: rust_decimal::Decimal) -> String {
        format!(
            "{}%",
            pad_fractional_kpi(&fraction.round_dp(2).to_string(), 2)
        )
    }

    /// A `CandidateKpis` with the two percents given and everything else fixed.
    fn kpis_with(
        total_return_pct: rust_decimal::Decimal,
        max_drawdown: rust_decimal::Decimal,
    ) -> backtest::CandidateKpis {
        backtest::CandidateKpis {
            sharpe: 1.42,
            sortino: 1.95,
            calmar: 2.32,
            total_return_pct,
            max_drawdown,
            trade_count: 38,
            turnover: rust_decimal::Decimal::ZERO,
        }
    }

    /// `ActiveWins` facts over `v0.sma` (the winner: +18.37 %, 6.12 % drawdown)
    /// and `v0.5.rsi` (a loser: -4.57 %, 18.72 %), built by `render_kpi_strings`
    /// itself, the producer, rather than hand-written strings.
    fn producer_facts() -> NarrationFacts {
        use rust_decimal_macros::dec;
        NarrationFacts {
            outcome: NarrationOutcome_::ActiveWins,
            winner_id: SmolStr::new("v0.sma"),
            candidate_ids: vec![SmolStr::new("v0.sma"), SmolStr::new("v0.5.rsi")],
            candidate_kpi_strings: vec![
                render_kpi_strings("v0.sma", &kpis_with(dec!(0.1837), dec!(0.0612))),
                render_kpi_strings("v0.5.rsi", &kpis_with(dec!(-0.0457), dec!(0.1872))),
            ],
            winner_robustness_label: Some(SmolStr::new("robust")),
            reason_codes: vec![SmolStr::new("highest Sharpe among robust candidates")],
        }
    }

    /// `#198`, two-sided: fractions that differ in the units dimension give
    /// different strings, each the fraction × 100, and each differs from the
    /// pre-fix string, so a reverted `× 100` cannot pass any case.
    #[test]
    fn kpi_return_pct_is_the_fraction_times_100() {
        use rust_decimal_macros::dec;
        let cases = [
            (dec!(0.1837), "18.37%"),
            (dec!(0.05), "5.00%"),
            (dec!(0.0041), "0.41%"),
            (dec!(1.5), "150.00%"),
            // Midpoints after × 100 (18.365, 18.375). `round_dp` is banker's rounding,
            // the very call `num::fmt_pct_signed` makes. Rounding a midpoint away from
            // zero prints 18.37% for the first; toward zero, 18.37% for the second.
            (dec!(0.18365), "18.36%"),
            (dec!(0.18375), "18.38%"),
        ];
        for (fraction, want) in cases {
            assert_eq!(kpi_return_pct(fraction), want, "return fraction {fraction}");
            assert_ne!(
                pre_198_unit(fraction),
                want,
                "case {fraction} must tell a fraction from a percent, or it pins nothing"
            );
        }
    }

    /// `#198`, sign: ASCII `-` on a loss, no `+` on a gain, none on zero. The sign
    /// is read off the UNROUNDED fraction, as the leaderboard's `fmt_pct_signed`
    /// reads it.
    #[test]
    fn kpi_return_pct_signs_a_loss_with_ascii_minus_and_never_writes_plus() {
        use rust_decimal_macros::dec;
        let cases = [
            (dec!(-0.0457), "-4.57%"),
            (dec!(-0.2884), "-28.84%"),
            (rust_decimal::Decimal::ZERO, "0.00%"),
            // Negative zero is zero: unsigned, as `fmt_pct_signed`'s `is_zero()` arm.
            (-rust_decimal::Decimal::ZERO, "0.00%"),
            // A loss that rounds to 0.00 keeps its sign; the screen shows "-0.00%".
            (dec!(-0.00001), "-0.00%"),
            // A gain that rounds to 0.00: the screen shows "+0.00%"; P3 strips '+'.
            (dec!(0.00001), "0.00%"),
            // A negative midpoint (-0.125): banker's rounding keeps -0.12, as on
            // screen; rounding away from zero would print "-0.13%".
            (dec!(-0.00125), "-0.12%"),
        ];
        for (fraction, want) in cases {
            assert_eq!(kpi_return_pct(fraction), want, "return fraction {fraction}");
        }
    }

    /// `#198`, drawdown: the unsigned magnitude at the leaderboard's digits
    /// (`format_pct_max_dd` takes `abs()` too). The negative input is outside the
    /// engine's contract (`max_drawdown` >= 0); it is here to pin the `abs`.
    #[test]
    fn kpi_drawdown_pct_is_the_unsigned_fraction_times_100() {
        use rust_decimal_macros::dec;
        let cases = [
            (dec!(0.0612), "6.12%"),
            (dec!(0.1872), "18.72%"),
            (dec!(0.4173), "41.73%"),
            (rust_decimal::Decimal::ZERO, "0.00%"),
            (dec!(-0.0612), "6.12%"),
            // Midpoints (0.125, 0.375): `format_pct_max_dd` rounds with `round_dp`
            // too. Away from zero prints 0.13% for the first; toward zero, 0.37%.
            (dec!(0.00125), "0.12%"),
            (dec!(0.00375), "0.38%"),
        ];
        for (fraction, want) in cases {
            assert_eq!(
                kpi_drawdown_pct(fraction),
                want,
                "drawdown fraction {fraction}"
            );
            if !fraction.is_zero() {
                assert_ne!(
                    pre_198_unit(fraction),
                    want,
                    "case {fraction} must tell the units apart"
                );
            }
        }
    }

    /// `#198`, the agent-side builder: `render_kpi_strings` routes each percent
    /// through its own fn. The rows differ in value and sign, so a swapped field,
    /// a dropped `× 100` or a dropped sign each fail at least one assertion.
    #[test]
    fn render_kpi_strings_carries_the_leaderboard_digits_per_field() {
        use rust_decimal_macros::dec;
        let win = render_kpi_strings("v0.sma", &kpis_with(dec!(0.1837), dec!(0.0612)));
        assert_eq!(win.total_return_pct, "18.37%");
        assert_eq!(win.max_drawdown, "6.12%");

        let loss = render_kpi_strings("v0.5.rsi", &kpis_with(dec!(-0.0457), dec!(0.1872)));
        assert_eq!(loss.total_return_pct, "-4.57%");
        assert_eq!(loss.max_drawdown, "18.72%");

        // Out of contract, but it tells the drawdown fn from the return fn.
        let odd = render_kpi_strings("odd", &kpis_with(dec!(0.01), dec!(-0.0612)));
        assert_eq!(odd.max_drawdown, "6.12%", "the drawdown is unsigned");

        // The fields `#198` does not touch keep their own forms.
        assert_eq!(win.sharpe, "1.4200");
        assert_eq!(win.trade_count, "38");
    }

    /// `#198`, the symptom in the bug's title: the prompt line the LLM reads.
    #[test]
    fn facts_user_turn_tells_the_llm_the_leaderboard_digits() {
        let turn = build_facts_user_turn(&producer_facts());
        assert!(
            turn.contains("Total Return: 18.37% | Max Drawdown: 6.12% | Trades: 38"),
            "the prompt must carry the screen's +18.37 % and 6.12 %; got:\n{turn}"
        );
        assert!(
            turn.contains("Total Return: -4.57% | Max Drawdown: 18.72%"),
            "the losing row keeps its sign; got:\n{turn}"
        );
        assert!(
            !turn.contains("Total Return: 0.18%"),
            "the pre-#198 unit must be gone; got:\n{turn}"
        );
    }

    /// `#198`, P3 round trip over PRODUCER-built facts. The figure the screen shows
    /// passes, with or without its glyphs; the pre-fix figure and every sign
    /// fabrication are rejected, and so are the forms D12 accepts as a cost.
    /// `check_faithful` itself is unchanged (ADR-0064 § D2 FROZEN): this pins that
    /// the strings it is given fit its tokenizer.
    #[test]
    fn p3_passes_the_leaderboard_figure_and_rejects_the_fraction_figure() {
        let facts = producer_facts();
        let verdict = |figure: &str| {
            let text = format!(
                "The strategy v0.sma was crowned the winner of this bake-off. \
                 One figure from the run was {figure} over the window."
            );
            check_faithful(&text, &facts)
        };
        let fabricated =
            |t: &str| FaithfulnessVerdict::Reject(RejectReason::FabricatedNumber(t.to_string()));

        // The winner's return: the prompt's form, and the screen's `+` form.
        assert_eq!(verdict("18.37%"), FaithfulnessVerdict::Pass);
        assert_eq!(verdict("+18.37%"), FaithfulnessVerdict::Pass);
        // The pre-#198 unit, and a flipped sign.
        assert_eq!(verdict("0.18%"), fabricated("0.18%"));
        assert_eq!(verdict("-18.37%"), fabricated("-18.37%"));
        // The loser's return: only the signed form passes.
        assert_eq!(verdict("-4.57%"), FaithfulnessVerdict::Pass);
        assert_eq!(verdict("4.57%"), fabricated("4.57%"));
        assert_eq!(verdict("+4.57%"), fabricated("+4.57%"));
        assert_eq!(verdict("\u{2212}4.57%"), fabricated("4.57%"));
        // The accepted cost (ADR-0064 D12): an en-dash loss falls back too.
        assert_eq!(verdict("\u{2013}4.57%"), fabricated("4.57%"));
        // The drawdown: the prompt's form and the screen's U+2212 form pass. An
        // ASCII-signed one falls back: the LLM is never shown it, though the
        // hand-written D11 corpus models a faithful drawdown that way (D12's
        // "Known divergence"). So does a hyphenated range of two drawdowns.
        assert_eq!(verdict("6.12%"), FaithfulnessVerdict::Pass);
        assert_eq!(verdict("\u{2212}6.12%"), FaithfulnessVerdict::Pass);
        assert_eq!(verdict("-6.12%"), fabricated("-6.12%"));
        assert_eq!(verdict("6.12%-18.72%"), fabricated("-18.72%"));
    }

    /// `#198`: the faithful fake's text (`build_faithful_text`, what
    /// `FaithfulFakeProvider` returns) over producer-built facts cites the new
    /// strings verbatim and still passes, so the `Ready` path is intact.
    #[test]
    fn faithful_text_over_producer_facts_passes() {
        let facts = producer_facts();
        let text = build_faithful_text(&facts);
        assert!(
            text.contains("total return 18.37%, max drawdown 6.12%"),
            "the faithful text cites the leaderboard's digits; got:\n{text}"
        );
        assert_eq!(check_faithful(&text, &facts), FaithfulnessVerdict::Pass);
    }
}
