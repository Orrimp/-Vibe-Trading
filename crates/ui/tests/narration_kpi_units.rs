//! Bug-log `#198`: the F9 "Explain" facts carry the leaderboard's percent digits.
//!
//! `LeaderRow.total_return_pct` and `.max_drawdown` are FRACTIONS (`0.1837` =
//! +18.37 %, `crates/ui/src/leaderboard/state.rs`). The leaderboard's row
//! builder (`crates/ui/src/screens/leaderboard.rs`) renders them through
//! `ui::widgets::num`:
//!
//! - return: `signed_pct(leader.total_return_pct, mode)`, which is
//!   `fmt_pct_signed(fraction)` plus a colour;
//! - drawdown: `format_pct_max_dd(leader.max_drawdown * dec_100(), mode)`.
//!
//! Until `#198`, `narration_request_from_mirror` printed the raw fraction with a
//! `%` (`"0.18%"` beside an on-screen `+18.37%`). The LLM was told the wrong
//! number, and P3 enforced it. Every expected string here is derived from those
//! two screen formatters, called for real, and never from the subject.
//!
//! The narration string is the screen string minus ONE presentation glyph: a
//! leading `+` on a gain, a leading U+2212 on a drawdown (ADR-0064 D12). Why each
//! glyph is dropped:
//!
//! - A stored `+` form would reject the plain `18.37%` a faithful narration
//!   writes. P3 tries the raw token, then the token with its leading `+` stripped
//!   (`trim_start_matches('+')`), and never adds one.
//! - A stored U+2212 form can never match, because U+2212 never joins a token.
//!
//! `agent::narration::kpi_return_pct` documents the full matrix.
//!
//! ## What the four tests pin
//!
//! 1. The ui builder's return string equals the leaderboard return cell, for
//!    every row of three fixtures plus edge rows (the zero, > 100 % and midpoint
//!    edges, where a different rounding call would print different digits).
//!    The midpoint rows tell banker's rounding from BOTH half-away-from-zero and
//!    half-toward-zero, and a guard asserts it.
//! 2. The same for the drawdown.
//! 3. The ui builder and `agent::NarrationFacts::from_report` (the agent
//!    builder, `render_kpi_strings`) agree on all seven strings (the id and the
//!    six KPIs) over one real engine report. `agent` cannot depend on `ui`, so
//!    this is the only target where both builders and the screen formatters meet.
//!    The agreement check alone cannot see a fault inside the SHARED
//!    `kpi_return_pct` / `kpi_drawdown_pct`, because both builders call them; the
//!    per-candidate screen checks catch that. Its sign pins rest on the run having
//!    a losing arm, so a guard asserts one.
//! 4. P3 round trip, over the ui builder's facts. Prose that copies the screen
//!    passes `check_faithful`, and so does the plain `18.37%` a faithful narration
//!    writes. The pre-fix figure and a dropped sign are rejected as fabricated.
//!
//! ## T-T4 falsification probes
//!
//! Each probe is ONE edit, located by symbol and quoted line. Snapshot the file,
//! apply the edit, run the named target, then restore the file and `touch` it.
//! The `forward_narration_relay.rs` module doc gives the full procedure, which
//! includes confirming `Compiling` and the SHA-256 after the restore.
//!
//! Probe ids are shared with the `#198` unit tests in `crates/agent/src/narration.rs`.
//!
//! | # | symbol (file) | quoted line | mutation | expected RED here |
//! |---|---|---|---|---|
//! | K1 | `agent::narration::pct_digits` | `let pct = (fraction * rust_decimal::Decimal::ONE_HUNDRED).round_dp(2);` | → `let pct = fraction.round_dp(2);` (both builders lose the × 100) | tests 1-4 (and all 7 agent `#198` tests) |
//! | K2 | `agent::narration::render_kpi_strings` | `total_return_pct: kpi_return_pct(kpis.total_return_pct),` | divide the argument by `rust_decimal::Decimal::ONE_HUNDRED` (agent builder only) | test 3 only: tests 1, 2 and 4 read the ui builder |
//! | K3 | `agent::narration::render_kpi_strings` | `max_drawdown: kpi_drawdown_pct(kpis.max_drawdown),` | divide the argument by `rust_decimal::Decimal::ONE_HUNDRED` (agent builder only) | test 3 only |
//! | K4 | `ui::live::narration_request_from_mirror` | `total_return_pct: agent::narration::kpi_return_pct(r.total_return_pct),` | divide the argument by `Decimal::ONE_HUNDRED` (ui builder only) | tests 1, 3 and 4; test 2 and every agent test stay green |
//! | K5 | `ui::live::fmt_pct2` | `agent::narration::kpi_drawdown_pct(d)` | → the pre-fix body `format!("{}%", pad_fractional_kpi(&d.round_dp(2).to_string(), 2))` (ui builder only) | tests 2, 3 and 4 |
//! | K6 | `agent::narration::kpi_return_pct` | `format!("-{digits}%")` | → `format!("{digits}%")` (a loss loses its sign) | tests 1 and 4; test 3 only if the synthetic run has a losing arm |
//! | K7 | `agent::narration::kpi_return_pct` | `format!("{digits}%")` (the `else` arm) | → `format!("+{digits}%")` | tests 1, 3 and 4 (test 4 at its plain-figure Pass) |
//! | K8 | `agent::narration::kpi_return_pct` | `format!("-{digits}%")` | → `format!("\u{2212}{digits}%")` | tests 1 and 4; test 3 only with a losing arm |
//! | K10 | `agent::narration::kpi_drawdown_pct` | `format!("{}%", pct_digits(fraction))` | → `kpi_return_pct(fraction)` | test 2, at the edge mirror's negative drawdown |
//! | K11 | `agent::narration::pct_digits` | `pad_fractional_kpi(&pct.abs().to_string(), 2)` | → `pad_fractional_kpi(&pct.to_string(), 2)` | tests 1, 2 and 4; test 3 only with a losing arm |
//! | K12 | `ui::live::fmt_pct2` | `agent::narration::kpi_drawdown_pct(d)` | → `agent::narration::kpi_return_pct(d)` (one fn for both percents) | test 2 only, at the edge mirror's negative drawdown. Test 3 stays green because engine drawdowns are >= 0 |
//! | K13 | `agent::narration::pct_digits` | `let pct = (fraction * rust_decimal::Decimal::ONE_HUNDRED).round_dp(2);` | `.round_dp(2)` → `.round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero)` | tests 1 and 2, at the edge mirror's midpoint rows (`0.18365`, `-0.00125`, `0.00125`) |
//! | K14 | `agent::narration::pct_digits` | the same line | `.round_dp(2)` → `.round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointTowardZero)` | tests 1 and 2, at the edge mirror's `0.18375` return and `0.00375` drawdown |
//!
//! K9 (drop the `!fraction.is_zero()` guard) is caught only by the agent unit
//! test's `-Decimal::ZERO` case: no fixture row is a negative zero.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use agent::narration::{
    CandidateKpiStrings, FaithfulnessVerdict, NarrationFacts, RejectReason, check_faithful,
    kpi_drawdown_pct, kpi_return_pct,
};
use rust_decimal::{Decimal, RoundingStrategy};
use rust_decimal_macros::dec;
use ui::leaderboard::BakeoffReportMirror;
use ui::strings::MINUS_SIGN_LITERAL;
use ui::theme::ThemeMode;
use ui::widgets::num::{fmt_pct_signed, format_pct_max_dd};

