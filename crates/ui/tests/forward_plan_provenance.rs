//! bug-log #222 + #223 (ADR-0070 § D6 as amended; ADR-0088 § D3 and § D5 as
//! amended): which plan the forward-plan header calls a confidence check, and which
//! plan "Export this plan" writes.
//!
//! ## What this file pins
//!
//! The button and the handler read one predicate,
//! `ui::screens::forward_plan::crowned_export_pair`: the plan on screen is `Ready`,
//! nothing is promoted, a bake-off is `Ready`, and the plan is that bake-off's
//! crowned launch (its coin, its crowned strategy, and its scorecard as the plan's
//! confidence, bit for bit). `export_artifact` is the file a press writes, or `None`.
//!
//! - E1 [`promoted_plan_is_never_exportable`]: from the stock bake-off's crowned plan
//!   (exportable), a promotion refuses it, on the crowned coin, on another coin, with
//!   no bake-off, and with the IDENTICAL crowned view landed again (the #161 pair:
//!   only the promotion differs).
//! - E2 [`crowned_plan_is_exportable_only_beside_the_bakeoff_that_crowned_it`]: the
//!   matching pair is exportable and is the pair on screen (`ptr::eq`); each case
//!   below it differs in one dimension and is refused: a re-run in flight, a failed
//!   re-run, no plan, a newer bake-off on another coin, another crown, each scorecard
//!   field, a scorecard on one side only, on neither side, no crown, and (two-sided)
//!   the same bake-off re-run at another budget before its plan lands, against the
//!   same re-run at the same budget, which still exports.
//! - E3 [`export_artifact_is_the_pair_on_screen_or_nothing`]: over every E1/E2 state,
//!   the artifact exists exactly when the pair does; every exported file carries the
//!   TRUST section and its "~1 in 5" note (the targets of the weak-evidence hint and
//!   of a "What this is NOT" bullet); the stock pair's file is
//!   `plan-BTCUSDT-2024-h1-10101010.md` and carries the weak-evidence caveat and hint.
//! - E4 [`a_refused_export_writes_nothing_and_says_nothing`] (house rule `#187`): a
//!   refused press writes no file, creates no directory and raises no toast; the
//!   same state with the promotion cleared writes the file `export_artifact` names,
//!   byte for byte, and toasts it once.
//! - E5 `real_chain::the_real_crowned_chain_is_exportable_and_the_promoted_one_is_not`
//!   (cfg `live`): the real launch → agent → adapter chain, as LR-213 builds it, on
//!   two crowned arms (`v0.sma`, `v0.5.sma`): each crowned launch is exportable, and
//!   each promoted launch is not, with the promotion pending or cleared. On the
//!   `v0.5.sma` crown the cleared promotion matches the crown's coin and strategy, so
//!   only the scorecard clause refuses it.
//! - W1 [`promoted_copy_says_tuned_not_crowned`]: the promoted headline and caption
//!   say "tuned", not "crowned strategy"; the loading line names no provenance.
//!
//! The pixel layer is LR-222 (`forward_plan_populated_render.rs::
//! promoted_header_is_not_the_crowned_header_on_one_bakeoff`) and LR-223
//! (`plan_export_button_render.rs::export_button_shows_only_beside_the_bakeoff_that_crowned_the_plan`),
//! macOS only. The lib pins are U1
//! (`screens::forward_plan::tests::header_copy_follows_the_promotion_not_the_bakeoff`)
//! and S1-S3 (`export::plan_export::tests`).
//!
//! ## How state is driven
//!
//! Every cockpit starts at `Cockpit::new()` and reaches its state through
//! `ui::state::update`: `BakeoffRunCompleted`, `BakeoffRunRequested`,
//! `ForwardPlanReceived`, `OpenTuneEditor`, `SweepRunRequested`,
//! `SweepRunCompleted`, `PromoteSweptConfig` and `ExportPlan`. One field is written
//! directly: `pending_forward_promotion = None` in E4's success side and E5 (c)/(d2),
//! because the clear is binary-side (`cockpit_live.rs`, ADR-0070 § D6) and no
//! message reaches it. The crowned plans E1-E4 land are
//! `ui::fixtures::fake_forward_plan_crowned_by` (the plan a crowned launch carries,
//! by construction); E5 runs the real chain instead.
//!
//! No `cfg` on the file: E1-E4 and W1 run on both CI legs. E5 sits under
//! `#[cfg(feature = "live")]`, a default ui feature on both legs, so the target lists
//! 6 tests there and 5 in a build without `live`; `scripts/ci_test_target_counts.py`
//! declares 6 on each leg, which turns a cfg-dropped E5 into a CI failure.
//!
//! ## T-T4 probes (bug-log #222, #223)
//!
//! One edit to production code per row (P222-4 and P222-6 edit `strings.rs`, PH-*
//! the serialiser, PF-1 the fixture, PL-1 `live.rs`), OLD found exactly once. Unless a
//! row says otherwise, the command is
//!
//! `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.sdk cargo test -p ui --no-fail-fast --lib --test forward_plan_provenance --test plan_export_button_render --test forward_plan_populated_render --test forward_plan_confidence_render`
//!
//! and after each, the file is restored byte-identical (`cmp` and SHA-256), `touch`ed,
//! and the run is green again. A test the RED cell does not name is expected GREEN.
//! The observed sets are recorded in bug-log `#222` and `#223`.
//!
//! | id | edit | RED (declared) |
//! |---|---|---|
//! | P222-1 | `header_copy`: `if model.pending_forward_promotion.is_some() {` → `if false {` | U1 (b)(c)(d); LR-222 (2a)(2b) |
//! | P222-2 | `(FORWARD_PLAN_PROMOTED_HEADLINE, FORWARD_PLAN_PROMOTED_CAPTION)` → `(FORWARD_PLAN_PROMOTED_HEADLINE, FORWARD_PLAN_CAPTION)` | U1 (b)(c)(d) caption; LR-222 (2b) |
//! | P222-3 | the `if` → `if matches!(&model.forward_plan_screen_state.plan, PanelState::Ready(p) if p.confidence.is_none()) {` | U1 (b)(c); LR-222 (5) (P lands the tuned plan, which has no confidence, so only the Loading frame mislabels) |
//! | P222-4 | 146 characters appended to `FORWARD_PLAN_PROMOTED_CAPTION` (3 lines at 720 px); runs `--lib`, this target and LR-222 alone (`--exact`) | LR-222 (4) |
//! | P222-5 | the `if` → `… .is_some() && matches!(model.forward_plan_screen_state.plan, PanelState::Ready(_)) {` | U1 (c); LR-222 (5) |
//! | P222-6 | `FORWARD_PLAN_LOADING` back to "Reading the crowned strategy’s standing rules…" | W1 only |
//! | P222-7 | `FORWARD_PLAN_PROMOTED_HEADLINE` → `"Your tuned config\nYour tuned config"` (a two-line headline) | LR-222 (2c)(4) |
//! | P223-1 | `view`: `crowned_export_pair(model).is_some()` → `matches!(st.plan, PanelState::Ready(_))` | LR-223 (P)(R)(X)(N) |
//! | P223-2 | `let crowned_here = model.pending_forward_promotion.is_none()` → `let crowned_here = true` | E1 (d); E4 refusing side (LR-223 (P) stays green: its tuned plan is refused by the strategy and scorecard clauses) |
//! | P223-3 | `&& plan.symbol == report.coin` → `&& true` | E2 (e); LR-223 (X) |
//! | P223-4 | `&& plan.strategy == crowned.strategy` → `&& true` | E2 (f) |
//! | P223-5 | `&& carries_its_scorecard(…);` → `&& true;` | E2 (g1)-(g7); E3 (g6)(g7); E5 (d2) |
//! | P223-6 | `c.n_candidates == s.n_candidates` → `true` | E2 (g1) |
//! | P223-7 | `&& c.deflated_sharpe.to_bits() == s.deflated_sharpe.to_bits()` → `&& true` | E2 (g2) |
//! | P223-8 | `&& c.crown_clears_dsr == s.crown_clears_dsr` → `&& true` | E2 (g3) |
//! | P223-9 | `&& c.min_btl_years.to_bits() == s.min_btl_years.to_bits()` → `&& true` | E2 (g4) |
//! | P223-10 | `carries_its_scorecard`: `return false;` → `return true;` | E2 (g5)(g6)(g7); E3 (g6)(g7); E5 (d2) |
//! | P223-11 | `export_artifact`: `crowned_export_pair(model)?` → pair whatever plan and bake-off are `Ready` (HEAD's handler) | E3 at E1 (a)(b)(d) and E2 (e)(f)(g1)-(g7)(h)(i); E4 refusing side |
//! | P223-12 | `view`: `crowned_export_pair(model).is_some()` → `false` | LR-223 (C); `export_button_paints_in_ready_header`; `export_button_absent_when_not_ready` |
//! | P223-13 | the budget clause `&& model.forward_budget.is_some_and(…);` → `;` | E2 (i) |
//! | PH-1 | `if trust.is_some() {` → `if true {` | S1 (bare side); S2 |
//! | PH-2 | the same → `if false {` | S1 (TRUST side); S3; E3 at the E2 (a) state |
//! | PH-3 | the `if let Some(confidence) = trust { push_trust(..) }` block moved above `push_measured_answer(..)` | S1 (order) only |
//! | PF-1 | `fake_forward_plan_crowned_by`: the confidence → `None` | E1 base; E2 (a)(i0); E3 at the E2 (a) state; E4 success side; LR-223 (C); both `export_button_*`; `state::tests::export_plan_writes_artifact_and_toasts_success` |
//! | PL-1 | `live.rs` `crowned_forward_run_config`: `confidence: bakeoff_confidence(mirror),` → `confidence: None,` (= P213-3; `--test forward_launch_config` added) | E5 (a)(d0); L2; L3 crowned side; LR-213 (3) |
//! | P213-1 | `live.rs` `promoted_forward_run_config`: `confidence: None,` → HEAD's #213 rule (the leaderboard's scorecard; `--test forward_launch_config` added) | L1 (a)(b)(e); L3; LR-213 (2)(4) under the banded count; E5 (d2) |
//! | PE4 | `state::tests::export_plan_writes_artifact_and_toasts_success`: its directory cleanup removed; `--lib --test forward_plan_provenance -- --nocapture` | E4 prints `UNMEASURED` (no test fails: the clause is reported unmeasured, not skipped silently) |

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

