//! advisor-param-promotion (ADR-0070 § D4): the `PromoteSweptConfig` reducer
//! arm, driven through `ui::state::update` (the LOAD-BEARING wiring proof).
//!
//! ## Why this file is the wiring proof (not the render PNG)
//!
//! Per ADR-0070 § T12 + the CLAUDE.md v3-vol-overlay-noop precedent: the
//! render-pixel proof shows the "Use this config" affordance paints as an enabled
//! accent button — but a button that paints accent yet drops its message on the
//! floor would STILL pass that pixel guard (the old visual-only pill looked
//! identical). The proof that the click actually CARRIES the tuned config forward
//! is THIS pure-state test: driving `ui::state::update` with
//! `Message::PromoteSweptConfig(params)` and asserting the model preseeds the
//! forward-launch target + navigates. RED before the message/handler existed
//! (the affordance was a visual `Container` pill with no `on_press`); GREEN after.
//! Reverting the handler — or weakening it to a no-op — breaks these.
//!
//! The binary-layer `ForwardCommand::Launch` dispatch off
//! `pending_forward_promotion` is exercised at integration; here we pin the PURE
//! half (the preseed + the nav), which is the contract the binary reads.
//!
//! ## How state is driven (story 2-63 Wave D)
//!
//! Every test starts from `Cockpit::new()` and reaches the moment "Use this
//! config" is pressed the way the operator does, through `update` only: the
//! "Tune…" drill-down (`OpenTuneEditor`, which preseeds the family, the coin
//! and the lookback, and navigates to Tune; today its one emitter is the Lab's
//! "Tune…" button, `ui::screens::lab::tune_entry_button`), Run
//! (`SweepRunRequested`) and the sweep's completion (`SweepRunCompleted`).
//! Some tests then change something under the landed grid before the press:
//! the leaderboard's coin (`SwitchScreen` + `BakeoffSelectCoin`), the family
//! picker (`SweepSelectFamily`), the lookback (a second `OpenTuneEditor`), or
//! the Tune coin (a second `OpenTuneEditor` on another coin, after the grid
//! lands or while the sweep is in flight). No model field is written directly,
//! and every step's effect is asserted (S0 collects its checks and fails once
//! with the whole list; of each `OpenTuneEditor` it checks the Tune coin, not
//! the navigation).
//! The crowned plan of the first test is handed to `update` as the
//! `ForwardPlanReceived` the agent's relay yields. The relay seam
//! (`ui::live::forward_plan_stream_impl`) exists only under `live`, and this
//! file carries no `cfg` gate; `advisor_recipe_state_gating.rs` drives the same
//! promotion with the plan relayed over a real channel.
//!
//! ## What each field is pinned against
//!
//! Every preseeded field is tested on both sides of its dimension, and its
//! expected value comes from the inputs, never from the cockpit:
//!
//! - **coin.** Production reads the landed grid's coin, and the Tune coin
//!   (`model.tune_coin`) only with no grid (bug-log #200, ADR-0070 § D10;
//!   `ui::state::promote_swept_config`:
//!   `.map_or_else(|| model.tune_coin.clone(), Symbol::new);`). In
//!   `promote_swept_config_preseeds_target_and_navigates` two cases swap two
//!   coins between the Tune screen and the leaderboard's next bake-off; neither
//!   is the cold default, and the grid was swept on the Tune coin (D-P17,
//!   D-P18). `promote_swept_config_carries_the_landed_grids_coin_not_the_tune_coin`
//!   moves the Tune coin away from the landed grid's and tells the two apart.
//! - **strategy id.** Four families, four different ids, with the family
//!   picker on the promoted family. In a second test the picker is moved to
//!   another family after the grid lands, and the id must still follow the
//!   promoted params (D-P25).
//! - **window label.** Two `Ready` grids with different windows each carry
//!   their own. A grid scored on 2024 H1 still carries "2024 H1" after Tune is
//!   re-opened on 2024 H2, so the label is the landed grid's and not the Tune
//!   lookback's (D-P24). A failed sweep and a sweep that scored no cells carry
//!   the fallback phrase (D-P19).
//! - **the plan.** A crowned plan is on screen before the promotion, so its
//!   flip to `Loading` can fail (D-P15).
//! - **the screen.** The promotion starts on Tune and must end on the forward
//!   plan.
//!
//! Two preconditions read a cold default, by necessity:
//!
//! - "nothing is promoted yet", before the press. `update` has one writer of
//!   `pending_forward_promotion` (`ui::state::promote_swept_config`) and
//!   nothing in it clears the field (the binary does, in `cockpit_live.rs`), so
//!   no message sequence can put a promotion there and take it away again. The
//!   check fails only if a message on the way to the press writes one, which is
//!   what it is for: it proves the promotion checked afterwards came from the
//!   press.
//! - the SMA family after `OpenTuneEditor` on SMA. SMA is the default family
//!   of `TuneScreenState`; the MACD, RSI and Bollinger cases do move it.
//!
//! ## bug-log #200 (was the known residual R2-X3, story 2-63 Wave D round 3)
//!
//! "Tune…" (`OpenTuneEditor`) moves `model.tune_coin` for the NEXT sweep and
//! leaves `tune_screen_state.result` alone (K1, ADR-0070 § D10), and a sweep in
//! flight lands its grid after the Tune coin moved. Before the fix the promotion
//! took `model.tune_coin`, so an ETHUSDT grid's params and window could be
//! paper-traded on SOLUSDT. It now takes the landed grid's coin
//! (`TuneScreenState::landed_coin`), the Tune coin only with no grid, and the
//! forward plan's provenance strip names it (ADR-0070 § D6 `{coin}`).
//!
//! Tests: S0 is `promote_swept_config_carries_the_landed_grids_coin_not_the_tune_coin`
//! (here); U1 is `tune::screen_state::tests::landed_coin_is_the_ready_grids_coin_and_none_without_a_grid`
//! and U2 `screens::forward_plan::tests::promote_provenance_line_names_the_promotions_coin`
//! (`--lib`); LR-200 is `param_sweep_render.rs::tune_reopened_on_another_coin_footer_names_only_the_landed_grids_coin`
//! (macOS only); LR-200b is `forward_plan_populated_render.rs::forward_plan_promoted_strip_names_the_landed_grids_coin`
//! (macOS only), whose numbers are its asserts (1)–(3). T1–T4 are this file's
//! other four tests; T5 is `advisor_recipe_state_gating.rs::relayed_plan_resolves_promotion_loading_to_ready`.
//!
//! T-T4 probes for `#200`: one edit to production code per row, except P6
//! (P1's edit plus its own), located by symbol and quoted line (leading spaces
//! belong to OLD; `\|` is `|`). The procedure is
//! `advisor_recipe_state_gating.rs`'s (snapshot, apply, run, restore, `touch`,
//! re-run until `Compiling ui` and the SHA-256 matches), but with this table's
//! command, because several rows go red outside that file's two targets:
//!
//! `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.sdk cargo test -p ui --no-fail-fast --lib --test promote_swept_config --test advisor_recipe_state_gating --test param_sweep_render --test forward_plan_populated_render`
//!
//! Each row's "targets" column is the smallest set that shows its RED and its
//! GREEN; the two render targets compile to nothing off macOS. The RED and
//! GREEN sets are derived from the code. Every test a row's targets run that
//! its RED cell does not name is expected GREEN, whether or not its GREEN cell
//! names it. The runs and the figures they measured are recorded in bug-log
//! `#200`.
//!
//! | # | symbol | OLD → NEW | targets | RED | GREEN |
//! |---|---|---|---|---|---|
//! | P1 | `ui::state::promote_swept_config` | `        .map_or_else(\|\| model.tune_coin.clone(), Symbol::new);` → `        .map_or_else(\|\| model.tune_coin.clone(), \|_\| model.tune_coin.clone());` | `--test promote_swept_config --test advisor_recipe_state_gating --test forward_plan_populated_render` | S0 cases b, c, d (coin); LR-200b (3) | S0 a, e, f; T1–T4; T5; LR-200b (1), (2); the other 9 tests in `advisor_recipe_state_gating.rs`; the other 8 in `forward_plan_populated_render.rs` |
//! | P2 | same | P1's OLD → `        .map_or_else(\|\| Symbol::new(crate::leaderboard::runner::DEFAULT_BAKEOFF_COIN), Symbol::new);` | `--test promote_swept_config` | S0 cases e, f (coin) | S0 a–d; T1–T4 |
//! | P3 | `ui::tune::TuneScreenState::landed_coin` | `            PanelState::Ready(mirror) => Some(mirror.coin.as_str()),` → `            PanelState::Ready(_) => None,` | `--lib --test promote_swept_config --test param_sweep_render --test forward_plan_populated_render` | U1; S0 b, c, d (coin); LR-200 (4); LR-200b (3) | U2; S0 a, e, f; T1–T4; LR-200 (1)–(3); LR-200b (1), (2); the rest of `--lib`; the other 9 in `param_sweep_render.rs`; the other 8 in `forward_plan_populated_render.rs` |
//! | P4 | `ui::screens::tune::view` | `    let coin = st.landed_coin().unwrap_or(model.tune_coin.0.as_str());` → `    let coin = model.tune_coin.0.as_str();` | `--test param_sweep_render` (macOS only) | LR-200 (4) | LR-200 (1)–(3); the file's other 9 tests |
//! | P5 | `ui::tune::TuneScreenState::landed_coin` | `            PanelState::Loading \| PanelState::Empty \| PanelState::Error(_) => None,` → `            PanelState::Loading \| PanelState::Empty \| PanelState::Error(_) => Some("BTCUSDT"),` | `--lib --test promote_swept_config --test param_sweep_render` | U1; S0 e, f (coin); LR-200 (1) | U2; S0 a–d; T1–T4; the other 9 tests in `param_sweep_render.rs` |
//! | P6 (O1) | P1 applied, plus `ui::state::update`, arm `Message::OpenTuneEditor` | `            model.tune_coin = coin;` → `            if model.tune_coin != coin { model.tune_screen_state.result = PanelState::Empty; } model.tune_coin = coin;` | `--test promote_swept_config --test advisor_recipe_state_gating --test param_sweep_render --test forward_plan_populated_render` | S0 b, c, e, f at the K1 `landed` precondition, b, c, d at the coin and b, c at the window (one failure lists them all); LR-200 and LR-200b, each at its K1 precondition | S0 a; T1–T4 (D-P24's re-open keeps the coin); T5; the other 9 tests in `advisor_recipe_state_gating.rs`; the other 9 in `param_sweep_render.rs`; the other 8 in `forward_plan_populated_render.rs` |
//! | P7 | `ui::screens::forward_plan::promote_provenance_line` | `        .replace("{coin}", promotion.coin.0.as_str())` → `        .replace("{coin}", "BTCUSDT")` | `--lib --test forward_plan_populated_render` | U2; LR-200b (2) | U1; LR-200b (1); the file's other 8 tests |
//! | P8 | `ui::strings::TUNE_PROMOTE_CONFIRM_FMT` | `     \u{20ac}200 in {coin}.";` → `     \u{20ac}200.";` | `--lib --test forward_plan_populated_render` | U2; LR-200b (2) | U1; LR-200b (1); the file's other 8 tests |
//!
//! D-P17 and D-P18 (the table in `advisor_recipe_state_gating.rs`) now edit
//! P1's OLD too; with `--test forward_plan_populated_render` added they also
//! turn LR-200b (2) red.
//!
//! The probe ids in the messages (`D-P15`, `D-P17` to `D-P20`, `D-P24`,
//! `D-P25`) are story 2-63 Wave D's. The procedure and their table are in the
//! module doc of `advisor_recipe_state_gating.rs`; P1–P8 are `#200`'s, above.
//!
//! NOT gated on `feature = "fixtures"` (story 2-63 Wave D). `ui::fixtures` is
//! compiled in every build (`crates/ui/src/lib.rs`: "Fixtures module is always
//! compiled"), and nothing below needs `live`. The `#![cfg(feature = "fixtures")]`
//! this file used to carry made it a 0-test target in the DEFAULT build, the one
//! the canonical macOS CI leg runs (`cargo test -p ui`, no `--features`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use smol_str::SmolStr;
use trading_core::Symbol;
use ui::leaderboard::LeaderboardLookback;
use ui::leaderboard::runner::DEFAULT_BAKEOFF_COIN;
use ui::state::{Cockpit, Message, PanelState, Screen, update};
use ui::strings::TUNE_PROMOTE_WINDOW_FALLBACK;
use ui::tune::runner::SweepRunResult;
use ui::tune::{PromoteParams, SweepReportMirror, TuneFamily};
use ui::widgets::bakeoff_input::lookback_copy;

