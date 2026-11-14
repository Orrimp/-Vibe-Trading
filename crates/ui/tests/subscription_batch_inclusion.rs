//! The cockpit's subscription batch, tested through the function production
//! calls (story 2-63, lab-recipe-test-harness v0.3, Waves C1/C2 S2).
//!
//! ## What this file drives
//!
//! Since Wave C1, `cockpit_live`'s `subscription()` returns
//! `ui::live::subscription_batch(&ctx)`: the bin only maps `AppState` fields
//! into a `ui::live::SubscriptionCtx`. The batch tests here call that same
//! function, or the descriptor and the `SubscriptionCtx::inputs` it is built
//! from (`ui::live::build_subscription_batch_descriptor_from`), and read the
//! batch through iced 0.14's public introspection, `Subscription::units()`. The
//! two `esc_closes_modal` tests call the fn both bins batch for the modal's
//! `Esc`, and the legacy pin calls the kept 3-arg descriptor.
//!
//! This file replaces three v0.2 files, deleted in Wave C1:
//! `crates/ui/tests/cockpit_subscription_server_time_always_batched.rs`,
//! `crates/ui/tests/cockpit_subscription_toast_dismiss_always_batched.rs` and
//! `crates/ui/tests/trail_mirror_subscription_handle_gating.rs`. All three
//! asserted membership in the 3-arg descriptor and nothing else. The bin
//! converted that descriptor itself and pushed five more recipes outside it, so
//! no test executed a conversion arm, and the comment in `ui::live` that a
//! diverging conversion would turn them red could not be true (bug-log `#203`).
//! Their premise that the batch "cannot be introspected" was also false:
//! `units()` and `iced::advanced::subscription::into_recipes` are public.
//!
//! ## What this file does not pin
//!
//! - **The bin's `SubscriptionCtx` literal (story 2-63 K3 #6).** `AppState`
//!   cannot be built in a test, so the literal in `cockpit_live`'s
//!   `subscription()` is checked by nothing automated: `cockpit-smoke` boots
//!   only the fixtures `cockpit` bin, never `cockpit_live`, and synthesises no
//!   input. A wrong literal compiles: the bake-off and sweep cells share one
//!   type (`ui::live::BakeoffProgressCell`), the four `u64` salts are
//!   interchangeable, and any `Option` field can be passed as `None`. Probes
//!   C1-RES1 and C1-RES2 below MEASURE this residual: both leave every target
//!   green, and are recorded as named survivors. The orchestrator's
//!   human-verification walk of `cockpit_live` catches a `None` passed for a
//!   cell a screen uses (C1-RES1), but NOT a swapped salt (C1-RES2): the bin
//!   clears each take-once cell when its run ends, so iced drops the recipe's id
//!   and the next run streams afresh whatever its salt. A swapped salt is
//!   checked by nothing, the walk included.
//! - **The fixtures `cockpit` bin's own `listen_with(esc_closes_modal)`.** That
//!   bin needs `--features fixtures`, so default-feature test runs never
//!   compile it, and `cockpit-smoke` synthesises no input. This file pins the fn
//!   it batches, not that it batches it.
//! - **Liveness of a batched unit.** In Wave C1 every arm is pinned on
//!   PRESENCE only (unit counts, per context). That a batched unit is wired to
//!   the ctx's own cell, bus and salt, and relays, is Wave C2's
//!   `batched_*_recipe_relays`. Until then that includes the two `Recipe::stream`
//!   bodies C1 moved into `ui::live` (`ServerTimeRecipe`, `ToastDismissRecipe`).
//! - **`build_subscription_batch_descriptor` (3 args) as production wiring.**
//!   Production no longer calls it. It stays for API compatibility (story 2-63
//!   decision 5), so its test and probe C1-P1 measure a COMPATIBILITY pin.
//!
//! ## Coverage
//!
//! | test | what it pins |
//! |---|---|
//! | `legacy_three_arg_descriptor_is_pinned_to_its_literal_order` | COMPATIBILITY pin: for all 8 inputs, the kept 3-arg fn returns exactly its written-out v0.2 list, so a leaked v0.3 input shows. Not compared against `_from(..)`, which it delegates to (that would be self-referential, bug-log `#161` / `#187`) |
//! | `descriptor_membership_keys_each_variant_on_its_own_input` | `build_subscription_batch_descriptor_from` over all 256 input combinations: each optional variant present iff ITS input is `Held::Yes`, the four always-on ones always, in the canonical order, each at most once |
//! | `ctx_inputs_key_on_outer_option_not_inner_receiver` | `SubscriptionCtx::inputs` keys each take-once cell on the OUTER `Option`: a cell whose receiver was already taken still gives `Held::Yes`, and `subscription_batch` still builds its unit (15 units for each of the six cells alone, 20 for all six), so the recipe stays batched (a frame without it makes iced drop the running stream, the only holder of that receiver) |
//! | `ctx_inputs_key_each_field_on_its_own_option` | `SubscriptionCtx::inputs` maps each of the 8 gated ctx fields to ITS input, and only to it |
//! | `subscription_batch_units_match_descriptor` | `subscription_batch`'s unit count, per context, against a written-out literal (14 / 15 / 22) AND against the descriptor (`11 + len - 1`): a descriptor entry whose arm builds nothing, and an arm dropped from the batch, each fail one half |
//! | `escape_keypress_maps_to_modal_closed` | `ui::widgets::journal_transaction_modal::esc_closes_modal`, the fn both cockpits batch while the tape-audit modal is open: an `Esc` PRESS closes the modal, whatever its event status |
//! | `non_escape_events_map_to_none` | the negative half: another key, an `Esc` RELEASE, and a non-key event close nothing, at both event statuses |
//! | (C2) `batched_*_recipe_relays`, `forward_plan_stays_batched_after_receiver_taken`, the identity tests | Wave C2: a batched unit is live and frame-stable. Not in this file yet |
//!
//! ## Non-vacuity contract (bug-log #66 / story 2-63 AC3)
//!
//! Every assertion names its row or case (a mask, a ctx label, an input
//! triple, a status), so the first failing row is in the message. The literal
//! expectations never come from the subject: `inputs_from_mask` (field → bit)
//! and `expected_for_mask` (variant → bit) are two independent tables keyed by
//! NAME; the unit literals 14 / 15 / 20 / 22 are written out; the 8
//! single-field expectations are a second table, keyed by label. The units
//! test's AGREEMENT half deliberately cross-checks two parts of the subject
//! (batch against descriptor); its literal half runs first. Every dimension is
//! two-sided: a field at `Some` and at `None`, a taken cell and a held one, a
//! bit set and clear, an `Esc` press and an `Esc` release, each `Esc` case at
//! both event statuses. Checks that hold by construction (no literal legacy row
//! lists a v0.3 variant; each optional variant appears in 128 of the 256
//! masks) are comments, not asserts; the one oracle self-check (each input bit
//! keys one variant) checks the hand-written table, not the subject. No test
//! here runs a stream, so none needs a timeout; the tokio runtime exists only
//! for its `Handle`.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4, Wave C1)
//!
//! Each probe is ONE edit to production code, located by symbol + the quoted
//! line text (line numbers rot). A `\|` in a table cell is a literal `|`.
//! Procedure:
//!
//! 1. snapshot the file (`cp -p <file> <snap>`) and apply the edit;
//! 2. run `cargo test -p ui --test subscription_batch_inclusion`. The named
//!    test(s) go RED (for C1-RES1 / C1-RES2: nothing does, which is the
//!    measurement);
//! 3. restore from the snapshot, THEN `touch <file>`. A restore that keeps the
//!    snapshot's older mtime (`cp -p`) leaves cargo's fingerprint "fresh": the
//!    next run compiles nothing and re-executes the MUTANT binary;
//! 4. re-run. The GREEN counts only if that run's output shows
//!    `Compiling ui` and the file's SHA-256 equals the pre-probe one.
//!
//! Each test stops at its first failing assertion, so "at" names the FIRST
//! failing row. Probes C1-P1..C1-P8, C1-P11..C1-P13 edit
//! `crates/ui/src/live.rs`; C1-P9, C1-P10, C1-P14 and C1-P15
//! `crates/ui/src/widgets/journal_transaction_modal.rs`; C1-RES1 and C1-RES2
//! `crates/ui/src/bin/cockpit_live.rs`.
//!
//! | # | symbol | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | C1-P1 | `ui::live::build_subscription_batch_descriptor` (the 3-arg wrapper) | `training_log: has_training_log.into(),` (the line above `..SubscriptionBatchInputs::default()`) | insert `forward_plan: Held::Yes,` between the two lines | `legacy_three_arg_descriptor_is_pinned_to_its_literal_order`, at its first row, "legacy 3-arg descriptor (false,false,false)" (`ForwardPlan` leaks in). `descriptor_membership_keys_each_variant_on_its_own_input` stays GREEN: it calls `_from` directly. A COMPATIBILITY-pin probe, not a production-wiring probe |
//! | C1-P2 | `ui::live::build_subscription_batch_descriptor_from` | `if inputs.sweep_progress == Held::Yes {` and the two lines under it | delete the three lines | `descriptor_membership_keys_each_variant_on_its_own_input`, at "descriptor for mask 0b01000000"; `subscription_batch_units_match_descriptor`, at its LITERAL half, "units for ctx `sweep`" (14, wanted 15). Its agreement half holds (descriptor and batch both drop the unit). `ctx_inputs_key_on_outer_option_not_inner_receiver`, at "the taken-receiver ctx's descriptor lists SweepProgress". The legacy pin stays GREEN |
//! | C1-P3 | `ui::live::build_subscription_batch_descriptor_from` | `if inputs.sweep_progress == Held::Yes {` and the two lines under it | → `desc.push(SubscriptionVariant::SweepProgress);` (unconditional) | the matrix, at "descriptor for mask 0b00000000"; the legacy pin, at "legacy 3-arg descriptor (false,false,false)"; the units test, at its AGREEMENT half, "descriptor⇔batch agreement for ctx `all-None`" (11 + 5 − 1 = 15, but 14 units). Its literal 14 holds: the Sweep arm builds nothing for a `None` field |
//! | C1-P4 | `ui::live::build_subscription_batch_descriptor_from` | `if inputs.bakeoff_progress == Held::Yes {` | → `if inputs.sweep_progress == Held::Yes {` (the bake-off variant keyed on the sweep input) | the matrix, at "descriptor for mask 0b00100000"; the units test, at its literal half, "units for ctx `bakeoff`" (14, wanted 15) |
//! | C1-P5 | `ui::live::build_subscription_batch_descriptor_from` | `if inputs.lab_progress == Held::Yes {` | → `if true {` (the story's probe (d), retargeted from the deleted legacy body) | the matrix, at "descriptor for mask 0b00000000"; the legacy pin, at "legacy 3-arg descriptor (false,false,false)"; the units test, at its agreement half, "descriptor⇔batch agreement for ctx `all-None`" (15, but 14 units) |
//! | C1-P6 | `ui::live::variant_subscription` (private, the descriptor → batch conversion) | `SubscriptionVariant::SweepProgress => match ctx.sweep_progress {` | → `SubscriptionVariant::SweepProgress => match None::<(&BakeoffProgressCell, u64)> {` (the arm never builds its recipe) | the units test, at its literal half, "units for ctx `sweep`" (14, wanted 15), while the descriptor still lists `SweepProgress`. The matrix stays GREEN: this is the descriptor⇔arm divergence no v0.2 test could see |
//! | C1-P7 | `ui::live::SubscriptionCtx::inputs` | `forward_plan: self.forward_plan.is_some().into(),` | → `forward_plan: self.forward_plan.is_some_and(\|c\| c.lock().is_ok_and(\|g\| g.is_some())).into(),` (keyed on the INNER receiver) | `ctx_inputs_key_on_outer_option_not_inner_receiver`, at "inputs() keys on the OUTER Option: forward_plan with a taken inner receiver". `ctx_inputs_key_each_field_on_its_own_option` stays GREEN (its inner receivers are present) |
//! | C1-P8 | `ui::live::subscription` (the Bus batch) | `subscription_for(Channel::MarketHealth, Arc::clone(&bus)),` | delete the line | `subscription_batch_units_match_descriptor`, at its first assertion, "ui::live::subscription batches one unit per bus channel" (10, wanted 11) |
//! | C1-P9 | `ui::widgets::journal_transaction_modal::esc_closes_modal` | `key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),` | `Named::Escape` → `Named::Enter` | `escape_keypress_maps_to_modal_closed`, at "Esc press (status Ignored) closes the modal"; `non_escape_events_map_to_none`, at "Enter press must not close the modal" |
//! | C1-P10 | `ui::widgets::journal_transaction_modal::esc_closes_modal` | `iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {` | `KeyPressed` → `KeyReleased` | `escape_keypress_maps_to_modal_closed`, at "Esc press (status Ignored) closes the modal"; `non_escape_events_map_to_none`, at "Esc release must not close the modal" |
//! | C1-P11 | `ui::live::SubscriptionCtx::inputs` | `modal_esc: self.tape_audit_modal_open.into(),` | → `modal_esc: Held::No,` | `ctx_inputs_key_each_field_on_its_own_option`, at "inputs() for ctx `modal`"; the units test, at its literal half, "units for ctx `modal`" (14, wanted 15) |
//! | C1-P12 | `ui::live::build_subscription_batch_descriptor_from` | `if inputs.trail == Held::Yes {` | → `if inputs.trail == Held::No {` | the matrix, at "descriptor for mask 0b00000000"; the legacy pin, at "legacy 3-arg descriptor (false,false,false)"; the units test, at its agreement half, "descriptor⇔batch agreement for ctx `all-None`" (15, but 14 units: the Trail arm builds nothing for a `None` handle) |
//! | C1-RES1 | `cockpit_live`'s `AppState::subscription` (`crates/ui/src/bin/cockpit_live.rs`) | `forward_plan: self.plan_rx.as_ref(),` | → `forward_plan: None,` | NONE: every target stays GREEN. The first measured K3 #6 survivor: the shipped cockpit would never batch the plan recipe, and the Plan screen would stay empty. Checked only by the human-verification walk (its "promote" step) |
//! | C1-RES2 | `cockpit_live`'s `AppState::subscription` | `.map(\|rx\| (rx, self.training_log_recipe_salt)),` | → `.map(\|rx\| (rx, self.lab_progress_recipe_salt)),` | NONE: every target stays GREEN. The second measured K3 #6 survivor: the four `u64` salts are interchangeable in the literal. A salt only matters when a new run replaces a cell that is still held; in the run-then-run-again flows the UI offers, the bin clears the cell when a run ends (`self.training_log_rx = None;`), iced drops the id on that frame, and the next run streams afresh. So no flow shows a symptom, and nothing checks this, the human-verification walk included |
//! | C1-P13 | `ui::live::variant_subscription` | `SubscriptionVariant::BakeoffProgress => match ctx.bakeoff_progress {` and the arm's 7 lines under it | the arm's `Some((rx, salt)) =>` gains the guard `if rx.lock().is_ok_and(\|g\| g.is_some())` and the arm ends `_ => iced::Subscription::none(),` (keyed on the INNER receiver; one contiguous replace) | `ctx_inputs_key_on_outer_option_not_inner_receiver`, at "units for a ctx holding a TAKEN bakeoff_progress cell" (14, wanted 15). Before the orchestrator's fix this was the falsifier's survivor OWN-1, 7/7 green: in production the bake-off bar would freeze one frame after its recipe took the receiver |
//! | C1-P14 | `ui::widgets::journal_transaction_modal::esc_closes_modal` | `}) => Some(Message::TapeAuditModalClosed),` | → `}) if _status == iced::event::Status::Ignored => Some(Message::TapeAuditModalClosed),` (a captured `Esc` no longer closes) | `escape_keypress_maps_to_modal_closed`, at "Esc press (status Captured) closes the modal" (the falsifier's OWN-2) |
//! | C1-P15 | `ui::widgets::journal_transaction_modal::esc_closes_modal` | `_ => None,` | insert `_ if _status == iced::event::Status::Captured => Some(Message::TapeAuditModalClosed),` above it (ANY captured event, such as a click inside the modal, closes it) | `non_escape_events_map_to_none`, at "Enter press (status Captured) must not close the modal". Before the orchestrator's fix this was the falsifier's survivor OWN-3m, 7/7 green |
//!
//! MEASURED, 2026-11-14, each restored SHA-identical with `Compiling ui` on the
//! re-run. The Wave C1 falsifier ran C1-P1..C1-P12, C1-RES1 and C1-RES2 one at
//! a time: each probe turned exactly the named tests red at the named rows (for
//! C1-P2 after this table added its third test), and both RES probes left every
//! target green, `cargo test -p ui --bin cockpit_live` included. Its own probes
//! OWN-1 and OWN-3m survived (7/7 green); the orchestrator then added the taken-
//! cell unit rows and the second status to the negatives, and re-measured them
//! as C1-P13 and C1-P15, red at the named rows. OWN-2 is C1-P14, red as named.
//!
//! Wave C2 adds its probes (C2-P1..C2-P20) to this table with its tests.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};

