//! Bug-log `#202`: an "Explain" answer lands only on its own request, and a
//! request nothing will answer ends at `FellBack` (ADR-0064 § D15).
//!
//! ## What this file pins
//!
//! Each Explain mints a `ui::leaderboard::NarrationTicket`. The request carries
//! it as `request_id`, the agent echoes it on an `agent::NarrationReply`, and
//! `Message::BakeoffNarrationCompleted(ticket, outcome)` lands only while the
//! narration is `InFlight(ticket)`. Before the fix, request and answer shared no
//! identity, so:
//!
//! - an answer for bake-off A landed on bake-off B, on screen and in B's
//!   hand-off export (S0-1, S0-3);
//! - a closed reply channel (the narration task ended, e.g. on a kill-switch
//!   halt) left the panel `InFlight` until the next Run (S0-4);
//! - a refused request (`try_send` `Full` / `Closed`, or no channel) was only
//!   logged, and the panel stayed `InFlight` too (S0-5).
//!
//! Correlation is driven through `ui::state::update`, the dispatch through
//! `ui::live::update_and_dispatch_explain` (the call the binary makes in place
//! of `update`) over real channels, and the reply path through the real
//! `ui::live::narration_outcome_stream_impl`. No model field is written
//! directly. The pixel proofs of the same sequences are in
//! `leaderboard_narration_render.rs` (R1, R1c, R2, R3).
//!
//! | test | sequence | expected |
//! |---|---|---|
//! | S0-1 `an_answer_for_bakeoff_a_does_not_land_on_bakeoff_b` | Explain A, Run B, B lands, A's answer | `NotRequested` |
//! | S0-3 `bakeoff_b_export_never_carries_bakeoff_a_prose` | S0-1, then B's export | no A prose |
//! | S0-4 `a_closed_outcome_channel_falls_back_an_explain_in_flight` | Explain, reply sender drops, drain | `FellBack` |
//! | S0-5 `a_closed_request_channel_falls_back` | Explain over a closed request channel | `Refused(Closed)`, `FellBack` |
//! | F1 `an_answer_for_the_request_on_screen_lands` | Explain A, A's answer (`Ready`, then `FellBack`) | it lands; A's export holds A's prose |
//! | F2 `a_late_answer_while_the_next_bakeoff_runs_is_dropped` | Explain A, Run B (`Loading`), A's answer, B lands | `NotRequested` |
//! | F3 `explain_on_b_ignores_a_queued_answer_for_a` | Explain A, Run B, Explain B, A's then B's answer | `InFlight(t_B)`, then B's prose |
//! | F4 `a_second_answer_to_one_request_is_dropped` | one request, two answers | the first |
//! | F6 `explain_round_trip_lands_its_own_answer` | dispatch, request, a wrong id, then `request.reply` | `InFlight`, then the prose |
//! | F7 `explain_dispatch_refusals` | open / full / closed / no channel | `Sent`; each refusal `FellBack` |
//! | F8 `a_repressed_explain_sends_one_request` | two presses | `Sent`, `Idle`; one request queued |
//! | K2b `the_task_ending_after_its_answer_keeps_the_answer` | an answer, then the sender drops | `[Completed, TaskEnded]`, the answer kept |
//!
//! F5 and the `fall_back_if_in_flight` unit test are in
//! `crates/ui/src/leaderboard/state.rs`'s `mod tests`; F9 (the agent echoes each
//! id) is in `crates/agent/tests/narration_task_uses_boot_provider.rs`.
//!
//! ## Step 0
//!
//! S0-1, S0-3 and S0-4 (with R1 = S0-2) were RED on the tree before the fix, in
//! its API; S0-5 (with R2) was RED on the refactor commit, which lifted the
//! binary's enqueue into `update_and_dispatch_explain` unchanged. S0-5's body is
//! byte-identical to its Step-0 version.
//!
//! ## T-T4 falsification probe table (bug-log `#202`)
//!
//! Each probe is ONE edit to production code, located by symbol + the quoted
//! line text (line numbers rot; each quoted line occurs exactly once in its
//! file). Procedure:
//!
//! 1. snapshot the file (`cp -p <file> <snap>`) and apply the edit;
//! 2. run the probe's targets (`cargo test -p ui` plus the flags given; C is
//!    `--test narration_request_correlation`, Rn is
//!    `--test leaderboard_narration_render`, macOS only); the named tests go
//!    RED;
//! 3. restore from the snapshot, THEN `touch <file>`, so the next run compiles
//!    the restored file;
//! 4. after the last probe, `cmp` every file with its snapshot and run the GREEN
//!    union once: `cargo test -p ui --lib --test narration_request_correlation
//!    --test leaderboard_narration_render --test forward_narration_relay --test
//!    advisor_recipe_state_gating --test subscription_batch_inclusion --test
//!    recipe_off_runtime_poll` and `cargo test -p agent --test
//!    narration_task_uses_boot_provider`. It counts only if it shows
//!    `Compiling ui` (and `Compiling agent`).
//!
//! "upd" is the update-only tests: S0-1, S0-3, F1, F2, F3, F4. A RED cell names
//! the assertion a test stops at; its later rows never run, so no GREEN cell
//! claims them, and a GREEN cell names only tests in the row's run.
//!
//! | # | symbol (file) | quoted line today | mutation | run | expected RED | stays GREEN |
//! |---|---|---|---|---|---|---|
//! | T1 | `ui::leaderboard::LeaderboardScreenState::set_narration` (`crates/ui/src/leaderboard/state.rs`) | `if self.narration != NarrationState::InFlight(ticket) {` | → `if false {` (every answer lands) | C Rn | S0-1, S0-3, F2, F3, F4, F6 (its wrong-id twin), R1 | F1, F7, F8, K2b, S0-4, S0-5, R1c, R2, R3 |
//! | T15 | same | same line | → `if true {` (no answer lands) | `--lib` C Rn | F1 (at its `Ready` half), F3, F4, F6, F7 (at its `[full]` row, the first refusal), K2b, S0-5, R1c, R2; lib `set_narration_ready_lands_prose`, `set_narration_fell_back_lands_fallback`, `begin_run_resets_a_prior_narration` | S0-1, S0-3, F2, F8, S0-4, R1, R3; lib F5 and `fall_back_if_in_flight_touches_only_in_flight` |
//! | T2 | same | same line | → `if self.narration.in_flight_ticket().is_none() {` (lands on ANY ticket in flight) | C | F3, F6 (its wrong-id twin) | S0-1, S0-3, F1, F2, F4 |
//! | T3 | `ui::leaderboard::LeaderboardScreenState::begin_narration` | `self.narration_seq = self.narration_seq.wrapping_add(1);` | delete the line (every ticket is the same) | `--lib` C | F3; lib `explain_on_two_bakeoffs_mints_distinct_tickets` (F5) | S0-1, F2, F4, F6, F8 |
//! | T10 | `ui::leaderboard::LeaderboardScreenState::begin_run` | `self.narration = NarrationState::NotRequested;` | insert `self.narration_seq = 0;` after it (a run resets the counter) | `--lib` C | F3; lib F5 | S0-1, F2, F4 |
//! | T4 | `agent::narration::NarrationRequest::reply` (`crates/agent/src/narration.rs`) | `request_id: self.request_id,` | → `request_id: 0,` | `cargo test -p agent --test narration_task_uses_boot_provider`, then C | all four agent tests, each at its id check: F9 `the_narration_task_echoes_each_request_id`, the `#201` test, and `a_configured_provider_is_called_and_its_narration_returns` and `no_provider_falls_back_to_the_templated_copy` (at `explain_once`'s id-1 echo); C: F6, at "F6: the request's own reply lands" | upd, F7, F8 |
//! | T5 | `ui::live::narration_reply_to_message` (`crates/ui/src/live.rs`) | `crate::leaderboard::NarrationTicket::from_raw(reply.request_id),` | → `crate::leaderboard::NarrationTicket::from_raw(0),` | C `--test forward_narration_relay` | F6, K2b; relay `narration_outcome_relay_ready_yields_message_and_sets_state`, `narration_outcome_relay_fellback_yields_message_and_sets_state`, the two-runs test, `narration_outcome_recipe_stream_takes_receiver_once_and_relays` | S0-4, upd |
//! | T6 | `ui::live::narration_request_from_mirror` | `request_id: ticket.raw(),` | → `request_id: 0,` | C | F6, at "F6: the request carries the ticket in flight" (0, not the first ticket, 1; its wrong-id twin never runs); F7, at "F7 [open]: the queued request carries the ticket" (its `[full]` filler check, 0 not 99, is never reached) | F8, S0-5, upd, K2b |
//! | T7 | `ui::live::update_and_dispatch_explain` | `Some(Err(TrySendError::Full(_))) => ExplainRefusal::Full,` | → `Some(Err(TrySendError::Full(_))) => return ExplainDispatch::Sent,` | C | F7, at "F7 [full]: a full channel refuses as Full" | F6, F8, S0-5 (F7's `[open]` row passes before it; F7 stops at the mutated row) |
//! | T8 | same | `Some(Err(TrySendError::Closed(_))) => ExplainRefusal::Closed,` | → `Some(Err(TrySendError::Closed(_))) => return ExplainDispatch::Sent,` | C Rn | F7, at "F7 [closed]: a closed channel refuses as Closed"; S0-5; R2 | F6, F8 (F7's `[open]` and `[full]` rows pass before it) |
//! | T11 | same | `None => ExplainRefusal::Unwired,` | → `None => return ExplainDispatch::Sent,` | C | F7, at "F7 [no channel]: no channel refuses as Unwired" | F6, F8, S0-5 (F7's three earlier rows pass before it) |
//! | T9 | same | `if before == Some(ticket) {` | → `if false {` (a re-press sends again) | C | F8 | F6, F7, S0-5 |
//! | T12 | same | same line | → `if st.narration.in_flight_ticket() == Some(ticket) {` (nothing is ever sent) | C Rn | F6, F7 (at its `[open]` row, the first), F8, S0-5, R2 | upd, S0-4, K2b |
//! | T13 | `ui::leaderboard::LeaderboardScreenState::fall_back_if_in_flight` | `if self.narration.in_flight_ticket().is_none() {` | → `if false {` (a task end overwrites any state) | `--lib` C | K2b; lib `fall_back_if_in_flight_touches_only_in_flight` | S0-4 |
//! | T14 | `ui::live::narration_outcome_stream_impl` | `yield Message::BakeoffNarrationTaskEnded;` | delete the line | C Rn `--test forward_narration_relay --test subscription_batch_inclusion --test recipe_off_runtime_poll` | S0-4, K2b, R3; the four relay stream-end checks (ready, fellback, two-runs, recipe); `batched_narration_outcome_recipe_relays`; `narration_outcome_recipe_drains_off_runtime` | upd |
//!
//! T5 also turns the echo checks red in `advisor_recipe_state_gating.rs`,
//! `subscription_batch_inclusion.rs` and `recipe_off_runtime_poll.rs`, which its
//! run does not include. The relay file's own rows (P4, P4b, P16 and the rest)
//! are in `forward_narration_relay.rs`.
//!
//! Gated on `live` (the build where `ui::live` and the `agent` types exist),
//! NOT on `target_os` (no pixels here: pure state and channel logic).

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use futures::StreamExt;
use futures::stream::BoxStream;
use smol_str::SmolStr;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::time::timeout;

