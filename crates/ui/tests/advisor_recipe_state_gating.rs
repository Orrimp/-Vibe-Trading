//! Story 2-63 (lab-recipe-test-harness v0.3), Wave D: the `update()`-driven
//! STATE MACHINE behind four post-v0.2 recipes (S2). `ForwardPlanRecipe`,
//! `NarrationOutcomeRecipe`, `BakeoffProgressRecipe` and `SweepProgressRecipe`
//! each relay into a reducer arm that GATES or CLEARS state.
//!
//! ## What this file pins, and what it leaves to its siblings
//!
//! Wave A's relay files (`forward_narration_relay.rs`,
//! `bakeoff_progress_relay.rs`, `sweep_progress_relay.rs`) prove that a channel
//! event becomes the right `Message` and that one `update` lands it. They do
//! not pin the reducer's guards: which arm refuses a message, which arm clears
//! what, and in which prior state. A refactor that "simplifies" one of these
//! guards changes what the operator sees and leaves every relay test green:
//!
//! - **§1 forward plan.** `ForwardPlanReceived` lands on whatever plan is on
//!   screen: here the cold `Empty`, the `Loading` a promotion sets, and a
//!   `Ready` plan already there. It leaves the promotion itself in place: the
//!   forward plan's "you tuned this" strip reads it over the tuned plan.
//! - **§2 narration.** "Explain" is refused unless a `Ready` bake-off result is
//!   on screen, and a re-press never clobbers a narration that has resolved
//!   (`Ready` or `FellBack`, the two resolved states of `NarrationState`).
//! - **§3 bake-off bar.** A run clears the previous run's stale bar when it
//!   starts and its own bar when it completes; a second Run press mid-run keeps
//!   the live bar and the run.
//! - **§4 sweep bar.** The same three transitions on the Tune screen, whose Run
//!   guard is `TuneScreenState::can_run` (not running AND a runnable form).
//!
//! Two pins are shared with Wave A, on purpose. D-P5 and D-P9 are the same
//! edits as Wave A's P9 (`bakeoff_progress_relay.rs`) and P24
//! (`sweep_progress_relay.rs`): the clear in `begin_run`. Wave A seeds the stale
//! bar by writing it into the field directly; its P9 then calls `begin_run()`
//! directly too, its P24 presses Run through `update`. Here the stale bar is
//! the one a late event leaves, and the clear is reached through `update`.
//!
//! The promotion's own preseed (target, coin, window label, the flip to
//! `Loading`, the navigation) is pinned in `promote_swept_config.rs`, which
//! carries no `cfg` gate. Its probes are rows of the table below, and its
//! module doc names a known residual (which coin a promotion carries once the
//! Tune coin has moved under a landed grid) that no test here pins.
//!
//! ## How state is driven
//!
//! Every test starts from `Cockpit::new()` and changes state ONLY through
//! `ui::state::update`, the real reducer. No model field is written directly.
//! The two messages the AGENT relays come out of their pub seams over a real
//! channel, seeds included: `ForwardPlanReceived` out of
//! `forward_plan_stream_impl`, `BakeoffNarrationCompleted` out of
//! `narration_outcome_stream_impl`. Progress events and run completions are
//! handed to `update` as the messages they become. Their relays are pinned in
//! Wave A, and a completion comes from the binary's spawned task, which has no
//! seam.
//!
//! ## Non-vacuity contract (bug-log #66 / #161 / story 2-63 AC3)
//!
//! - **Every "cleared" or "kept" precondition is SEEDED through `update`, so it
//!   can fail.** The stale bar is a LATE progress event that arrives after its
//!   run completed. Production does not order the two: progress comes from the
//!   recipe, completion from the run's task. The stale narration is a prior
//!   run's resolved `Ready`. The stale plan is a crowned plan already on screen.
//!   Seed asserts say `seed:` in their message.
//! - **Every guard is tested on both sides of the dimension it guards.** The
//!   two sides differ in that dimension and expect different outcomes. They
//!   also differ in whatever `update` couples to that dimension in every state
//!   it can reach: a `Loading` bake-off result is always a running one, and a
//!   run that is over shows the result it ended with. The pairs: a `Loading` /
//!   `Error` / `Empty` result refuses Explain where `Ready` accepts it; a
//!   resolved narration is kept where `NotRequested` flips to `InFlight`; a
//!   press mid-run keeps the bar and the run where an idle press clears the bar
//!   and starts one; an idle press on an unrunnable sweep form is refused where
//!   a runnable one starts. `set_plan` is unconditional, so its pair (an
//!   `Empty` or a `Ready` plan before the relay) expects the same outcome on
//!   both sides, by design.
//! - **Expected values are derived from the inputs, never read off the
//!   subject:** `ForwardPlanView::from_plan(&plan)` for a relayed plan, the
//!   fixture mirror itself for a landed result, the params and grid sent for a
//!   promotion.
//! - **Every relay is checked:** a send that fails, a relay that times out,
//!   and a relay that ends instead of yielding all panic.
//! - **A limit, stated.** The seeds depend on `Message::BakeoffProgress` and
//!   `Message::SweepProgress` storing an event whether or not a run is in
//!   flight. If production starts dropping late events, the `seed:` asserts
//!   fail. The clear-on-start would then be unreachable through `update`, and
//!   these tests must be rewritten, not weakened. The doc comment on
//!   `ui::leaderboard::LeaderboardScreenState::set_progress` says the event is
//!   "ignored after the run completes". That is false today: the method stores
//!   unconditionally, and the `seed:` asserts measure it. The comment, not this
//!   premise, is what is wrong.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4; Wave D ids `D-P<n>`)
//!
//! One edit each to production code, located by symbol and the quoted line
//! text (line numbers rot). Procedure:
//!
//! 1. snapshot the file (`cp -p <file> <snap>`) and apply the edit;
//! 2. run `cargo test -p ui --no-fail-fast --test advisor_recipe_state_gating
//!    --test promote_swept_config`. Run both targets: several rows are red in
//!    `promote_swept_config.rs`. Pass `--no-fail-fast`: without it cargo stops
//!    at the first target that fails and never runs the second. The named test
//!    goes RED at the assertion whose message CONTAINS the probe id (many
//!    messages lead with a case label such as `[Error]`), or at the `seed:` /
//!    `precondition:` named in the row;
//! 3. restore from the snapshot, THEN `touch <file>`. A restore that keeps the
//!    snapshot's older mtime (`cp -p`) leaves cargo's fingerprint "fresh": the
//!    next run compiles nothing and re-executes the MUTANT binary;
//! 4. re-run. The GREEN counts only if that run's output shows `Compiling ui`
//!    and the file's SHA-256 equals the pre-probe one.
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | D-P1 | `ui::state::update`, arm `Message::ForwardPlanReceived` (`crates/ui/src/state.rs`) | `model.forward_plan_screen_state.set_plan(view);` | wrap in `if !matches!(model.forward_plan_screen_state.plan, PanelState::Loading) { … }` | `relayed_plan_resolves_promotion_loading_to_ready` (the tuned plan never resolves the promotion's `Loading`) |
//! | D-P2 | same arm | `model.forward_plan_screen_state.set_plan(view);` | wrap in `if !matches!(model.forward_plan_screen_state.plan, PanelState::Ready(_)) { … }` | `relayed_second_plan_replaces_the_first` (plan A stays on screen). `relayed_plan_resolves_promotion_loading_to_ready` stays green: it relays only onto `Empty` and `Loading` |
//! | D-P16 | same arm | `model.forward_plan_screen_state.set_plan(view);` | insert `model.pending_forward_promotion = None;` on the line before it (the promotion "consumed" once its plan arrives) | `relayed_plan_resolves_promotion_loading_to_ready`, at its `D-P16` check (the tuned plan lands, but the promotion the provenance strip reads is gone) |
//! | D-P15 | `ui::state::promote_swept_config` (private; `crates/ui/src/state.rs`) | `model.forward_plan_screen_state.plan = PanelState::Loading;` | delete the line | `relayed_plan_resolves_promotion_loading_to_ready`, at its `precondition:` that the promotion flipped the seeded crowned plan to `Loading`. Also `promote_swept_config.rs::promote_swept_config_preseeds_target_and_navigates`, at its `D-P15` check (its seeded crowned plan stays `Ready`) |
//! | D-P17 | `ui::state::promote_swept_config` | `coin: model.tune_coin.clone(),` | → `coin: Symbol::new(crate::leaderboard::runner::DEFAULT_BAKEOFF_COIN),` | `promote_swept_config.rs::promote_swept_config_preseeds_target_and_navigates`, at its `D-P17 / D-P18` check, first case (the cold default is neither coin) |
//! | D-P18 | `ui::state::promote_swept_config` | `coin: model.tune_coin.clone(),` | → `coin: model.leaderboard_screen_state.coin.clone(),` | `promote_swept_config.rs::promote_swept_config_preseeds_target_and_navigates`, at its `D-P17 / D-P18` check, first case (the leaderboard has moved on to the other coin) |
//! | D-P19 | `ui::tune::TuneScreenState::range_label_or_default` (`crates/ui/src/tune/screen_state.rs`) | `_ => SmolStr::new(crate::strings::TUNE_PROMOTE_WINDOW_FALLBACK),` | → `_ => SmolStr::default(),` | `promote_swept_config.rs::promote_swept_config_carries_the_swept_window_label`, at its `D-P19` check, first non-`Ready` case (a failed sweep). Its three `Ready` cases pass |
//! | D-P20 | `ui::tune::TuneScreenState::range_label_or_default` | `PanelState::Ready(mirror) => mirror.range_label.clone(),` | → `PanelState::Ready(_) => SmolStr::new("2024 H1"),` | `promote_swept_config.rs::promote_swept_config_carries_the_swept_window_label`, at its `D-P20` check (the `2024 H2` grid). Also `relayed_plan_resolves_promotion_loading_to_ready`, at its `precondition:` that the promotion names the grid's window |
//! | D-P24 | `ui::state::promote_swept_config` | `let window_label = model.tune_screen_state.range_label_or_default();` | → on a `Ready` result, the Tune lookback's chip copy: `let window_label = match &model.tune_screen_state.result { PanelState::Ready(_) => SmolStr::new(crate::widgets::bakeoff_input::lookback_copy(model.tune_lookback)), _ => model.tune_screen_state.range_label_or_default(), };` | `promote_swept_config.rs::promote_swept_config_carries_the_swept_window_label`, at its `D-P24` check (the 2024 H1 grid after Tune is re-opened on 2024 H2). Its other two `Ready` cases pass, and so does `relayed_plan_resolves_promotion_loading_to_ready`: there the lookback names the grid's own window |
//! | D-P25 | `ui::state::promote_swept_config` | `let strategy_id = match params {` (and its four arms) | → match on `model.tune_screen_state.family` instead, each family to the same id (plus `let _ = matches!(params, PromoteParams::Sma { .. });` so the import stays used) | `promote_swept_config.rs::promote_swept_config_strategy_id_follows_the_params_not_the_family_picker`, at its `D-P25` check, first case (SMA params with the picker on RSI). `promote_swept_config_maps_every_family_to_its_forward_id` passes: there the picker is on the params' family |
//! | D-P3 | `ui::state::update`, arm `Message::BakeoffNarrationRequested` | `if matches!(st.result, PanelState::Ready(_)) && !st.narration.is_requested() {` | → `if !st.narration.is_requested() {` | `explain_request_ignored_until_a_ready_result_is_on_screen`, at the `Loading` side |
//! | D-P11 | same arm | `if matches!(st.result, PanelState::Ready(_)) && !st.narration.is_requested() {` | → `if !matches!(st.result, PanelState::Loading) && !st.narration.is_requested() {` | `explain_request_ignored_until_a_ready_result_is_on_screen`, at the `Error` side (the `Loading` side stays green, which is why the test walks all three non-`Ready` results) |
//! | D-P4 | same arm | `if matches!(st.result, PanelState::Ready(_)) && !st.narration.is_requested() {` | → `if matches!(st.result, PanelState::Ready(_)) {` | `explain_rerequest_does_not_clobber_a_resolved_narration` (the re-press flips `Ready(p)` back to `InFlight`) |
//! | D-P28 | same arm | `if matches!(st.result, PanelState::Ready(_)) && !st.narration.is_requested() {` | → `if matches!(st.result, PanelState::Ready(_)) && !matches!(st.narration, crate::leaderboard::NarrationState::InFlight \| crate::leaderboard::NarrationState::Ready(_)) {` (a guard that forgets `FellBack`; round 3's extra R3-X3) | `explain_rerequest_does_not_clobber_a_resolved_narration`, at its second `D-P4` site (the re-press after `FellBack`). D-P4 reaches only the first site |
//! | D-P5 | `ui::leaderboard::LeaderboardScreenState::begin_run` (`crates/ui/src/leaderboard/state.rs`) | `self.progress = None;` (under the comment `(a stale "7 of 7" from the last run must never show on the new one).`) | delete the line | `bakeoff_run_requested_clears_prior_runs_progress`; also `second_bakeoff_request_mid_run_keeps_progress` at its idle side |
//! | D-P6 | `ui::leaderboard::LeaderboardScreenState::finish_run` | `self.progress = None;` (under the comment `lingers under the (now Ready/Error) result.`) | delete the line | `bakeoff_progress_cleared_on_completion` |
//! | D-P13 | `ui::leaderboard::LeaderboardScreenState::finish_run` | `self.running = false;` (its first line) | delete the line | `bakeoff_progress_cleared_on_completion`, at its `D-P13` check. The other bake-off tests go red too, each at a `!running` check, because the run never ends: the explain tests at `seed: the prior run is over` (in `cockpit_after_an_explained_run`), `bakeoff_run_requested_clears_prior_runs_progress` and `second_bakeoff_request_mid_run_keeps_progress` at their `precondition:` that the run completed |
//! | D-P7 | `ui::state::update`, arm `Message::BakeoffRunRequested` | `if !model.leaderboard_screen_state.running {` | drop the guard: `model.leaderboard_screen_state.begin_run();` unconditionally | `second_bakeoff_request_mid_run_keeps_progress`, at its mid-run `D-P7` check (the second press wipes the live 3-of-7 bar) |
//! | D-P21 | `ui::state::update`, arm `Message::BakeoffRunRequested` | `if !model.leaderboard_screen_state.running {` | close that `if` with `} else { model.leaderboard_screen_state.running = false; }` (a Run press that doubles as Cancel) | `second_bakeoff_request_mid_run_keeps_progress`, at its mid-run `D-P21` check (the bar is kept, the run is ended) |
//! | D-P26 | `ui::state::update`, arm `Message::BakeoffRunRequested` | `if !model.leaderboard_screen_state.running {` | close that `if` with `} else { model.leaderboard_screen_state.result = PanelState::Error(SmolStr::new("cancelled")); }` (a refused press that writes the result; round 3's survivor R3-X1) | `second_bakeoff_request_mid_run_keeps_progress`, at its mid-run `D-P26` check (bar and run kept, result `Error`) |
//! | D-P8 | `ui::state::update`, arm `Message::SweepRunRequested` | `if model.tune_screen_state.can_run() {` | → `if true {` | `sweep_double_run_request_mid_run_keeps_progress`, at its first row (idle, unrunnable form: the press starts a sweep) |
//! | D-P12 | `ui::tune::TuneScreenState::can_run` (`crates/ui/src/tune/screen_state.rs`) | `self.family.is_runnable() && !self.running && self.grid_estimate().is_runnable()` | drop `!self.running && ` | `sweep_double_run_request_mid_run_keeps_progress`, at its third row's `D-P12` check (mid-sweep: the press wipes the live 4-of-12 bar). Its first two rows pass: the form conjunct still refuses the unrunnable form |
//! | D-P22 | `ui::state::update`, arm `Message::SweepRunRequested` | `if model.tune_screen_state.can_run() {` | close that `if` with `} else { model.tune_screen_state.running = false; }` (a Run press that doubles as Cancel) | `sweep_double_run_request_mid_run_keeps_progress`, at its third row's `D-P22` check. Its first two rows pass: an idle refusal sets a `running` that is already `false` |
//! | D-P27 | `ui::state::update`, arm `Message::SweepRunRequested` | `if model.tune_screen_state.can_run() {` | close that `if` with `} else if model.tune_screen_state.running { model.tune_screen_state.result = PanelState::Error(SmolStr::new("cancelled")); }` (a mid-sweep refusal that writes the result; round 3's survivor R3-X2) | `sweep_double_run_request_mid_run_keeps_progress`, at its third row's `D-P27` check. Its first row passes: the write fires only mid-sweep |
//! | D-P23 | `ui::state::update`, arm `Message::SweepAxisEdit` | `model.tune_screen_state.edit_axis(axis, field, value);` | append `model.tune_screen_state.select_family(crate::tune::TuneFamily::Rsi);` after it | `sweep_double_run_request_mid_run_keeps_progress`, at its first `precondition:` (the edit moved the family off SMA, so the row no longer differs in the form only) |
//! | D-P9 | `ui::tune::TuneScreenState::begin_run` | `self.progress = None;` (the line after `self.running = true;`) | delete the line | `sweep_rerun_clears_late_stale_progress`, at its `D-P9` check, first outcome (`Ready`) |
//! | D-P10 | `ui::tune::TuneScreenState::finish_run` | `self.progress = None;` (the line after `self.running = false;`) | delete the line | `sweep_lifecycle_running_and_progress_transitions`, at its `D-P10` check (first outcome, `Ready`) |
//! | D-P14 | `ui::tune::TuneScreenState::finish_run` | `self.running = false;` (its first line) | delete the line | `sweep_lifecycle_running_and_progress_transitions`, at its `D-P14` check; also `sweep_rerun_clears_late_stale_progress`, at its `precondition:` that the sweep completed, and `sweep_double_run_request_mid_run_keeps_progress`, at its `seed: the prior sweep is over` |
//!
//! Not pinned here: the binary's half of each arm (the `spawn_bakeoff` /
//! `spawn_sweep` dispatch, the Explain `try_send`, the promotion's
//! `ForwardCommand::Launch`). They live in `cockpit_live.rs`, a bin target an
//! integration test cannot link (the K3 #6 `AppState` residual).
//!
//! Gated on `live` (the build where the relay seams and the `agent` types
//! exist), NOT on `fixtures` (`ui::fixtures` is always compiled) and NOT on
//! `target_os` (no pixels here: pure state logic, deterministic on every OS).

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use futures::StreamExt;
use futures::stream::BoxStream;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use tokio::time::timeout;

use backtest::progress::BakeoffProgress;
use trading_core::Symbol;
use ui::forward_plan::ForwardPlanView;
use ui::leaderboard::runner::BakeoffRunResult;
use ui::leaderboard::{BakeoffReportMirror, LeaderboardLookback, NarrationState};
use ui::state::{Cockpit, Message, PanelState, Screen, update};
use ui::strings::TUNE_PROMOTE_WINDOW_FALLBACK;
use ui::tune::runner::SweepRunResult;
use ui::tune::{AxisField, PromoteParams, SweepReportMirror, TuneAxisKind, TuneFamily};

// ── Observers ─────────────────────────────────────────────────────────────────

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

/// A progress bar's `(done, total, current_id)`, `None` when no bar is stored.
/// `BakeoffProgress` has no `PartialEq`, so the fields are compared as a tuple.
fn bar(progress: Option<&BakeoffProgress>) -> Option<(u16, u16, String)> {
    progress.map(|p| (p.done, p.total, p.current_id.to_string()))
}

fn bakeoff_bar(cockpit: &Cockpit) -> Option<(u16, u16, String)> {
    bar(cockpit.leaderboard_screen_state.progress.as_ref())
}

fn tune_bar(cockpit: &Cockpit) -> Option<(u16, u16, String)> {
    bar(cockpit.tune_screen_state.progress.as_ref())
}

/// The expected bar, written the way the observers return it.
fn want(done: u16, total: u16, id: &str) -> Option<(u16, u16, String)> {
    Some((done, total, id.to_string()))
}

fn progress(done: u16, total: u16, id: &str) -> BakeoffProgress {
    BakeoffProgress {
        done,
        total,
        current_id: SmolStr::new(id),
    }
}

/// The plan on the Forward Plan screen, `None` unless it is `Ready`.
fn ready_plan(cockpit: &Cockpit) -> Option<&ForwardPlanView> {
    match &cockpit.forward_plan_screen_state.plan {
        PanelState::Ready(view) => Some(view),
        _ => None,
    }
}

/// The promotion the forward plan's "you tuned this" strip reads
/// (`ui::screens::forward_plan`: `if let Some(promotion) =
/// &model.pending_forward_promotion {`), as `(strategy id, params, window
/// label)`. `None` when nothing is promoted.
fn promotion(cockpit: &Cockpit) -> Option<(String, PromoteParams, String)> {
    cockpit.pending_forward_promotion.as_ref().map(|p| {
        (
            p.strategy_id.0.to_string(),
            p.params.clone(),
            p.window_label.to_string(),
        )
    })
}

/// Whether the Tune result is exactly the outcome a sweep completed with:
/// `Ready` of that grid, or `Error` of that message.
fn sweep_landed(result: &PanelState<SweepReportMirror>, outcome: &SweepRunResult) -> bool {
    match (result, outcome) {
        (PanelState::Ready(landed), Ok(sent)) => landed == sent,
        (PanelState::Error(landed), Err(sent)) => landed == sent,
        _ => false,
    }
}

// ── Builders and relays ───────────────────────────────────────────────────────

/// A populated agent-side `ForwardPlan` (the type the supervisor sends over
/// `plan_tx`): LONG, last signal BUY, BTCUSDT, 7-day horizon, an SMA rule with
/// the given windows. Tests vary single fields on the returned value.
fn agent_plan(strategy: &str, fast_len: u32, slow_len: u32) -> agent::config::ForwardPlan {
    use trading_core::{Money, Price, Quantity, StrategyId, Timestamp, Usdt};
    agent::config::ForwardPlan {
        strategy: StrategyId::new(strategy),
        symbol: Symbol::new("BTCUSDT"),
        stance: agent::config::PlanStance::Long,
        latest_signal: Some(agent::config::PlanSignal::Buy),
        rule: agent::config::PlanRuleKind::SmaCross { fast_len, slow_len },
        last_close: Price::new(dec!(40000.00)).unwrap(),
        last_bar_ts: Timestamp::now(),
        budget: Money::<Usdt>::from_decimal(dec!(200)),
        projected_units: Quantity::new(dec!(0.005)).unwrap(),
        sizing_capped: false,
        horizon_days: 7,
        confidence: None,
    }
}

/// Await the next item, FAILING the test if nothing resolves in `secs`.
/// `Some(msg)` is a relayed message, `None` is the stream's END; a timeout is
/// never read as either.
async fn next_or_fail(
    stream: &mut BoxStream<'static, Message>,
    secs: u64,
    what: &str,
) -> Option<Message> {
    timeout(Duration::from_secs(secs), stream.next())
        .await
        .unwrap_or_else(|_| panic!("timed out after {secs}s waiting for {what}"))
}

/// Send one plan into the agent→ui plan channel and return what the relay
/// yields for it, which must be a `ForwardPlanReceived`.
async fn relay_plan(
    tx: &tokio::sync::mpsc::Sender<agent::ForwardPlan>,
    stream: &mut BoxStream<'static, Message>,
    plan: &agent::config::ForwardPlan,
) -> Message {
    tx.send(plan.clone())
        .await
        .expect("the plan entered the channel (receiver alive)");
    let msg = next_or_fail(stream, 2, "the relayed plan")
        .await
        .expect("the relay yielded the plan, it did not END");
    assert!(
        matches!(msg, Message::ForwardPlanReceived(_)),
        "the plan relay yields ForwardPlanReceived, got {msg:?}"
    );
    msg
}

/// Send one narration outcome into the agent→ui narration channel and return
/// what the relay yields for it, which must be a `BakeoffNarrationCompleted`.
async fn relay_narration(
    tx: &tokio::sync::mpsc::Sender<agent::NarrationOutcome>,
    stream: &mut BoxStream<'static, Message>,
    outcome: agent::NarrationOutcome,
) -> Message {
    tx.send(outcome)
        .await
        .expect("the outcome entered the channel (receiver alive)");
    let msg = next_or_fail(stream, 2, "the relayed narration outcome")
        .await
        .expect("the relay yielded the outcome, it did not END");
    assert!(
        matches!(msg, Message::BakeoffNarrationCompleted(_)),
        "the narration relay yields BakeoffNarrationCompleted, got {msg:?}"
    );
    msg
}

/// A ranked mirror with no candidate rows: what a bake-off that scored nothing
/// returns. `finish_run` lands it as `Empty`.
fn bakeoff_mirror_without_rows() -> BakeoffReportMirror {
    let mut mirror = ui::fixtures::fake_bakeoff_report_mirror();
    mirror.rows.clear();
    mirror
}

// ── §1 ForwardPlanRecipe: the plan the relay delivers always lands ──────────

/// **D-P1, D-P16 (and D-P15, D-P20).** The production promotion path, end to
/// end through `update`: a crowned plan is on screen; the operator opens Tune
/// ("Tune…", `OpenTuneEditor`), runs a sweep, and presses "Use this config" on
/// a `Ready` grid; `PromoteSweptConfig` flips the plan to `Loading` (no stale
/// crowned plan) and navigates; the agent resolves the tuned plan and the
/// relay delivers it. That plan must resolve the `Loading`, and the promotion
/// must survive it.
///
/// - The crowned plan is seeded first, so the `Loading` precondition can fail:
///   with it on screen, a promotion that leaves the plan alone leaves it
///   `Ready` (D-P15 is red at that precondition).
/// - The grid's window is "2024 H2" (Tune is opened on that lookback), not the
///   fixture's "2024 H1" and not the fallback phrase, so a promotion that names
///   either of those fails the D-P20 precondition. The Tune lookback names the same
///   window here, so this test does not tell the grid's label from the
///   lookback's; `promote_swept_config.rs` does (D-P24).
/// - "Nothing is promoted yet" reads the cold default by necessity: nothing in
///   `update` clears a promotion once one is made (see the
///   `promote_swept_config.rs` module doc).
/// - Only the relayed tuned plan can turn `Loading` into `Ready`, and the plan
///   on screen is compared with `ForwardPlanView::from_plan` of what was sent,
///   not with what arrived.
/// - The promotion is compared, before and after the relay, with the params
///   and the grid that were sent. The forward plan's provenance strip reads it
///   over the tuned plan, and the binary reads it rather than taking it
///   (`cockpit_live.rs`: "We READ (do NOT take) the promotion").
#[tokio::test]
async fn relayed_plan_resolves_promotion_loading_to_ready() {
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::ForwardPlan>(4);
    let mut stream = ui::live::forward_plan_stream_impl(Some(rx));
    let mut cockpit = Cockpit::new();

    // Seed: the crowned plan is on screen.
    let crowned = agent_plan("v0.sma", 20, 50);
    let crowned_view = ForwardPlanView::from_plan(&crowned);
    let msg = relay_plan(&tx, &mut stream, &crowned).await;
    update(&mut cockpit, msg);
    assert_eq!(
        ready_plan(&cockpit),
        Some(&crowned_view),
        "seed: the crowned plan is on screen (Ready)"
    );

    // "Tune…" (`OpenTuneEditor`), then a sweep whose grid lands Ready: where
    // "Use this config" lives.
    let mut grid = ui::fixtures::fake_sweep_report_mirror();
    grid.range_label = SmolStr::new("2024 H2");
    assert_ne!(
        grid.range_label.as_str(),
        ui::fixtures::fake_sweep_report_mirror()
            .range_label
            .as_str(),
        "fixture: the grid's window is not the stock fixture's"
    );
    assert_ne!(
        grid.range_label.as_str(),
        TUNE_PROMOTE_WINDOW_FALLBACK,
        "fixture: the grid's window is not the fallback phrase"
    );
    update(
        &mut cockpit,
        Message::OpenTuneEditor {
            family: TuneFamily::Sma,
            coin: Symbol::new("BTCUSDT"),
            lookback: LeaderboardLookback::H2_2024,
        },
    );
    assert_eq!(
        cockpit.current_screen,
        Screen::Tune,
        "precondition: the Tune… drill-down navigated to the Tune screen"
    );
    update(&mut cockpit, Message::SweepRunRequested);
    assert!(
        cockpit.tune_screen_state.running,
        "precondition: SweepRunRequested started a sweep"
    );
    update(&mut cockpit, Message::SweepRunCompleted(Ok(grid.clone())));
    assert!(
        matches!(&cockpit.tune_screen_state.result, PanelState::Ready(m) if *m == grid),
        "precondition: the sweep grid is Ready, got {}",
        kind(&cockpit.tune_screen_state.result)
    );
    assert_eq!(
        promotion(&cockpit),
        None,
        "precondition: no message on the way here wrote a promotion (only PromoteSweptConfig writes one)"
    );

    let params = PromoteParams::Sma {
        fast_len: 10,
        slow_len: 20,
    };
    let want_promotion = Some((
        "v0.5.sma".to_string(),
        params.clone(),
        grid.range_label.to_string(),
    ));
    update(&mut cockpit, Message::PromoteSweptConfig(params));
    assert_eq!(
        cockpit.current_screen,
        Screen::ForwardPlan,
        "precondition: the promotion navigated from Tune to the forward plan"
    );
    assert_eq!(
        kind(&cockpit.forward_plan_screen_state.plan),
        "Loading",
        "precondition: the promotion flipped the crowned plan to Loading (D-P15)"
    );
    assert_eq!(
        promotion(&cockpit),
        want_promotion,
        "precondition: the promotion names the tuned config and the window of the grid it came from (D-P20)"
    );

    // The agent resolves the tuned config and the relay delivers its plan.
    let tuned = agent_plan("v0.5.sma", 10, 20);
    let tuned_view = ForwardPlanView::from_plan(&tuned);
    assert_ne!(
        tuned_view, crowned_view,
        "fixture pair: the tuned plan differs from the crowned one"
    );
    let msg = relay_plan(&tx, &mut stream, &tuned).await;
    update(&mut cockpit, msg);

    match &cockpit.forward_plan_screen_state.plan {
        PanelState::Ready(view) => {
            assert_eq!(view.strategy, "v0.5.sma", "the tuned strategy is on screen");
            assert_eq!(
                view, &tuned_view,
                "the plan on screen is exactly the relayed tuned plan"
            );
        }
        other => panic!(
            "D-P1: the relayed tuned plan must resolve the promotion's Loading to Ready; \
             the plan is {}",
            kind(other)
        ),
    }
    assert_eq!(
        promotion(&cockpit),
        want_promotion,
        "D-P16: the relayed tuned plan must leave the promotion in place \
         (the forward plan's \"you tuned this\" strip reads it over the tuned plan)"
    );
}

/// **D-P2.** One plan stream for the session, two plans relayed. The first
/// lands on the cold `Empty`; the second must REPLACE it (a re-crowned pick, or
/// the agent's next resolution). The two plans differ in strategy, stance,
/// last signal, rule (an SMA cross against an RSI reversion), last close and
/// horizon, each asserted below as a fixture pair, so a screen that kept the
/// first plan cannot match the second.
#[tokio::test]
async fn relayed_second_plan_replaces_the_first() {
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::ForwardPlan>(4);
    let mut stream = ui::live::forward_plan_stream_impl(Some(rx));
    let mut cockpit = Cockpit::new();

    let first = agent_plan("v0.sma", 20, 50);
    let mut second = agent_plan("v0.5.rsi", 20, 50);
    second.rule = agent::config::PlanRuleKind::RsiReversion { len: 14, lower: 30 };
    second.stance = agent::config::PlanStance::Flat;
    second.latest_signal = Some(agent::config::PlanSignal::Sell);
    second.last_close = trading_core::Price::new(dec!(41250.50)).unwrap();
    second.horizon_days = 14;
    let (first_view, second_view) = (
        ForwardPlanView::from_plan(&first),
        ForwardPlanView::from_plan(&second),
    );
    assert_ne!(
        first_view.strategy, second_view.strategy,
        "fixture pair: the strategies differ"
    );
    assert_ne!(
        first_view.stance, second_view.stance,
        "fixture pair: the stances differ"
    );
    assert_ne!(
        first_view.latest_signal, second_view.latest_signal,
        "fixture pair: the last signals differ"
    );
    assert_ne!(
        first_view.rule, second_view.rule,
        "fixture pair: the rules differ"
    );
    assert_ne!(
        first_view.last_close, second_view.last_close,
        "fixture pair: the last closes differ"
    );
    assert_ne!(
        first_view.horizon_days, second_view.horizon_days,
        "fixture pair: the horizons differ"
    );

    let msg = relay_plan(&tx, &mut stream, &first).await;
    update(&mut cockpit, msg);
    assert_eq!(
        ready_plan(&cockpit),
        Some(&first_view),
        "precondition: the first plan landed (Ready)"
    );

    let msg = relay_plan(&tx, &mut stream, &second).await;
    update(&mut cockpit, msg);
    assert_eq!(
        ready_plan(&cockpit),
        Some(&second_view),
        "D-P2: the second relayed plan must REPLACE the first on screen (got {})",
        kind(&cockpit.forward_plan_screen_state.plan)
    );
}

// ── §2 NarrationOutcomeRecipe: when "Explain" is accepted ───────────────────

/// A cockpit whose PREVIOUS run was ranked, explained and resolved to
/// `Ready(STALE)`: the state each new run must reset. Without it, a
/// `NotRequested` precondition holds by `Default` alone. The resolution comes
/// out of `narration_outcome_stream_impl` over the caller's channel, the way
/// production delivers it.
async fn cockpit_after_an_explained_run(
    tx: &tokio::sync::mpsc::Sender<agent::NarrationOutcome>,
    stream: &mut BoxStream<'static, Message>,
) -> Cockpit {
    const STALE: &str = "A prior run's narration, which must not carry into the next run.";
    let mut cockpit = Cockpit::new();
    update(&mut cockpit, Message::BakeoffRunRequested);
    assert!(
        cockpit.leaderboard_screen_state.running,
        "seed: the prior run started"
    );
    update(
        &mut cockpit,
        Message::BakeoffRunCompleted(Ok(ui::fixtures::fake_bakeoff_report_mirror())),
    );
    assert!(
        !cockpit.leaderboard_screen_state.running,
        "seed: the prior run is over"
    );
    update(&mut cockpit, Message::BakeoffNarrationRequested);
    assert_eq!(
        cockpit.leaderboard_screen_state.narration,
        NarrationState::InFlight,
        "seed: Explain on the prior run's Ready result was accepted"
    );
    let msg = relay_narration(
        tx,
        stream,
        agent::NarrationOutcome::Ready(SmolStr::new(STALE)),
    )
    .await;
    update(&mut cockpit, msg);
    assert_eq!(
        cockpit.leaderboard_screen_state.narration,
        NarrationState::Ready(SmolStr::new(STALE)),
        "seed: the prior run's narration resolved"
    );
    cockpit
}

/// **D-P3, D-P11.** The Explain guard over the RESULT dimension. Four cockpits,
/// one per result of the new run. Each starts from a prior explained run whose
/// seed proves that run is over, then presses Run, so "the new run started" is
/// this press's doing and the narration is a proven `NotRequested`. They differ in
/// the result, and the `Loading` one also in `running`, which `update` couples
/// to a `Loading` result in every state it can reach:
///
/// | result on screen | after Explain |
/// |---|---|
/// | `Loading` (the run is in flight) | `NotRequested` |
/// | `Error` (the run failed) | `NotRequested` |
/// | `Empty` (the run ranked nothing) | `NotRequested` |
/// | `Ready` (a ranked leaderboard) | `InFlight` |
///
/// The three refusals are separate cases because a guard can refuse one and
/// accept another (D-P11 refuses `Loading` only; it is red at `Error`).
#[tokio::test]
async fn explain_request_ignored_until_a_ready_result_is_on_screen() {
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::NarrationOutcome>(4);
    let mut stream = ui::live::narration_outcome_stream_impl(Some(rx));
    let cases: [(&str, Option<BakeoffRunResult>, NarrationState); 4] = [
        ("Loading", None, NarrationState::NotRequested),
        (
            "Error",
            Some(Err(SmolStr::new("boom"))),
            NarrationState::NotRequested,
        ),
        (
            "Empty",
            Some(Ok(bakeoff_mirror_without_rows())),
            NarrationState::NotRequested,
        ),
        (
            "Ready",
            Some(Ok(ui::fixtures::fake_bakeoff_report_mirror())),
            NarrationState::InFlight,
        ),
    ];

    for (want_result, outcome, want_narration) in cases {
        let mut cockpit = cockpit_after_an_explained_run(&tx, &mut stream).await;
        update(&mut cockpit, Message::BakeoffRunRequested);
        assert!(
            cockpit.leaderboard_screen_state.running,
            "[{want_result}] precondition: the new run started"
        );
        if let Some(outcome) = outcome {
            update(&mut cockpit, Message::BakeoffRunCompleted(outcome));
        }
        assert_eq!(
            kind(&cockpit.leaderboard_screen_state.result),
            want_result,
            "[{want_result}] precondition: the result on screen"
        );
        assert_eq!(
            cockpit.leaderboard_screen_state.narration,
            NarrationState::NotRequested,
            "[{want_result}] precondition: the new run reset the prior run's resolved narration"
        );

        update(&mut cockpit, Message::BakeoffNarrationRequested);

        let probe = if want_narration == NarrationState::InFlight {
            "the Ready side"
        } else {
            "D-P3 / D-P11"
        };
        assert_eq!(
            cockpit.leaderboard_screen_state.narration, want_narration,
            "[{want_result}] {probe}: Explain with a {want_result} result on screen"
        );
    }
}

/// **D-P4.** The Explain guard over the NARRATION dimension, with a `Ready`
/// result on screen at every press. Each outcome arrives through
/// `narration_outcome_stream_impl` over a real channel, one stream for the
/// whole test, as production streams one receiver for the session.
///
/// - Run 1 follows a prior explained run, so its `NotRequested` is a reset
///   that `update` made, not the `Default`: Explain is accepted
///   (`NotRequested` → `InFlight`).
/// - Resolved to `Ready(p)`: a re-press leaves `Ready(p)`.
/// - Run 2, resolved to `FellBack`: a re-press leaves `FellBack`.
#[tokio::test]
async fn explain_rerequest_does_not_clobber_a_resolved_narration() {
    const PROSE: &str = "SMA crossover led on risk-adjusted return over this window.";
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::NarrationOutcome>(4);
    let mut stream = ui::live::narration_outcome_stream_impl(Some(rx));
    let mut cockpit = cockpit_after_an_explained_run(&tx, &mut stream).await;

    // Run 1, resolved to Ready(PROSE).
    update(&mut cockpit, Message::BakeoffRunRequested);
    assert_eq!(
        cockpit.leaderboard_screen_state.narration,
        NarrationState::NotRequested,
        "precondition: run 1 reset the prior run's resolved narration"
    );
    update(
        &mut cockpit,
        Message::BakeoffRunCompleted(Ok(ui::fixtures::fake_bakeoff_report_mirror())),
    );
    assert_eq!(
        kind(&cockpit.leaderboard_screen_state.result),
        "Ready",
        "precondition: a ranked result is on screen"
    );
    update(&mut cockpit, Message::BakeoffNarrationRequested);
    assert_eq!(
        cockpit.leaderboard_screen_state.narration,
        NarrationState::InFlight,
        "the other side of the pair: with nothing requested, Explain is accepted"
    );
    let msg = relay_narration(
        &tx,
        &mut stream,
        agent::NarrationOutcome::Ready(SmolStr::new(PROSE)),
    )
    .await;
    update(&mut cockpit, msg);
    let resolved = NarrationState::Ready(SmolStr::new(PROSE));
    assert_eq!(
        cockpit.leaderboard_screen_state.narration, resolved,
        "precondition: the relayed outcome resolved the narration"
    );

    update(&mut cockpit, Message::BakeoffNarrationRequested);
    assert_eq!(
        cockpit.leaderboard_screen_state.narration, resolved,
        "D-P4: a re-press of Explain must not clobber the resolved Ready narration"
    );

    // Run 2, resolved to FellBack.
    update(&mut cockpit, Message::BakeoffRunRequested);
    assert_eq!(
        cockpit.leaderboard_screen_state.narration,
        NarrationState::NotRequested,
        "precondition: the new run reset the resolved narration"
    );
    update(
        &mut cockpit,
        Message::BakeoffRunCompleted(Ok(ui::fixtures::fake_bakeoff_report_mirror())),
    );
    update(&mut cockpit, Message::BakeoffNarrationRequested);
    assert_eq!(
        cockpit.leaderboard_screen_state.narration,
        NarrationState::InFlight,
        "precondition: run 2's Explain was accepted"
    );
    let msg = relay_narration(&tx, &mut stream, agent::NarrationOutcome::FellBack).await;
    update(&mut cockpit, msg);
    assert_eq!(
        cockpit.leaderboard_screen_state.narration,
        NarrationState::FellBack,
        "precondition: the relayed outcome resolved run 2 to FellBack"
    );

    update(&mut cockpit, Message::BakeoffNarrationRequested);
    assert_eq!(
        cockpit.leaderboard_screen_state.narration,
        NarrationState::FellBack,
        "D-P4: a re-press of Explain must not clobber the resolved FellBack"
    );
}

// ── §3 BakeoffProgressRecipe: the leaderboard bar across runs ───────────────

/// **D-P5.** A run reaches 6 of 7 and completes (failed, then ranked: both
/// outcomes). A LATE progress event from that run is relayed after the
/// completion and leaves a stale "7 of 7". Pressing Run again must start the
/// new run from the indeterminate spinner: running, `Loading`, and no bar.
#[test]
fn bakeoff_run_requested_clears_prior_runs_progress() {
    let outcomes: [(&str, BakeoffRunResult); 2] = [
        ("failed", Err(SmolStr::new("boom"))),
        ("ranked", Ok(ui::fixtures::fake_bakeoff_report_mirror())),
    ];
    for (label, outcome) in outcomes {
        let mut cockpit = Cockpit::new();
        update(&mut cockpit, Message::BakeoffRunRequested);
        assert!(
            cockpit.leaderboard_screen_state.running,
            "[{label}] precondition: the first run started"
        );
        update(
            &mut cockpit,
            Message::BakeoffProgress(progress(6, 7, "v0.buyhold")),
        );
        assert_eq!(
            bakeoff_bar(&cockpit),
            want(6, 7, "v0.buyhold"),
            "[{label}] precondition: the bar shows 6 of 7"
        );
        update(&mut cockpit, Message::BakeoffRunCompleted(outcome));
        assert!(
            !cockpit.leaderboard_screen_state.running,
            "[{label}] precondition: the first run completed"
        );

        update(
            &mut cockpit,
            Message::BakeoffProgress(progress(7, 7, "v0.buyhold")),
        );
        assert_eq!(
            bakeoff_bar(&cockpit),
            want(7, 7, "v0.buyhold"),
            "[{label}] seed: the late event left a stale 7-of-7 bar"
        );
        assert!(
            !cockpit.leaderboard_screen_state.running,
            "[{label}] seed: a late progress event does not restart the run"
        );

        update(&mut cockpit, Message::BakeoffRunRequested);
        assert!(
            cockpit.leaderboard_screen_state.running,
            "[{label}] precondition: the second Run press started a run"
        );
        assert_eq!(
            kind(&cockpit.leaderboard_screen_state.result),
            "Loading",
            "[{label}] the new run shows Loading"
        );
        assert_eq!(
            bakeoff_bar(&cockpit),
            None,
            "[{label}] D-P5: the new run must not show the prior run's stale bar"
        );
    }
}

/// **D-P6, D-P13.** A run reaches 3 of 7, then completes. For both outcomes
/// the bar is gone, the run is over, and the result lands: `Error("boom")` for
/// a failure, the ranked mirror itself for a success.
#[test]
fn bakeoff_progress_cleared_on_completion() {
    let ranked = ui::fixtures::fake_bakeoff_report_mirror();
    assert!(
        !ranked.rows.is_empty(),
        "fixture: the ranked mirror has rows (an empty one lands Empty, not Ready)"
    );
    let outcomes: [(&str, BakeoffRunResult); 2] = [
        ("Error", Err(SmolStr::new("boom"))),
        ("Ready", Ok(ranked.clone())),
    ];
    for (want_result, outcome) in outcomes {
        let mut cockpit = Cockpit::new();
        update(&mut cockpit, Message::BakeoffRunRequested);
        update(
            &mut cockpit,
            Message::BakeoffProgress(progress(3, 7, "v0.5.bbands")),
        );
        assert!(
            cockpit.leaderboard_screen_state.running,
            "[{want_result}] precondition: the run is in flight"
        );
        assert_eq!(
            bakeoff_bar(&cockpit),
            want(3, 7, "v0.5.bbands"),
            "[{want_result}] precondition: the bar shows 3 of 7"
        );

        update(&mut cockpit, Message::BakeoffRunCompleted(outcome));

        assert_eq!(
            bakeoff_bar(&cockpit),
            None,
            "[{want_result}] D-P6: completion must clear the bar (no stale 3 of 7 under the result)"
        );
        assert!(
            !cockpit.leaderboard_screen_state.running,
            "[{want_result}] D-P13: completion must end the run"
        );
        match &cockpit.leaderboard_screen_state.result {
            PanelState::Error(msg) if want_result == "Error" => {
                assert_eq!(msg.as_str(), "boom", "the failure message lands");
            }
            PanelState::Ready(mirror) if want_result == "Ready" => {
                assert_eq!(mirror, &ranked, "the ranked mirror lands as sent");
            }
            other => panic!("[{want_result}] the result landed as {}", kind(other)),
        }
    }
}

/// **D-P7, D-P21, D-P26.** The Run guard over the `running` dimension. One cockpit:
///
/// - mid-run (3 of 7 on the bar), a second Run press is refused: the bar is
///   kept (D-P7), the run is kept (D-P21) and the result stays `Loading`
///   (D-P26). The last check does not separate refusing from accepting (both
///   leave `Loading`; D-P7 does that). It catches a refusal path that writes
///   the result, such as a press that doubles as Cancel and lands an `Error`;
/// - idle (the run failed, a late event left a stale bar), the same press is
///   accepted: a new run starts and the stale bar is cleared.
#[test]
fn second_bakeoff_request_mid_run_keeps_progress() {
    let mut cockpit = Cockpit::new();
    update(&mut cockpit, Message::BakeoffRunRequested);
    update(
        &mut cockpit,
        Message::BakeoffProgress(progress(3, 7, "v0.5.bbands")),
    );
    assert!(
        cockpit.leaderboard_screen_state.running,
        "precondition: the run is in flight"
    );
    assert_eq!(
        bakeoff_bar(&cockpit),
        want(3, 7, "v0.5.bbands"),
        "precondition: the bar shows 3 of 7"
    );

    // Mid-run side.
    update(&mut cockpit, Message::BakeoffRunRequested);
    assert_eq!(
        bakeoff_bar(&cockpit),
        want(3, 7, "v0.5.bbands"),
        "D-P7: a second Run press mid-run must keep the live bar"
    );
    assert!(
        cockpit.leaderboard_screen_state.running,
        "D-P21: a second Run press mid-run must not end the run"
    );
    assert_eq!(
        kind(&cockpit.leaderboard_screen_state.result),
        "Loading",
        "D-P26: a refused Run press mid-run must leave the in-flight run's Loading result"
    );

    // Idle side.
    update(
        &mut cockpit,
        Message::BakeoffRunCompleted(Err(SmolStr::new("boom"))),
    );
    assert!(
        !cockpit.leaderboard_screen_state.running,
        "precondition: the run completed"
    );
    assert_eq!(
        kind(&cockpit.leaderboard_screen_state.result),
        "Error",
        "precondition: the failed run's Error is on screen"
    );
    update(
        &mut cockpit,
        Message::BakeoffProgress(progress(4, 7, "v0.5.rsi")),
    );
    assert_eq!(
        bakeoff_bar(&cockpit),
        want(4, 7, "v0.5.rsi"),
        "seed: the late event left a stale bar"
    );
    update(&mut cockpit, Message::BakeoffRunRequested);
    assert!(
        cockpit.leaderboard_screen_state.running,
        "the other side of the pair: an idle Run press starts a run"
    );
    assert_eq!(
        kind(&cockpit.leaderboard_screen_state.result),
        "Loading",
        "the new run shows Loading"
    );
    assert_eq!(
        bakeoff_bar(&cockpit),
        None,
        "the new run starts with no bar (D-P5)"
    );
}

// ── §4 SweepProgressRecipe: the Tune bar across sweeps ──────────────────────

/// **D-P14, D-P10.** One sweep's lifecycle, two-sided over the outcome
/// (`Ok(grid)` → `Ready(grid)`, `Err("boom")` → `Error("boom")`), each on a
/// fresh cockpit:
///
/// 1. Requested: running, `Loading`.
/// 2. Three progress events: still running, and the bar is the LATEST event.
/// 3. Completed: not running, no bar, the result landed. The bar stored in
///    step 2 is what makes "no bar" observable.
///
/// Completion is checked before anything that depends on it, so D-P14 and
/// D-P10 are each red at their own check, not at a later precondition. What
/// follows a completion (a late event, then a re-run) is
/// `sweep_rerun_clears_late_stale_progress`.
#[test]
fn sweep_lifecycle_running_and_progress_transitions() {
    let grid = ui::fixtures::fake_sweep_report_mirror();
    assert!(
        !grid.cells.is_empty(),
        "fixture: the grid has cells (an empty grid lands Empty, not Ready)"
    );
    let outcomes: [(&str, SweepRunResult); 2] = [
        ("Ready", Ok(grid.clone())),
        ("Error", Err(SmolStr::new("boom"))),
    ];
    for (want_result, outcome) in outcomes {
        let mut cockpit = Cockpit::new();

        // 1) Requested.
        update(&mut cockpit, Message::SweepRunRequested);
        assert!(
            cockpit.tune_screen_state.running,
            "[{want_result}] SweepRunRequested starts the sweep"
        );
        assert_eq!(
            kind(&cockpit.tune_screen_state.result),
            "Loading",
            "[{want_result}] the sweep result is Loading"
        );

        // 2) Progress: the bar follows the latest event.
        for (done, id) in [
            (0u16, "fast=10, slow=30"),
            (2, "fast=10, slow=50"),
            (4, "fast=15, slow=30"),
        ] {
            update(&mut cockpit, Message::SweepProgress(progress(done, 5, id)));
            assert!(
                cockpit.tune_screen_state.running,
                "[{want_result}] a progress event ({done} of 5) does not end the sweep"
            );
            assert_eq!(
                tune_bar(&cockpit),
                want(done, 5, id),
                "[{want_result}] the bar shows the latest event"
            );
        }

        // 3) Completed. The loop's last check is the precondition: a bar is
        //    stored (4 of 5), so its clearing is observable.
        update(&mut cockpit, Message::SweepRunCompleted(outcome));
        assert!(
            !cockpit.tune_screen_state.running,
            "[{want_result}] D-P14: completion must end the sweep"
        );
        assert_eq!(
            tune_bar(&cockpit),
            None,
            "[{want_result}] D-P10: completion must clear the bar (no stale 4 of 5 under the result)"
        );
        match &cockpit.tune_screen_state.result {
            PanelState::Ready(mirror) if want_result == "Ready" => {
                assert_eq!(mirror, &grid, "the swept grid lands as sent");
            }
            PanelState::Error(msg) if want_result == "Error" => {
                assert_eq!(msg.as_str(), "boom", "the failure message lands");
            }
            other => panic!("[{want_result}] the result landed as {}", kind(other)),
        }
    }
}

/// **D-P9.** A sweep reaches 11 of 12 and completes, for both outcomes (its
/// grid lands, or it fails). A LATE progress event from it arrives afterwards:
/// it is stored, but the sweep stays over and the landed result stays on
/// screen. Re-running must then start from no bar. This is the only D-P9 site
/// in this file: the lifecycle test above stops at the completion.
///
/// Not pinned here: whether the operator SEES the late bar before the re-run.
/// That is decided at render time by the private
/// `ui::screens::tune::progress_strip` (`if !st.running {` returns `None`), and
/// this file has no pixels. The test pins the one input that function reads,
/// `running`, as a `seed:`.
#[test]
fn sweep_rerun_clears_late_stale_progress() {
    let grid = ui::fixtures::fake_sweep_report_mirror();
    let outcomes: [(&str, SweepRunResult); 2] =
        [("Ready", Ok(grid)), ("Error", Err(SmolStr::new("boom")))];
    for (landed, outcome) in outcomes {
        let mut cockpit = Cockpit::new();
        update(&mut cockpit, Message::SweepRunRequested);
        update(
            &mut cockpit,
            Message::SweepProgress(progress(11, 12, "fast=25, slow=60")),
        );
        assert_eq!(
            tune_bar(&cockpit),
            want(11, 12, "fast=25, slow=60"),
            "[{landed}] precondition: the bar shows 11 of 12"
        );
        update(&mut cockpit, Message::SweepRunCompleted(outcome.clone()));
        assert!(
            !cockpit.tune_screen_state.running,
            "[{landed}] precondition: the sweep completed"
        );
        assert!(
            sweep_landed(&cockpit.tune_screen_state.result, &outcome),
            "[{landed}] precondition: the outcome landed as sent, got {}",
            kind(&cockpit.tune_screen_state.result)
        );

        update(
            &mut cockpit,
            Message::SweepProgress(progress(12, 12, "fast=25, slow=60")),
        );
        assert_eq!(
            tune_bar(&cockpit),
            want(12, 12, "fast=25, slow=60"),
            "[{landed}] seed: the late event left a stale 12-of-12 bar"
        );
        assert!(
            !cockpit.tune_screen_state.running,
            "[{landed}] seed: a late progress event does not restart the sweep"
        );
        assert!(
            sweep_landed(&cockpit.tune_screen_state.result, &outcome),
            "[{landed}] seed: a late progress event leaves the landed result on screen, got {}",
            kind(&cockpit.tune_screen_state.result)
        );

        update(&mut cockpit, Message::SweepRunRequested);
        assert!(
            cockpit.tune_screen_state.running,
            "[{landed}] precondition: the re-run started"
        );
        assert_eq!(
            kind(&cockpit.tune_screen_state.result),
            "Loading",
            "[{landed}] the re-run shows Loading"
        );
        assert_eq!(
            tune_bar(&cockpit),
            None,
            "[{landed}] D-P9: the re-run must not show the late stale bar"
        );
    }
}

/// **D-P8, D-P12, D-P22, D-P27 (and D-P23).** The Tune Run guard,
/// `TuneScreenState::can_run`, over the two dimensions it reads: the form, and
/// whether a sweep is running. One cockpit, seeded idle by a sweep that ran and
/// landed its grid, then three presses. Each row changes one of the two
/// dimensions from the row above it; the running row also carries what
/// `update` couples to `running` (a `Loading` result and a live bar):
///
/// | sweep | form | press | outcome |
/// |---|---|---|---|
/// | idle (a prior grid on screen) | a blank Step field | refused | still idle, the prior grid kept |
/// | idle | the Step re-typed | accepted | a sweep starts |
/// | running (4 of 12) | runnable | refused | bar, run and `Loading` kept |
///
/// The order gives each probe its own row. With no guard at all (D-P8) the
/// first press is accepted. With a `can_run` that ignores `running` (D-P12)
/// the first two rows pass and the third press wipes the live bar. With a Run
/// press that doubles as Cancel (D-P22) the third press keeps the bar but ends
/// the sweep. The first row's family precondition proves the edit changed the
/// form only (D-P23 moves the family and is red there). Both form rows read the
/// form through `grid_estimate().is_runnable()`, not through `can_run`, the
/// guard under test. The third row also checks that the result stays `Loading`
/// (D-P27). Refusing and accepting both leave `Loading`, so that check does
/// not separate them, but it catches a refusal path that writes the result.
#[test]
fn sweep_double_run_request_mid_run_keeps_progress() {
    let mut cockpit = Cockpit::new();

    // Seed: a prior sweep ran and landed its grid, so "idle" is a state
    // `update` reached, not the cold default.
    let prior: SweepRunResult = Ok(ui::fixtures::fake_sweep_report_mirror());
    update(&mut cockpit, Message::SweepRunRequested);
    assert!(
        cockpit.tune_screen_state.running,
        "seed: the prior sweep started"
    );
    update(&mut cockpit, Message::SweepRunCompleted(prior.clone()));
    assert!(
        !cockpit.tune_screen_state.running,
        "seed: the prior sweep is over"
    );
    assert!(
        sweep_landed(&cockpit.tune_screen_state.result, &prior),
        "seed: the prior sweep's grid is on screen, got {}",
        kind(&cockpit.tune_screen_state.result)
    );

    // Idle, unrunnable form: refused.
    update(
        &mut cockpit,
        Message::SweepAxisEdit {
            axis: TuneAxisKind::SmaFast,
            field: AxisField::Step,
            value: String::new(),
        },
    );
    let st = &cockpit.tune_screen_state;
    assert_eq!(
        st.family,
        TuneFamily::Sma,
        "precondition: the edit left the family on SMA, the family whose axis it edited (D-P23)"
    );
    assert!(
        !st.grid_estimate().is_runnable(),
        "precondition: the blank Step field makes the form unrunnable"
    );
    update(&mut cockpit, Message::SweepRunRequested);
    assert!(
        !cockpit.tune_screen_state.running,
        "D-P8: a Run press on an unrunnable form must not start a sweep"
    );
    assert!(
        sweep_landed(&cockpit.tune_screen_state.result, &prior),
        "the refused press leaves the prior grid on screen, got {}",
        kind(&cockpit.tune_screen_state.result)
    );

    // Idle, the Step re-typed: accepted.
    update(
        &mut cockpit,
        Message::SweepAxisEdit {
            axis: TuneAxisKind::SmaFast,
            field: AxisField::Step,
            value: "5".to_string(),
        },
    );
    assert!(
        cockpit.tune_screen_state.grid_estimate().is_runnable(),
        "precondition: re-typing the Step made the form runnable again"
    );
    update(&mut cockpit, Message::SweepRunRequested);
    assert!(
        cockpit.tune_screen_state.running,
        "the other side of the form dimension: the same press on a runnable form starts a sweep"
    );
    assert_eq!(
        kind(&cockpit.tune_screen_state.result),
        "Loading",
        "the sweep result is Loading"
    );
    update(
        &mut cockpit,
        Message::SweepProgress(progress(4, 12, "fast=15, slow=40")),
    );
    assert_eq!(
        tune_bar(&cockpit),
        want(4, 12, "fast=15, slow=40"),
        "precondition: the bar shows 4 of 12"
    );

    // Running, runnable form: refused.
    update(&mut cockpit, Message::SweepRunRequested);
    assert_eq!(
        tune_bar(&cockpit),
        want(4, 12, "fast=15, slow=40"),
        "D-P12: a second Run press mid-sweep must keep the live bar"
    );
    assert!(
        cockpit.tune_screen_state.running,
        "D-P22: a second Run press mid-sweep must not end the sweep"
    );
    assert_eq!(
        kind(&cockpit.tune_screen_state.result),
        "Loading",
        "D-P27: a refused Run press mid-sweep must leave the in-flight sweep's Loading result"
    );
}