use agent::EventBus;
use agent::config::BusConfig;
use iced::event::Status;
use iced::keyboard::key::{Code, Named, Physical};
use iced::keyboard::{Key, Location, Modifiers};
use iced::window;
use reflection::trail_mirror::TrailMirrorHandle;
use ui::live::{
    BakeoffProgressCell, Channel, ForwardPlanCell, Held, LabProgressCell, NarrationOutcomeCell,
    SubscriptionBatchInputs, SubscriptionCtx, SubscriptionVariant as V, TrainingLogCell,
    build_subscription_batch_descriptor, build_subscription_batch_descriptor_from,
    subscription_batch,
};
use ui::state::Message;
use ui::widgets::journal_transaction_modal::esc_closes_modal;

// ── Tables, written out (NOT read from the subject) ─────────────────────────

/// The batch order, written out.
const CANONICAL: [V; 12] = [
    V::Bus,
    V::ServerTime,
    V::Trail,
    V::LabProgress,
    V::Activity,
    V::TrainingLog,
    V::ToastDismiss,
    V::ForwardPlan,
    V::NarrationOutcome,
    V::BakeoffProgress,
    V::SweepProgress,
    V::ModalEsc,
];

/// The 11 bus channels, in `ui::live::subscription`'s order, written out.
const BUS_CHANNELS: [Channel; 11] = [
    Channel::Fills,
    Channel::Positions,
    Channel::Pnl,
    Channel::Ticks,
    Channel::Bars,
    Channel::Mode,
    Channel::StrategyLoaded,
    Channel::StrategySwapped,
    Channel::StrategyError,
    Channel::MarketHealth,
    Channel::RiskTelemetry,
];