/// The coin the family-map and window-label tests open Tune on. Not the cold
/// default, so the helper's coin precondition is a change `OpenTuneEditor`
/// made.
const COIN: &str = "ETHUSDT";

/// The variant of a panel, as a name. `PanelState` has no `PartialEq`, and a
/// name in the failure message says which state the reducer left behind.
fn kind<T>(panel: &PanelState<T>) -> &'static str {
    match panel {
        PanelState::Loading => "Loading",
        PanelState::Empty => "Empty",
        PanelState::Error(_) => "Error",
        PanelState::Ready(_) => "Ready",
    }
}

/// Whether the Tune result is what `TuneScreenState::finish_run` documents for
/// this outcome: `Empty` for a grid with no cells, `Ready` of the grid
/// otherwise, `Error` of the message.
fn landed(result: &PanelState<SweepReportMirror>, outcome: &SweepRunResult) -> bool {
    match (result, outcome) {
        (PanelState::Empty, Ok(sent)) => sent.cells.is_empty(),
        (PanelState::Ready(landed), Ok(sent)) => !sent.cells.is_empty() && landed == sent,
        (PanelState::Error(landed), Err(sent)) => landed == sent,
        _ => false,
    }
}

/// `grid`, swept on `coin` over the window labelled `window`.
fn swept_on(mut grid: SweepReportMirror, coin: &str, window: &str) -> SweepReportMirror {
    grid.coin = SmolStr::new(coin);
    grid.range_label = SmolStr::new(window);
    grid
}