// ── The screen's strings: the oracle ─────────────────────────────────────────

/// The leaderboard return cell for a fraction: `signed_pct` → `fmt_pct_signed`.
fn screen_return(fraction: Decimal) -> String {
    fmt_pct_signed(fraction)
}

/// The leaderboard drawdown cell for a fraction: `format_pct_max_dd(fraction × 100)`.
fn screen_drawdown(fraction: Decimal) -> String {
    format_pct_max_dd(fraction * Decimal::ONE_HUNDRED, ThemeMode::Dark).0
}

/// The screen string minus one leading presentation glyph (`+` or U+2212).
fn minus_glyph<'a>(screen: &'a str, glyph: &str) -> &'a str {
    screen.strip_prefix(glyph).unwrap_or(screen)
}

/// The string both builders printed before `#198`: the FRACTION at 2 dp plus
/// `%`. The negative control: a row where it equals the expected string cannot
/// tell the units apart.
fn pre_198_unit(fraction: Decimal) -> String {
    let raw = fraction.round_dp(2).to_string();
    match raw.split_once('.') {
        Some((int, frac)) => format!("{int}.{frac:0<2}%"),
        None => format!("{raw}.00%"),
    }
}

/// The request's facts for `mirror`, with one KPI-string set per row asserted
/// (a silent `zip` truncation would make every per-row loop below vacuous).
fn facts_for(label: &str, mirror: &BakeoffReportMirror) -> NarrationFacts {
    let facts = ui::live::narration_request_from_mirror(mirror).facts;
    assert_eq!(
        facts.candidate_kpi_strings.len(),
        mirror.rows.len(),
        "{label}: one KPI-string set per row"
    );
    facts
}