/// Each `Channel`'s index in [`BUS_CHANNELS`], by an exhaustive match: a 12th
/// `Channel` variant fails to COMPILE here until both tables are updated, so
/// the bus unit count this file expects cannot go stale silently.
const fn bus_channel_index(channel: Channel) -> usize {
    match channel {
        Channel::Fills => 0,
        Channel::Positions => 1,
        Channel::Pnl => 2,
        Channel::Ticks => 3,
        Channel::Bars => 4,
        Channel::Mode => 5,
        Channel::StrategyLoaded => 6,
        Channel::StrategySwapped => 7,
        Channel::StrategyError => 8,
        Channel::MarketHealth => 9,
        Channel::RiskTelemetry => 10,
    }
}

// Compile time: `BUS_CHANNELS` lists each channel exactly once.
const _: () = {
    let mut i = 0;
    while i < BUS_CHANNELS.len() {
        assert!(
            bus_channel_index(BUS_CHANNELS[i]) == i,
            "BUS_CHANNELS must list every Channel once, in match order"
        );
        i += 1;
    }
};

/// Bit `i` of `mask` → one input FIELD, by name. Independent of
/// [`input_bit_of`], which maps VARIANTS to bits.
fn inputs_from_mask(mask: u8) -> SubscriptionBatchInputs {
    let bit = |i: u8| Held::from(mask & (1 << i) != 0);
    SubscriptionBatchInputs {
        trail: bit(0),
        lab_progress: bit(1),
        training_log: bit(2),
        forward_plan: bit(3),
        narration_outcome: bit(4),
        bakeoff_progress: bit(5),
        sweep_progress: bit(6),
        modal_esc: bit(7),
    }
}

