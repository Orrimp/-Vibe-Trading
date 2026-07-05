//! P2-1 narration faithfulness hardening — adversarial test corpus.
//!
//! ADR-0064 amendment 2026-07-01. Exercises `agent::narration::check_faithful`
//! through the crate's PUBLIC surface (`agent::{check_faithful, NarrationFacts,
//! FaithfulnessVerdict, RejectReason, NarrationOutcome_, CandidateKpiStrings}`)
//! exactly as an external consumer would — the same discipline as every other
//! integration test under `crates/agent/tests/`.
//!
//! Five categories, per the P2-1 task spec:
//!
//! 1. **Positive** — a well-behaved narration that only cites `NarrationFacts`
//!    numbers and uses no banned phrase → `Pass`.
//! 2. **Number-invention** — a narration number NOT in `allowed_numbers()` →
//!    `Reject(FabricatedNumber(token))`.
//! 3. **Prediction** — a prediction-verb banned phrase → `Reject(BannedPhrase(_))`.
//! 4. **Causation** — a causation-clause banned phrase → `Reject(BannedPhrase(_))`.
//! 5. **Recommendation** — an advice/recommendation banned phrase →
//!    `Reject(BannedPhrase(_))`.
//!
//! Plus the mandatory backward-compat proof: an existing F9 fallback-path
//! narration that produced `Pass` before P2-1 must still produce `Pass` after.

use agent::{
    CandidateKpiStrings, FaithfulnessVerdict, NarrationFacts, RejectReason, check_faithful,
};

/// Build a `NarrationFacts` fixture identical in shape to the one used by
/// `narration.rs`'s own `make_facts_active_wins()` — an `ActiveWins` outcome
/// with three candidates (a runner-up SMA, the winner MACD, and buy-and-hold).
fn make_facts() -> NarrationFacts {
    use agent::NarrationOutcome_;
    use smol_str::SmolStr;

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
                total_return_pct: "11.24%".to_string(),
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

// ── 1. Positive: a faithful narration passes ────────────────────────────────

#[test]
fn positive_faithful_narration_citing_only_facts_passes() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner of this bake-off. \
                It had a Sharpe ratio of 0.5678, a Sortino of 0.6789, and a Calmar \
                of 0.7890, with a total return of 11.24% and a max drawdown of \
                -5.00% across 88 trades. This is a summary of a simulated \
                paper-trading bake-off. Past performance does not indicate future \
                results. No real money was used.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Pass,
        "a narration that cites only NarrationFacts numbers and no banned \
         phrase must PASS"
    );
}

// ── 2. Number-invention: verbatim-number match ──────────────────────────────

#[test]
fn number_invention_rounded_return_is_rejected() {
    let facts = make_facts();
    // The fixture's real total_return_pct for the winner is "11.24%" — the
    // narration invents "12.3%" instead (a plausible LLM rounding/rephrasing,
    // NOT a wholly-fabricated value — P3 must still reject it: no tolerance).
    let text = "The strategy v0.5.macd was crowned the winner. \
                Its total return was 12.3%, an excellent result.";

    let verdict = check_faithful(text, &facts);
    match verdict {
        FaithfulnessVerdict::Reject(RejectReason::FabricatedNumber(ref token)) => {
            assert_eq!(
                token, "12.3%",
                "the rejected token must be the invented one"
            );
        }
        other => panic!(
            "expected Reject(FabricatedNumber(\"12.3%\")) for a rounded/rephrased \
             number not verbatim in NarrationFacts; got {other:?}"
        ),
    }
}

#[test]
fn number_invention_wholly_fabricated_sharpe_is_rejected() {
    let facts = make_facts();
    // 9999.9999 is not the return of any candidate at all — a pure invention.
    let text = "The strategy v0.5.macd achieved a Sharpe ratio of 9999.9999, \
                which is extraordinary.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::FabricatedNumber("9999.9999".to_string())),
        "a wholly invented number must be rejected"
    );
}

#[test]
fn number_invention_correct_value_wrong_candidate_still_passes_if_present_elsewhere() {
    // P3 does membership over ALL candidates' KPI strings, not per-candidate —
    // this documents the existing (intentional) scope of the check: a number
    // that is a real KPI value for ANY candidate is accepted, even if the
    // narration's prose attributes it to a different one. Attribution
    // mismatches are the concern of P1 (wrong crown), not P3.
    let facts = make_facts();
    // "5.00%" is v0.sma's total_return_pct, not v0.5.macd's — still a real
    // number token, so P3 passes (P1 doesn't fire because v0.sma isn't
    // co-located with a crown lexeme here).
    let text = "The strategy v0.5.macd was crowned the winner. \
                One of the candidates returned 5.00% over the period.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Pass,
        "a number present in allowed_numbers() for ANY candidate passes P3"
    );
}