/// Three fixtures that differ in outcome, row count and sign mix, plus edge
/// rows that no fixture has.
fn mirrors() -> Vec<(&'static str, BakeoffReportMirror)> {
    vec![
        ("ActiveWins", ui::fixtures::fake_bakeoff_report_mirror()),
        (
            "BenchmarkWins",
            ui::fixtures::fake_bakeoff_report_mirror_benchmark_wins(),
        ),
        (
            "with shorts",
            ui::fixtures::fake_bakeoff_report_mirror_with_shorts(),
        ),
        ("edge rows", edge_mirror()),
    ]
}

/// The ActiveWins fixture with six rows moved to the formatters' edges:
///
/// - a loss and a gain that round to 0.00 (the screen shows `-0.00%` / `+0.00%`);
/// - a return above 100 % with a drawdown that rounds up to 100.00 %;
/// - a NEGATIVE drawdown. It is outside the engine's contract, but
///   `format_pct_max_dd` takes its `abs()`, so the narration must as well;
/// - midpoints after × 100. Banker's rounding (`round_dp`, the screen's own call)
///   agrees with half-toward-zero on `18.365`, `-0.125` and `0.125`, and with
///   half-away-from-zero on `18.375` and `0.375`. Each percent therefore has a
///   row against EACH rival call; the tests' guards measure that with
///   `rounds_apart`, so no claim here is trusted. No fixture and no engine report
///   lands on a midpoint.
fn edge_mirror() -> BakeoffReportMirror {
    let mut m = ui::fixtures::fake_bakeoff_report_mirror();
    assert!(m.rows.len() >= 6, "the edge mirror rewrites six rows");
    m.rows[0].total_return_pct = dec!(-0.00001);
    m.rows[1].total_return_pct = dec!(0.00001);
    m.rows[2].total_return_pct = dec!(1.5);
    m.rows[2].max_drawdown = dec!(0.99999);
    m.rows[3].total_return_pct = dec!(0.18375);
    m.rows[3].max_drawdown = dec!(-0.0612);
    m.rows[4].total_return_pct = dec!(0.18365);
    m.rows[4].max_drawdown = dec!(0.00125);
    m.rows[5].total_return_pct = dec!(-0.00125);
    m.rows[5].max_drawdown = dec!(0.00375);
    m
}

/// Whether `fraction × 100` sits on a 2-dp midpoint that `strategy` rounds
/// differently from `round_dp`. Computed from `rust_decimal` alone, never from
/// the subject: it is what makes the midpoint rows a pin and not decoration.
fn rounds_apart(fraction: Decimal, strategy: RoundingStrategy) -> bool {
    let pct = fraction * Decimal::ONE_HUNDRED;
    pct.round_dp_with_strategy(2, strategy) != pct.round_dp(2)
}

/// Strictly below zero (a negative zero is zero, as `fmt_pct_signed` treats it).
fn is_negative(fraction: Decimal) -> bool {
    fraction.is_sign_negative() && !fraction.is_zero()
}