/// The oracle's variant → input bit, by name; `None` is an always-on variant.
fn input_bit_of(variant: V) -> Option<u8> {
    match variant {
        V::Bus | V::ServerTime | V::Activity | V::ToastDismiss => None,
        V::Trail => Some(0),
        V::LabProgress => Some(1),
        V::TrainingLog => Some(2),
        V::ForwardPlan => Some(3),
        V::NarrationOutcome => Some(4),
        V::BakeoffProgress => Some(5),
        V::SweepProgress => Some(6),
        V::ModalEsc => Some(7),
    }
}

/// The oracle: [`CANONICAL`] filtered by [`input_bit_of`].
fn expected_for_mask(mask: u8) -> Vec<V> {
    CANONICAL
        .iter()
        .copied()
        .filter(|&v| input_bit_of(v).is_none_or(|b| mask & (1 << b) != 0))
        .collect()
}

/// The inputs a single-field ctx must give, keyed by its label: a second table,
/// independent of [`Fixture::singles`], which builds the ctxs.
fn expected_single_inputs(label: &str) -> SubscriptionBatchInputs {
    let no = SubscriptionBatchInputs::default();
    match label {
        "trail" => SubscriptionBatchInputs {
            trail: Held::Yes,
            ..no
        },
        "lab" => SubscriptionBatchInputs {
            lab_progress: Held::Yes,
            ..no
        },
        "training" => SubscriptionBatchInputs {
            training_log: Held::Yes,
            ..no
        },
        "plan" => SubscriptionBatchInputs {
            forward_plan: Held::Yes,
            ..no
        },
        "narration" => SubscriptionBatchInputs {
            narration_outcome: Held::Yes,
            ..no
        },
        "bakeoff" => SubscriptionBatchInputs {
            bakeoff_progress: Held::Yes,
            ..no
        },
        "sweep" => SubscriptionBatchInputs {
            sweep_progress: Held::Yes,
            ..no
        },
        "modal" => SubscriptionBatchInputs {
            modal_esc: Held::Yes,
            ..no
        },
        other => panic!("no expected inputs for ctx `{other}`"),
    }
}