use ui::leaderboard::{BakeoffReportMirror, NarrationOutcome, NarrationState, NarrationTicket};
use ui::live::{ExplainDispatch, ExplainRefusal, update_and_dispatch_explain};
use ui::state::{Cockpit, Message, PanelState, update};

/// The answer to bake-off A's Explain. It names A's winner and A's numbers.
const PROSE_A: &str = "SMA crossover came out on top here, with a Sharpe of 1.42 against 0.73 \
                       for simply holding the coin over this window.";
/// The answer to bake-off B's Explain. Differs from [`PROSE_A`].
const PROSE_B: &str = "Simply holding the coin came out on top here; no active strategy beat \
                       buy-and-hold on risk-adjusted return over this window.";

// ── Builders ──────────────────────────────────────────────────────────────────

/// The cockpit every test starts from: the Leaderboard screen, nothing on it.
fn cockpit() -> Cockpit {
    ui::fixtures::fake_cockpit_leaderboard(PanelState::Empty)
}

/// Bake-off A: `ActiveWins`, `v0.sma` crowned.
fn bakeoff_a() -> BakeoffReportMirror {
    ui::fixtures::fake_bakeoff_report_mirror()
}

/// Bake-off B: `BenchmarkWins`, a different crown and field.
fn bakeoff_b() -> BakeoffReportMirror {
    ui::fixtures::fake_bakeoff_report_mirror_benchmark_wins()
}

