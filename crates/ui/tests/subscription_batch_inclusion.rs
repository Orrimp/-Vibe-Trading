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
//! Since Wave C2 the batch is also RUN. `iced::advanced::subscription::into_recipes`
//! unpacks it into the units iced's tracker receives; each `batched_*_relays`
//! test picks ONE unit by iced's own recipe id (`id_of`, the hash `Tracker::update`
//! keys running streams by, of a reference recipe the test builds), streams it
//! through `Recipe::stream` as the tracker would, and asserts the `Message`
//! VARIANT it emits and, where the test chose one, its payload (the
//! `ServerTime` and `ToastDismiss` tickers and `ModalEsc` are checked by
//! variant only; see "Payload fidelity" below). The identity tests hash every
//! unit of the all-Some batch the same way.
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
//!
//!   Wave C2 closed the OTHER gap C1 left, ON THE BATCH `subscription_batch`
//!   BUILDS FROM A TEST CTX: an arm that builds the sibling recipe (bug-log
//!   `#203`'s open part; C2-P12, MEASURED). `units()` only counted units; the
//!   relays now pick each unit by its recipe id and check the message it
//!   emits, so an arm building `BakeoffProgressRecipe` for `SweepProgress`
//!   fails at `take_unit`, and a body emitting the sibling's message fails at
//!   the variant match. The bin's ctx literal stays as above: no relay can
//!   reach it, so a swapped salt in it (C1-RES2) is still checked by nothing.
//! - **The fixtures `cockpit` bin's own `listen_with(esc_closes_modal)`.** That
//!   bin needs `--features fixtures`, so default-feature test runs never
//!   compile it, and `cockpit-smoke` synthesises no input. This file pins the fn
//!   it batches, not that it batches it.
//! - **Executor fidelity.** Every relay runs inside `#[tokio::test]`, which
//!   supplies the tokio context production's executor lacks, so a unit that
//!   needs a reactor passes here. That is `recipe_off_runtime_poll.rs`'s scope
//!   (it drives the recipes off-runtime, not through this batch).
//! - **Every bus channel's own relay.** The `Bus` arm is one call,
//!   `subscription(Arc::clone(ctx.bus))`, that builds all eleven bus units, so
//!   `batched_bus_recipe_relays` pins the arm's bus through one channel
//!   (`Mode`). Each channel's stream is `live_subscription_full_bus.rs`'s and
//!   `recipe_off_runtime_poll.rs`'s. Which unit is `Mode`'s is read off
//!   [`BUS_CHANNELS`]'s order, which nothing here pins against production
//!   (see that test's doc).
//! - **Bus isolation.** The activity and bus relays take their reference id
//!   from a recipe over ANOTHER bus, but publish nothing there: no unit
//!   subscribes to that bus, so an event on it would reach no receiver and
//!   could never be heard. Their 200 ms silence check is a LIVENESS check
//!   (open and silent with nothing published on the unit's own bus), not a
//!   two-sided bus pair; the ctx-bus wiring is pinned by their positive half.
//!   Production holds one bus per process, so "hears a foreign bus" is not a
//!   fault class there.
//! - **Payload fidelity beyond the field each relay checks.** Nine of the
//!   twelve single-unit relays, and the two both-cells tests, send distinctive
//!   values and check each arrives, in the right variant, with the field the
//!   test set. The `ServerTime` and `ToastDismiss` relays check
//!   the VARIANT only: their payload is a `now()` stamp, whose freshness is
//!   pinned on the seams, by `server_time_recipe_stream.rs`'s T-ST-1b (each
//!   stamp fresh, two-sided) and by Wave B's `recipe_off_runtime_poll.rs`
//!   (its B-P15, a stale toast stamp, and B-P26, a future one). `ModalEsc`
//!   emits `TapeAuditModalClosed`, which carries nothing; its unit is checked
//!   by the exact list it emits. The field-by-field mapping is the per-recipe
//!   files' (`forward_narration_relay.rs`, `bakeoff_progress_relay.rs`,
//!   `sweep_progress_relay.rs` and the v0.2 recipe-stream files).
//! - **A `ModalEsc` arm that filters by event kind.** Unit B is fed three
//!   non-closing event kinds (an `Enter` press, an `Esc` release, a
//!   `ModifiersChanged`), at both statuses; no mouse, window or touch event.
//!   An arm-level filter that closed the modal on one of those passes here
//!   (the fn the arm batches is pinned by C1's two `esc_closes_modal` tests).
//! - **Frame-2 retention by relay, for four of the six take-once cells.**
//!   `forward_plan_stays_batched_after_receiver_taken` relays after a second
//!   frame for the plan and narration cells. For the four salted cells,
//!   retention is pinned by C1's taken-cell unit rows plus the frame-stable
//!   ids, not by a relay.
//! - **The `ModalEsc` id across processes.** It hashes a fn pointer, so it is
//!   stable within one process only; this file compares it within one, as
//!   iced does.
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
//! | (C2) `batched_forward_plan_recipe_relays` | the `ForwardPlan` unit: none at boot, exactly one while the cell is held; its `stream()` takes THIS ctx's receiver (the cell is empty right after it); it relays a plan as `ForwardPlanReceived` with the strategy sent, then a SECOND plan while the sender lives (a body that relays once and ends fails there); it ends when the sender drops. Sender alive and dropped are the two sides of one pair: an item arrives, then the end |
//! | (C2) `batched_narration_outcome_recipe_relays` | the `NarrationOutcome` unit, the same shape: `Ready(prose)` relayed as `BakeoffNarrationCompleted(Ready(prose))`, then a second outcome, `FellBack`, then the end |
//! | (C2) `batched_bakeoff_progress_recipe_relays` | the `BakeoffProgress` unit: none at boot (neither the salt-7 id nor the boot salt's, 0); at salt 7 the salt-7 reference id matches exactly one unit and the salt-6 one none; it takes THIS ctx's receiver, relays `Message::BakeoffProgress` with the payload sent, a second while the sender lives, and ends |
//! | (C2) `batched_sweep_progress_recipe_relays` | the `SweepProgress` unit, with the bake-off cell absent: the same boot and salt pairs and take, and it relays `Message::SweepProgress` twice, never the sibling's `BakeoffProgress` |
//! | (C2) `batched_bakeoff_and_sweep_units_drain_own_cells_equal_salts` | BOTH `BakeoffProgressCell`-typed cells held at once, salts 7 and 7 (the realistic case: the bin bumps the two salts independently): each unit, taken by its own reference id, takes ITS cell's receiver (checked right after its `stream()`) and relays ITS sender's value in ITS variant (`BakeoffProgress` / `SweepProgress`), then both end. The only fixture that STREAMS with both cells held (the all-Some identity fixtures hold both too, at equal salts, but only hash them): an arm that drains the sibling's cell whenever it is held (the falsifier's OWN-1, C2-P21 / C2-P22) passes every other test |
//! | (C2) `batched_bakeoff_and_sweep_units_drain_own_cells_distinct_salts` | the same at salts 7 and 8, where an arm reading the sibling's cell also carries the sibling's salt, so it fails at `take_unit` |
//! | (C2) `batched_lab_progress_recipe_relays` | the `LabProgress` unit: boot and salt pairs, take, two `LabRunProgress` with the bars sent while the sender lives, then `LabRunProgressDone` after the drop, then the end |
//! | (C2) `batched_training_log_recipe_relays` | the `TrainingLog` unit (std receiver, OS-thread bridge): boot and salt pairs, take, two lines sent as `TrainingLogLine` while the sender lives, then the end. Its waits are bounded by [`RELAY_XTHREAD`] (10 s): each item crosses the bridge thread |
//! | (C2) `batched_trail_recipe_relays` | the `Trail` unit: none without a handle, exactly one with it, subscribed to THIS handle's sender (a send finds a receiver), relaying `TrailMirrorTick(TrailUpdated(id))` twice while the sender lives, ending once every sender is gone |
//! | (C2) `batched_activity_recipe_relays` | the always-on `Activity` unit: a LIVENESS check first (open and silent with nothing published on its own bus; C2-P13 ENDS here), then the positive half, which pins the ctx-bus wiring: an activity started on the ctx's bus is relayed as its `Start`, then a second one. Not two-sided on the bus (see "Bus isolation" above). No end is asserted: the bus is process-lifetime |
//! | (C2) `batched_bus_recipe_relays` | the always-on `Bus` arm, through its `Mode` unit: the same liveness check, then the positive half, which pins the arm's bus: a mode published on the ctx's bus arrives as `AgentModeChanged(Paper)`, then a second (`Halted`) as `AgentHaltedExternally` with its reason. Before C2 this arm's bus was pinned by nothing (the bus ids never hash the bus). It reads `Mode`'s unit off [`BUS_CHANNELS`]'s order, which its positive failure text names as the table to update on a reorder |
//! | (C2) `batched_server_time_recipe_relays` | the always-on `ServerTime` unit, through the moved `Recipe::stream`, on a paused clock: its first item is a `ServerTimeTick` (the variant only) |
//! | (C2) `batched_toast_dismiss_recipe_relays` | the always-on `ToastDismiss` unit, the same way: its first item is a `ToastTick` (the variant only; the pair tells the two moved bodies apart) |
//! | (C2) `batched_modal_esc_relays_escape_only` | the `ModalEsc` unit, found as the one id opening the modal adds: fed two `Esc` presses (both statuses) it emits exactly two `TapeAuditModalClosed`; fed three non-closing event kinds (an `Enter` press, an `Esc` release, a `ModifiersChanged`) at both statuses it emits nothing; each stream ENDS with its input. No mouse, window or touch event is fed |
//! | (C2) `forward_plan_stays_batched_after_receiver_taken` | the frame-2 condition: after frame 1's units took both receivers, the same ctx still reports both cells held and batches both units under frame 1's ids, and frame 1's streams still relay |
//! | (C2) `recipe_ids_are_frame_stable` | two all-Some batches, alive at once (distinct allocations, asserted): all 22 ids equal index by index, the eleven bus units' among them |
//! | (C2) `salted_recipe_ids_change_with_salt` | salts 1 vs 2 over the same cells: the ids that differ are EXACTLY units 13, 15, 19 and 20 (`LabProgress`, `TrainingLog`, `BakeoffProgress`, `SweepProgress`, indices derived from [`CANONICAL`] and [`BUS_CHANNELS`]); the other 18 do not move |
//! | (C2) `recipe_ids_are_pairwise_distinct` | the all-Some batch at the boot salts (0) and after one run of each (1): 22 distinct ids. iced skips a duplicate id, so a collision would silently drop a unit |
//!
//! ## Non-vacuity contract (bug-log #66 / story 2-63 AC3)
//!
//! Every assertion names its row or case (a mask, a ctx label, an input
//! triple, a status, a relay label, a unit index), so the first failing row is
//! in the message. The literal expectations never come from the subject:
//! `inputs_from_mask` (field → bit) and `expected_for_mask` (variant → bit) are
//! two independent tables keyed by NAME; the unit literals 14 / 15 / 20 / 22
//! are written out; the 8 single-field expectations are a second table, keyed
//! by label. The units test's AGREEMENT half deliberately cross-checks two
//! parts of the subject (batch against descriptor); its literal half runs
//! first. Every dimension is two-sided: a field at `Some` and at `None`, a taken
//! cell and a held one, a bit set and clear, an `Esc` press and an `Esc`
//! release, each `Esc` case at both event statuses. Checks that hold by
//! construction (no literal legacy row lists a v0.3 variant; each optional
//! variant appears in 128 of the 256 masks) are comments, not asserts; the one
//! oracle self-check (each input bit keys one variant) checks the hand-written
//! table, not the subject.
//!
//! The Wave C2 tests add, for the streams they run:
//!
//! - **A reference id is never read off the unit under test, except
//!   `ModalEsc`'s.** It is iced's hash of a recipe the TEST builds (salt 7 /
//!   salt 6 / salt 0, a fresh cell), or of a production constructor's output
//!   over ANOTHER handle or bus, for the two private recipe types. The one
//!   exception is `ModalEsc`, whose id IS read off the subject: the one id in
//!   the open-modal batch and not in the closed one, a two-sided set
//!   difference, never a fn pointer hashed in this crate. So its behaviour is
//!   pinned and its identity is not. The unit indices 13 / 15 / 19 / 20 are
//!   DERIVED from the written-out tables.
//! - **Every awaited item fails with its own text** (`nth_item` / `next_item`,
//!   `assert_silent`, `assert_ends`): a timeout ("NO item within 2s"), an end
//!   ("stream ENDED before its first item", "... its second item") and a wrong
//!   message ("expected X, got ...") are told apart, and no timeout is ever
//!   accepted as a positive claim. The real-time bound is [`RELAY`] (2 s), and
//!   [`RELAY_XTHREAD`] (10 s) for the one relay whose items cross an OS thread
//!   (training log); the `ServerTime` and `ToastDismiss` relays run on a paused
//!   clock, so their bound is virtual time and load-independent.
//! - **Silence is only ever a negative (liveness) control** ([`SILENCE`],
//!   200 ms; the activity and bus relays only): the unit stays OPEN and yields
//!   nothing with nothing published on its own bus. Its timeout is its PASS,
//!   as a negative control only, never as a positive claim: the positive claim
//!   (an event on the ctx's bus IS relayed) follows on the same stream, through
//!   `nth_item`, whose timeout FAILS. Load can make it pass too easily, never
//!   fail spuriously. It is NOT a bus-isolation check (see "Bus isolation"
//!   above): the ctx-bus wiring is pinned by the positive half, and C2-P13 and
//!   C2-P18 go red at the liveness check's ENDED and at the positive half's
//!   timeout, never by hearing another bus.
//! - **Two-sided pairs:** the boot ctx gives 0 units with the salt-7 reference
//!   id and the held ctx exactly 1 (the salted four also check the boot salt's
//!   id, 0, at boot: a one-sided extra negative with no held-at-0 twin, backed
//!   by C1's 14-unit boot count);
//!   the salt-7 reference matches one unit and the salt-6 one none (checked on
//!   the whole batch, before the unit is taken); while the sender lives a
//!   SECOND item arrives (the stream stays open after an item) and once it
//!   drops the stream ends, in the seven relays whose sender the test owns
//!   (plan, narration, bake-off, sweep, lab, training log, trail; every
//!   production body there loops over its receiver), while the activity and
//!   bus relays, over the process-lifetime bus, check the second item only;
//!   `Esc` presses close and the three other event kinds do not; salts 1 and 2
//!   move exactly the four salted ids and no other.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4, Waves C1 and C2)
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
//! `crates/ui/src/bin/cockpit_live.rs`. Every C2 probe edits
//! `crates/ui/src/live.rs`, except C2-P3 (`crates/ui/src/lab/progress.rs`) and
//! C2-P4 (`crates/ui/src/lab/training_log.rs`). A row whose expected RED names
//! a test outside this target ("Outside this target") also needs that target
//! run (`--test server_time_recipe_stream`, `--test toast_dismiss_recipe_stream`,
//! `--test forward_narration_relay`, `--test bakeoff_progress_relay`).
//!
//! Several probes quote ONE line, in this table and across files: apply one at
//! a time, and restore between them. In this table: C2-P1 and C2-P8; C2-P2 and
//! C2-P5; C2-P11 and C2-P26; C1-P6 and C2-P21
//! (`SubscriptionVariant::SweepProgress => match ctx.sweep_progress {`); C1-P13
//! and C2-P22 (`SubscriptionVariant::BakeoffProgress => match ctx.bakeoff_progress {`).
//! Across files: C2-P19 here and E1-P21 in `server_time_recipe_stream.rs`
//! (`server_time_stream_impl(&self.rt_handle)`); C2-P20 here and E1-P22 and
//! E1-P23 in `toast_dismiss_recipe_stream.rs`
//! (`toast_dismiss_stream_impl(&self.rt_handle)`). C2-P20 and E1-P22 are the
//! EQUIVALENT edit, not the same text: C2-P20's NEW spells the item type
//! (`futures::stream::empty::<Message>()`), E1-P22's leaves it to inference;
//! C2-P19 and E1-P21 are different edits of one line. C2-P23 is the same edit
//! as `forward_narration_relay.rs`'s P1, of the line its P2 and
//! `recipe_off_runtime_poll.rs`'s B-P3 also quote; C2-P25 is the same edit as
//! `forward_narration_relay.rs`'s P4, of B-P4's line; C2-P24's mutated
//! `while let` line is quoted by `bakeoff_progress_relay.rs`'s P20 and by B-P5
//! and B-P17.
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
//! | C2-P1 | `<ui::live::BakeoffProgressRecipe as Recipe>::hash` | `// Per-run salt (NOT a static discriminant) — a fresh identity each run.` | delete the `self.salt.hash(state);` line under it (the story's AC4-designated salt probe) | `batched_bakeoff_progress_recipe_relays`, at "bakeoff_progress: salt-6 reference must match no unit"; `salted_recipe_ids_change_with_salt`, at "the units whose id changes with the salt are exactly the four salted take-once units" (`left: [13, 15, 20]`, `right: [13, 15, 19, 20]`: unit 19 no longer moves). `recipe_ids_are_pairwise_distinct` stays GREEN: the `TypeId` still separates 19 from 20. MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier). The both-cells tests, added after that measurement, stay GREEN (reasoned: their reference ids go through the probed impl too) |
//! | C2-P2 | `<ui::live::SweepProgressRecipe as Recipe>::hash` | `impl Recipe for SweepProgressRecipe {` | in its `hash`, delete `self.salt.hash(state);` | `batched_sweep_progress_recipe_relays`, at "sweep_progress: salt-6 reference must match no unit"; `salted_recipe_ids_change_with_salt`, at "the units whose id changes with the salt are exactly the four salted take-once units" (`left: [13, 15, 19]`). MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier) |
//! | C2-P3 | `<ui::lab::progress::LabProgressRecipe as Recipe>::hash` (`crates/ui/src/lab/progress.rs`) | `self.salt.hash(state);` | delete the line | `batched_lab_progress_recipe_relays`, at "lab_progress: salt-6 reference must match no unit"; `salted_recipe_ids_change_with_salt`, at "the units whose id changes with the salt are exactly the four salted take-once units" (`left: [15, 19, 20]`). MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier) |
//! | C2-P4 | `<ui::lab::training_log::TrainingLogRecipe as Recipe>::hash` (`crates/ui/src/lab/training_log.rs`) | `self.salt.hash(state);` | delete the line | `batched_training_log_recipe_relays`, at "training_log: salt-6 reference must match no unit"; `salted_recipe_ids_change_with_salt`, at "the units whose id changes with the salt are exactly the four salted take-once units" (`left: [13, 19, 20]`). MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier) |
//! | C2-P5 | `<ui::live::SweepProgressRecipe as Recipe>::hash` | `impl Recipe for SweepProgressRecipe {` | in its `hash`, `TypeId::of::<Self>()` → `TypeId::of::<BakeoffProgressRecipe>()` (the story's TypeId-swap probe) | `recipe_ids_are_pairwise_distinct`, at "all 22 units of the all-Some batch have distinct ids (every salt 0)" (21: units 19 and 20 collide at equal salts). The relays stay GREEN (each reference id goes through the probed impl too), and so does the salted test (19 and 20 still move with the salt). MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier). Added after that measurement (REASONED): `batched_bakeoff_and_sweep_units_drain_own_cells_equal_salts` is red too, at "both cells, salts 7/7: bake-off unit: exactly one batched unit carries the reference id" (2 hits: at equal salts the sweep unit now hashes as the bake-off one); the distinct-salts twin stays GREEN |
//! | C2-P6 | `<ui::live::BusRecipe as Recipe>::hash` (private) | `self.channel.hash(state);` | delete the line | `recipe_ids_are_pairwise_distinct`, at "all 22 units of the all-Some batch have distinct ids (every salt 0)" (12: the 11 bus ids collapse to one); `batched_bus_recipe_relays`, at "bus/mode: exactly one batched unit carries the reference id" (11 hits). `recipe_ids_are_frame_stable` stays GREEN (the hash is still deterministic). MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier) |
//! | C2-P7 | `<ui::live::ForwardPlanRecipe as Recipe>::hash` | `0xF6_F6u16.hash(state);` | add, under it, `{ static PROBE_N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0); PROBE_N.fetch_add(1, std::sync::atomic::Ordering::Relaxed).hash(state); }` (an id that changes every frame) | `recipe_ids_are_frame_stable`, at "unit 17 id is frame-stable"; `batched_forward_plan_recipe_relays`, at "forward_plan: exactly one batched unit carries the reference id" (0 hits); `forward_plan_stays_batched_after_receiver_taken`, at "frame 1 forward_plan: exactly one batched unit carries the reference id"; `salted_recipe_ids_change_with_salt`, at "the units whose id changes with the salt are exactly the four salted take-once units" (`left: [13, 15, 17, 19, 20]`: unit 17 moves without a salt). MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier) |
//! | C2-P8 | `<ui::live::BakeoffProgressRecipe as Recipe>::hash` | `// Per-run salt (NOT a static discriminant) — a fresh identity each run.` | add `std::ptr::from_ref(self).hash(state);` under the `self.salt.hash(state);` line below it (an id that hashes an address) | `recipe_ids_are_frame_stable`, at "unit 19 id is frame-stable" (only because both batches are alive at once: the premise asserts two allocations); `batched_bakeoff_progress_recipe_relays`, at "bakeoff_progress: exactly one batched unit carries the reference id" (0 hits: the reference recipe lives elsewhere). The salted test stays GREEN (19 moves either way). MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier). Added after that measurement (REASONED): both both-cells tests are red too, at "both cells, salts 7/7: bake-off unit: exactly one batched unit carries the reference id" and "both cells, salts 7/8: bake-off unit: exactly one batched unit carries the reference id" (0 hits each) |
//! | C2-P9 | `ui::live::variant_subscription`'s `ModalEsc` arm | `iced::event::listen_with(crate::widgets::journal_transaction_modal::esc_closes_modal)` | → `iced::event::listen_with(\|_, _, _\| None)` | `batched_modal_esc_relays_escape_only`, at "batched ModalEsc unit closes the modal on each Esc press: []". The `esc_closes_modal` tests and the unit counts stay GREEN: the fn is intact and one unit is still batched. MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier) |
//! | C2-P10 | `ui::live::variant_subscription`'s `ForwardPlan` arm | `Some(rx) => from_recipe(ForwardPlanRecipe {` | its `rx: Arc::clone(rx),` → `rx: Arc::new(std::sync::Mutex::new(None)),` (the unit drains a fresh, empty cell) | `batched_forward_plan_recipe_relays`, at "forward_plan: the batched unit took THIS ctx's receiver"; `forward_plan_stays_batched_after_receiver_taken`, at "frame 1's unit took the plan receiver". MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier) |
//! | C2-P11 | `ui::live::variant_subscription`'s `BakeoffProgress` arm | `Some((rx, salt)) => from_recipe(BakeoffProgressRecipe {` | its `salt,` → `salt: 0,` (the ctx's salt is dropped) | `batched_bakeoff_progress_recipe_relays`, at "bakeoff_progress: exactly one batched unit carries the reference id" (0 hits for salt 7); `salted_recipe_ids_change_with_salt`, at "the units whose id changes with the salt are exactly the four salted take-once units" (`left: [13, 15, 20]`). MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier). Added after that measurement (REASONED): both both-cells tests are red too, at "both cells, salts 7/7: bake-off unit: exactly one batched unit carries the reference id" and "both cells, salts 7/8: bake-off unit: exactly one batched unit carries the reference id" (0 hits each). The relay's boot-salt (0) check passes first: the boot ctx batches no bake-off unit at all |
//! | C2-P12 | `ui::live::variant_subscription`'s `SweepProgress` arm | `Some((rx, salt)) => from_recipe(SweepProgressRecipe {` | `SweepProgressRecipe` → `BakeoffProgressRecipe` (the arm builds the SIBLING recipe: bug-log `#203`'s open part) | `batched_sweep_progress_recipe_relays`, at "sweep_progress: exactly one batched unit carries the reference id" (0 hits); `recipe_ids_are_pairwise_distinct`, at "... distinct ids (every salt 0)" (19 and 20 collide). Every C1 test stays GREEN: the unit count is unchanged. MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier). Added after that measurement (REASONED): `batched_bakeoff_and_sweep_units_drain_own_cells_equal_salts`, at "both cells, salts 7/7: bake-off unit: exactly one batched unit carries the reference id" (2 hits), and `..._distinct_salts`, at "both cells, salts 7/8: sweep unit: exactly one batched unit carries the reference id" (0 hits) |
//! | C2-P13 | `ui::live::variant_subscription`'s `Activity` arm | `SubscriptionVariant::Activity => from_recipe(ActivityRecipe {` | its `bus: Arc::clone(ctx.bus),` → `bus: Arc::new(EventBus::new(&agent::config::BusConfig::default())),` | `batched_activity_recipe_relays`, at "activity: open and silent with nothing published on its own bus: stream ENDED, expected it open and silent" (the liveness check). NOT a timeout: the recipe is the fresh bus's only owner and drops it at the end of `ActivityRecipe::stream`, so the receiver reads `Closed` and the stream ends. MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier) at this assert under its earlier label, "activity must not hear a different bus", which claimed a bus isolation the check never measured; relabelled afterwards, the assert and its mechanism unchanged; re-MEASURED at the new label by the `wf_be1acf7b-3e7` falsifier (2026-11-14) |
//! | C2-P14 | `ui::live::variant_subscription`'s `Trail` arm | `Some(handle) => trail_mirror_subscription(handle.clone()),` | → `Some(_handle) => iced::Subscription::none(),` | `batched_trail_recipe_relays`, at "trail: exactly one batched unit carries the reference id" (0 hits); also every test that counts units: `subscription_batch_units_match_descriptor`, at its literal half, "units for ctx `trail`" (14, wanted 15), and the three identity tests at their 22-unit premise ("both all-Some batches have 22 units", "the all-Some batch has 22 units (every salt 0)"). MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier) |
//! | C2-P15 | `ui::live::variant_subscription`'s `LabProgress` arm | `Some((rx, salt)) => from_recipe(crate::lab::progress::LabProgressRecipe {` | its `salt,` → `salt: 0,` | `batched_lab_progress_recipe_relays`, at "lab_progress: exactly one batched unit carries the reference id" (0 hits); `salted_recipe_ids_change_with_salt`, at "the units whose id changes with the salt are exactly the four salted take-once units" (`left: [15, 19, 20]`). MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier) |
//! | C2-P16 | `ui::live::variant_subscription`'s `TrainingLog` arm | `Some((rx, salt)) => from_recipe(crate::lab::training_log::TrainingLogRecipe {` | its `salt,` → `salt: 0,` | `batched_training_log_recipe_relays`, at "training_log: exactly one batched unit carries the reference id" (0 hits); `salted_recipe_ids_change_with_salt`, at "the units whose id changes with the salt are exactly the four salted take-once units" (`left: [13, 19, 20]`). MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier) |
//! | C2-P17 | `ui::live::variant_subscription`'s `NarrationOutcome` arm | `Some(rx) => from_recipe(NarrationOutcomeRecipe {` | its `rx: Arc::clone(rx),` → `rx: Arc::new(std::sync::Mutex::new(None)),` | `batched_narration_outcome_recipe_relays`, at "narration_outcome: the batched unit took THIS ctx's receiver"; `forward_plan_stays_batched_after_receiver_taken`, at "frame 1's unit took the narration receiver". MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier) |
//! | C2-P18 | `ui::live::variant_subscription`'s `Bus` arm | `SubscriptionVariant::Bus => subscription(Arc::clone(ctx.bus)),` | → `SubscriptionVariant::Bus => subscription(Arc::new(EventBus::new(&agent::config::BusConfig::default()))),` | `batched_bus_recipe_relays`, at "bus/mode relays the ctx bus (if `ui::live::subscription` reordered its channels, update BUS_CHANNELS): NO item within 2s" (the positive half: no reorder here, the hint names the test's one false-red cause). Its liveness check passes first: the fresh bus stays open, because the ten sibling bus units still in `units` hold it. Every count, matrix, ctx and identity test stays GREEN (a bus id never hashes the bus). MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier) at this assert under its earlier label, "bus/mode relays the ctx bus"; the label gained the `BUS_CHANNELS` hint afterwards, the assert unchanged |
//! | C2-P19 | `<ui::live::ServerTimeRecipe as Recipe>::stream` | `server_time_stream_impl(&self.rt_handle)` | → `toast_dismiss_stream_impl(&self.rt_handle)` (the moved body cross-wired) | `batched_server_time_recipe_relays`, at "server-time first item: ToastTick(..)" (the swapped body ticks at 500 ms of virtual time). The ids are unchanged, so every identity test stays GREEN. MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier). Outside this target, MEASURED in the same run (the falsifier also ran that target) and again by the `wf_be1acf7b-3e7` falsifier: `server_time_recipe_stream::server_time_recipe_stream_yields_tick`, at "server time recipe: the first message is a ServerTimeTick: ToastTick(..)" |
//! | C2-P20 | `<ui::live::ToastDismissRecipe as Recipe>::stream` | `toast_dismiss_stream_impl(&self.rt_handle)` | → `Box::pin(futures::stream::empty::<Message>())` | `batched_toast_dismiss_recipe_relays`, at "toast: stream ENDED before its first item". MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier). Outside this target, MEASURED by the `wf_be1acf7b-3e7` falsifier (2026-11-14; the first run ran only this target): `toast_dismiss_recipe_stream::toast_dismiss_recipe_stream_yields_tick`, at "toast recipe: `ToastDismissRecipe::stream` returns a live stream: it ENDED at its first poll" (E1-P22, the equivalent edit, was MEASURED red there at that text) |
//! | C2-P21 | `ui::live::variant_subscription`'s `SweepProgress` arm | `SubscriptionVariant::SweepProgress => match ctx.sweep_progress {` | → `SubscriptionVariant::SweepProgress => match ctx.bakeoff_progress.or(ctx.sweep_progress) {` (the arm drains the BAKE-OFF cell, at its salt, whenever that cell is held: the falsifier's OWN-1) | `batched_bakeoff_and_sweep_units_drain_own_cells_equal_salts`, at "both cells, salts 7/7: the sweep unit took the SWEEP cell's receiver"; `batched_bakeoff_and_sweep_units_drain_own_cells_distinct_salts`, at "both cells, salts 7/8: sweep unit: exactly one batched unit carries the reference id" (0 hits: the unit carries the bake-off salt, 7). Every other test stays GREEN: no other fixture STREAMS with both cells held (the all-Some identity fixtures hold both at equal salts and only hash, so no id moves; OWN-1 left this target 23/23 green before these two tests). MEASURED (2026-11-14, `wf_be1acf7b-3e7` falsifier; OWN-1's mutant is byte-identical) |
//! | C2-P22 | `ui::live::variant_subscription`'s `BakeoffProgress` arm | `SubscriptionVariant::BakeoffProgress => match ctx.bakeoff_progress {` | → `SubscriptionVariant::BakeoffProgress => match ctx.sweep_progress.or(ctx.bakeoff_progress) {` (the mirror: the arm drains the SWEEP cell whenever it is held) | `batched_bakeoff_and_sweep_units_drain_own_cells_equal_salts`, at "both cells, salts 7/7: the bake-off unit took the BAKE-OFF cell's receiver"; `batched_bakeoff_and_sweep_units_drain_own_cells_distinct_salts`, at "both cells, salts 7/8: bake-off unit: exactly one batched unit carries the reference id" (0 hits: the unit carries the sweep salt, 8). Every other test stays GREEN: the bake-off relay holds no sweep cell, and the all-Some fixtures hold equal salts, so no id moves. MEASURED (2026-11-14, `wf_be1acf7b-3e7` falsifier) |
//! | C2-P23 | `ui::live::forward_plan_stream_impl` | `while let Some(plan) = rx.recv().await {` | → `if let Some(plan) = rx.recv().await {` (the plan stream relays ONE plan and then ends, whatever the sender does: the falsifier's OWN-3) | `batched_forward_plan_recipe_relays`, at "forward_plan: stream ENDED before its second item". `forward_plan_stays_batched_after_receiver_taken` stays GREEN (one plan per stream). MEASURED (2026-11-14, `wf_be1acf7b-3e7` falsifier; OWN-3's mutant is byte-identical). Outside this target, MEASURED as OWN-3 (2026-11-14, `wf_a82b74dd-7bb` falsifier; this target was then 23/23 green) and again in `wf_be1acf7b-3e7`: `forward_narration_relay::forward_plan_relay_yields_one_message_per_plan_in_order`, at "exactly one ForwardPlanReceived per plan sent" |
//! | C2-P24 | `ui::live::bakeoff_progress_stream_impl` | `yield Message::BakeoffProgress(progress);` | the `while let Some(progress) = rx.recv().await {` line above it → `if let Some(progress) = rx.recv().await {`. That line alone occurs twice in `live.rs` (here and in `sweep_progress_stream_impl`), so the OLD is the two lines together, which occur once | `batched_bakeoff_progress_recipe_relays`, at "bakeoff_progress: stream ENDED before its second item". The both-cells tests stay GREEN (one item per unit). MEASURED (2026-11-14, `wf_be1acf7b-3e7` falsifier). Outside this target, MEASURED in the same run: `bakeoff_progress_relay::bakeoff_progress_relay_forwards_every_event_in_order_then_ends`, at "every event forwarded, in send order" (`left: [0]`), and `bakeoff_progress_relay::bakeoff_progress_relay_latest_event_wins_on_state`, at "every one of the 3 sent events was relayed" (`left: 1`) |
//! | C2-P25 | `ui::live::narration_outcome_stream_impl` | `while let Some(outcome) = rx.recv().await {` | → `if let Some(outcome) = rx.recv().await {` | `batched_narration_outcome_recipe_relays`, at "narration_outcome: stream ENDED before its second item". `forward_plan_stays_batched_after_receiver_taken` stays GREEN (one outcome per stream). MEASURED (2026-11-14, `wf_be1acf7b-3e7` falsifier). Outside this target, MEASURED in the same run: `forward_narration_relay::narration_outcome_stream_resolves_two_bakeoff_runs_through_one_receiver`, at "after run 1's outcome (exactly one message for run 1; the relay must stay live for run 2): the stream already ENDED (expected it live on its receiver)" |
//! | C2-P26 | `ui::live::variant_subscription`'s `BakeoffProgress` arm | `Some((rx, salt)) => from_recipe(BakeoffProgressRecipe {` | `BakeoffProgressRecipe` → `SweepProgressRecipe` (the arm builds the SIBLING recipe: C2-P12's reverse direction). The edited line then reads like the Sweep arm's own (C2-P12's quoted line), so it occurs twice: restore from the snapshot, never by reversing the text | `batched_bakeoff_progress_recipe_relays`, at "bakeoff_progress: exactly one batched unit carries the reference id" (0 hits); `batched_bakeoff_and_sweep_units_drain_own_cells_equal_salts`, at "both cells, salts 7/7: bake-off unit: exactly one batched unit carries the reference id" (0 hits); `batched_bakeoff_and_sweep_units_drain_own_cells_distinct_salts`, at "both cells, salts 7/8: bake-off unit: exactly one batched unit carries the reference id" (0 hits); `recipe_ids_are_pairwise_distinct`, at "all 22 units of the all-Some batch have distinct ids (every salt 0)" (21: units 19 and 20 collide). The salted test and every C1 test stay GREEN (both ids still move with the salt; the count is unchanged). MEASURED (2026-11-14, `wf_be1acf7b-3e7` falsifier) |
//! | C2-P27 | `ui::live::variant_subscription`'s `ServerTime` arm | `SubscriptionVariant::ServerTime => from_recipe(ServerTimeRecipe {` | → `SubscriptionVariant::ServerTime => from_recipe(ToastDismissRecipe {` (the arm builds the toast recipe: C2-P19's arm-level twin) | `batched_server_time_recipe_relays`, at "server-time: exactly one batched unit carries the reference id" (0 hits); `batched_toast_dismiss_recipe_relays`, at "toast: exactly one batched unit carries the reference id" (2 hits); `recipe_ids_are_pairwise_distinct`, at "all 22 units of the all-Some batch have distinct ids (every salt 0)" (21). The salted, frame-stable and C1 tests stay GREEN (the count is unchanged and every id is stable), and so does the `ModalEsc` set difference (opening the modal still adds one id). MEASURED (2026-11-14, `wf_be1acf7b-3e7` falsifier) |
//!
//! The C1 rows name the reds MEASURED with C1's seven tests. Under the C2
//! tests some of them go red in more places (reasoned, not measured): C1-P2
//! and C1-P6 also redden `batched_sweep_progress_recipe_relays` at
//! `take_unit`, and both both-cells tests at their sweep unit's `take_unit`
//! ("both cells, salts 7/7: sweep unit: exactly one batched unit carries the
//! reference id", and the same at 7/8); C1-P2, C1-P6, C1-P8, C1-P11 and C1-P12
//! the three identity tests at their 22-unit premise; C1-P4 reddens
//! `batched_bakeoff_progress_recipe_relays` at `take_unit`; C1-P12
//! `batched_trail_recipe_relays` at `take_unit`; C1-P11
//! `batched_modal_esc_relays_escape_only` at "opening the modal adds exactly
//! one unit"; C1-P9, C1-P10 and C1-P14 its `Esc`-press assert, C1-P15 its
//! "maps only an Esc press"; C1-P7 `forward_plan_stays_batched_after_receiver_taken`
//! at "frame 2: both cells are still held".
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
//! MEASURED (C2), 2026-11-14, by the falsifier of workflow `wf_a82b74dd-7bb`:
//! C2-P1..C2-P20, and E1-P21..E1-P23 in the two E1b files, each applied ALONE
//! (the shared lines in separate runs), each OLD found exactly once, each
//! restored SHA-identical with `Compiling ui` on the re-run, and each turned
//! exactly the named tests red at the designed assertion: 23 of 23. Its own
//! probes: OWN-2 (a `ModalEsc` arm that drops captured events) red at
//! "batched ModalEsc unit closes the modal on each Esc press: [TapeAuditModalClosed]";
//! OWN-1 (the Sweep arm drains the bake-off cell) and OWN-3 (a single-shot
//! plan body) SURVIVED this target, 23/23 green. They are rows C2-P21 and
//! C2-P23 now, with the tests added to kill them (the both-cells pair; a second
//! item in every relay whose body loops).
//!
//! Changed in this file AFTER that measurement (the C2 follow-up, test-only):
//! rows C2-P21..C2-P27; the both-cells tests, which add reds to C2-P5, C2-P8,
//! C2-P11 and C2-P12 (named in those rows) and to C1-P2 / C1-P6 (above); two
//! relabelled asserts (C2-P13's liveness check, C2-P18's positive half); the
//! second items, and the boot-salt (0) checks of the four salted relays. Every
//! new check sits AFTER the measured red sites it shares a test with, except
//! the boot-salt checks, which sit before the salt-6 / `take_unit` reds of
//! C2-P1..C2-P4, C2-P11, C2-P15 and C2-P16 (the boot ctx batches no salted
//! unit at all, so they pass under each). The bus-only half of
//! `recipe_ids_are_frame_stable` is gone (no measured red was there), and the
//! training-log relay's bounds are [`RELAY_XTHREAD`], which can only make it
//! fail later.
//!
//! MEASURED (C2 follow-up), 2026-11-14, by the falsifier of workflow
//! `wf_be1acf7b-3e7`, on the follow-up's files: C2-P21..C2-P27, C2-P13 at its
//! new label, C2-P19 and C2-P20 in their E1b targets as well, and
//! `recipe_off_runtime_poll.rs`'s B-P34, each applied ALONE, each OLD found
//! exactly once, each restored SHA-identical with `Compiling ui` on the
//! re-run: 11 of 11 red at the designed assertion. OWN-1's and OWN-3's
//! mutants are byte-identical to C2-P21's and C2-P23's, so both survivors of
//! the first pass are now killed in this target. Three unmutated runs: 25/25
//! each. Still REASONED, outside that pass: the both-cells tests' added reds
//! under C2-P5, C2-P8, C2-P11, C2-P12 and C1-P2 / C1-P6; C2-P18's relabelled
//! positive half; and the boot-salt (0) checks passing under C2-P1..C2-P4,
//! C2-P11, C2-P15 and C2-P16.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;
use std::hash::Hasher as _;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use agent::config::BusConfig;
use agent::{ActivityKind, ActivityPhase, EventBus};
use futures::StreamExt;
use futures::stream::BoxStream;
use iced::advanced::subscription::{Event as SubEvent, EventStream, Hasher, Recipe, into_recipes};
use iced::event::Status;
use iced::keyboard::key::{Code, Named, Physical};
use iced::keyboard::{Key, Location, Modifiers};
use iced::window;
use reflection::trail_mirror::{TrailMirrorHandle, TrailMirrorTick};
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use tokio::time::timeout;
use ui::lab::progress::LabProgressRecipe;
use ui::lab::training_log::TrainingLogRecipe;
use ui::live::{
    ActivityRecipe, BakeoffProgressCell, BakeoffProgressRecipe, Channel, ForwardPlanCell,
    ForwardPlanRecipe, Held, LabProgressCell, NarrationOutcomeCell, NarrationOutcomeRecipe,
    ServerTimeRecipe, SubscriptionBatchInputs, SubscriptionCtx, SubscriptionVariant as V,
    SweepProgressRecipe, ToastDismissRecipe, TrainingLogCell, build_subscription_batch_descriptor,
    build_subscription_batch_descriptor_from, subscription_batch,
};
use ui::state::{Message, TrailMirrorUiTick};
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