// ── Fixture ──────────────────────────────────────────────────────────────────

/// A receiver cell in the shape `cockpit_live` holds it before the first
/// `stream()`: the inner `Option` is `Some`.
fn held<T>(rx: T) -> Arc<Mutex<Option<T>>> {
    Arc::new(Mutex::new(Some(rx)))
}

/// A receiver cell after its recipe's first `stream()` took the receiver.
fn taken<T>() -> Arc<Mutex<Option<T>>> {
    Arc::new(Mutex::new(None))
}

/// Everything a [`SubscriptionCtx`] borrows. No stream runs in these tests, so
/// the runtime exists only for its `Handle`, and each sender is dropped at once.
struct Fixture {
    rt: tokio::runtime::Runtime,
    bus: Arc<EventBus>,
    trail: TrailMirrorHandle,
    lab: LabProgressCell,
    training: TrainingLogCell,
    plan: ForwardPlanCell,
    narration: NarrationOutcomeCell,
    bakeoff: BakeoffProgressCell,
    sweep: BakeoffProgressCell,
}

impl Fixture {
    fn new() -> Self {
        let (req_tx, _req_rx) = tokio::sync::mpsc::channel(1);
        let (tick_tx, _tick_rx) = tokio::sync::broadcast::channel(1);
        Self {
            rt: tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("a current-thread runtime, for its Handle"),
            bus: Arc::new(EventBus::new(&BusConfig::default())),
            trail: TrailMirrorHandle { req_tx, tick_tx },
            lab: held(tokio::sync::mpsc::channel(1).1),
            training: held(std::sync::mpsc::channel().1),
            plan: held(tokio::sync::mpsc::channel(1).1),
            narration: held(tokio::sync::mpsc::channel(1).1),
            bakeoff: held(tokio::sync::mpsc::channel(1).1),
            sweep: held(tokio::sync::mpsc::channel(1).1),
        }
    }