/// Press Run and land `mirror`, through `update`, as the binary does.
fn run(cockpit: &mut Cockpit, mirror: BakeoffReportMirror) {
    update(cockpit, Message::BakeoffRunRequested);
    update(cockpit, Message::BakeoffRunCompleted(Ok(mirror.clone())));
    assert!(
        matches!(&cockpit.leaderboard_screen_state.result, PanelState::Ready(m) if *m == mirror),
        "precondition: the bake-off landed Ready"
    );
}

/// The narration on screen.
fn narration(cockpit: &Cockpit) -> &NarrationState {
    &cockpit.leaderboard_screen_state.narration
}

/// The ticket in flight, or a panic naming the state instead.
fn in_flight(cockpit: &Cockpit, what: &str) -> NarrationTicket {
    narration(cockpit)
        .in_flight_ticket()
        .unwrap_or_else(|| panic!("{what}: expected InFlight, got {:?}", narration(cockpit)))
}

/// Press Explain through `update` and return the ticket it put in flight.
fn explain(cockpit: &mut Cockpit) -> NarrationTicket {
    update(cockpit, Message::BakeoffNarrationRequested);
    in_flight(
        cockpit,
        "precondition: Explain on a Ready result was accepted",
    )
}

/// The message the relay yields for an answer to `ticket`.
fn completed(ticket: NarrationTicket, outcome: NarrationOutcome) -> Message {
    Message::BakeoffNarrationCompleted(ticket, outcome)
}