/// A fresh, empty bus, as `cockpit_live` builds one when no agent hands it in.
fn new_bus() -> Arc<EventBus> {
    Arc::new(EventBus::new(&BusConfig::default()))
}

/// Every optional input absent and the modal closed: `AppState` at boot,
/// before any run.
fn boot_ctx<'a>(rt: &'a tokio::runtime::Handle, bus: &'a Arc<EventBus>) -> SubscriptionCtx<'a> {
    SubscriptionCtx {
        rt_handle: rt,
        bus,
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

/// Everything a [`SubscriptionCtx`] borrows, for the tests that run no stream:
/// the runtime exists only for its `Handle`, and each sender is dropped at once.
/// The relays build their own inputs inside `#[tokio::test]` (a `Runtime` must
/// not be dropped inside another one).
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
            bus: new_bus(),
            trail: TrailMirrorHandle { req_tx, tick_tx },
            lab: held(tokio::sync::mpsc::channel(1).1),
            training: held(std::sync::mpsc::channel().1),
            plan: held(tokio::sync::mpsc::channel(1).1),
            narration: held(tokio::sync::mpsc::channel(1).1),
            bakeoff: held(tokio::sync::mpsc::channel(1).1),
            sweep: held(tokio::sync::mpsc::channel(1).1),
        }
    }

    /// [`boot_ctx`] over this fixture.
    fn base(&self) -> SubscriptionCtx<'_> {
        boot_ctx(self.rt.handle(), &self.bus)
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

    /// [`Fixture::all_some`] with all four salts set to `salt`: `salt` 0 is the
    /// boot state (the bin's `bakeoff_progress_recipe_salt: 0,` and its three
    /// siblings), `salt` 1 the state after one run of each (one
    /// `self.bakeoff_progress_recipe_salt = self.bakeoff_progress_recipe_salt.wrapping_add(1);`
    /// and its three siblings).
    fn all_some_salted(&self, salt: u64) -> SubscriptionCtx<'_> {
        SubscriptionCtx {
            lab_progress: Some((&self.lab, salt)),
            training_log: Some((&self.training, salt)),
            bakeoff_progress: Some((&self.bakeoff, salt)),
            sweep_progress: Some((&self.sweep, salt)),
            ..self.all_some()
        }
    }
}