/// The SMA grid fixture, swept on `coin` over the window labelled `window`.
fn sma_grid(coin: &str, window: &str) -> SweepReportMirror {
    swept_on(ui::fixtures::fake_sweep_report_mirror(), coin, window)
}

/// Take `cockpit` to the moment "Use this config" is pressed, through `update`
/// only:
///
/// 1. `OpenTuneEditor` on `family`, `coin` and `lookback` (the "Tune…"
///    drill-down, which also navigates to Tune);
/// 2. Run, then the sweep's completion with `outcome`.
///
/// Every step's effect is asserted, so a reducer that drops one fails here,
/// named, before the promotion is pressed. `coin` is never the cold default,
/// so the coin precondition can fail. For SMA the family precondition reads the
/// cold default (see the module doc).
fn open_tune_and_sweep(
    cockpit: &mut Cockpit,
    family: TuneFamily,
    coin: &str,
    lookback: LeaderboardLookback,
    outcome: SweepRunResult,
) {
    assert_ne!(
        coin, DEFAULT_BAKEOFF_COIN,
        "fixture: the Tune coin is not the cold default, so the coin precondition can fail"
    );
    update(
        cockpit,
        Message::OpenTuneEditor {
            family,
            coin: Symbol::new(coin),
            lookback,
        },
    );
    assert_eq!(
        cockpit.current_screen,
        Screen::Tune,
        "precondition: the Tune… drill-down navigated to the Tune screen"
    );
    assert_eq!(
        cockpit.tune_screen_state.family, family,
        "precondition: OpenTuneEditor preselected the family"
    );
    assert_eq!(
        cockpit.tune_coin.0.as_str(),
        coin,
        "precondition: OpenTuneEditor preseeded the Tune coin"
    );
    update(cockpit, Message::SweepRunRequested);
    assert!(
        cockpit.tune_screen_state.running,
        "precondition: Run started a {family:?} sweep"
    );
    update(cockpit, Message::SweepRunCompleted(outcome.clone()));
    assert!(
        landed(&cockpit.tune_screen_state.result, &outcome),
        "precondition: the sweep's outcome landed as sent, got {}",
        kind(&cockpit.tune_screen_state.result)
    );
    assert!(
        cockpit.pending_forward_promotion.is_none(),
        "precondition: no message on the way here wrote a promotion (only PromoteSweptConfig writes one)"
    );
}