use rust_decimal_macros::dec;
use smol_str::SmolStr;
use trading_core::{Money, Usdt};
use ui::forward_plan::{ForwardPlanView, PlanRuleView};
use ui::leaderboard::{BakeoffReportMirror, LeaderboardLookback};
use ui::screens::forward_plan::{crowned_export_pair, export_artifact};
use ui::state::{Cockpit, Message, PanelState, ToastSeverity, update};
use ui::strings::{
    FORWARD_PLAN_CAPTION, FORWARD_PLAN_HEADLINE, FORWARD_PLAN_LOADING,
    FORWARD_PLAN_PROMOTED_CAPTION, FORWARD_PLAN_PROMOTED_HEADLINE, LEADERBOARD_CROWN_WEAK_EVIDENCE,
    LEADERBOARD_CROWN_WEAK_EVIDENCE_HINT, PLAN_EXPORT_ONE_IN_FIVE_NOTE, PLAN_EXPORT_SECTION_TRUST,
};
use ui::tune::{PromoteParams, TuneFamily};

// ── Helpers ─────────────────────────────────────────────────────────────────────

/// The stock bake-off: BTCUSDT, 2024 H1, crowned `v0.sma`, ActiveWins below DSR
/// (scorecard 13 / 0.61 / false / 6.4), run seed `[0x10; 32]`.
fn stock() -> BakeoffReportMirror {
    ui::fixtures::fake_bakeoff_report_mirror()
}