/// **(1) `#198`, return.** For every row, the narration's return string is the
/// leaderboard return cell minus a leading `+`, and `agent`'s own fn agrees.
#[test]
fn narration_return_string_is_the_leaderboard_return_cell_minus_its_plus() {
    let (mut units_differ, mut losses, mut gains) = (0_usize, 0_usize, 0_usize);
    let (mut away_apart, mut toward_apart) = (0_usize, 0_usize);
    for (label, mirror) in mirrors() {
        let facts = facts_for(label, &mirror);
        for (kpi, row) in facts.candidate_kpi_strings.iter().zip(&mirror.rows) {
            let screen = screen_return(row.total_return_pct);
            let want = minus_glyph(&screen, "+");
            assert_eq!(
                kpi.total_return_pct, want,
                "{label} / {}: the narration must say the screen's {screen:?}",
                row.strategy
            );
            assert_eq!(
                kpi_return_pct(row.total_return_pct),
                want,
                "{label} / {}: agent::narration::kpi_return_pct agrees",
                row.strategy
            );
            units_differ += usize::from(pre_198_unit(row.total_return_pct) != want);
            losses += usize::from(is_negative(row.total_return_pct));
            gains += usize::from(screen.starts_with('+'));
            let ret = row.total_return_pct;
            away_apart += usize::from(rounds_apart(ret, RoundingStrategy::MidpointAwayFromZero));
            toward_apart += usize::from(rounds_apart(ret, RoundingStrategy::MidpointTowardZero));
        }
    }
    // The loop pins a dimension only where its rows differ in it (bug-log #161).
    assert!(units_differ > 0, "no row tells a fraction from a percent");
    assert!(
        losses > 0,
        "no losing row: the ASCII-minus branch is unexercised"
    );
    assert!(
        gains > 0,
        "no gaining row: the dropped-'+' branch is unexercised"
    );
    assert!(
        away_apart > 0 && toward_apart > 0,
        "the midpoint rows must tell banker's rounding from both half-up and half-down \
         (away-apart rows {away_apart}, toward-apart rows {toward_apart})"
    );
}

/// **(2) `#198`, drawdown.** For every row, the narration's drawdown string is
/// the leaderboard drawdown cell minus its leading U+2212, and `agent`'s own fn
/// agrees.
#[test]
fn narration_drawdown_string_is_the_leaderboard_drawdown_cell_minus_its_minus() {
    let (mut units_differ, mut negative_inputs) = (0_usize, 0_usize);
    let (mut away_apart, mut toward_apart) = (0_usize, 0_usize);
    for (label, mirror) in mirrors() {
        let facts = facts_for(label, &mirror);
        for (kpi, row) in facts.candidate_kpi_strings.iter().zip(&mirror.rows) {
            let screen = screen_drawdown(row.max_drawdown);
            let want = minus_glyph(&screen, MINUS_SIGN_LITERAL);
            assert_eq!(
                kpi.max_drawdown, want,
                "{label} / {}: the narration must say the screen's {screen:?}",
                row.strategy
            );
            assert_eq!(
                kpi_drawdown_pct(row.max_drawdown),
                want,
                "{label} / {}: agent::narration::kpi_drawdown_pct agrees",
                row.strategy
            );
            units_differ += usize::from(pre_198_unit(row.max_drawdown) != want);
            negative_inputs += usize::from(is_negative(row.max_drawdown));
            let dd = row.max_drawdown;
            away_apart += usize::from(rounds_apart(dd, RoundingStrategy::MidpointAwayFromZero));
            toward_apart += usize::from(rounds_apart(dd, RoundingStrategy::MidpointTowardZero));
        }
    }
    assert!(units_differ > 0, "no row tells a fraction from a percent");
    assert!(
        away_apart > 0 && toward_apart > 0,
        "the midpoint rows must tell banker's rounding from both half-up and half-down"
    );
    assert!(
        negative_inputs > 0,
        "the edge mirror's negative drawdown is what pins the abs()"
    );
}

/// The seven strings of one candidate, comparable (`CandidateKpiStrings` has no
/// `PartialEq`).
fn kpi_fields(k: &CandidateKpiStrings) -> [&str; 7] {
    [
        k.strategy_id.as_str(),
        k.sharpe.as_str(),
        k.sortino.as_str(),
        k.calmar.as_str(),
        k.total_return_pct.as_str(),
        k.max_drawdown.as_str(),
        k.trade_count.as_str(),
    ]
}