fn ready(prose: &str) -> NarrationOutcome {
    NarrationOutcome::Ready(SmolStr::new(prose))
}

/// B's hand-off export, as the Plan screen's "Export this plan" writes it.
fn export_of(cockpit: &Cockpit) -> String {
    let st = &cockpit.leaderboard_screen_state;
    let PanelState::Ready(report) = &st.result else {
        panic!("precondition: a bake-off is on screen");
    };
    ui::export::serialize_plan_export(
        &ui::fixtures::fake_forward_plan(),
        report,
        &st.narration,
        None,
    )
}

/// Explain on A, Run B, B lands, THEN A's answer arrives (a real provider takes
/// seconds). The state S0-1 and S0-3 read.
fn answer_for_a_arrives_on_b() -> Cockpit {
    let mut cockpit = cockpit();
    run(&mut cockpit, bakeoff_a());
    let t_a = explain(&mut cockpit);
    run(&mut cockpit, bakeoff_b());
    assert_eq!(
        *narration(&cockpit),
        NarrationState::NotRequested,
        "precondition: running B reset A's narration (begin_run)"
    );
    update(&mut cockpit, completed(t_a, ready(PROSE_A)));
    cockpit
}

/// Await the next item, FAILING the test if nothing resolves in `secs` (the
/// `forward_narration_relay.rs` helper).
async fn next_or_fail(
    stream: &mut BoxStream<'static, Message>,
    secs: u64,
    what: &str,
) -> Option<Message> {
    timeout(Duration::from_secs(secs), stream.next())
        .await
        .unwrap_or_else(|_| panic!("timed out after {secs}s waiting for {what}"))
}

/// Drain a stream whose senders are already dropped, until its END, each step
/// under a failing timeout (the `forward_narration_relay.rs` helper).
async fn drain_to_end(stream: &mut BoxStream<'static, Message>, what: &str) -> Vec<Message> {
    let mut out = Vec::new();
    loop {
        match next_or_fail(stream, 2, what).await {
            Some(msg) => out.push(msg),
            None => return out,
        }
        assert!(out.len() <= 64, "{what}: stream did not end after 64 items");
    }
}