/// Core proof: `PromoteSweptConfig(SMA)` sets the forward-launch target (which is
/// also the provenance signal), flips the plan to `Loading` and navigates. This
/// is the exact path the now-wired "Use this config" button drives — a no-op
/// before.
///
/// Each case: a crowned plan arrives; the operator opens Tune on one coin and
/// sweeps it; back on the leaderboard they pick the OTHER coin for the next
/// bake-off; on Tune again they press "Use this config". The two cases swap the
/// coins, and neither is the cold default, so a promotion that takes the
/// leaderboard's coin (D-P18) or the default (D-P17) fails.
#[test]
fn promote_swept_config_preseeds_target_and_navigates() {
    let cases = [("ETHUSDT", "SOLUSDT"), ("SOLUSDT", "ETHUSDT")];
    for (tune, board) in cases {
        assert_ne!(tune, board, "fixture: the two coins differ");
        assert_ne!(
            board, DEFAULT_BAKEOFF_COIN,
            "fixture: the leaderboard's next coin is not the cold default"
        );
    }
    let params = PromoteParams::Sma {
        fast_len: 10,
        slow_len: 20,
    };

    for (tune, board) in cases {
        let mut cockpit = Cockpit::new();

        // Seed: the crowned plan is on screen.
        let crowned = ui::fixtures::fake_forward_plan();
        update(&mut cockpit, Message::ForwardPlanReceived(crowned.clone()));
        assert!(
            matches!(&cockpit.forward_plan_screen_state.plan, PanelState::Ready(v) if *v == crowned),
            "[{tune}] seed: the crowned plan is on screen, got {}",
            kind(&cockpit.forward_plan_screen_state.plan)
        );

        // Tune on one coin; then the leaderboard moves on to the other.
        open_tune_and_sweep(
            &mut cockpit,
            TuneFamily::Sma,
            tune,
            LeaderboardLookback::H1_2024,
            Ok(sma_grid(tune, "2024 H1")),
        );
        update(&mut cockpit, Message::SwitchScreen(Screen::Leaderboard));
        assert_eq!(
            cockpit.current_screen,
            Screen::Leaderboard,
            "[{tune}] precondition: the operator went back to the leaderboard"
        );
        update(&mut cockpit, Message::BakeoffSelectCoin(Symbol::new(board)));
        assert_eq!(
            cockpit.leaderboard_screen_state.coin.0.as_str(),
            board,
            "[{tune}] precondition: the leaderboard's next bake-off targets {board}"
        );
        update(&mut cockpit, Message::SwitchScreen(Screen::Tune));
        assert_eq!(
            cockpit.current_screen,
            Screen::Tune,
            "[{tune}] precondition: the operator is back on the Tune screen"
        );
        assert_eq!(
            cockpit.tune_coin.0.as_str(),
            tune,
            "[{tune}] precondition: the Tune coin, model.tune_coin, is still {tune} \
             (picking the leaderboard's coin did not move it)"
        );

        update(&mut cockpit, Message::PromoteSweptConfig(params.clone()));

        // (a) Navigated to the forward plan (the operator reviews, then their
        // €200 begins).
        assert_eq!(
            cockpit.current_screen,
            Screen::ForwardPlan,
            "[{tune}] PromoteSweptConfig must navigate from Tune to the forward-plan screen"
        );
        // (b)+(c) The forward-launch target is set — the binary launches from it
        // AND the forward-plan view renders the "you tuned this" provenance
        // header from its Some-ness (one field is the launch target AND the
        // provenance signal).
        let promotion = cockpit
            .pending_forward_promotion
            .as_ref()
            .unwrap_or_else(|| {
                panic!("[{tune}] PromoteSweptConfig must set pending_forward_promotion")
            });
        // The structured tuned params carry through verbatim (the
        // self-contained message — not a mirror re-lookup).
        assert_eq!(
            promotion.params, params,
            "[{tune}] the preseeded promotion must carry the exact tuned params"
        );
        // SMA → the StrategyId the forward resolvers dispatch on.
        assert_eq!(
            promotion.strategy_id.0.as_str(),
            "v0.5.sma",
            "[{tune}] SMA promotion must resolve to the v0.5.sma forward id"
        );
        // The landed grid's coin, here also the Tune coin: the grid was swept on
        // it (the #200 test below tells the two apart).
        assert_eq!(
            promotion.coin.0.as_str(),
            tune,
            "[{tune}] D-P17 / D-P18: the promotion must carry the grid's coin, the Tune coin \
             ({tune}), not the leaderboard's next pick ({board}) or the cold default \
             ({DEFAULT_BAKEOFF_COIN})"
        );
        // (d) The plan flips to Loading so the operator never sees the stale
        // crowned plan while the agent resolves the tuned one.
        assert_eq!(
            kind(&cockpit.forward_plan_screen_state.plan),
            "Loading",
            "[{tune}] D-P15: the promotion must flip the crowned plan to Loading \
             (no stale crowned plan while the tuned one resolves)"
        );
    }
}