/// The index of `variant`'s unit in the all-Some batch, DERIVED from the
/// written-out tables (never read off the subject): `Bus` expands to
/// `BUS_CHANNELS.len()` units and every other variant to one, in [`CANONICAL`]
/// order. `LabProgress` 13, `TrainingLog` 15, `ForwardPlan` 17,
/// `BakeoffProgress` 19, `SweepProgress` 20.
fn all_some_unit_index(variant: V) -> usize {
    assert_ne!(
        variant,
        V::Bus,
        "Bus is {} units, not one",
        BUS_CHANNELS.len()
    );
    let position = CANONICAL
        .iter()
        .position(|&v| v == variant)
        .expect("every variant is in CANONICAL");
    BUS_CHANNELS.len() + position - 1
}

/// The all-Some batch's unit count: 11 bus units plus the other 11 variants
/// (written out, as in `subscription_batch_units_match_descriptor`).
const ALL_SOME_UNITS: usize = 22;

// ── Recipe identity and relay helpers ────────────────────────────────────────

/// One unit of a batch, as `into_recipes` hands it to iced.
type Unit = Box<dyn Recipe<Output = Message>>;

/// iced's own recipe id: `iced_futures` 0.14's `Tracker::update` hashes each
/// recipe into a default `Hasher` (an `FxHasher`, seed 0, so deterministic) and
/// keys the running stream by `finish()`.
fn id_of(recipe: &dyn Recipe<Output = Message>) -> u64 {
    let mut hasher = Hasher::default();
    recipe.hash(&mut hasher);
    hasher.finish()
}