// ── Step 0 ────────────────────────────────────────────────────────────────────

/// **S0-1.** Explain on A, Run B, B lands, then A's answer: B's narration must
/// stay `NotRequested`. Before the fix `set_narration` stored it, so A's prose
/// showed under B's table. Its landing twin is F1.
#[test]
fn an_answer_for_bakeoff_a_does_not_land_on_bakeoff_b() {
    let cockpit = answer_for_a_arrives_on_b();
    assert_eq!(
        *narration(&cockpit),
        NarrationState::NotRequested,
        "S0-1 (bug-log #202): bake-off A's answer landed on bake-off B"
    );
}

/// **S0-3.** The same sequence, read through the durable consumer: B's
/// hand-off export (`serialize_plan_export` embeds a `Ready` narration) must
/// not carry A's prose. Its two-sided twin is F1's export check.
#[test]
fn bakeoff_b_export_never_carries_bakeoff_a_prose() {
    let cockpit = answer_for_a_arrives_on_b();
    assert!(
        !export_of(&cockpit).contains(PROSE_A),
        "S0-3 (bug-log #202): bake-off B's hand-off export carries bake-off A's prose"
    );
}

/// **S0-4 (K2a).** Explain is in flight and the narration task ends (its reply
/// sender drops, as on a kill-switch halt). Every message the real relay yields
/// is applied; the narration must end at `FellBack` (ADR-0064 § D6). Before the
/// fix the stream ended with nothing, so it stayed `InFlight` until the next
/// Run.
#[tokio::test]
async fn a_closed_outcome_channel_falls_back_an_explain_in_flight() {
    let mut cockpit = cockpit();
    run(&mut cockpit, bakeoff_a());
    explain(&mut cockpit);
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::NarrationReply>(4);
    let mut stream = ui::live::narration_outcome_stream_impl(Some(rx));
    drop(tx);
    for msg in drain_to_end(&mut stream, "the relay after its sender dropped").await {
        update(&mut cockpit, msg);
    }
    assert_eq!(
        *narration(&cockpit),
        NarrationState::FellBack,
        "S0-4 (bug-log #202, ADR-0064 § D6): the narration task ended with an Explain in flight"
    );
}

/// **S0-5 (bug-log #202, ADR-0064 § D6).** Explain while the request channel is
/// closed (the narration task has ended), through the dispatch the binary calls
/// in place of `update`: it must report the refusal AND land `FellBack`, so the
/// panel never spins on a request nothing will answer.
#[test]
fn a_closed_request_channel_falls_back() {
    let mut cockpit = cockpit();
    run(&mut cockpit, bakeoff_a());
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::narration::NarrationRequest>(4);
    drop(rx);
    let dispatch = ui::live::update_and_dispatch_explain(
        &mut cockpit,
        Message::BakeoffNarrationRequested,
        Some(&tx),
    );
    assert_eq!(
        dispatch,
        ui::live::ExplainDispatch::Refused(ui::live::ExplainRefusal::Closed),
        "S0-5 (bug-log #202): a closed request channel refuses the Explain as Closed"
    );
    assert_eq!(
        cockpit.leaderboard_screen_state.narration,
        NarrationState::FellBack,
        "S0-5 (bug-log #202, ADR-0064 § D6): a refused Explain must land FellBack, not spin"
    );
}

// ── Correlation through `update` ──────────────────────────────────────────────

/// **F1.** S0-1 without the Run in between: the answer to the request on
/// screen lands, for both outcomes. The `Ready` one reaches A's export (S0-3's
/// two-sided twin: the export does carry a landed prose).
#[test]
fn an_answer_for_the_request_on_screen_lands() {
    let mut landed = cockpit();
    run(&mut landed, bakeoff_a());
    let t_a = explain(&mut landed);
    update(&mut landed, completed(t_a, ready(PROSE_A)));
    assert_eq!(
        *narration(&landed),
        NarrationState::Ready(SmolStr::new(PROSE_A)),
        "F1: the answer to the request on screen lands (Ready)"
    );
    assert!(
        export_of(&landed).contains(PROSE_A),
        "F1: a landed prose reaches the export (S0-3's twin)"
    );

    let mut fell_back = cockpit();
    run(&mut fell_back, bakeoff_a());
    let t_a = explain(&mut fell_back);
    update(&mut fell_back, completed(t_a, NarrationOutcome::FellBack));
    assert_eq!(
        *narration(&fell_back),
        NarrationState::FellBack,
        "F1: the answer to the request on screen lands (FellBack)"
    );
}