/// The plan `report`'s crowned launch carries.
fn crowned_plan(report: &BakeoffReportMirror) -> ForwardPlanView {
    ui::fixtures::fake_forward_plan_crowned_by(report).expect("the bake-off crowns a row")
}

/// The forward run's budget the crowned fixtures are sized for: `fake_forward_plan`'s
/// €200 ≈ 200 USDT.
fn stock_budget() -> Money<Usdt> {
    Money::<Usdt>::from_decimal(dec!(200))
}

/// `Cockpit::new()`, then `report` landed (`BakeoffRunCompleted`) and, when it crowned
/// a row, the launch's budget (`ForwardPaperTradeStarted`, which the binary sends for
/// every crowned launch before its plan can land), then `plan` landed
/// (`ForwardPlanReceived`), each only if given.
fn landed(report: Option<&BakeoffReportMirror>, plan: Option<ForwardPlanView>) -> Cockpit {
    let mut cockpit = Cockpit::new();
    if let Some(report) = report {
        update(
            &mut cockpit,
            Message::BakeoffRunCompleted(Ok(report.clone())),
        );
        if report.crowned_row().is_some() {
            update(
                &mut cockpit,
                Message::ForwardPaperTradeStarted(stock_budget(), None),
            );
        }
    }
    if let Some(plan) = plan {
        update(&mut cockpit, Message::ForwardPlanReceived(plan));
    }
    cockpit
}

/// The stock bake-off with its crowned plan landed: the exportable base.
fn crowned_base() -> Cockpit {
    let report = stock();
    landed(Some(&report), Some(crowned_plan(&report)))
}

/// `cockpit` after `message`.
fn after(mut cockpit: Cockpit, message: Message) -> Cockpit {
    update(&mut cockpit, message);
    cockpit
}