/// Every unit's id, in batch order.
fn ids(units: &[Unit]) -> Vec<u64> {
    units.iter().map(|unit| id_of(unit.as_ref())).collect()
}

/// How many units carry `id`.
fn count_id(units: &[Unit], id: u64) -> usize {
    units
        .iter()
        .filter(|unit| id_of(unit.as_ref()) == id)
        .count()
}

/// Removes the ONE unit carrying `id` (a reference recipe's id, computed by
/// iced's own hash, never an index). Keep the rest of `units` bound until the
/// test ends: the bus relay's fresh-bus probe (C2-P18) depends on the sibling
/// units still holding their bus.
fn take_unit(units: &mut Vec<Unit>, id: u64, label: &str) -> Unit {
    let hits: Vec<usize> = units
        .iter()
        .enumerate()
        .filter_map(|(i, unit)| (id_of(unit.as_ref()) == id).then_some(i))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "{label}: exactly one batched unit carries the reference id (ids: {:?})",
        ids(units)
    );
    units.remove(hits[0])
}

/// The iced runtime event stream a `Recipe::stream` receives; only `ModalEsc`
/// reads it, so an empty one is faithful for the others.
fn empty_event_stream() -> EventStream {
    Box::pin(futures::stream::empty::<SubEvent>())
}