/// **F2.** Explain on A, then Run B, and A's answer arrives while B is still
/// `Loading`: it is dropped, and B landing does not revive it.
#[test]
fn a_late_answer_while_the_next_bakeoff_runs_is_dropped() {
    let mut cockpit = cockpit();
    run(&mut cockpit, bakeoff_a());
    let t_a = explain(&mut cockpit);
    update(&mut cockpit, Message::BakeoffRunRequested);
    assert!(
        matches!(cockpit.leaderboard_screen_state.result, PanelState::Loading),
        "precondition: bake-off B is running"
    );
    update(&mut cockpit, completed(t_a, ready(PROSE_A)));
    assert_eq!(
        *narration(&cockpit),
        NarrationState::NotRequested,
        "F2: A's answer during B's run is dropped"
    );
    update(&mut cockpit, Message::BakeoffRunCompleted(Ok(bakeoff_b())));
    assert_eq!(
        *narration(&cockpit),
        NarrationState::NotRequested,
        "F2: B landing leaves its narration NotRequested"
    );
}

/// **F3, the queued case.** Explain on A, Run B, B lands, Explain on B. The
/// sequential task serves A's request first, so A's answer arrives while B's
/// is in flight: it must leave `InFlight(t_B)`. Then B's own answer lands. A
/// pending flag that `begin_run` clears fails here (A's answer would land as
/// B's), and so does a counter `begin_run` resets (T10).
#[test]
fn explain_on_b_ignores_a_queued_answer_for_a() {
    let mut cockpit = cockpit();
    run(&mut cockpit, bakeoff_a());
    let t_a = explain(&mut cockpit);
    run(&mut cockpit, bakeoff_b());
    let t_b = explain(&mut cockpit);
    update(&mut cockpit, completed(t_a, ready(PROSE_A)));
    assert_eq!(
        *narration(&cockpit),
        NarrationState::InFlight(t_b),
        "F3: A's queued answer must not land on B's request in flight"
    );
    update(&mut cockpit, completed(t_b, ready(PROSE_B)));
    assert_eq!(
        *narration(&cockpit),
        NarrationState::Ready(SmolStr::new(PROSE_B)),
        "F3: B's own answer lands"
    );
}

/// **F4.** One request, two answers that differ: the first lands, the second
/// is dropped. The entry's generation-counter option (bumped by `begin_run`)
/// would let the second land; the ticket names the request, not the run.
#[test]
fn a_second_answer_to_one_request_is_dropped() {
    const P1: &str = "The first answer to this request.";
    const P2: &str = "A second answer to the same request, which must not replace the first.";
    assert_ne!(P1, P2, "precondition: the two answers differ");
    let mut cockpit = cockpit();
    run(&mut cockpit, bakeoff_a());
    let t = explain(&mut cockpit);
    update(&mut cockpit, completed(t, ready(P1)));
    assert_eq!(
        *narration(&cockpit),
        NarrationState::Ready(SmolStr::new(P1)),
        "precondition: the first answer landed"
    );
    update(&mut cockpit, completed(t, ready(P2)));
    assert_eq!(
        *narration(&cockpit),
        NarrationState::Ready(SmolStr::new(P1)),
        "F4: a second answer to one request is dropped"
    );
}

// ── The dispatch and the reply path ───────────────────────────────────────────