/// Promote an SMA grid swept on `coin` (`Sma{10,20}`) on top of `cockpit`, through
/// `update` only: "Tune…", Run, the grid lands, "Use this config". The plan goes
/// `Loading`.
fn promote(cockpit: &mut Cockpit, coin: &str) {
    update(
        cockpit,
        Message::OpenTuneEditor {
            family: TuneFamily::Sma,
            coin: trading_core::Symbol::new(coin),
            lookback: LeaderboardLookback::H1_2024,
        },
    );
    update(cockpit, Message::SweepRunRequested);
    let mut grid = ui::fixtures::fake_sweep_report_mirror();
    grid.coin = SmolStr::new(coin);
    update(cockpit, Message::SweepRunCompleted(Ok(grid)));
    update(
        cockpit,
        Message::PromoteSweptConfig(PromoteParams::Sma {
            fast_len: 10,
            slow_len: 20,
        }),
    );
}

/// The tuned plan a `Sma{10,20}` promotion on `coin` lands: id `v0.5.sma`
/// (`ui::state::promote_swept_config`), the tuned rules, no confidence (ADR-0070
/// § D11). A fixture; E5 runs the real chain.
fn tuned_plan(coin: &str) -> ForwardPlanView {
    let mut plan = ui::fixtures::fake_forward_plan();
    plan.strategy = SmolStr::new("v0.5.sma");
    plan.symbol = SmolStr::new(coin);
    plan.rule = PlanRuleView::SmaCross {
        fast_len: 10,
        slow_len: 20,
    };
    plan.confidence = None;
    plan
}

/// `crowned_base()` promoted on `coin`, then `plan` landed.
fn promoted_with(base: Cockpit, coin: &str, plan: ForwardPlanView) -> Cockpit {
    let mut cockpit = base;
    promote(&mut cockpit, coin);
    update(&mut cockpit, Message::ForwardPlanReceived(plan));
    cockpit
}

/// The stock bake-off changed by `edit`, landed as a newer bake-off under the
/// crowned base's plan.
fn newer(edit: impl FnOnce(&mut BakeoffReportMirror)) -> Cockpit {
    let mut report = stock();
    edit(&mut report);
    after(crowned_base(), Message::BakeoffRunCompleted(Ok(report)))
}

/// Every E1 and E2 state, with whether it is exportable.
fn states() -> Vec<(&'static str, Cockpit, bool)> {
    let no_scorecard = {
        let mut report = stock();
        report.scorecard = None;
        report
    };
    let bare_crowned = {
        let mut plan = crowned_plan(&stock());
        plan.confidence = None;
        plan
    };
    vec![
        // E1
        ("E1 base", crowned_base(), true),
        (
            "E1 (a)",
            promoted_with(crowned_base(), "BTCUSDT", tuned_plan("BTCUSDT")),
            false,
        ),
        (
            "E1 (b)",
            promoted_with(crowned_base(), "ETHUSDT", tuned_plan("ETHUSDT")),
            false,
        ),
        (
            "E1 (c)",
            promoted_with(Cockpit::new(), "BTCUSDT", tuned_plan("BTCUSDT")),
            false,
        ),
        (
            "E1 (d)",
            promoted_with(crowned_base(), "BTCUSDT", crowned_plan(&stock())),
            false,
        ),
        // E2
        ("E2 (a)", crowned_base(), true),
        (
            "E2 (b)",
            after(crowned_base(), Message::BakeoffRunRequested),
            false,
        ),
        (
            "E2 (c)",
            after(
                after(crowned_base(), Message::BakeoffRunRequested),
                Message::BakeoffRunCompleted(Err(SmolStr::new("the re-run failed"))),
            ),
            false,
        ),
        ("E2 (d)", landed(Some(&stock()), None), false),
        ("E2 (e)", newer(|r| r.coin = SmolStr::new("ETHUSDT")), false),
        ("E2 (f)", newer(|r| r.crowned = Some(1)), false),
        (
            "E2 (g1)",
            newer(|r| {
                if let Some(sc) = r.scorecard.as_mut() {
                    sc.n_candidates = 14;
                }
            }),
            false,
        ),
        (
            "E2 (g2)",
            newer(|r| {
                if let Some(sc) = r.scorecard.as_mut() {
                    sc.deflated_sharpe = 0.62;
                }
            }),
            false,
        ),
        (
            "E2 (g3)",
            newer(|r| {
                if let Some(sc) = r.scorecard.as_mut() {
                    sc.crown_clears_dsr = true;
                }
            }),
            false,
        ),
        (
            "E2 (g4)",
            newer(|r| {
                if let Some(sc) = r.scorecard.as_mut() {
                    sc.min_btl_years = 6.5;
                }
            }),
            false,
        ),
        ("E2 (g5)", newer(|r| r.scorecard = None), false),
        (
            "E2 (g6)",
            landed(
                Some(&no_scorecard),
                ui::fixtures::fake_forward_plan_crowned_by(&no_scorecard),
            ),
            false,
        ),
        ("E2 (g7)", landed(Some(&stock()), Some(bare_crowned)), false),
        ("E2 (h)", newer(|r| r.crowned = None), false),
        // The budget dimension, two-sided (#161): the same bake-off re-run (it is
        // budget-independent and seeded with a constant, so it crowns the same row
        // with a bit-identical scorecard), before its new plan lands.
        ("E2 (i0)", rerun_at(stock_budget()), true),
        (
            "E2 (i)",
            rerun_at(Money::<Usdt>::from_decimal(dec!(162.50))),
            false,
        ),
    ]
}