/// A finite runtime event stream: it ENDS after `events`.
fn event_stream(events: Vec<SubEvent>) -> EventStream {
    Box::pin(futures::stream::iter(events))
}

/// One runtime interaction, as iced's tracker broadcasts it.
fn interaction(event: iced::Event, status: Status) -> SubEvent {
    SubEvent::Interaction {
        window: window::Id::unique(),
        event,
        status,
    }
}

/// Bound on an awaited relay item, in real time (the two paused-clock relays
/// spend it in virtual time, at no real cost). Every real-time relay but one
/// queues its item in the same task before awaiting it, and tokio's `Timeout`
/// polls the inner future before its delay, so the item returns on the first
/// poll however loaded the box is.
const RELAY: Duration = Duration::from_secs(2);
/// The bound for the one relay whose items cross an OS thread:
/// `batched_training_log_recipe_relays`, whose stream is fed by the
/// `training-log-bridge` thread `TrainingLogRecipe`'s body spawns. That thread
/// must be spawned, scheduled, forward and (for the end) exit, all outside this
/// task, so a saturated box can delay it past [`RELAY`]. Used ONLY there. It
/// costs nothing on the green path: the wait returns as soon as the item (or the
/// end) arrives, and a longer bound can only make that test fail later, never
/// pass a missing item.
const RELAY_XTHREAD: Duration = Duration::from_secs(10);
/// How long a must-stay-silent stream is watched. A negative (liveness)
/// control: load can only make it pass too easily, never fail spuriously.
const SILENCE: Duration = Duration::from_millis(200);

/// The `nth` item ("first", "second", ...), within `bound`. A timeout, an ended
/// stream and (at the caller) a wrong message each fail with their OWN text; no
/// outcome is accepted as "still open".
async fn nth_item(
    stream: &mut BoxStream<'static, Message>,
    label: &str,
    nth: &str,
    bound: Duration,
) -> Message {
    match timeout(bound, stream.next()).await {
        Ok(Some(message)) => message,
        Ok(None) => panic!("{label}: stream ENDED before its {nth} item"),
        Err(_) => panic!("{label}: NO item within {bound:?}"),
    }
}

/// The first item, within [`RELAY`] (see [`nth_item`]).
async fn next_item(stream: &mut BoxStream<'static, Message>, label: &str) -> Message {
    nth_item(stream, label, "first", RELAY).await
}

/// The stream stays OPEN and yields nothing for [`SILENCE`]. A timeout is this
/// check's PASS, as a negative (liveness) control only: it never stands for a
/// positive claim, and every positive claim follows on the same stream, through
/// [`nth_item`], whose timeout FAILS.
async fn assert_silent(stream: &mut BoxStream<'static, Message>, label: &str) {
    match timeout(SILENCE, stream.next()).await {
        Err(_) => {}
        Ok(None) => panic!("{label}: stream ENDED, expected it open and silent"),
        Ok(Some(message)) => panic!("{label}: unexpected item {message:?}"),
    }
}

/// The stream ENDS, with nothing more, within `bound`.
async fn assert_ends_within(
    stream: &mut BoxStream<'static, Message>,
    label: &str,
    bound: Duration,
) {
    match timeout(bound, stream.next()).await {
        Ok(None) => {}
        Ok(Some(message)) => panic!("{label}: expected the end, got {message:?}"),
        Err(_) => panic!("{label}: did NOT end within {bound:?}"),
    }
}

/// The stream ENDS, with nothing more, within [`RELAY`].
async fn assert_ends(stream: &mut BoxStream<'static, Message>, label: &str) {
    assert_ends_within(stream, label, RELAY).await;
}

/// A populated `agent::ForwardPlan` (the type the supervisor sends over
/// `plan_tx`), told apart by its `strategy` id. The shape of
/// `forward_narration_relay.rs`'s `agent_forward_plan_for`.
fn forward_plan_for(strategy: &str) -> agent::ForwardPlan {
    use trading_core::{Money, Price, Quantity, StrategyId, Symbol, Timestamp, Usdt};
    agent::ForwardPlan {
        strategy: StrategyId::new(strategy),
        symbol: Symbol::new("BTCUSDT"),
        stance: agent::PlanStance::Long,
        latest_signal: Some(agent::PlanSignal::Buy),
        rule: agent::PlanRuleKind::SmaCross {
            fast_len: 20,
            slow_len: 50,
        },
        last_close: Price::new(dec!(40000.00)).unwrap(),
        last_bar_ts: Timestamp::now(),
        budget: Money::<Usdt>::from_decimal(dec!(200)),
        projected_units: Quantity::new(dec!(0.005)).unwrap(),
        sizing_capped: false,
        horizon_days: 7,
        confidence: None,
    }
}