/// The family → forward `StrategyId` map is exhaustive and correct for every
/// family (the SAME ids `build_registry_for` matches). A wrong id would launch the
/// wrong (or no) strategy.
///
/// Each case opens Tune on its own family and lands a `Ready` grid before the
/// press, and the family picker stays on that family. The grid is the family's
/// own fixture where one exists (SMA, MACD) and the SMA one otherwise: the
/// promotion reads only the grid's window label.
/// `promote_swept_config_strategy_id_follows_the_params_not_the_family_picker`
/// is the other side: the picker moved off the promoted family.
#[test]
fn promote_swept_config_maps_every_family_to_its_forward_id() {
    let cases = [
        (
            TuneFamily::Sma,
            PromoteParams::Sma {
                fast_len: 7,
                slow_len: 14,
            },
            "v0.5.sma",
            ui::fixtures::fake_sweep_report_mirror(),
        ),
        (
            TuneFamily::Macd,
            PromoteParams::Macd {
                fast: 8,
                slow: 20,
                signal: 5,
            },
            "v0.5.macd",
            ui::fixtures::fake_sweep_report_mirror_macd(),
        ),
        (
            TuneFamily::Rsi,
            PromoteParams::Rsi {
                period: 10,
                oversold: 25,
            },
            "v0.5.rsi",
            ui::fixtures::fake_sweep_report_mirror(),
        ),
        (
            TuneFamily::Bollinger,
            PromoteParams::Bollinger {
                period: 20,
                k_tenths: 25,
            },
            "v0.5.bbands",
            ui::fixtures::fake_sweep_report_mirror(),
        ),
    ];

    for (family, params, expected_id, grid) in cases {
        let mut cockpit = Cockpit::new();
        open_tune_and_sweep(
            &mut cockpit,
            family,
            COIN,
            LeaderboardLookback::H1_2024,
            Ok(swept_on(grid, COIN, "2024 H1")),
        );

        update(&mut cockpit, Message::PromoteSweptConfig(params.clone()));

        let promotion = cockpit
            .pending_forward_promotion
            .expect("every family must preseed a promotion");
        assert_eq!(
            promotion.strategy_id.0.as_str(),
            expected_id,
            "{params:?} must map to the {expected_id} forward id"
        );
        assert_eq!(
            promotion.params, params,
            "params must carry through verbatim"
        );
    }
}