/// **F6, the round trip.** The dispatch sends a request carrying the ticket in
/// flight. A reply under ANOTHER id (the twin) relays but does not land; the
/// request's own `reply` relays and lands. Both go through the real
/// `narration_outcome_stream_impl` and `update`.
#[tokio::test]
async fn explain_round_trip_lands_its_own_answer() {
    let mut cockpit = cockpit();
    run(&mut cockpit, bakeoff_a());
    let (req_tx, mut req_rx) = tokio::sync::mpsc::channel::<agent::narration::NarrationRequest>(4);
    let dispatch = update_and_dispatch_explain(
        &mut cockpit,
        Message::BakeoffNarrationRequested,
        Some(&req_tx),
    );
    assert_eq!(
        dispatch,
        ExplainDispatch::Sent,
        "F6: an open channel takes the Explain"
    );
    let t = in_flight(&cockpit, "F6: the dispatched Explain");
    let request = req_rx
        .try_recv()
        .expect("F6: the dispatch queued its request");
    assert_eq!(
        request.request_id,
        t.raw(),
        "F6: the request carries the ticket in flight"
    );

    let (out_tx, out_rx) = tokio::sync::mpsc::channel::<agent::NarrationReply>(4);
    let mut stream = ui::live::narration_outcome_stream_impl(Some(out_rx));

    out_tx
        .send(agent::NarrationReply {
            request_id: request.request_id.wrapping_add(1),
            outcome: agent::NarrationOutcome::Ready(SmolStr::new(PROSE_B)),
        })
        .await
        .expect("the relay holds the reply receiver");
    let wrong = next_or_fail(&mut stream, 2, "the wrong-id reply")
        .await
        .expect("the relay yielded the wrong-id reply");
    update(&mut cockpit, wrong);
    assert_eq!(
        *narration(&cockpit),
        NarrationState::InFlight(t),
        "F6 twin: a reply under another request's id does not land"
    );

    out_tx
        .send(request.reply(agent::NarrationOutcome::Ready(SmolStr::new(PROSE_A))))
        .await
        .expect("the relay holds the reply receiver");
    let own = next_or_fail(&mut stream, 2, "the request's own reply")
        .await
        .expect("the relay yielded the request's own reply");
    update(&mut cockpit, own);
    assert_eq!(
        *narration(&cockpit),
        NarrationState::Ready(SmolStr::new(PROSE_A)),
        "F6: the request's own reply lands"
    );
}

/// **F7, every dispatch outcome.** From A on screen, one press each: an open
/// channel is `Sent` (and stays `InFlight`); a full channel, a closed one and
/// no channel are each `Refused` with their kind, and each lands `FellBack`.
/// The full channel still holds only what filled it.
#[test]
fn explain_dispatch_refusals() {
    // Open: Sent, InFlight, and the queued request carries the ticket.
    let mut open = cockpit();
    run(&mut open, bakeoff_a());
    let (tx, mut rx) = tokio::sync::mpsc::channel::<agent::narration::NarrationRequest>(4);
    let dispatch =
        update_and_dispatch_explain(&mut open, Message::BakeoffNarrationRequested, Some(&tx));
    assert_eq!(
        dispatch,
        ExplainDispatch::Sent,
        "F7 [open]: the Explain is sent"
    );
    let t = in_flight(&open, "F7 [open]");
    assert_eq!(
        rx.try_recv()
            .expect("F7 [open]: one request queued")
            .request_id,
        t.raw(),
        "F7 [open]: the queued request carries the ticket"
    );

    // Full: a depth-1 channel already holding another request.
    let mut full = cockpit();
    run(&mut full, bakeoff_a());
    let (tx, mut rx) = tokio::sync::mpsc::channel::<agent::narration::NarrationRequest>(1);
    tx.try_send(ui::live::narration_request_from_mirror(
        &bakeoff_a(),
        NarrationTicket::from_raw(99),
    ))
    .expect("precondition: the filler request took the only slot");
    let dispatch =
        update_and_dispatch_explain(&mut full, Message::BakeoffNarrationRequested, Some(&tx));
    assert_eq!(
        dispatch,
        ExplainDispatch::Refused(ExplainRefusal::Full),
        "F7 [full]: a full channel refuses as Full"
    );
    assert_eq!(
        *narration(&full),
        NarrationState::FellBack,
        "F7 [full]: FellBack"
    );
    assert_eq!(
        rx.try_recv()
            .expect("F7 [full]: the filler is still queued")
            .request_id,
        99,
        "F7 [full]: the slot holds the filler"
    );
    assert!(
        matches!(rx.try_recv(), Err(TryRecvError::Empty)),
        "F7 [full]: the refused request was not queued"
    );

    // Closed: the receiver is gone.
    let mut closed = cockpit();
    run(&mut closed, bakeoff_a());
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::narration::NarrationRequest>(4);
    drop(rx);
    let dispatch =
        update_and_dispatch_explain(&mut closed, Message::BakeoffNarrationRequested, Some(&tx));
    assert_eq!(
        dispatch,
        ExplainDispatch::Refused(ExplainRefusal::Closed),
        "F7 [closed]: a closed channel refuses as Closed"
    );
    assert_eq!(
        *narration(&closed),
        NarrationState::FellBack,
        "F7 [closed]: FellBack"
    );

    // No channel wired.
    let mut unwired = cockpit();
    run(&mut unwired, bakeoff_a());
    let dispatch =
        update_and_dispatch_explain(&mut unwired, Message::BakeoffNarrationRequested, None);
    assert_eq!(
        dispatch,
        ExplainDispatch::Refused(ExplainRefusal::Unwired),
        "F7 [no channel]: no channel refuses as Unwired"
    );
    assert_eq!(
        *narration(&unwired),
        NarrationState::FellBack,
        "F7 [no channel]: FellBack"
    );
}