/// A `BakeoffProgress` event (the bake-off's and the sweep's shared wire type).
fn progress_event(done: u16, total: u16, current_id: &str) -> backtest::progress::BakeoffProgress {
    backtest::progress::BakeoffProgress {
        done,
        total,
        current_id: SmolStr::new(current_id),
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

// ── Wave C2: each batched unit relays, from ITS ctx input ────────────────────
//
// Every relay below drives the REAL batch: it builds a `SubscriptionCtx`, calls
// `subscription_batch`, takes the ONE unit carrying a reference recipe's id
// (`take_unit`), streams it as iced would, and asserts the emitted `Message`
// VARIANT, and its payload where it has one the test chose (the two tickers
// and `ModalEsc` are checked by variant only). A unit built from a sibling
// recipe carries the sibling's id, so it fails at `take_unit`; a stream body
// that emits the sibling's message fails at the variant match (bug-log
// `#203`'s open part). Each relay whose production body LOOPS over its
// receiver also relays a SECOND item while the sender is alive, so a body that
// relays once and ends fails there, at "stream ENDED before its second item".

/// The F6 plan unit: absent at boot, exactly one when the cell is held, takes
/// THIS ctx's receiver in `stream()`, relays a plan by its strategy id, then a
/// second one while the sender lives, and ends when the sender drops.
#[tokio::test]
async fn batched_forward_plan_recipe_relays() {
    const LABEL: &str = "forward_plan";
    let rt = tokio::runtime::Handle::current();
    let bus = new_bus();
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    let cell: ForwardPlanCell = held(rx);
    let reference = id_of(&ForwardPlanRecipe {
        rt_handle: rt.clone(),
        rx: taken(),
    });

    let base = boot_ctx(&rt, &bus);
    assert_eq!(
        count_id(&into_recipes(subscription_batch(&base)), reference),
        0,
        "{LABEL}: the boot ctx batches no unit with the reference id"
    );
    let ctx = SubscriptionCtx {
        forward_plan: Some(&cell),
        ..base
    };
    let mut units = into_recipes(subscription_batch(&ctx));
    let mut stream = take_unit(&mut units, reference, LABEL).stream(empty_event_stream());
    assert!(
        cell.lock().unwrap().is_none(),
        "{LABEL}: the batched unit took THIS ctx's receiver"
    );

    tx.send(forward_plan_for("v0.c2"))
        .await
        .expect("the receiver is held by the stream");
    match next_item(&mut stream, LABEL).await {
        Message::ForwardPlanReceived(view) => assert_eq!(
            view.strategy, "v0.c2",
            "{LABEL}: the relayed plan is the one sent"
        ),
        other => panic!("{LABEL}: expected ForwardPlanReceived, got {other:?}"),
    }
    tx.send(forward_plan_for("v0.c2.second"))
        .await
        .expect("the receiver is still held by the stream");
    match nth_item(&mut stream, LABEL, "second", RELAY).await {
        Message::ForwardPlanReceived(view) => assert_eq!(
            view.strategy, "v0.c2.second",
            "{LABEL}: the second relayed plan is the second one sent"
        ),
        other => panic!("{LABEL}: expected a second ForwardPlanReceived, got {other:?}"),
    }
    drop(tx);
    assert_ends(&mut stream, LABEL).await;
}

/// The F9 narration unit: the same shape, relaying `Ready(prose)` intact, then
/// a second outcome (`FellBack`) while the sender lives.
#[tokio::test]
async fn batched_narration_outcome_recipe_relays() {
    const LABEL: &str = "narration_outcome";
    let rt = tokio::runtime::Handle::current();
    let bus = new_bus();
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    let cell: NarrationOutcomeCell = held(rx);
    let reference = id_of(&NarrationOutcomeRecipe {
        rt_handle: rt.clone(),
        rx: taken(),
    });

    let base = boot_ctx(&rt, &bus);
    assert_eq!(
        count_id(&into_recipes(subscription_batch(&base)), reference),
        0,
        "{LABEL}: the boot ctx batches no unit with the reference id"
    );
    let ctx = SubscriptionCtx {
        narration_outcome: Some(&cell),
        ..base
    };
    let mut units = into_recipes(subscription_batch(&ctx));
    let mut stream = take_unit(&mut units, reference, LABEL).stream(empty_event_stream());
    assert!(
        cell.lock().unwrap().is_none(),
        "{LABEL}: the batched unit took THIS ctx's receiver"
    );

    tx.send(agent::NarrationOutcome::Ready(SmolStr::new("c2 prose")))
        .await
        .expect("the receiver is held by the stream");
    match next_item(&mut stream, LABEL).await {
        Message::BakeoffNarrationCompleted(ui::leaderboard::NarrationOutcome::Ready(prose)) => {
            assert_eq!(
                prose, "c2 prose",
                "{LABEL}: the relayed prose is the one sent"
            );
        }
        other => panic!("{LABEL}: expected BakeoffNarrationCompleted(Ready(..)), got {other:?}"),
    }
    tx.send(agent::NarrationOutcome::FellBack)
        .await
        .expect("the receiver is still held by the stream");
    match nth_item(&mut stream, LABEL, "second", RELAY).await {
        Message::BakeoffNarrationCompleted(ui::leaderboard::NarrationOutcome::FellBack) => {}
        other => {
            panic!("{LABEL}: expected a second BakeoffNarrationCompleted, FellBack, got {other:?}")
        }
    }
    drop(tx);
    assert_ends(&mut stream, LABEL).await;
}

/// The bake-off progress unit, two-sided on the SALT: at salt 7 the salt-7
/// reference matches exactly one unit and the salt-6 one none (checked on the
/// whole batch, before the unit is taken). At boot neither the salt-7 id nor
/// the boot salt's (0) is batched.
#[tokio::test]
async fn batched_bakeoff_progress_recipe_relays() {
    const LABEL: &str = "bakeoff_progress";
    let rt = tokio::runtime::Handle::current();
    let bus = new_bus();
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    let cell: BakeoffProgressCell = held(rx);
    let reference = |salt| {
        id_of(&BakeoffProgressRecipe {
            rt_handle: rt.clone(),
            rx: taken(),
            salt,
        })
    };

    let base = boot_ctx(&rt, &bus);
    let boot = into_recipes(subscription_batch(&base));
    assert_eq!(
        count_id(&boot, reference(7)),
        0,
        "{LABEL}: the boot ctx batches no unit with the reference id"
    );
    assert_eq!(
        count_id(&boot, reference(0)),
        0,
        "{LABEL}: the boot ctx batches no unit with the boot-salt (0) reference id"
    );
    let ctx = SubscriptionCtx {
        bakeoff_progress: Some((&cell, 7)),
        ..base
    };
    let mut units = into_recipes(subscription_batch(&ctx));
    assert_eq!(
        count_id(&units, reference(6)),
        0,
        "{LABEL}: salt-6 reference must match no unit"
    );
    let mut stream = take_unit(&mut units, reference(7), LABEL).stream(empty_event_stream());
    assert!(
        cell.lock().unwrap().is_none(),
        "{LABEL}: the batched unit took THIS ctx's receiver"
    );

    tx.send(progress_event(3, 7, "v0.sma"))
        .await
        .expect("the receiver is held by the stream");
    match next_item(&mut stream, LABEL).await {
        Message::BakeoffProgress(p) => assert_eq!(
            (p.done, p.total, p.current_id.as_str()),
            (3, 7, "v0.sma"),
            "{LABEL}: the relayed progress is the one sent"
        ),
        other => panic!("{LABEL}: expected BakeoffProgress, got {other:?}"),
    }
    tx.send(progress_event(4, 7, "v0.ema"))
        .await
        .expect("the receiver is still held by the stream");
    match nth_item(&mut stream, LABEL, "second", RELAY).await {
        Message::BakeoffProgress(p) => assert_eq!(
            (p.done, p.total, p.current_id.as_str()),
            (4, 7, "v0.ema"),
            "{LABEL}: the second relayed progress is the second one sent"
        ),
        other => panic!("{LABEL}: expected a second BakeoffProgress, got {other:?}"),
    }
    drop(tx);
    assert_ends(&mut stream, LABEL).await;
}

/// The sweep progress unit, with the bake-off cell ABSENT: it must relay
/// `Message::SweepProgress`, never the sibling's `BakeoffProgress` (the two
/// share one wire type and one cell type). Both cells held at once are
/// [`both_progress_cells_relay`]'s.
#[tokio::test]
async fn batched_sweep_progress_recipe_relays() {
    const LABEL: &str = "sweep_progress";
    let rt = tokio::runtime::Handle::current();
    let bus = new_bus();
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    let cell: BakeoffProgressCell = held(rx);
    let reference = |salt| {
        id_of(&SweepProgressRecipe {
            rt_handle: rt.clone(),
            rx: taken(),
            salt,
        })
    };

    let base = boot_ctx(&rt, &bus);
    let boot = into_recipes(subscription_batch(&base));
    assert_eq!(
        count_id(&boot, reference(7)),
        0,
        "{LABEL}: the boot ctx batches no unit with the reference id"
    );
    assert_eq!(
        count_id(&boot, reference(0)),
        0,
        "{LABEL}: the boot ctx batches no unit with the boot-salt (0) reference id"
    );
    let ctx = SubscriptionCtx {
        sweep_progress: Some((&cell, 7)),
        ..base
    };
    let mut units = into_recipes(subscription_batch(&ctx));
    assert_eq!(
        count_id(&units, reference(6)),
        0,
        "{LABEL}: salt-6 reference must match no unit"
    );
    let mut stream = take_unit(&mut units, reference(7), LABEL).stream(empty_event_stream());
    assert!(
        cell.lock().unwrap().is_none(),
        "{LABEL}: the batched unit took THIS ctx's receiver"
    );

    tx.send(progress_event(4, 12, "fast=15, slow=40"))
        .await
        .expect("the receiver is held by the stream");
    match next_item(&mut stream, LABEL).await {
        Message::SweepProgress(p) => assert_eq!(
            (p.done, p.total, p.current_id.as_str()),
            (4, 12, "fast=15, slow=40"),
            "{LABEL}: the relayed progress is the one sent"
        ),
        other => panic!("{LABEL}: expected SweepProgress (not BakeoffProgress), got {other:?}"),
    }
    tx.send(progress_event(5, 12, "fast=20, slow=50"))
        .await
        .expect("the receiver is still held by the stream");
    match nth_item(&mut stream, LABEL, "second", RELAY).await {
        Message::SweepProgress(p) => assert_eq!(
            (p.done, p.total, p.current_id.as_str()),
            (5, 12, "fast=20, slow=50"),
            "{LABEL}: the second relayed progress is the second one sent"
        ),
        other => panic!("{LABEL}: expected a second SweepProgress, got {other:?}"),
    }
    drop(tx);
    assert_ends(&mut stream, LABEL).await;
}

/// Both `BakeoffProgressCell`-typed cells held AT ONCE, as production holds
/// them while a bake-off and a sweep run together (`cockpit_live` sets each
/// cell on its own run request and clears each only on its own completion).
/// Each unit is taken by ITS reference id; each must take ITS cell's receiver
/// (checked right after its `stream()`, so the red names the cell whichever
/// unit streams first) and relay ITS sender's distinctive value in ITS
/// variant. An arm that drains the sibling cell whenever that cell is held
/// fails here and nowhere else (the falsifier's OWN-1): every other fixture
/// that streams holds at most one of the two cells (the all-Some identity
/// fixtures hold both, at equal salts, and only hash them).
///
/// Run at two salt pairs, one test each: EQUAL salts (7, 7), the realistic
/// case (production bumps the two salts independently, so they are often
/// equal), where only the `TypeId` tells the two ids apart and a stolen cell
/// shows at the receiver check; and DISTINCT salts (7, 8), where an arm that
/// reads the sibling's cell also carries the sibling's salt, so it fails at
/// `take_unit`.
async fn both_progress_cells_relay(bakeoff_salt: u64, sweep_salt: u64) {
    let case = format!("both cells, salts {bakeoff_salt}/{sweep_salt}");
    let rt = tokio::runtime::Handle::current();
    let bus = new_bus();
    let (bakeoff_tx, bakeoff_rx) = tokio::sync::mpsc::channel(1);
    let (sweep_tx, sweep_rx) = tokio::sync::mpsc::channel(1);
    let bakeoff_cell: BakeoffProgressCell = held(bakeoff_rx);
    let sweep_cell: BakeoffProgressCell = held(sweep_rx);
    let bakeoff_ref = id_of(&BakeoffProgressRecipe {
        rt_handle: rt.clone(),
        rx: taken(),
        salt: bakeoff_salt,
    });
    let sweep_ref = id_of(&SweepProgressRecipe {
        rt_handle: rt.clone(),
        rx: taken(),
        salt: sweep_salt,
    });

    let ctx = SubscriptionCtx {
        bakeoff_progress: Some((&bakeoff_cell, bakeoff_salt)),
        sweep_progress: Some((&sweep_cell, sweep_salt)),
        ..boot_ctx(&rt, &bus)
    };
    let mut units = into_recipes(subscription_batch(&ctx));
    let bakeoff_label = format!("{case}: bake-off unit");
    let sweep_label = format!("{case}: sweep unit");
    let bakeoff_unit = take_unit(&mut units, bakeoff_ref, &bakeoff_label);
    let sweep_unit = take_unit(&mut units, sweep_ref, &sweep_label);

    let mut bakeoff_stream = bakeoff_unit.stream(empty_event_stream());
    assert!(
        bakeoff_cell.lock().unwrap().is_none(),
        "{case}: the bake-off unit took the BAKE-OFF cell's receiver"
    );
    let mut sweep_stream = sweep_unit.stream(empty_event_stream());
    assert!(
        sweep_cell.lock().unwrap().is_none(),
        "{case}: the sweep unit took the SWEEP cell's receiver"
    );

    bakeoff_tx
        .send(progress_event(3, 7, "bake-off cell"))
        .await
        .expect("the bake-off receiver is held");
    sweep_tx
        .send(progress_event(4, 12, "sweep cell"))
        .await
        .expect("the sweep receiver is held");
    match next_item(&mut bakeoff_stream, &bakeoff_label).await {
        Message::BakeoffProgress(p) => assert_eq!(
            (p.done, p.total, p.current_id.as_str()),
            (3, 7, "bake-off cell"),
            "{case}: the bake-off unit relays the BAKE-OFF cell's value"
        ),
        other => panic!("{case}: the bake-off unit: expected BakeoffProgress, got {other:?}"),
    }
    match next_item(&mut sweep_stream, &sweep_label).await {
        Message::SweepProgress(p) => assert_eq!(
            (p.done, p.total, p.current_id.as_str()),
            (4, 12, "sweep cell"),
            "{case}: the sweep unit relays the SWEEP cell's value"
        ),
        other => panic!("{case}: the sweep unit: expected SweepProgress, got {other:?}"),
    }
    drop(bakeoff_tx);
    drop(sweep_tx);
    assert_ends(&mut bakeoff_stream, &bakeoff_label).await;
    assert_ends(&mut sweep_stream, &sweep_label).await;
}

/// [`both_progress_cells_relay`] at EQUAL salts (7, 7), the realistic case.
#[tokio::test]
async fn batched_bakeoff_and_sweep_units_drain_own_cells_equal_salts() {
    both_progress_cells_relay(7, 7).await;
}

/// [`both_progress_cells_relay`] at DISTINCT salts (7, 8).
#[tokio::test]
async fn batched_bakeoff_and_sweep_units_drain_own_cells_distinct_salts() {
    both_progress_cells_relay(7, 8).await;
}

/// The Lab run progress unit: salt pair, receiver ownership, two bars relayed
/// while the sender lives, then `LabRunProgressDone` once it drops, then the
/// end.
#[tokio::test]
async fn batched_lab_progress_recipe_relays() {
    const LABEL: &str = "lab_progress";
    let rt = tokio::runtime::Handle::current();
    let bus = new_bus();
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    let cell: LabProgressCell = held(rx);
    let reference = |salt| {
        id_of(&LabProgressRecipe {
            rt_handle: rt.clone(),
            rx: taken(),
            salt,
        })
    };

    let base = boot_ctx(&rt, &bus);
    let boot = into_recipes(subscription_batch(&base));
    assert_eq!(
        count_id(&boot, reference(7)),
        0,
        "{LABEL}: the boot ctx batches no unit with the reference id"
    );
    assert_eq!(
        count_id(&boot, reference(0)),
        0,
        "{LABEL}: the boot ctx batches no unit with the boot-salt (0) reference id"
    );
    let ctx = SubscriptionCtx {
        lab_progress: Some((&cell, 7)),
        ..base
    };
    let mut units = into_recipes(subscription_batch(&ctx));
    assert_eq!(
        count_id(&units, reference(6)),
        0,
        "{LABEL}: salt-6 reference must match no unit"
    );
    let mut stream = take_unit(&mut units, reference(7), LABEL).stream(empty_event_stream());
    assert!(
        cell.lock().unwrap().is_none(),
        "{LABEL}: the batched unit took THIS ctx's receiver"
    );

    tx.send(backtest::progress::Progress {
        current_bar: 5,
        total_bars: 10,
        elapsed_ms: 1,
    })
    .await
    .expect("the receiver is held by the stream");
    match next_item(&mut stream, LABEL).await {
        Message::LabRunProgress(p) => assert_eq!(
            (p.current_bar, p.total_bars, p.elapsed_ms),
            (5, 10, 1),
            "{LABEL}: the relayed progress is the one sent"
        ),
        other => panic!("{LABEL}: expected LabRunProgress, got {other:?}"),
    }
    tx.send(backtest::progress::Progress {
        current_bar: 6,
        total_bars: 10,
        elapsed_ms: 2,
    })
    .await
    .expect("the receiver is still held by the stream");
    match nth_item(&mut stream, LABEL, "second", RELAY).await {
        Message::LabRunProgress(p) => assert_eq!(
            (p.current_bar, p.total_bars, p.elapsed_ms),
            (6, 10, 2),
            "{LABEL}: the second relayed progress is the second one sent"
        ),
        other => panic!("{LABEL}: expected a second LabRunProgress, got {other:?}"),
    }
    drop(tx);
    match nth_item(&mut stream, LABEL, "third", RELAY).await {
        Message::LabRunProgressDone => {}
        other => panic!("{LABEL}: expected LabRunProgressDone after the drop, got {other:?}"),
    }
    assert_ends(&mut stream, LABEL).await;
}

/// The training-log unit (a std receiver, bridged by an OS thread): salt pair,
/// receiver ownership, two lines relayed while the sender lives, then the end
/// once it drops. Its waits are bounded by [`RELAY_XTHREAD`], not [`RELAY`]:
/// each item crosses the `training-log-bridge` thread.
#[tokio::test]
async fn batched_training_log_recipe_relays() {
    const LABEL: &str = "training_log";
    let rt = tokio::runtime::Handle::current();
    let bus = new_bus();
    let (tx, rx) = std::sync::mpsc::channel();
    let cell: TrainingLogCell = held(rx);
    let reference = |salt| {
        id_of(&TrainingLogRecipe {
            rt_handle: rt.clone(),
            rx: taken(),
            salt,
        })
    };

    let base = boot_ctx(&rt, &bus);
    let boot = into_recipes(subscription_batch(&base));
    assert_eq!(
        count_id(&boot, reference(7)),
        0,
        "{LABEL}: the boot ctx batches no unit with the reference id"
    );
    assert_eq!(
        count_id(&boot, reference(0)),
        0,
        "{LABEL}: the boot ctx batches no unit with the boot-salt (0) reference id"
    );
    let ctx = SubscriptionCtx {
        training_log: Some((&cell, 7)),
        ..base
    };
    let mut units = into_recipes(subscription_batch(&ctx));
    assert_eq!(
        count_id(&units, reference(6)),
        0,
        "{LABEL}: salt-6 reference must match no unit"
    );
    let mut stream = take_unit(&mut units, reference(7), LABEL).stream(empty_event_stream());
    assert!(
        cell.lock().unwrap().is_none(),
        "{LABEL}: the batched unit took THIS ctx's receiver"
    );

    tx.send(ui::lab::trainer::TrainingLogLine {
        text: SmolStr::new("epoch 1"),
        is_stderr: false,
    })
    .expect("the receiver is held by the bridge");
    match nth_item(&mut stream, LABEL, "first", RELAY_XTHREAD).await {
        Message::TrainingLogLine(text) => {
            assert_eq!(text, "epoch 1", "{LABEL}: the relayed line is the one sent");
        }
        other => panic!("{LABEL}: expected TrainingLogLine, got {other:?}"),
    }
    tx.send(ui::lab::trainer::TrainingLogLine {
        text: SmolStr::new("epoch 2"),
        is_stderr: true,
    })
    .expect("the receiver is still held by the bridge");
    match nth_item(&mut stream, LABEL, "second", RELAY_XTHREAD).await {
        Message::TrainingLogLine(text) => {
            assert_eq!(
                text, "epoch 2",
                "{LABEL}: the second relayed line is the second one sent"
            );
        }
        other => panic!("{LABEL}: expected a second TrainingLogLine, got {other:?}"),
    }
    drop(tx);
    assert_ends_within(&mut stream, LABEL, RELAY_XTHREAD).await;
}

/// The trail-mirror unit: absent without a handle, exactly one with it,
/// subscribed to THIS handle's sender, relaying a tick and then a second one
/// while the sender lives, and ending once every sender is gone.
#[tokio::test]
async fn batched_trail_recipe_relays() {
    const LABEL: &str = "trail";
    let rt = tokio::runtime::Handle::current();
    let bus = new_bus();
    let (req_tx, _req_rx) = tokio::sync::mpsc::channel(1);
    let handle = TrailMirrorHandle {
        req_tx,
        tick_tx: tokio::sync::broadcast::channel(4).0,
    };
    // `TrailMirrorRecipe` is private: take its id from the production
    // constructor over ANOTHER handle (its hash never reads the handle).
    let reference = {
        let other = TrailMirrorHandle {
            req_tx: tokio::sync::mpsc::channel(1).0,
            tick_tx: tokio::sync::broadcast::channel(1).0,
        };
        let recipes = into_recipes(ui::live::trail_mirror_subscription(other));
        id_of(
            recipes
                .first()
                .expect("trail_mirror_subscription builds one recipe")
                .as_ref(),
        )
    };

    let base = boot_ctx(&rt, &bus);
    assert_eq!(
        count_id(&into_recipes(subscription_batch(&base)), reference),
        0,
        "{LABEL}: the boot ctx batches no unit with the reference id"
    );
    let ctx = SubscriptionCtx {
        trail: Some(&handle),
        ..base
    };
    let mut units = into_recipes(subscription_batch(&ctx));
    let mut stream = take_unit(&mut units, reference, LABEL).stream(empty_event_stream());

    handle
        .tick_tx
        .send(TrailMirrorTick::TrailUpdated("audit-c2".to_owned()))
        .unwrap_or_else(|_| panic!("{LABEL}: the batched unit subscribed to THIS ctx's sender"));
    match next_item(&mut stream, LABEL).await {
        Message::TrailMirrorTick(TrailMirrorUiTick::TrailUpdated(audit_id)) => {
            assert_eq!(
                audit_id, "audit-c2",
                "{LABEL}: the relayed tick is the one sent"
            );
        }
        other => panic!("{LABEL}: expected TrailMirrorTick(TrailUpdated(..)), got {other:?}"),
    }
    handle
        .tick_tx
        .send(TrailMirrorTick::TrailUpdated("audit-c2-second".to_owned()))
        .unwrap_or_else(|_| panic!("{LABEL}: the batched unit is still subscribed"));
    match nth_item(&mut stream, LABEL, "second", RELAY).await {
        Message::TrailMirrorTick(TrailMirrorUiTick::TrailUpdated(audit_id)) => {
            assert_eq!(
                audit_id, "audit-c2-second",
                "{LABEL}: the second relayed tick is the second one sent"
            );
        }
        other => {
            panic!("{LABEL}: expected a second TrailMirrorTick(TrailUpdated(..)), got {other:?}")
        }
    }
    drop(handle);
    assert_ends(&mut stream, LABEL).await;
}

/// The always-on activity unit. Its id comes from a reference over ANOTHER
/// bus (the hash never reads the bus); nothing is published there, since an
/// event on a bus no unit subscribed to reaches no receiver and could never be
/// heard. First a LIVENESS check: with nothing published on its own bus, the
/// unit stays open and silent (C2-P13, a unit over a bus nobody else holds,
/// ENDS here). Then the positive half, which is what pins the ctx-bus wiring:
/// an activity started on the ctx's bus is relayed as its `Start`, and a
/// second one too (the body loops over its receiver).
#[tokio::test]
async fn batched_activity_recipe_relays() {
    const LABEL: &str = "activity";
    let rt = tokio::runtime::Handle::current();
    let bus = new_bus();
    let other_bus = new_bus();
    let reference = id_of(&ActivityRecipe {
        bus: Arc::clone(&other_bus),
    });

    let mut units = into_recipes(subscription_batch(&boot_ctx(&rt, &bus)));
    let mut stream = take_unit(&mut units, reference, LABEL).stream(empty_event_stream());

    assert_silent(
        &mut stream,
        "activity: open and silent with nothing published on its own bus",
    )
    .await;
    let _activity = bus.activity().start(ActivityKind::LabRun, "c2-activity");
    match next_item(&mut stream, "activity relays the ctx bus").await {
        Message::ActivityEventReceived(event) => {
            assert_eq!(
                event.label, "c2-activity",
                "{LABEL}: the relayed event is the one started"
            );
            assert!(
                matches!(event.phase, ActivityPhase::Start { .. }),
                "{LABEL}: the relayed event is the Start: {event:?}"
            );
        }
        other => panic!("{LABEL}: expected ActivityEventReceived, got {other:?}"),
    }
    let _second = bus
        .activity()
        .start(ActivityKind::LabRun, "c2-activity-second");
    match nth_item(&mut stream, "activity relays the ctx bus", "second", RELAY).await {
        Message::ActivityEventReceived(event) => {
            assert_eq!(
                event.label, "c2-activity-second",
                "{LABEL}: the second relayed event is the second one started"
            );
            assert!(
                matches!(event.phase, ActivityPhase::Start { .. }),
                "{LABEL}: the second relayed event is its Start: {event:?}"
            );
        }
        other => panic!("{LABEL}: expected a second ActivityEventReceived, got {other:?}"),
    }
    drop(units);
}

/// The positive label of [`batched_bus_recipe_relays`]. It carries the hint for
/// the one false red that test can show: the reference id is taken at
/// `Mode`'s index in [`BUS_CHANNELS`], so a reorder of `ui::live::subscription`'s
/// channels makes `take_unit` pick another channel's unit, which then never
/// receives the mode published.
const BUS_MODE_RELAYS: &str = "bus/mode relays the ctx bus (if `ui::live::subscription` reordered \
                               its channels, update BUS_CHANNELS)";

/// The always-on bus batch's `Mode` unit. `BusRecipe` is private, and its id
/// hashes only its type and channel, never the bus: the reference comes from
/// `ui::live::subscription` over ANOTHER bus, at `Mode`'s index in
/// [`BUS_CHANNELS`]. So this test depends on [`BUS_CHANNELS`] matching
/// `ui::live::subscription`'s channel ORDER, which nothing in this file pins
/// (C1's units test only counts the bus units; outside it,
/// `recipe_off_runtime_poll.rs`'s `bus_recipes_relay_off_runtime` goes red on a
/// reorder, at its `BUS_BATCH` table). A reorder shows here as a false red at
/// the positive half's "NO item within 2s", whose text names the table to
/// update ([`BUS_MODE_RELAYS`]).
///
/// As for the activity unit: nothing is published on the other bus, a
/// LIVENESS check (open and silent with nothing published on its own bus)
/// runs first, and the positive half pins the ctx-bus wiring: a mode published
/// on the ctx's bus arrives as `AgentModeChanged(Paper)`, then a second one
/// (`Halted`) as `AgentHaltedExternally` with its reason.
#[tokio::test]
async fn batched_bus_recipe_relays() {
    let rt = tokio::runtime::Handle::current();
    let bus = new_bus();
    let other_bus = new_bus();
    let mode_ix = BUS_CHANNELS
        .iter()
        .position(|&c| c == Channel::Mode)
        .expect("BUS_CHANNELS lists Mode");
    let reference =
        id_of(into_recipes(ui::live::subscription(Arc::clone(&other_bus)))[mode_ix].as_ref());

    // `units` stays bound to the end: its ten sibling bus units hold the
    // batch's bus (see `take_unit`).
    let mut units = into_recipes(subscription_batch(&boot_ctx(&rt, &bus)));
    let mut stream = take_unit(&mut units, reference, "bus/mode").stream(empty_event_stream());

    assert_silent(
        &mut stream,
        "bus/mode: open and silent with nothing published on its own bus",
    )
    .await;
    bus.publish_mode(agent::AgentMode::Running);
    match next_item(&mut stream, BUS_MODE_RELAYS).await {
        Message::AgentModeChanged(ui::state::AgentMode::Paper) => {}
        other => panic!("bus/mode: expected AgentModeChanged(Paper), got {other:?}"),
    }
    bus.publish_mode(agent::AgentMode::Halted {
        reason: "c2-halt".to_owned(),
    });
    match nth_item(&mut stream, BUS_MODE_RELAYS, "second", RELAY).await {
        Message::AgentHaltedExternally(reason) => assert_eq!(
            reason, "c2-halt",
            "bus/mode: the second relayed mode is the second one published"
        ),
        other => panic!("bus/mode: expected a second item, AgentHaltedExternally, got {other:?}"),
    }
    drop(units);
}

/// The always-on status-bar clock, through the moved `ServerTimeRecipe::stream`,
/// on a PAUSED clock: its first item is a `ServerTimeTick` (due at 1 s of
/// virtual time). Paired with the toast relay: the two moved bodies must yield
/// different first variants.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn batched_server_time_recipe_relays() {
    let rt = tokio::runtime::Handle::current();
    let bus = new_bus();
    let reference = id_of(&ServerTimeRecipe {
        rt_handle: rt.clone(),
    });
    let mut units = into_recipes(subscription_batch(&boot_ctx(&rt, &bus)));
    let mut stream = take_unit(&mut units, reference, "server-time").stream(empty_event_stream());
    let first = next_item(&mut stream, "server-time").await;
    assert!(
        matches!(first, Message::ServerTimeTick(_)),
        "server-time first item: {first:?}"
    );
}

/// The always-on toast ticker, through the moved `ToastDismissRecipe::stream`,
/// on a PAUSED clock: its first item is a `ToastTick` (due at 500 ms).
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn batched_toast_dismiss_recipe_relays() {
    let rt = tokio::runtime::Handle::current();
    let bus = new_bus();
    let reference = id_of(&ToastDismissRecipe {
        rt_handle: rt.clone(),
    });
    let mut units = into_recipes(subscription_batch(&boot_ctx(&rt, &bus)));
    let mut stream = take_unit(&mut units, reference, "toast").stream(empty_event_stream());
    let first = next_item(&mut stream, "toast").await;
    assert!(
        matches!(first, Message::ToastTick(_)),
        "toast first item: {first:?}"
    );
}

/// The `ModalEsc` unit, found as the ONE id that opening the modal adds (set
/// difference; no fn-pointer id is computed in this crate). Two units from two
/// batches: one fed `Esc` presses, one fed three non-closing event kinds (an
/// `Enter` press, an `Esc` release, a `ModifiersChanged`), each at both
/// statuses. No mouse, window or touch event is fed: an arm-level filter that
/// closed the modal on one of those would pass unit B (the fn it batches is
/// C1's tests'). `listen_with`'s stream ends only when its input ends, so each
/// collect is bounded by [`RELAY`].
#[tokio::test]
async fn batched_modal_esc_relays_escape_only() {
    let rt = tokio::runtime::Handle::current();
    let bus = new_bus();
    let base = boot_ctx(&rt, &bus);
    let open = SubscriptionCtx {
        tape_audit_modal_open: true,
        ..base
    };
    let closed_ids: HashSet<u64> = ids(&into_recipes(subscription_batch(&base)))
        .into_iter()
        .collect();
    let added: Vec<u64> = ids(&into_recipes(subscription_batch(&open)))
        .into_iter()
        .filter(|id| !closed_ids.contains(id))
        .collect();
    assert_eq!(
        added.len(),
        1,
        "opening the modal adds exactly one unit: {added:?}"
    );

    let statuses = [Status::Ignored, Status::Captured];
    let mut esc_units = into_recipes(subscription_batch(&open));
    let esc = take_unit(&mut esc_units, added[0], "modal-esc (Esc presses)").stream(event_stream(
        statuses
            .iter()
            .map(|&s| interaction(key_pressed(Named::Escape, Code::Escape), s))
            .collect(),
    ));
    let mut other_units = into_recipes(subscription_batch(&open));
    let others =
        take_unit(&mut other_units, added[0], "modal-esc (other events)").stream(event_stream(
            statuses
                .iter()
                .flat_map(|&s| {
                    [
                        interaction(key_pressed(Named::Enter, Code::Enter), s),
                        interaction(key_released(Named::Escape, Code::Escape), s),
                        interaction(
                            iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(
                                Modifiers::empty(),
                            )),
                            s,
                        ),
                    ]
                })
                .collect(),
        ));

    let got = timeout(RELAY, esc.collect::<Vec<_>>())
        .await
        .expect("ModalEsc unit must end when its input ends (Esc presses)");
    assert!(
        matches!(
            got.as_slice(),
            [Message::TapeAuditModalClosed, Message::TapeAuditModalClosed]
        ),
        "batched ModalEsc unit closes the modal on each Esc press: {got:?}"
    );
    let got = timeout(RELAY, others.collect::<Vec<_>>())
        .await
        .expect("ModalEsc unit must end when its input ends (other events)");
    assert!(
        got.is_empty(),
        "batched ModalEsc unit maps only an Esc press: {got:?}"
    );
}