/// The strategy id follows the promoted params, not the family picker. The
/// view's "Use this config" sends the landed row's own params
/// (`ui::screens::tune`: `.on_press(Message::PromoteSweptConfig(cell.promote_params.clone()))`),
/// and the family picker leaves the result on screen (`ui::state::update`, arm
/// `Message::SweepSelectFamily`: "We do NOT clear the existing result"). So a
/// grid swept for one family stays on screen, with its buttons, while the
/// picker shows another.
///
/// Each case sweeps one family, moves the picker to another, and presses with
/// the params of the grid's first promotable row. The id must stay the swept
/// family's (D-P25).
#[test]
fn promote_swept_config_strategy_id_follows_the_params_not_the_family_picker() {
    let cases = [
        (
            TuneFamily::Sma,
            ui::fixtures::fake_sweep_report_mirror(),
            TuneFamily::Rsi,
            "v0.5.sma",
        ),
        (
            TuneFamily::Macd,
            ui::fixtures::fake_sweep_report_mirror_macd(),
            TuneFamily::Bollinger,
            "v0.5.macd",
        ),
    ];

    for (swept, grid, picked, expected_id) in cases {
        assert_ne!(swept, picked, "fixture: the picker moves to another family");
        let grid = swept_on(grid, COIN, "2024 H1");
        let params = grid
            .cells
            .iter()
            .find(|cell| cell.promotable)
            .map(|cell| cell.promote_params.clone())
            .expect("fixture: the grid has a promotable row (the only rows with the button)");
        assert!(
            matches!(
                (swept, &params),
                (TuneFamily::Sma, PromoteParams::Sma { .. })
                    | (TuneFamily::Macd, PromoteParams::Macd { .. })
            ),
            "fixture: the row's params are the swept family's, got {params:?}"
        );

        let mut cockpit = Cockpit::new();
        open_tune_and_sweep(
            &mut cockpit,
            swept,
            COIN,
            LeaderboardLookback::H1_2024,
            Ok(grid.clone()),
        );
        update(&mut cockpit, Message::SweepSelectFamily(picked));
        assert_eq!(
            cockpit.tune_screen_state.family, picked,
            "[{swept:?}] precondition: the picker moved to {picked:?}"
        );
        assert!(
            matches!(&cockpit.tune_screen_state.result, PanelState::Ready(m) if *m == grid),
            "[{swept:?}] precondition: the {swept:?} grid is still on screen, got {}",
            kind(&cockpit.tune_screen_state.result)
        );

        update(&mut cockpit, Message::PromoteSweptConfig(params.clone()));

        let promotion = cockpit
            .pending_forward_promotion
            .as_ref()
            .unwrap_or_else(|| panic!("[{swept:?}] the promotion was preseeded"));
        assert_eq!(
            promotion.strategy_id.0.as_str(),
            expected_id,
            "[{swept:?} grid, picker on {picked:?}] D-P25: the strategy id must follow the \
             promoted params ({params:?}), not the family picker"
        );
        assert_eq!(
            promotion.params, params,
            "[{swept:?}] the params carry through verbatim"
        );
    }
}