/// The crowned base, then the stock bake-off re-run and landed again, and the
/// launch's budget sent as `budget` (`ForwardPaperTradeStarted`), before the re-run's
/// crowned plan lands: the old plan is still on screen.
fn rerun_at(budget: Money<Usdt>) -> Cockpit {
    let mut cockpit = after(crowned_base(), Message::BakeoffRunRequested);
    update(&mut cockpit, Message::BakeoffRunCompleted(Ok(stock())));
    update(
        &mut cockpit,
        Message::ForwardPaperTradeStarted(budget, None),
    );
    cockpit
}

/// Check every state whose id starts with `prefix` against its expectation.
fn check_exportable(prefix: &str) -> Vec<String> {
    states()
        .into_iter()
        .filter(|(id, _, _)| id.starts_with(prefix))
        .filter_map(|(id, cockpit, expected)| {
            let got = crowned_export_pair(&cockpit).is_some();
            (got != expected).then(|| {
                format!(
                    "{id}: crowned_export_pair is {}, expected {}",
                    if got { "Some" } else { "None" },
                    if expected { "Some" } else { "None" }
                )
            })
        })
        .collect()
}

fn report(test: &str, fails: &[String]) {
    assert!(
        fails.is_empty(),
        "{test}: {} failure(s):\n  {}",
        fails.len(),
        fails.join("\n  ")
    );
}

// ── E1 ──────────────────────────────────────────────────────────────────────────

/// **E1 (bug-log #223).** A promoted plan is never exportable. The precondition is
/// the stock crowned base (exportable); then (a) a BTCUSDT SMA promotion with its
/// tuned plan, (b) the same on ETHUSDT, (c) the same with no bake-off, and (d) a
/// promotion with the IDENTICAL crowned view landed again, so only the promotion
/// differs from the base (the #161 pair).
#[test]
fn promoted_plan_is_never_exportable() {
    let mut fails = check_exportable("E1");
    let tuned = tuned_plan("BTCUSDT");
    let stock_crowned = crowned_plan(&stock());
    if !(tuned.strategy != stock_crowned.strategy
        && tuned.confidence.is_none()
        && stock_crowned.confidence.is_some())
    {
        fails.push("precondition: the tuned plan is not the crowned one".to_string());
    }
    report("E1", &fails);
}

// ── E2 ──────────────────────────────────────────────────────────────────────────

/// **E2 (bug-log #223).** A crowned plan is exportable only beside the bake-off that
/// crowned it. (a) The matching pair is `Some`, and both references are the values
/// on screen. Each case below differs from (a) in one dimension and is `None`: (b) a
/// re-run in flight (`BakeoffRunRequested`); (c) that re-run failed; (d) the
/// bake-off with no plan; (e) a newer bake-off differing only in its coin; (f) only in
/// its crown (`v0.5.macd`); (g1)-(g4) only in one scorecard field each (13→14,
/// 0.61→0.62, false→true, 6.4→6.5); (g5) the mirror's scorecard `None`, the plan's
/// `Some`; (g6) both `None`; (g7) the matching pair with only the plan's confidence
/// `None`; (h) no crown. The budget dimension, two-sided: (i) the same bake-off re-run
/// with the launch's budget changed, before its new plan lands, is `None`; (i0) the same
/// re-run at the same budget is `Some`.
#[test]
fn crowned_plan_is_exportable_only_beside_the_bakeoff_that_crowned_it() {
    let mut fails = check_exportable("E2");
    let base = crowned_base();
    let on_screen = (
        &base.forward_plan_screen_state.plan,
        &base.leaderboard_screen_state.result,
    );
    match (crowned_export_pair(&base), on_screen) {
        (Some((plan, report)), (PanelState::Ready(p), PanelState::Ready(r)))
            if std::ptr::eq(plan, p) && std::ptr::eq(report, r) => {}
        _ => fails.push("E2 (a): the pair must be the plan and bake-off on screen".to_string()),
    }
    report("E2", &fails);
}