/// **F8.** A second press while the first request is in flight starts nothing:
/// `Sent`, then `Idle`, and exactly one request is queued.
#[test]
fn a_repressed_explain_sends_one_request() {
    let mut cockpit = cockpit();
    run(&mut cockpit, bakeoff_a());
    let (tx, mut rx) = tokio::sync::mpsc::channel::<agent::narration::NarrationRequest>(4);
    let first =
        update_and_dispatch_explain(&mut cockpit, Message::BakeoffNarrationRequested, Some(&tx));
    let t = in_flight(&cockpit, "F8: the first press");
    let second =
        update_and_dispatch_explain(&mut cockpit, Message::BakeoffNarrationRequested, Some(&tx));
    assert_eq!(
        (first, second),
        (ExplainDispatch::Sent, ExplainDispatch::Idle),
        "F8: the first press sends, the re-press does not"
    );
    assert_eq!(
        *narration(&cockpit),
        NarrationState::InFlight(t),
        "F8: the re-press keeps the request in flight"
    );
    assert!(
        rx.try_recv().is_ok(),
        "F8: the first press queued a request"
    );
    assert!(
        matches!(rx.try_recv(), Err(TryRecvError::Empty)),
        "F8: the re-press queued no second request"
    );
}

/// **K2b.** The task answers, THEN ends: the relay yields exactly the answer
/// and one `BakeoffNarrationTaskEnded`, and applying both keeps the answer (the
/// task end changes only an `InFlight` narration).
#[tokio::test]
async fn the_task_ending_after_its_answer_keeps_the_answer() {
    let mut cockpit = cockpit();
    run(&mut cockpit, bakeoff_a());
    let t = explain(&mut cockpit);
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::NarrationReply>(4);
    let mut stream = ui::live::narration_outcome_stream_impl(Some(rx));
    tx.send(agent::NarrationReply {
        request_id: t.raw(),
        outcome: agent::NarrationOutcome::Ready(SmolStr::new(PROSE_A)),
    })
    .await
    .expect("the relay holds the reply receiver");
    drop(tx);

    let msgs = drain_to_end(&mut stream, "the answer, then the task end").await;
    assert!(
        matches!(
            msgs.as_slice(),
            [
                Message::BakeoffNarrationCompleted(ticket, NarrationOutcome::Ready(p)),
                Message::BakeoffNarrationTaskEnded,
            ] if *ticket == t && p.as_str() == PROSE_A
        ),
        "K2b: the answer under its ticket, then one task end (got {msgs:?})"
    );
    for msg in msgs {
        update(&mut cockpit, msg);
    }
    assert_eq!(
        *narration(&cockpit),
        NarrationState::Ready(SmolStr::new(PROSE_A)),
        "K2b: the task end keeps a landed answer"
    );
}