/// The window label is carried from the `Ready` sweep mirror (the "robust on
/// THIS window" honesty copy reads it). Pins that the promotion names the window
/// the gate actually scored, not a default and not the Tune lookback, on both
/// sides of `TuneScreenState::range_label_or_default`:
///
/// | result on Tune | Tune lookback at the press | the promotion's window |
/// |---|---|---|
/// | `Ready`, a grid scored on 2024 H1 | 2024 H1 | "2024 H1" |
/// | `Ready`, a grid scored on 2024 H2 | 2024 H2 | "2024 H2" |
/// | `Ready`, a grid scored on 2024 H1 | 2024 H2 (Tune re-opened) | "2024 H1" |
/// | `Error` (the sweep failed) | 2024 H1 | the fallback phrase |
/// | `Empty` (the sweep scored no cells) | 2024 H1 | the fallback phrase |
///
/// The third row is the one where the lookback and the landed grid name
/// different windows: `OpenTuneEditor` moves the lookback and leaves the result
/// on screen. Today's one emitter of `OpenTuneEditor` (the Lab's "Tune…",
/// `ui::screens::lab::tune_entry_button`) always sends `H1_2024`, so the GUI
/// does not reach that row. `update` does, and the row pins the reducer for any
/// emitter that sends another lookback.
///
/// The view offers "Use this config" only on a `Ready` grid row, so the last
/// two rows are the reducer's defensive branch. The reducer does not gate on
/// the result, so `update` reaches that branch.
#[test]
fn promote_swept_config_carries_the_swept_window_label() {
    /// One row of the table above.
    struct Case {
        name: &'static str,
        lookback: LeaderboardLookback,
        outcome: SweepRunResult,
        reopen_on: Option<LeaderboardLookback>,
        want_window: &'static str,
        probe: &'static str,
    }
    let mut no_cells = sma_grid(COIN, "2024 H1");
    no_cells.cells.clear();
    let cases = [
        Case {
            name: "a 2024 H1 grid",
            lookback: LeaderboardLookback::H1_2024,
            outcome: Ok(sma_grid(COIN, "2024 H1")),
            reopen_on: None,
            want_window: "2024 H1",
            probe: "the Ready side",
        },
        Case {
            name: "a 2024 H2 grid",
            lookback: LeaderboardLookback::H2_2024,
            outcome: Ok(sma_grid(COIN, "2024 H2")),
            reopen_on: None,
            want_window: "2024 H2",
            probe: "D-P20",
        },
        Case {
            name: "a 2024 H1 grid, Tune re-opened on 2024 H2",
            lookback: LeaderboardLookback::H1_2024,
            outcome: Ok(sma_grid(COIN, "2024 H1")),
            reopen_on: Some(LeaderboardLookback::H2_2024),
            want_window: "2024 H1",
            probe: "D-P24",
        },
        Case {
            name: "a failed sweep",
            lookback: LeaderboardLookback::H1_2024,
            outcome: Err(SmolStr::new("boom")),
            reopen_on: None,
            want_window: TUNE_PROMOTE_WINDOW_FALLBACK,
            probe: "D-P19",
        },
        Case {
            name: "a sweep that scored no cells",
            lookback: LeaderboardLookback::H1_2024,
            outcome: Ok(no_cells),
            reopen_on: None,
            want_window: TUNE_PROMOTE_WINDOW_FALLBACK,
            probe: "D-P19",
        },
    ];
    assert!(
        !TUNE_PROMOTE_WINDOW_FALLBACK.is_empty()
            && TUNE_PROMOTE_WINDOW_FALLBACK != "2024 H1"
            && TUNE_PROMOTE_WINDOW_FALLBACK != "2024 H2",
        "fixture: the fallback phrase is neither grid's window, nor empty"
    );

    for Case {
        name,
        lookback,
        outcome,
        reopen_on,
        want_window,
        probe,
    } in cases
    {
        let mut cockpit = Cockpit::new();
        open_tune_and_sweep(
            &mut cockpit,
            TuneFamily::Sma,
            COIN,
            lookback,
            outcome.clone(),
        );
        if let Some(reopen_on) = reopen_on {
            update(
                &mut cockpit,
                Message::OpenTuneEditor {
                    family: TuneFamily::Sma,
                    coin: Symbol::new(COIN),
                    lookback: reopen_on,
                },
            );
            assert_eq!(
                cockpit.tune_lookback, reopen_on,
                "[{name}] precondition: the second Tune… moved the Tune lookback"
            );
            assert!(
                landed(&cockpit.tune_screen_state.result, &outcome),
                "[{name}] precondition: the second Tune… left the landed grid on screen, got {}",
                kind(&cockpit.tune_screen_state.result)
            );
            assert_ne!(
                lookback_copy(cockpit.tune_lookback),
                want_window,
                "[{name}] precondition: the Tune lookback now names another window than \
                 the landed grid"
            );
        }

        update(
            &mut cockpit,
            Message::PromoteSweptConfig(PromoteParams::Sma {
                fast_len: 10,
                slow_len: 20,
            }),
        );

        let promotion = cockpit
            .pending_forward_promotion
            .unwrap_or_else(|| panic!("[{name}] the promotion was preseeded"));
        assert_eq!(
            promotion.window_label.as_str(),
            want_window,
            "[{name}] {probe}: the promotion must carry the window the landed grid scored \
             (for the honesty copy), or the fallback phrase when no grid landed"
        );
    }
}

/// Record `what` under case `case` unless `holds`. S0 collects every check,
/// preconditions included, so one run names every case a probe turns red.
fn note(failures: &mut Vec<String>, case: &str, holds: bool, what: &str) {
    if !holds {
        failures.push(format!("[{case}] {what}"));
    }
}