    /// Every optional input absent and the modal closed: `AppState` at boot,
    /// before any run.
    fn base(&self) -> SubscriptionCtx<'_> {
        SubscriptionCtx {
            rt_handle: self.rt.handle(),
            bus: &self.bus,
            trail: None,
            lab_progress: None,
            training_log: None,
            forward_plan: None,
            narration_outcome: None,
            bakeoff_progress: None,
            sweep_progress: None,
            tape_audit_modal_open: false,
        }
    }

    /// The 8 single-field ctxs: [`Fixture::base`] plus exactly one gated field.
    fn singles(&self) -> [(&'static str, SubscriptionCtx<'_>); 8] {
        let base = self.base();
        [
            (
                "trail",
                SubscriptionCtx {
                    trail: Some(&self.trail),
                    ..base
                },
            ),
            (
                "lab",
                SubscriptionCtx {
                    lab_progress: Some((&self.lab, 0)),
                    ..base
                },
            ),
            (
                "training",
                SubscriptionCtx {
                    training_log: Some((&self.training, 0)),
                    ..base
                },
            ),
            (
                "plan",
                SubscriptionCtx {
                    forward_plan: Some(&self.plan),
                    ..base
                },
            ),
            (
                "narration",
                SubscriptionCtx {
                    narration_outcome: Some(&self.narration),
                    ..base
                },
            ),
            (
                "bakeoff",
                SubscriptionCtx {
                    bakeoff_progress: Some((&self.bakeoff, 0)),
                    ..base
                },
            ),
            (
                "sweep",
                SubscriptionCtx {
                    sweep_progress: Some((&self.sweep, 0)),
                    ..base
                },
            ),
            (
                "modal",
                SubscriptionCtx {
                    tape_audit_modal_open: true,
                    ..base
                },
            ),
        ]
    }

    /// Every gated input present and the modal open.
    fn all_some(&self) -> SubscriptionCtx<'_> {
        SubscriptionCtx {
            rt_handle: self.rt.handle(),
            bus: &self.bus,
            trail: Some(&self.trail),
            lab_progress: Some((&self.lab, 0)),
            training_log: Some((&self.training, 0)),
            forward_plan: Some(&self.plan),
            narration_outcome: Some(&self.narration),
            bakeoff_progress: Some((&self.bakeoff, 0)),
            sweep_progress: Some((&self.sweep, 0)),
            tape_audit_modal_open: true,
        }
    }
}

// ── Keyboard events ──────────────────────────────────────────────────────────

fn key_pressed(named: Named, code: Code) -> iced::Event {
    iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
        key: Key::Named(named),
        modified_key: Key::Named(named),
        physical_key: Physical::Code(code),
        location: Location::Standard,
        modifiers: Modifiers::empty(),
        text: None,
        repeat: false,
    })
}

fn key_released(named: Named, code: Code) -> iced::Event {
    iced::Event::Keyboard(iced::keyboard::Event::KeyReleased {
        key: Key::Named(named),
        modified_key: Key::Named(named),
        physical_key: Physical::Code(code),
        location: Location::Standard,
        modifiers: Modifiers::empty(),
    })
}

// ── The descriptor ───────────────────────────────────────────────────────────

/// COMPATIBILITY pin (story 2-63 decision 5): production calls
/// `subscription_batch`, never the 3-arg fn. Each flag has a `true` row and a
/// `false` row with different expected lists. By construction no row lists a
/// v0.3 variant, so any leaked v0.3 input fails its row.
#[test]
fn legacy_three_arg_descriptor_is_pinned_to_its_literal_order() {
    let rows: [((bool, bool, bool), Vec<V>); 8] = [
        (
            (false, false, false),
            vec![V::Bus, V::ServerTime, V::Activity, V::ToastDismiss],
        ),
        (
            (true, false, false),
            vec![
                V::Bus,
                V::ServerTime,
                V::Trail,
                V::Activity,
                V::ToastDismiss,
            ],
        ),
        (
            (false, true, false),
            vec![
                V::Bus,
                V::ServerTime,
                V::LabProgress,
                V::Activity,
                V::ToastDismiss,
            ],
        ),
        (
            (false, false, true),
            vec![
                V::Bus,
                V::ServerTime,
                V::Activity,
                V::TrainingLog,
                V::ToastDismiss,
            ],
        ),
        (
            (true, true, false),
            vec![
                V::Bus,
                V::ServerTime,
                V::Trail,
                V::LabProgress,
                V::Activity,
                V::ToastDismiss,
            ],
        ),
        (
            (true, false, true),
            vec![
                V::Bus,
                V::ServerTime,
                V::Trail,
                V::Activity,
                V::TrainingLog,
                V::ToastDismiss,
            ],
        ),
        (
            (false, true, true),
            vec![
                V::Bus,
                V::ServerTime,
                V::LabProgress,
                V::Activity,
                V::TrainingLog,
                V::ToastDismiss,
            ],
        ),
        (
            (true, true, true),
            vec![
                V::Bus,
                V::ServerTime,
                V::Trail,
                V::LabProgress,
                V::Activity,
                V::TrainingLog,
                V::ToastDismiss,
            ],
        ),
    ];
    for ((trail, lab, training), want) in rows {
        assert_eq!(
            build_subscription_batch_descriptor(trail, lab, training),
            want,
            "legacy 3-arg descriptor ({trail},{lab},{training})"
        );
    }
}