// ── E3 ──────────────────────────────────────────────────────────────────────────

/// **E3 (bug-log #223).** Over every E1/E2 state, `export_artifact` is `Some`
/// exactly when `crowned_export_pair` is. Every exported file carries the TRUST
/// section and its "~1 in 5" note, the targets of the weak-evidence hint and of the
/// "What this is NOT" chance bullet. The stock pair's file is
/// `plan-BTCUSDT-2024-h1-10101010.md` (written out, not derived) and carries the
/// weak-evidence caveat and the hint.
#[test]
fn export_artifact_is_the_pair_on_screen_or_nothing() {
    let mut fails = Vec::new();
    for (id, cockpit, _) in states() {
        let artifact = export_artifact(&cockpit);
        if artifact.is_some() != crowned_export_pair(&cockpit).is_some() {
            fails.push(format!(
                "{id}: export_artifact is {}, crowned_export_pair is not",
                if artifact.is_some() { "Some" } else { "None" }
            ));
        }
        if let Some((_, text)) = &artifact {
            for (name, target) in [
                ("PLAN_EXPORT_SECTION_TRUST", PLAN_EXPORT_SECTION_TRUST),
                ("PLAN_EXPORT_ONE_IN_FIVE_NOTE", PLAN_EXPORT_ONE_IN_FIVE_NOTE),
            ] {
                if !text.contains(target) {
                    fails.push(format!("{id}: the exported file lacks {name}"));
                }
            }
        }
        if id == "E2 (a)" {
            match &artifact {
                Some((filename, text)) => {
                    if filename != "plan-BTCUSDT-2024-h1-10101010.md" {
                        fails.push(format!("E2 (a): filename {filename:?}"));
                    }
                    for (name, line) in [
                        (
                            "LEADERBOARD_CROWN_WEAK_EVIDENCE",
                            LEADERBOARD_CROWN_WEAK_EVIDENCE,
                        ),
                        (
                            "LEADERBOARD_CROWN_WEAK_EVIDENCE_HINT",
                            LEADERBOARD_CROWN_WEAK_EVIDENCE_HINT,
                        ),
                    ] {
                        if !text.contains(line) {
                            fails.push(format!("E2 (a): the exported file lacks {name}"));
                        }
                    }
                }
                None => fails.push("E2 (a): the stock pair must export".to_string()),
            }
        }
    }
    report("E3", &fails);
}

// ── E4 ──────────────────────────────────────────────────────────────────────────