/// bug-log #200 (ADR-0070 § D10, S0): the promotion carries the coin the
/// LANDED grid was swept on. "Tune…" moves the Tune coin for the NEXT sweep and
/// leaves the landed grid on screen (K1), and a sweep in flight lands its grid
/// after the Tune coin moved. With no grid the Tune coin is the fallback.
///
/// | case | swept on | outcome | "Tune…" again on | when | expected coin |
/// |---|---|---|---|---|---|
/// | a | ETHUSDT | `Ok` | ETHUSDT | after the grid landed | ETHUSDT |
/// | b | ETHUSDT | `Ok` | SOLUSDT | after the grid landed | ETHUSDT |
/// | c | SOLUSDT | `Ok` | ETHUSDT | after the grid landed | SOLUSDT |
/// | d | ETHUSDT | `Ok` | SOLUSDT | after Run, before the sweep completed | ETHUSDT |
/// | e | ETHUSDT | `Err` | SOLUSDT | after the sweep failed | SOLUSDT |
/// | f | SOLUSDT | `Err` | ETHUSDT | after the sweep failed | ETHUSDT |
///
/// The expected coin and window come from the row: the swept coin and
/// "2024 H1" on `Ok`, else the re-open coin and the fallback phrase. Two-sided
/// pairs: a/c (one re-open coin, two grid coins), e/f (no grid, two Tune
/// coins), b/e (one path, `Ok` vs `Err`). Case d cannot use
/// `open_tune_and_sweep`, which completes the sweep before returning.
#[test]
fn promote_swept_config_carries_the_landed_grids_coin_not_the_tune_coin() {
    /// One row of the table above.
    struct Case {
        name: &'static str,
        swept: &'static str,
        ok: bool,
        reopen: &'static str,
        mid_sweep: bool,
    }
    let cases = [
        Case {
            name: "a",
            swept: "ETHUSDT",
            ok: true,
            reopen: "ETHUSDT",
            mid_sweep: false,
        },
        Case {
            name: "b",
            swept: "ETHUSDT",
            ok: true,
            reopen: "SOLUSDT",
            mid_sweep: false,
        },
        Case {
            name: "c",
            swept: "SOLUSDT",
            ok: true,
            reopen: "ETHUSDT",
            mid_sweep: false,
        },
        Case {
            name: "d",
            swept: "ETHUSDT",
            ok: true,
            reopen: "SOLUSDT",
            mid_sweep: true,
        },
        Case {
            name: "e",
            swept: "ETHUSDT",
            ok: false,
            reopen: "SOLUSDT",
            mid_sweep: false,
        },
        Case {
            name: "f",
            swept: "SOLUSDT",
            ok: false,
            reopen: "ETHUSDT",
            mid_sweep: false,
        },
    ];
    let params = PromoteParams::Sma {
        fast_len: 10,
        slow_len: 20,
    };
    let open = |cockpit: &mut Cockpit, coin: &str| {
        update(
            cockpit,
            Message::OpenTuneEditor {
                family: TuneFamily::Sma,
                coin: Symbol::new(coin),
                lookback: LeaderboardLookback::H1_2024,
            },
        );
    };

    let mut failures: Vec<String> = Vec::new();
    for Case {
        name,
        swept,
        ok,
        reopen,
        mid_sweep,
    } in cases
    {
        for coin in [swept, reopen] {
            assert_ne!(
                coin, DEFAULT_BAKEOFF_COIN,
                "fixture: [{name}] no coin is the cold default, so a fallback to it fails"
            );
        }
        let outcome: SweepRunResult = if ok {
            Ok(sma_grid(swept, "2024 H1"))
        } else {
            Err(SmolStr::new("boom"))
        };
        let (want_coin, want_window) = if ok {
            (swept, "2024 H1")
        } else {
            (reopen, TUNE_PROMOTE_WINDOW_FALLBACK)
        };

        let mut cockpit = Cockpit::new();
        open(&mut cockpit, swept);
        note(
            &mut failures,
            name,
            cockpit.tune_coin.0.as_str() == swept,
            "precondition: the first Tune… preseeded the swept coin",
        );
        update(&mut cockpit, Message::SweepRunRequested);
        note(
            &mut failures,
            name,
            cockpit.tune_screen_state.running,
            "precondition: Run started the sweep",
        );
        if mid_sweep {
            open(&mut cockpit, reopen);
            note(
                &mut failures,
                name,
                cockpit.tune_screen_state.running,
                "precondition: the second Tune… came while the sweep was in flight",
            );
            update(&mut cockpit, Message::SweepRunCompleted(outcome.clone()));
        } else {
            update(&mut cockpit, Message::SweepRunCompleted(outcome.clone()));
            open(&mut cockpit, reopen);
        }
        note(
            &mut failures,
            name,
            cockpit.tune_coin.0.as_str() == reopen,
            "precondition: the second Tune… moved the Tune coin to the re-open coin",
        );
        let on_screen = kind(&cockpit.tune_screen_state.result);
        note(
            &mut failures,
            name,
            landed(&cockpit.tune_screen_state.result, &outcome),
            &format!(
                "precondition (K1): the sweep's outcome is on screen after both, got {on_screen}"
            ),
        );

        update(&mut cockpit, Message::PromoteSweptConfig(params.clone()));

        let Some(promotion) = cockpit.pending_forward_promotion.as_ref() else {
            note(
                &mut failures,
                name,
                false,
                "PromoteSweptConfig stored no promotion",
            );
            continue;
        };
        note(
            &mut failures,
            name,
            promotion.coin.0.as_str() == want_coin,
            &format!(
                "#200: the promotion must carry the coin the landed grid was swept on (the \
                 Tune coin only with no grid): want {want_coin}, got {}",
                promotion.coin.0
            ),
        );
        note(
            &mut failures,
            name,
            promotion.window_label.as_str() == want_window,
            &format!(
                "the window comes off the same grid: want {want_window:?}, got {:?}",
                promotion.window_label
            ),
        );
    }
    assert!(
        failures.is_empty(),
        "{} check(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