// ── 3. Prediction: banned prediction verbs ──────────────────────────────────

#[test]
fn prediction_will_rise_next_week_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. \
                Based on its track record, the strategy will rise next week.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("will rise".to_string())),
        "'will rise' must be rejected as a prediction verb"
    );
}

#[test]
fn prediction_expected_to_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. It is expected to \
                keep performing well.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("expected to".to_string())),
        "'expected to' must be rejected as a prediction verb"
    );
}

#[test]
fn prediction_forecast_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. Analysts forecast \
                continued strength.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("forecast".to_string())),
        "'forecast' must be rejected as a prediction verb"
    );
}

#[test]
fn prediction_predict_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. We predict it will \
                keep doing well.";

    // Note: "predict" occurs earlier in the scan order than "will keep" for
    // this text, but BANNED_PHRASES is scanned in array order regardless of
    // position in text — the ORIGINAL list's "will keep" is checked before
    // the P2-1 "predict" entry, and "will keep" IS a substring of this text
    // too. Assert only that SOME banned phrase fires (the P1 wrong-crown /
    // P4 ordering is not the property under test here); a stricter test
    // pins the exact phrase using text with no earlier-list collision.
    match check_faithful(text, &facts) {
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase(_)) => {}
        other => panic!("expected Reject(BannedPhrase(_)); got {other:?}"),
    }
}

#[test]
fn prediction_predict_alone_is_rejected() {
    let facts = make_facts();
    // Isolate "predict" with no other banned phrase present.
    let text = "The strategy v0.5.macd was crowned the winner. Some analysts predict more gains.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("predict".to_string())),
        "'predict' must be rejected as a prediction verb"
    );
}

#[test]
fn prediction_probably_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. It will probably do fine too.";

    match check_faithful(text, &facts) {
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase(_)) => {}
        other => panic!("expected Reject(BannedPhrase(_)); got {other:?}"),
    }
}

#[test]
fn prediction_likely_to_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. It is likely to \
                remain strong.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("likely to".to_string())),
        "'likely to' must be rejected as a prediction verb"
    );
}

#[test]
fn prediction_anticipates_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. The model anticipates \
                further gains.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("anticipates".to_string())),
        "'anticipates' must be rejected as a prediction verb"
    );
}

#[test]
fn prediction_projected_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. Its projected path \
                looks strong.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("projected".to_string())),
        "'projected' must be rejected as a prediction verb"
    );
}

// ── 4. Causation: banned causation clauses ──────────────────────────────────

#[test]
fn causation_drawdown_because_of_macro_headwinds_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. \
                The drawdown happened because of macro headwinds.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("because of".to_string())),
        "'because of' must be rejected as a causation clause"
    );
}

#[test]
fn causation_driven_by_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. \
                Its return was driven by strong trend signals.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("driven by".to_string())),
        "'driven by' must be rejected as a causation clause"
    );
}

#[test]
fn causation_caused_by_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. \
                The gain was caused by favorable market conditions.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("caused by".to_string())),
        "'caused by' must be rejected as a causation clause"
    );
}

#[test]
fn causation_due_to_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. \
                Its performance improved due to lower volatility.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("due to".to_string())),
        "'due to' must be rejected as a causation clause"
    );
}

// ── 5. Recommendation: banned advice phrases ────────────────────────────────

#[test]
fn recommendation_you_should_buy_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. You should buy \
                this asset right away.";

    // "you should buy" (the original ADR-0064 list) fires before the P2-1
    // "you should" entry since it appears earlier in BANNED_PHRASES — but
    // both are valid rejections of the same underlying hazard (advice).
    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("you should buy".to_string())),
        "'you should buy' must be rejected as a recommendation phrase"
    );
}

#[test]
fn recommendation_you_should_bare_is_rejected() {
    let facts = make_facts();
    // No "buy"/"sell"/"invest" suffix — isolates the new bare "you should".
    let text = "The strategy v0.5.macd was crowned the winner. You should feel \
                confident about this pick.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("you should".to_string())),
        "'you should' (bare) must be rejected as a recommendation phrase"
    );
}