/// The frame-2 condition, exactly: frame 1's units took both receivers, and
/// frame 2 is built from the SAME ctx. Frame 2 must still batch each unit under
/// frame 1's id (so iced keeps frame 1's running stream, the only holder of the
/// receiver), and frame 1's streams must still relay.
#[tokio::test]
async fn forward_plan_stays_batched_after_receiver_taken() {
    let rt = tokio::runtime::Handle::current();
    let bus = new_bus();
    let (plan_tx, plan_rx) = tokio::sync::mpsc::channel(1);
    let plan: ForwardPlanCell = held(plan_rx);
    let (narration_tx, narration_rx) = tokio::sync::mpsc::channel(1);
    let narration: NarrationOutcomeCell = held(narration_rx);
    let plan_id = id_of(&ForwardPlanRecipe {
        rt_handle: rt.clone(),
        rx: taken(),
    });
    let narration_id = id_of(&NarrationOutcomeRecipe {
        rt_handle: rt.clone(),
        rx: taken(),
    });

    let base = boot_ctx(&rt, &bus);
    let boot = into_recipes(subscription_batch(&base));
    assert_eq!(
        count_id(&boot, plan_id),
        0,
        "control: the boot ctx batches no plan unit"
    );
    assert_eq!(
        count_id(&boot, narration_id),
        0,
        "control: the boot ctx batches no narration unit"
    );

    let ctx = SubscriptionCtx {
        forward_plan: Some(&plan),
        narration_outcome: Some(&narration),
        ..base
    };
    let mut frame1 = into_recipes(subscription_batch(&ctx));
    let mut plan_stream =
        take_unit(&mut frame1, plan_id, "frame 1 forward_plan").stream(empty_event_stream());
    let mut narration_stream = take_unit(&mut frame1, narration_id, "frame 1 narration_outcome")
        .stream(empty_event_stream());
    assert!(
        plan.lock().unwrap().is_none(),
        "frame 1's unit took the plan receiver"
    );
    assert!(
        narration.lock().unwrap().is_none(),
        "frame 1's unit took the narration receiver"
    );

    let inputs = ctx.inputs();
    assert_eq!(
        (inputs.forward_plan, inputs.narration_outcome),
        (Held::Yes, Held::Yes),
        "frame 2: both cells are still held (their receivers are taken)"
    );
    let frame2 = into_recipes(subscription_batch(&ctx));
    assert_eq!(
        count_id(&frame2, plan_id),
        1,
        "frame 2 batches the plan unit under frame 1's id"
    );
    assert_eq!(
        count_id(&frame2, narration_id),
        1,
        "frame 2 batches the narration unit under frame 1's id"
    );

    plan_tx
        .send(forward_plan_for("v0.frame2"))
        .await
        .expect("frame 1's stream holds the plan receiver");
    match next_item(&mut plan_stream, "frame 1's plan stream, after frame 2").await {
        Message::ForwardPlanReceived(view) => assert_eq!(
            view.strategy, "v0.frame2",
            "frame 1's plan stream relays the plan sent after frame 2"
        ),
        other => panic!("frame 1's plan stream: expected ForwardPlanReceived, got {other:?}"),
    }
    narration_tx
        .send(agent::NarrationOutcome::FellBack)
        .await
        .expect("frame 1's stream holds the narration receiver");
    match next_item(
        &mut narration_stream,
        "frame 1's narration stream, after frame 2",
    )
    .await
    {
        Message::BakeoffNarrationCompleted(ui::leaderboard::NarrationOutcome::FellBack) => {}
        other => panic!(
            "frame 1's narration stream: expected BakeoffNarrationCompleted(FellBack), got {other:?}"
        ),
    }
}