/// **E4 (bug-log #223, house rule `#187`: the refusing path writes nothing).** E1
/// (d)'s state (promoted, the identical crowned view on screen) on a mirror with run
/// seed `[0x23; 32]`, so its file `F = plan-BTCUSDT-2024-h1-23232323.md` belongs to
/// this test alone. `D` is the directory `plan_exports_dir()` resolves to.
///
/// - Refusing side: `ExportPlan` raises no toast and leaves `F` absent; if `D` was
///   absent before, it stays absent (the refusal returns before `create_dir_all`).
/// - Success side: the promotion cleared (the clear is binary-side, ADR-0070 § D6),
///   `ExportPlan` raises exactly one Success toast naming `F`, and `F` holds
///   `export_artifact`'s text, byte for byte.
///
/// It removes a stale `F` first and, after, only what it created (`F`, and `D` with
/// `remove_dir` if it created `D`). Residual race: another process creating `D`
/// during the one refused `update` would fail the `D` check; its message says so.
#[test]
fn a_refused_export_writes_nothing_and_says_nothing() {
    const F: &str = "plan-BTCUSDT-2024-h1-23232323.md";
    let dir: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "plan-exports"]
        .iter()
        .collect();
    let file = dir.join(F);
    let _ = std::fs::remove_file(&file);
    let d_existed = dir.exists();
    // House rule #187: the success path is observable. The directory clause runs only
    // when the directory is absent before the press; say which, never skip silently.
    if d_existed {
        let entries = std::fs::read_dir(&dir).map_or(0, Iterator::count);
        eprintln!(
            "E4 UNMEASURED: the directory clause (a refused press creates no directory) is \
             not checked: {} existed before the test, with {entries} entr(y/ies)",
            dir.display()
        );
    } else {
        eprintln!(
            "E4 MEASURED: the directory clause runs: {} is absent before the refused press",
            dir.display()
        );
    }

    let mut mirror = stock();
    mirror.run_seed = [0x23; 32];
    let mut c = promoted_with(
        landed(Some(&mirror), Some(crowned_plan(&mirror))),
        "BTCUSDT",
        crowned_plan(&mirror),
    );
    let mut fails = Vec::new();
    if !(c.pending_forward_promotion.is_some() && crowned_export_pair(&c).is_none()) {
        fails.push("precondition: E1 (d)'s state, promoted and refused".to_string());
    }

    // Refusing side.
    let toasts = c.toast_queue.len();
    update(&mut c, Message::ExportPlan);
    if c.toast_queue.len() != toasts {
        fails.push(format!(
            "refusing side: a refused press must raise no toast, got {:?}",
            c.toast_queue.back()
        ));
    }
    if file.exists() {
        fails.push(format!(
            "refusing side: {} must not be written",
            file.display()
        ));
    }
    if !d_existed && dir.exists() {
        fails.push(format!(
            "refusing side: {} did not exist and must not be created (or another process \
             created it during this one update: the named residual race)",
            dir.display()
        ));
    }

    // Success side.
    c.pending_forward_promotion = None;
    let expected = export_artifact(&c);
    let toasts = c.toast_queue.len();
    update(&mut c, Message::ExportPlan);
    match (&expected, c.toast_queue.back()) {
        (Some((name, text)), Some(toast)) => {
            if name != F {
                fails.push(format!(
                    "success side: export_artifact names {name:?}, not {F}"
                ));
            }
            if c.toast_queue.len() != toasts + 1
                || toast.severity != ToastSeverity::Success
                || !toast.message.contains(F)
            {
                fails.push(format!(
                    "success side: exactly one Success toast naming {F}, got {} new, last {toast:?}",
                    c.toast_queue.len() - toasts
                ));
            }
            match std::fs::read_to_string(&file) {
                Ok(written) if written == *text => {}
                Ok(_) => fails.push(format!(
                    "success side: {} differs from export_artifact's text",
                    file.display()
                )),
                Err(e) => fails.push(format!("success side: {} unreadable: {e}", file.display())),
            }
        }
        (None, _) => fails.push("success side: the cleared state must export".to_string()),
        (_, None) => fails.push("success side: the press must toast".to_string()),
    }

    let _ = std::fs::remove_file(&file);
    if !d_existed {
        let _ = std::fs::remove_dir(&dir);
    }
    report("E4", &fails);
}

// ── E5 (cfg live) ───────────────────────────────────────────────────────────────

/// **E5 (bug-log #223).** The real chain as LR-213 builds it:
/// `ui::live::{crowned,promoted}_forward_run_config` →
/// `agent::build_forward_plan_from_registry` → `ForwardPlanView::from_plan` →
/// `ForwardPlanReceived`. It guards against false negatives on the two crowned arms
/// it runs; it is not a claim about every arm.
#[cfg(feature = "live")]
mod real_chain {
    use super::{landed, promote, report, stock};
    use rust_decimal_macros::dec;
    use smol_str::SmolStr;
    use trading_core::{Money, Usdt};
    use ui::forward_plan::ForwardPlanView;
    use ui::leaderboard::BakeoffReportMirror;
    use ui::screens::forward_plan::crowned_export_pair;
    use ui::state::{Cockpit, Message, PanelState, update};

    fn budget() -> Money<Usdt> {
        Money::<Usdt>::from_decimal(dec!(187.42))
    }