#[test]
fn recommendation_we_recommend_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. We recommend \
                this approach going forward.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("we recommend".to_string())),
        "'we recommend' must be rejected as a recommendation phrase"
    );
}

#[test]
fn recommendation_buy_now_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. Buy now while it's hot.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("buy now".to_string())),
        "'buy now' must be rejected as a recommendation phrase"
    );
}

#[test]
fn recommendation_sell_now_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. If you're in \
                cash, sell now.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("sell now".to_string())),
        "'sell now' must be rejected as a recommendation phrase"
    );
}

#[test]
fn recommendation_invest_in_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. You could invest \
                in this asset today.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("invest in".to_string())),
        "'invest in' must be rejected as a recommendation phrase"
    );
}

#[test]
fn recommendation_stay_away_from_is_rejected() {
    let facts = make_facts();
    let text = "The strategy v0.5.macd was crowned the winner. Some say you \
                should stay away from the runner-up.";

    // Note: "should stay away from" contains BOTH "you should" (bare) and
    // "stay away from" — the loop scans BANNED_PHRASES in array order and
    // "you should" appears earlier in the P2-1 block than "stay away from",
    // so "you should" fires first here. This test documents that ordering
    // rather than asserting a specific phrase collision; the isolated case
    // below pins "stay away from" alone.
    match check_faithful(text, &facts) {
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase(_)) => {}
        other => panic!("expected Reject(BannedPhrase(_)); got {other:?}"),
    }
}

#[test]
fn recommendation_stay_away_from_isolated_is_rejected() {
    let facts = make_facts();
    // Isolate "stay away from" with no "you should" prefix in the sentence.
    let text = "The strategy v0.5.macd was crowned the winner. \
                Investors ought to stay away from the runner-up strategy.";

    assert_eq!(
        check_faithful(text, &facts),
        FaithfulnessVerdict::Reject(RejectReason::BannedPhrase("stay away from".to_string())),
        "'stay away from' must be rejected as a recommendation phrase"
    );
}

// ── Backward-compat: the existing F9 fallback path stays green ──────────────

/// The pre-P2-1 `build_faithful_text` fixture (the same shape
/// `narration.rs`'s own `FaithfulFakeProvider` emits) must STILL produce
/// `Pass` after the hardening — proving the extension is additive, not a
/// regression on the happy path. This mirrors `narration.rs`'s own
/// `d4_faithful_narration_passes` test, run here through the public API as
/// an independent backward-compat proof.
#[test]
fn backward_compat_pre_p2_1_faithful_text_still_passes() {
    let facts = make_facts();
    let text = agent::build_faithful_text(&facts);

    assert_eq!(
        check_faithful(&text, &facts),
        FaithfulnessVerdict::Pass,
        "the pre-P2-1 build_faithful_text() output must still PASS after the \
         P2-1 hardening (verbatim-number match + extended banned-phrase list) \
         — this is the mandatory backward-compat proof"
    );
}

/// A second backward-compat proof using an `AllFragile` outcome with the
/// mandatory fragility caveat present — the P2 predicate's happy path.
#[test]
fn backward_compat_all_fragile_faithful_text_still_passes() {
    use agent::NarrationOutcome_;
    use smol_str::SmolStr;

    let facts = NarrationFacts {
        outcome: NarrationOutcome_::AllFragile,
        winner_id: SmolStr::new("v0.5.rsi"),
        candidate_ids: vec![
            SmolStr::new("v0.sma"),
            SmolStr::new("v0.5.rsi"),
            SmolStr::new("v0.buyhold"),
        ],
        candidate_kpi_strings: vec![CandidateKpiStrings {
            strategy_id: SmolStr::new("v0.5.rsi"),
            sharpe: "0.0800".to_string(),
            sortino: "0.0900".to_string(),
            calmar: "0.0600".to_string(),
            total_return_pct: "2.00%".to_string(),
            max_drawdown: "-22.00%".to_string(),
            trade_count: "15".to_string(),
        }],
        winner_robustness_label: Some(SmolStr::new("fragile")),
        reason_codes: vec![SmolStr::new(
            "all candidates flagged fragile under resampling",
        )],
    };

    let text = agent::build_faithful_text(&facts);

    assert_eq!(
        check_faithful(&text, &facts),
        FaithfulnessVerdict::Pass,
        "an AllFragile faithful narration must still PASS after P2-1 hardening"
    );
}