/// All 2^8 input combinations, in order. Every bit has pairs of masks that
/// differ in that bit only, so a variant keyed on a wrong input, on none, or on
/// a constant fails at the first mask that separates them.
#[test]
fn descriptor_membership_keys_each_variant_on_its_own_input() {
    // Oracle self-check (of the hand-written table, not of the subject): each of
    // the 8 input bits keys exactly one variant, so the 256 masks below separate
    // all 8 dimensions. Two variants on one bit would leave a mis-keyed pair
    // unseparated, and the matrix would still pass.
    for bit in 0..8_u8 {
        let keyed: Vec<V> = CANONICAL
            .iter()
            .copied()
            .filter(|&v| input_bit_of(v) == Some(bit))
            .collect();
        assert_eq!(keyed.len(), 1, "oracle: input bit {bit} keys {keyed:?}");
    }
    for mask in 0..=u8::MAX {
        let desc = build_subscription_batch_descriptor_from(inputs_from_mask(mask));
        assert_eq!(
            desc,
            expected_for_mask(mask),
            "descriptor for mask {mask:#010b}"
        );
    }
    // By construction, not asserted: once every mask matches the oracle, each
    // optional variant is listed in 128 masks and each always-on one in 256.
}

// ── SubscriptionCtx::inputs ──────────────────────────────────────────────────