    /// The agent's plan for `cfg`, mirrored the way the relay mirrors it.
    fn plan_view(cfg: &agent::ForwardRunConfig) -> ForwardPlanView {
        let price = trading_core::Price::new(dec!(64000)).expect("64000 is a valid price");
        let ts = trading_core::Timestamp::new(
            time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1_700_000_000),
        );
        let plan = agent::build_forward_plan_from_registry(
            &agent::config::Config::default(),
            cfg,
            price,
            ts,
            7,
        )
        .expect("the agent resolves a plan for an SMA config");
        ForwardPlanView::from_plan(&plan)
    }

    /// `report` landed, then its crowned launch's plan landed.
    fn crowned_chain(report: &BakeoffReportMirror) -> Cockpit {
        let mut cockpit = landed(Some(report), None);
        let PanelState::Ready(on_screen) = &cockpit.leaderboard_screen_state.result else {
            panic!("precondition: the bake-off is on the leaderboard");
        };
        let cfg = ui::live::crowned_forward_run_config(on_screen, budget())
            .expect("the bake-off crowned a row");
        // The binary sends the launch's budget with every crowned launch.
        update(
            &mut cockpit,
            Message::ForwardPaperTradeStarted(budget(), None),
        );
        update(&mut cockpit, Message::ForwardPlanReceived(plan_view(&cfg)));
        cockpit
    }

    /// `crowned_chain(report)`, then a BTCUSDT SMA promotion and its launch's plan.
    fn promoted_chain(report: &BakeoffReportMirror) -> Cockpit {
        let mut cockpit = crowned_chain(report);
        promote(&mut cockpit, "BTCUSDT");
        let cfg = ui::live::promoted_forward_run_config(&cockpit, budget())
            .expect("a promotion is pending");
        update(
            &mut cockpit,
            Message::ForwardPaperTradeStarted(budget(), None),
        );
        update(&mut cockpit, Message::ForwardPlanReceived(plan_view(&cfg)));
        cockpit
    }

    fn cleared(mut cockpit: Cockpit) -> Cockpit {
        cockpit.pending_forward_promotion = None;
        cockpit
    }

    #[test]
    fn the_real_crowned_chain_is_exportable_and_the_promoted_one_is_not() {
        let mut fails = Vec::new();
        let sma5 = {
            let mut r = stock();
            let i = r.crowned.expect("the stock bake-off crowns a row");
            r.rows[i].strategy = SmolStr::new("v0.5.sma");
            r
        };
        let d2 = cleared(promoted_chain(&sma5));
        let d2_matches_the_crown = matches!(
            &d2.forward_plan_screen_state.plan,
            PanelState::Ready(p) if p.symbol == sma5.coin
                && sma5.crowned_row().is_some_and(|row| row.strategy == p.strategy)
                && p.confidence.is_none()
        );
        if !d2_matches_the_crown {
            fails.push(
                "precondition (d2): the tuned plan matches the v0.5.sma crown's coin and \
                 strategy and carries no confidence, so only the scorecard clause refuses it"
                    .to_string(),
            );
        }
        for (case, cockpit, expected) in [
            (
                "(a) the stock crowned chain (v0.sma)",
                crowned_chain(&stock()),
                true,
            ),
            (
                "(b) the promoted chain on it",
                promoted_chain(&stock()),
                false,
            ),
            (
                "(c) (b) with the promotion cleared",
                cleared(promoted_chain(&stock())),
                false,
            ),
            (
                "(d0) the v0.5.sma crowned chain",
                crowned_chain(&sma5),
                true,
            ),
            (
                "(d1) the promoted chain on it",
                promoted_chain(&sma5),
                false,
            ),
            ("(d2) (d1) with the promotion cleared", d2, false),
        ] {
            let got = crowned_export_pair(&cockpit).is_some();
            if got != expected {
                fails.push(format!(
                    "{case}: crowned_export_pair is {}, expected {}",
                    if got { "Some" } else { "None" },
                    if expected { "Some" } else { "None" }
                ));
            }
        }
        report("E5", &fails);
    }
}

// ── W1 ──────────────────────────────────────────────────────────────────────────

/// **W1 (bug-log #222, the `honesty_surface_wording` division).** The promoted
/// headline is not the crowned one; the promoted caption says "tuned" and "not the
/// bake-off", and neither "crowned strategy" nor "confidence check on that pick";
/// the loading line, which only a promotion shows today, does not say "crowned";
/// both new keys are registered in `strings::all()`.
#[test]
fn promoted_copy_says_tuned_not_crowned() {
    let mut fails = Vec::new();
    if FORWARD_PLAN_PROMOTED_HEADLINE == FORWARD_PLAN_HEADLINE {
        fails.push("the promoted headline equals the crowned one".to_string());
    }
    for phrase in ["tuned", "not the bake-off"] {
        if !FORWARD_PLAN_PROMOTED_CAPTION.contains(phrase) {
            fails.push(format!("the promoted caption must say {phrase:?}"));
        }
    }
    for phrase in ["crowned strategy", "confidence check on that pick"] {
        if FORWARD_PLAN_PROMOTED_CAPTION.contains(phrase) {
            fails.push(format!("the promoted caption must not say {phrase:?}"));
        }
        if !FORWARD_PLAN_CAPTION.contains(phrase) {
            fails.push(format!(
                "control: the crowned caption says {phrase:?}, so the check can fail"
            ));
        }
    }
    if FORWARD_PLAN_LOADING.contains("crowned") {
        fails.push(format!(
            "the loading line must name no provenance, got {FORWARD_PLAN_LOADING:?}"
        ));
    }
    for (key, value) in [
        (
            "FORWARD_PLAN_PROMOTED_HEADLINE",
            FORWARD_PLAN_PROMOTED_HEADLINE,
        ),
        (
            "FORWARD_PLAN_PROMOTED_CAPTION",
            FORWARD_PLAN_PROMOTED_CAPTION,
        ),
    ] {
        if !ui::strings::all().contains(&(key, value)) {
            fails.push(format!("{key} must be registered in strings::all()"));
        }
    }
    report("W1", &fails);
}