/// **(3) `#198`, cross-crate identity.** One real engine report goes through
/// BOTH builders: `agent::NarrationFacts::from_report` (its `render_kpi_strings`)
/// and `ui::live::narration_request_from_mirror` over
/// `BakeoffReportMirror::from_report`. They must give the same seven strings per
/// candidate, and the percents must be the leaderboard's digits for the report's
/// own fractions.
///
/// The agent builder has no production caller (the cockpit builds from the
/// mirror). This test is what stops the two copies drifting apart again.
#[tokio::test]
async fn ui_mirror_and_agent_from_report_agree_on_one_engine_report() {
    use backtest::cancel::cancellation_pair;
    use backtest::engine::{DateRange, ScenarioDataSource};
    use backtest::progress::{ProgressSender, bakeoff_progress_pair};
    use backtest::resample::Horizon;
    use backtest::{BakeoffConfig, BakeoffRequest, RobustnessMode, run_bakeoff};
    use smol_str::SmolStr;
    use trading_core::{StrategyId, Symbol};

    // The `crates/backtest/tests/fdr_annex_identity.rs` field: three arms the
    // engine resolves, plus the benchmark it appends.
    let cfg = BakeoffConfig {
        request: BakeoffRequest {
            symbol: Symbol::new("BTCUSDT"),
            range: DateRange::Last30d,
            seed: [0xAB; 32],
            field: ["v0.sma", "v0.obv", "v0.roc_momentum"]
                .into_iter()
                .map(|id| StrategyId(SmolStr::new(id)))
                .collect(),
            timeframe: Horizon::OneHour,
            initial_capital: dec!(100_000),
            fdr_ledger: None,
        },
        data_source: ScenarioDataSource::Synthetic,
        robustness: RobustnessMode::Skip,
    };
    let (_cancel, cancel_rx) = cancellation_pair();
    let (progress_tx, _progress_rx) = bakeoff_progress_pair();
    let report = run_bakeoff(cfg, cancel_rx, ProgressSender::disabled(), progress_tx)
        .await
        .expect("a synthetic bake-off must succeed");

    let agent_facts = NarrationFacts::from_report(&report);
    let mirror = BakeoffReportMirror::from_report(&report);
    let ui_facts = facts_for("engine report", &mirror);

    assert!(
        report.candidates.len() >= 2,
        "the run must yield candidates to compare (got {})",
        report.candidates.len()
    );
    assert_eq!(
        agent_facts.candidate_kpi_strings.len(),
        report.candidates.len(),
        "agent: one KPI-string set per candidate"
    );
    assert_eq!(
        mirror.rows.len(),
        report.candidates.len(),
        "ui: one mirror row per candidate (the zip below must not truncate)"
    );

    let (mut units_differ, mut losses) = (0_usize, 0_usize);
    let pairs = agent_facts
        .candidate_kpi_strings
        .iter()
        .zip(&ui_facts.candidate_kpi_strings);
    for ((a, u), c) in pairs.zip(&report.candidates) {
        let id = c.strategy.0.as_str();
        assert_eq!(
            kpi_fields(a),
            kpi_fields(u),
            "{id}: the agent and ui builders must agree on every KPI string"
        );
        let ret_screen = screen_return(c.kpis.total_return_pct);
        let dd_screen = screen_drawdown(c.kpis.max_drawdown);
        assert_eq!(
            a.total_return_pct,
            minus_glyph(&ret_screen, "+"),
            "{id}: the return is the screen's {ret_screen:?}"
        );
        assert_eq!(
            a.max_drawdown,
            minus_glyph(&dd_screen, MINUS_SIGN_LITERAL),
            "{id}: the drawdown is the screen's {dd_screen:?}"
        );
        units_differ += usize::from(pre_198_unit(c.kpis.total_return_pct) != a.total_return_pct);
        units_differ += usize::from(pre_198_unit(c.kpis.max_drawdown) != a.max_drawdown);
        losses += usize::from(is_negative(c.kpis.total_return_pct));
    }
    assert!(
        units_differ > 0,
        "every figure of the run is below 0.005 %: it cannot tell a fraction from a percent"
    );
    // The sign checks above pin the ASCII-minus branch only if some arm lost
    // (bug-log #161). The seeded synthetic run gives one today; if the engine or
    // the synthetic data change so that none does, this must fail, not go quiet.
    assert!(
        losses > 0,
        "no losing arm in the run: the return-sign checks are unexercised"
    );
}