/// The post-take shape: every take-once cell is held (`Some`) with its inner
/// receiver gone, exactly as production holds it from the frame after the
/// recipe's first `stream()`. iced drops a recipe absent from a frame, and
/// with it the running stream, the only holder of that receiver.
#[test]
fn ctx_inputs_key_on_outer_option_not_inner_receiver() {
    let fx = Fixture::new();
    let lab: LabProgressCell = taken();
    let training: TrainingLogCell = taken();
    let plan: ForwardPlanCell = taken();
    let narration: NarrationOutcomeCell = taken();
    let bakeoff: BakeoffProgressCell = taken();
    let sweep: BakeoffProgressCell = taken();
    let ctx = SubscriptionCtx {
        lab_progress: Some((&lab, 0)),
        training_log: Some((&training, 0)),
        forward_plan: Some(&plan),
        narration_outcome: Some(&narration),
        bakeoff_progress: Some((&bakeoff, 0)),
        sweep_progress: Some((&sweep, 0)),
        ..fx.base()
    };
    let got = ctx.inputs();
    let rows: [(&str, Held, Held); 6] = [
        (
            "lab_progress",
            got.lab_progress,
            SubscriptionCtx {
                lab_progress: None,
                ..ctx
            }
            .inputs()
            .lab_progress,
        ),
        (
            "training_log",
            got.training_log,
            SubscriptionCtx {
                training_log: None,
                ..ctx
            }
            .inputs()
            .training_log,
        ),
        (
            "forward_plan",
            got.forward_plan,
            SubscriptionCtx {
                forward_plan: None,
                ..ctx
            }
            .inputs()
            .forward_plan,
        ),
        (
            "narration_outcome",
            got.narration_outcome,
            SubscriptionCtx {
                narration_outcome: None,
                ..ctx
            }
            .inputs()
            .narration_outcome,
        ),
        (
            "bakeoff_progress",
            got.bakeoff_progress,
            SubscriptionCtx {
                bakeoff_progress: None,
                ..ctx
            }
            .inputs()
            .bakeoff_progress,
        ),
        (
            "sweep_progress",
            got.sweep_progress,
            SubscriptionCtx {
                sweep_progress: None,
                ..ctx
            }
            .inputs()
            .sweep_progress,
        ),
    ];
    for (field, with_taken_cell, with_none) in rows {
        assert_eq!(
            with_taken_cell,
            Held::Yes,
            "inputs() keys on the OUTER Option: {field} with a taken inner receiver"
        );
        assert_eq!(with_none, Held::No, "inputs(): {field} passed as None");
    }
    let desc = build_subscription_batch_descriptor_from(got);
    for variant in [
        V::LabProgress,
        V::TrainingLog,
        V::ForwardPlan,
        V::NarrationOutcome,
        V::BakeoffProgress,
        V::SweepProgress,
    ] {
        assert!(
            desc.contains(&variant),
            "the taken-receiver ctx's descriptor lists {variant:?}: {desc:?}"
        );
    }
    // The batch itself, not only the flag: each ARM must build its unit for a
    // taken cell. One taken cell on the boot ctx is 14 + 1 = 15 units (literal,
    // as in `subscription_batch_units_match_descriptor`); all six are 20. An arm
    // keyed on the inner receiver builds nothing here, and in production the
    // running stream is dropped on the next frame (its progress bar freezes).
    let base = fx.base();
    let units: [(&str, SubscriptionCtx<'_>, usize); 7] = [
        (
            "lab_progress",
            SubscriptionCtx {
                lab_progress: Some((&lab, 0)),
                ..base
            },
            15,
        ),
        (
            "training_log",
            SubscriptionCtx {
                training_log: Some((&training, 0)),
                ..base
            },
            15,
        ),
        (
            "forward_plan",
            SubscriptionCtx {
                forward_plan: Some(&plan),
                ..base
            },
            15,
        ),
        (
            "narration_outcome",
            SubscriptionCtx {
                narration_outcome: Some(&narration),
                ..base
            },
            15,
        ),
        (
            "bakeoff_progress",
            SubscriptionCtx {
                bakeoff_progress: Some((&bakeoff, 0)),
                ..base
            },
            15,
        ),
        (
            "sweep_progress",
            SubscriptionCtx {
                sweep_progress: Some((&sweep, 0)),
                ..base
            },
            15,
        ),
        ("all six", ctx, 20),
    ];
    for (field, taken_ctx, want) in units {
        assert_eq!(
            subscription_batch(&taken_ctx).units(),
            want,
            "units for a ctx holding a TAKEN {field} cell"
        );
    }
}

/// A two-sided pair per dimension: the base ctx gives every input `No`, and
/// each single-field ctx gives `Yes` for ITS input only. A cross-keyed field
/// or a constant fails one labelled row.
#[test]
fn ctx_inputs_key_each_field_on_its_own_option() {
    let fx = Fixture::new();
    assert_eq!(
        fx.base().inputs(),
        SubscriptionBatchInputs::default(),
        "inputs() for ctx `all-None`"
    );
    for (label, ctx) in fx.singles() {
        assert_eq!(
            ctx.inputs(),
            expected_single_inputs(label),
            "inputs() for ctx `{label}`"
        );
    }
}

// ── subscription_batch ───────────────────────────────────────────────────────

/// The batch's unit count per context, two ways. LITERAL: 11 bus units plus
/// `ServerTime`, `Activity` and `ToastDismiss` is 14; one gated input adds one
/// unit (15); all eight add eight (22). AGREEMENT: the bus expands to 11 units
/// and every other descriptor entry to exactly one. The literal half catches an
/// arm that builds nothing for a present input; the agreement half catches a
/// descriptor entry whose arm builds nothing for an absent one.
#[test]
fn subscription_batch_units_match_descriptor() {
    let fx = Fixture::new();
    assert_eq!(
        ui::live::subscription(Arc::clone(&fx.bus)).units(),
        BUS_CHANNELS.len(),
        "ui::live::subscription batches one unit per bus channel"
    );
    let mut cases: Vec<(&str, SubscriptionCtx<'_>, usize)> = vec![("all-None", fx.base(), 14)];
    cases.extend(fx.singles().map(|(label, ctx)| (label, ctx, 15)));
    cases.push(("all-Some", fx.all_some(), 22));
    for (label, ctx, want) in cases {
        let units = subscription_batch(&ctx).units();
        assert_eq!(units, want, "units for ctx `{label}`");
        let desc = build_subscription_batch_descriptor_from(ctx.inputs());
        assert_eq!(
            units,
            BUS_CHANNELS.len() + desc.len() - 1,
            "descriptor⇔batch agreement for ctx `{label}`: {desc:?}"
        );
    }
}

// ── esc_closes_modal ─────────────────────────────────────────────────────────

/// Both statuses: the extracted fn keeps the inline closures' behaviour, which
/// ignored the status (an `Esc` a widget captured still closes the modal).
#[test]
fn escape_keypress_maps_to_modal_closed() {
    for status in [Status::Ignored, Status::Captured] {
        let got = esc_closes_modal(
            key_pressed(Named::Escape, Code::Escape),
            status,
            window::Id::unique(),
        );
        assert!(
            matches!(got, Some(Message::TapeAuditModalClosed)),
            "Esc press (status {status:?}) closes the modal: {got:?}"
        );
    }
}

/// The negative half of the key dimension (another key) and of the phase
/// dimension (an `Esc` release), plus a keyboard event that is no key at all,
/// each at BOTH statuses: the positive test covers both, so a fn that closed the
/// modal on any captured event (a click inside it) must fail here.
#[test]
fn non_escape_events_map_to_none() {
    let cases = [
        ("Enter press", key_pressed(Named::Enter, Code::Enter)),
        ("Esc release", key_released(Named::Escape, Code::Escape)),
        (
            "ModifiersChanged",
            iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(Modifiers::empty())),
        ),
    ];
    for status in [Status::Ignored, Status::Captured] {
        for (label, event) in cases.clone() {
            let got = esc_closes_modal(event, status, window::Id::unique());
            assert!(
                got.is_none(),
                "{label} (status {status:?}) must not close the modal: {got:?}"
            );
        }
    }
}

// ── Wave C2 (not yet in this file) ───────────────────────────────────────────
//
// `batched_*_recipe_relays` (forward_plan, narration_outcome, bakeoff_progress,
// sweep_progress, lab_progress, training_log, trail, activity, bus, server_time,
// toast_dismiss, modal_esc), `forward_plan_stays_batched_after_receiver_taken`,
// `recipe_ids_are_frame_stable`, `salted_recipe_ids_change_with_salt` and
// `recipe_ids_are_pairwise_distinct` land here in Wave C2, with the relay
// helpers they need (`next_item`, `assert_silent`, `assert_ends`, `take_unit`).