// ── Wave C2: recipe identity ─────────────────────────────────────────────────

/// Two batches from one ctx, both ALIVE at once (so no allocation is reused):
/// every unit's id is equal across them, index by index. An id that hashed an
/// address or a counter would differ. The eleven bus units are among the 22
/// (the `Bus` arm is the one call `subscription(Arc::clone(ctx.bus))`), so the
/// bus ids are covered here, with both batches alive; no separate bus-only
/// half is needed (an earlier one built its two batches as temporaries, so an
/// allocation could be reused between them).
#[test]
fn recipe_ids_are_frame_stable() {
    let fx = Fixture::new();
    let ctx = fx.all_some();
    let a = into_recipes(subscription_batch(&ctx));
    let b = into_recipes(subscription_batch(&ctx));
    assert_eq!(
        (a.len(), b.len()),
        (ALL_SOME_UNITS, ALL_SOME_UNITS),
        "both all-Some batches have {ALL_SOME_UNITS} units"
    );
    for (k, (x, y)) in a.iter().zip(&b).enumerate() {
        assert!(
            !std::ptr::addr_eq(&**x, &**y),
            "premise: unit {k} of the two live batches is two allocations"
        );
    }
    for (k, (x, y)) in ids(&a).into_iter().zip(ids(&b)).enumerate() {
        assert_eq!(x, y, "unit {k} id is frame-stable");
    }
}

/// Two all-Some ctxs over the SAME cells, all four salts 1 in one and 2 in the
/// other: the ids that differ are EXACTLY the four salted take-once units, at
/// the indices derived from the written-out tables (13, 15, 19, 20), and no
/// other unit's id moves.
#[test]
fn salted_recipe_ids_change_with_salt() {
    let fx = Fixture::new();
    let one = ids(&into_recipes(subscription_batch(&fx.all_some_salted(1))));
    let two = ids(&into_recipes(subscription_batch(&fx.all_some_salted(2))));
    assert_eq!(
        (one.len(), two.len()),
        (ALL_SOME_UNITS, ALL_SOME_UNITS),
        "both all-Some batches have {ALL_SOME_UNITS} units"
    );
    let differing: Vec<usize> = one
        .iter()
        .zip(&two)
        .enumerate()
        .filter_map(|(k, (x, y))| (x != y).then_some(k))
        .collect();
    let mut want: Vec<usize> = [
        V::LabProgress,
        V::TrainingLog,
        V::BakeoffProgress,
        V::SweepProgress,
    ]
    .map(all_some_unit_index)
    .to_vec();
    want.sort_unstable();
    assert_eq!(
        differing, want,
        "the units whose id changes with the salt are exactly the four salted take-once units \
         (LabProgress, TrainingLog, BakeoffProgress, SweepProgress)"
    );
}

/// Every unit of the all-Some batch has its own id, at the boot salts (0) and
/// after one run of each (1). With equal salts only the `TypeId` separates
/// `BakeoffProgress` from `SweepProgress` and `LabProgress` from `TrainingLog`,
/// and only the channel separates the eleven bus units: iced skips a duplicate
/// id, so a collision silently drops a unit.
#[test]
fn recipe_ids_are_pairwise_distinct() {
    let fx = Fixture::new();
    for salt in [0, 1] {
        let units = into_recipes(subscription_batch(&fx.all_some_salted(salt)));
        let all = ids(&units);
        let distinct: HashSet<u64> = all.iter().copied().collect();
        assert_eq!(
            units.len(),
            ALL_SOME_UNITS,
            "the all-Some batch has {ALL_SOME_UNITS} units (every salt {salt})"
        );
        assert_eq!(
            distinct.len(),
            ALL_SOME_UNITS,
            "all 22 units of the all-Some batch have distinct ids (every salt {salt}): {all:?}"
        );
    }
}