/// **(4) `#198`, P3 round trip.** Prose that copies the leaderboard's own strings
/// for the crowned row (glyphs included) passes `check_faithful`. The same prose
/// with the pre-fix figure is rejected as fabricated, and so is a losing row's
/// return with its sign dropped.
#[test]
fn prose_citing_the_screen_passes_p3_and_the_fraction_figure_is_rejected() {
    let mirror = ui::fixtures::fake_bakeoff_report_mirror();
    let facts = facts_for("ActiveWins", &mirror);
    let allowed = facts.allowed_numbers();
    let winner = mirror
        .rows
        .iter()
        .find(|r| r.strategy == mirror.recommendation.winner)
        .expect("the crowned row is in the mirror");
    let loser = mirror
        .rows
        .iter()
        .find(|r| is_negative(r.total_return_pct))
        .expect("the ActiveWins fixture has a losing row");

    let ret = screen_return(winner.total_return_pct);
    let dd = screen_drawdown(winner.max_drawdown);
    let loss = screen_return(loser.total_return_pct);
    assert!(
        ret.starts_with('+') && dd.starts_with(MINUS_SIGN_LITERAL) && loss.starts_with('-'),
        "fixture: the screen glyphs are present, so the copy is verbatim \
         (return {ret:?}, drawdown {dd:?}, loss {loss:?})"
    );
    let prose = |cite_ret: &str, cite_dd: &str, cite_loss: &str| {
        format!(
            "The strategy {} was crowned the winner of this bake-off. Over the window \
             it returned {cite_ret}, with a max drawdown of {cite_dd}. Another candidate \
             returned {cite_loss}.",
            winner.strategy
        )
    };
    let fabricated =
        |t: &str| FaithfulnessVerdict::Reject(RejectReason::FabricatedNumber(t.to_string()));

    assert_eq!(
        check_faithful(&prose(&ret, &dd, &loss), &facts),
        FaithfulnessVerdict::Pass,
        "prose that copies the screen ({ret}, {dd}, {loss}) must pass P3"
    );

    // The other half of the `+` claim: the plain figures a faithful narration
    // writes must pass too. A builder that stored the `+` form would reject them,
    // because P3 strips a leading `+` from the prose and never adds one.
    let plain_ret = minus_glyph(&ret, "+");
    let plain_dd = minus_glyph(&dd, MINUS_SIGN_LITERAL);
    assert!(
        plain_ret != ret && plain_dd != dd,
        "fixture: the plain forms differ from the screen forms ({plain_ret:?}, {plain_dd:?})"
    );
    assert_eq!(
        check_faithful(&prose(plain_ret, plain_dd, &loss), &facts),
        FaithfulnessVerdict::Pass,
        "prose with the plain figures ({plain_ret}, {plain_dd}, {loss}) must pass P3"
    );

    let stale = pre_198_unit(winner.total_return_pct);
    assert!(
        !allowed.contains(&stale),
        "fixture: the pre-fix figure {stale:?} must not be some other KPI"
    );
    assert_eq!(
        check_faithful(&prose(&stale, &dd, &loss), &facts),
        fabricated(&stale),
        "the pre-#198 figure for the winner's return must be rejected"
    );

    let unsigned_loss = loss.trim_start_matches('-').to_string();
    assert!(
        !allowed.contains(&unsigned_loss),
        "fixture: {unsigned_loss:?} must not be some other KPI"
    );
    assert_eq!(
        check_faithful(&prose(&ret, &dd, &unsigned_loss), &facts),
        fabricated(&unsigned_loss),
        "a loss cited without its sign must be rejected"
    );
}
