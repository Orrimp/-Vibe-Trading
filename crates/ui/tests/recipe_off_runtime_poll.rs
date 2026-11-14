//! Story 2-63 (lab-recipe-test-harness v0.3), Wave B: EXECUTOR-FIDELITY S1.
//! The recipe streams listed under "What runs here" are built where iced
//! builds them and polled where iced polls them, and neither place has a tokio
//! context.
//!
//! ## What runs here, and how close to production
//!
//! - Through the production `Recipe::stream` itself: `ForwardPlanRecipe`,
//!   `NarrationOutcomeRecipe`, `BakeoffProgressRecipe`, `SweepProgressRecipe`,
//!   `ActivityRecipe` (`crates/ui/src/live.rs`), `LabProgressRecipe`
//!   (`crates/ui/src/lab/progress.rs`) and `TrainingLogRecipe`
//!   (`crates/ui/src/lab/training_log.rs`).
//! - Through the production constructor and iced's own `into_recipes`, the way
//!   iced unpacks a `Subscription`: `TrailMirrorRecipe` (private, built by
//!   `ui::live::trail_mirror_subscription`) and the eleven always-on
//!   `BusRecipe` relays (private, built by `ui::live::subscription(bus)` over a
//!   real `agent::EventBus`; their count and their iced ids checked first, one
//!   checked publish per channel, and every other relay checked for CROSS-TALK
//!   around each publish). Every one of the eleven channels has a
//!   `pub fn publish_*` on `EventBus`, so none is left out.
//! - Through their seams ONLY: `ServerTimeRecipe` and `ToastDismissRecipe`.
//!   When this file was written they were private to the `cockpit_live` bin,
//!   which no integration test can reach; story 2-63 Wave C1 moved both into
//!   `ui::live` as `pub` structs, but this file still drives only their seams.
//!   Each `stream()` body is one call, to `ui::live::server_time_stream_impl` /
//!   `toast_dismiss_stream_impl` with the recipe's runtime `Handle`, and that
//!   call is what runs here. The bodies themselves are now streamed ON a tokio
//!   runtime by `server_time_recipe_stream_yields_tick` /
//!   `toast_dismiss_recipe_stream_yields_tick` (Wave E1b) and by
//!   `subscription_batch_inclusion.rs`'s `batched_server_time_recipe_relays` /
//!   `batched_toast_dismiss_recipe_relays` (Wave C2, on the batch
//!   `subscription_batch` builds from a TEST ctx). OFF-runtime nothing
//!   builds them, so a body that built its interval outside the seam's guard
//!   is still covered by no test: every test that reaches the body supplies a
//!   tokio context.
//!
//! ## Why this file exists
//!
//! Every other recipe test in `crates/ui/tests/` is a `#[tokio::test]`, so the
//! stream under test is built AND polled inside a tokio runtime context.
//! Production does neither there:
//!
//! - `crates/ui/Cargo.toml` builds iced with `features = [..., "thread-pool", ...]`;
//! - `iced_futures-0.14.0/src/backend/native/thread_pool.rs`:
//!   `pub type Executor = futures::executor::ThreadPool;`
//! - `iced_futures-0.14.0/src/backend/default.rs` selects that backend only
//!   while neither `tokio` nor `smol` is enabled, and
//!   `cargo tree -p ui --features live -e features -i iced_futures` lists
//!   exactly `default` + `thread-pool` (measured 2026-11-11);
//! - `Recipe::stream` runs on iced's EVENT-LOOP thread. `iced_winit-0.14.0/src/lib.rs`
//!   calls `runtime.track(subscription::into_recipes(` in `run` and
//!   `runtime.track(recipes);` in `update`, the latter reached from the winit
//!   handler's `let poll = self.instance.as_mut().poll(&mut self.context);`.
//!   `iced_futures-0.14.0/src/runtime.rs`, `Runtime::track`, runs
//!   `let futures = executor.enter(|| {` around `subscriptions.update(...)`,
//!   and `subscription/tracker.rs`, `Tracker::update`, calls
//!   `let mut stream = recipe.stream(event_receiver.boxed());` synchronously
//!   inside it, for every new recipe of the batch. For the thread pool,
//!   `enter` is the trait default, plain `f()` (`executor.rs`), so it sets no
//!   tokio context;
//! - only the DRAIN goes to the pool (`executor.spawn(future);` in
//!   `Runtime::track`, a loop over
//!   `futures::future::select(&mut canceled, stream.next())`). A pool worker
//!   has no tokio context either.
//!
//! So a recipe that needs a reactor while it is built (an `interval` created
//! outside the runtime guard) or while it is polled (a `tokio::time::timeout`,
//! a `spawn_blocking`) is green in every sibling and panics in the cockpit.
//! Bug-log `#196` was exactly that: `TrainingLogRecipe`'s `spawn_blocking`,
//! found only because Wave 0 polled it off-runtime. The regression test that
//! landed with its fix (`recipe_stream_drains_without_a_tokio_runtime`,
//! `crates/ui/src/lab/training_log.rs`) calls `stream()` and polls on ONE
//! thread, and checks the premise only before `stream()`. A guard leaked in
//! `stream()` would feed every one of its polls a context, so TrainingLog runs
//! here too (`training_log_recipe_drains_off_runtime`, B-P20..B-P22).
//!
//! ## The pattern (every test)
//!
//! - A plain `#[test]`, whose first statement takes the binary's timing lock
//!   (see "Timing isolation" below). The producer side is a multi-thread
//!   tokio runtime ([`ProducerRuntime`], `enable_all`, kept alive for the
//!   whole test). The harness thread never enters it: tasks reach it through
//!   `Runtime::spawn`, and it is shut down on a helper thread.
//! - The executor is iced's own, [`iced_executor`]: built with
//!   `iced::Executor::new` and spawned onto with `iced::Executor::spawn`, the
//!   two calls iced makes. Its return type is a COMPILE-TIME pin. It
//!   type-checks only while `iced::executor::Default` IS
//!   `futures::executor::ThreadPool`. If a `tokio` backend is ever enabled,
//!   this file stops building, because its premise has changed.
//! - [`poll_batch_on_iced_executor`] mirrors `Runtime::track`, with ONE step
//!   left out. On a fresh std thread that stands in for iced's event loop, the
//!   REAL recipes are built and `.stream(empty EventStream)`-ed inside ONE
//!   `iced::Executor::enter`. Only then is each drain spawned onto the pool
//!   with `iced::Executor::spawn`. [`poll_on_iced_executor`] is the one-recipe
//!   case. Left out: `Tracker::update` first hashes every recipe and skips one
//!   whose id is already tracked (also within one batch). Here every recipe is
//!   streamed, which is what Tracker does only while the ids are distinct:
//!   trivially for one recipe on a fresh tracker, and for the bus batch
//!   because its test asserts it first, as a production fact ([`tracker_ids`],
//!   B-P35).
//! - The premise is `tokio::runtime::Handle::try_current().is_err()`. It is
//!   checked on every thread that touches the stream, wherever a context could
//!   appear:
//!   - on the event-loop stand-in, before and after `stream()`. BEFORE is a
//!     harness self-check, true by construction (the stand-in is a freshly
//!     spawned std thread): it can fail only if the harness broke, so it is no
//!     evidence about production. AFTER is the one about production, and it is
//!     sampled per RECIPE, right after its own `stream()` returned ([`Built`]),
//!     so in a batch the first recipe that leaked is the one named (shape (h)).
//!     A `stream()` that leaks its `EnterGuard` leaves a context on iced's
//!     event-loop thread (B-P10, B-P11, B-P21, B-P22). It does not move the
//!     drain onto the runtime, because the drain runs on the pool. What it does
//!     is give every LATER `stream()` call on that thread a context, which in
//!     the cockpit would hide a B-P1-style guard deletion in ANOTHER recipe. No
//!     drain is spawned for a stream that fails this premise: its test fails
//!     there, and a drain would only print a second, consequential panic;
//!   - on the pool, immediately before and after EVERY poll, on the thread
//!     that made it, inside the same `poll` call ([`poll_watched`]). The pool
//!     can move the task to another worker on any wake, so one sample at the
//!     end would see only whichever worker made the last poll. The first
//!     violation fails the test. A stream BODY that leaks a context
//!     (`std::mem::forget(h.enter())` before a `tokio::time::timeout`) would
//!     otherwise drain inside the runtime and pass for the wrong reason;
//!   - after the stream is dropped (its `Drop` runs on the pool too).
//!
//!   The stand-alone control is `harness_thread_has_no_tokio_context`, which
//!   is two-sided: the same detector reports a context where one exists.
//! - The stream is polled once with the task's own waker BEFORE anything is
//!   produced. It must be `Pending`: live on its source, nothing buffered.
//! - Then producer and consumer TAKE TURNS, one item at a time:
//!   1. the harness waits for `Report::Waiting`, which the drain sends from
//!      inside a poll that returned `Pending`, after the stream registered its
//!      waker: the consumer is PARKED, with nothing buffered;
//!   2. ONE checked send, from a task on the runtime, while a sender stays
//!      alive;
//!   3. that item must be relayed before anything else is sent.
//!
//!   The last sender is dropped only after every item was observed, and only
//!   then must the stream END (after its closing message, where it has one).
//!   So each item crosses from a producer task on the runtime to a parked
//!   consumer on the pool through a wake, the hop production depends on. A
//!   stream that buffers until its channel closes hangs at its first item
//!   (B-P17) instead of passing on the close wake.
//! - Producer sends are CHECKED: a `Result` the producer returns, never a
//!   lossy `try_send` wrapper. Where production's sender discards its result
//!   (`ActivitySender`, `EventBus::publish_*`), a control subscriber must hold
//!   the event right after the send.
//! - Teardown is iced's too. Each drain is CANCELLABLE: iced keeps a
//!   `oneshot::Sender<()>` per execution and runs the drain as
//!   `select(&mut canceled, stream.next())` (`subscription/tracker.rs`).
//!   Here the harness holds that sender in a [`Drain`]. Dropping the `Drain`
//!   cancels the drain and waits, bounded, until the task and its stream are
//!   gone. Every test that HAS a [`ProducerRuntime`] declares it before its
//!   drains (right after its timing guard), so its drains drop before the
//!   runtime shuts down, on a failing test's unwind as well. A failing test
//!   leaves no drain behind, and the panic its drain used to print once the
//!   runtime shut down ("A Tokio 1.x context was found, but it is being
//!   shutdown", after the real cause) is gone.
//!   `a_dropped_drain_stops_before_the_runtime_shuts_down` proves it two-sided.
//!   The cancel branch drops the stream outside the drain's `catch_unwind`, so
//!   a `Drop` that panicked there would only be printed. No production stream
//!   reaches that branch in a passing run: every drain has run to its `Exit`,
//!   inside the capture, before its `Drain` drops ([`spawn_drain`]).
//!
//!   That ordering claim is about tests WITH a runtime and a drain. Five tests
//!   differ, and each is safe for its own reason:
//!   - `activity_recipe_subscribes_before_its_first_poll` has NO
//!     `ProducerRuntime`. Its producer steps run on the event-loop stand-in
//!     (start and drop an activity inside `make_stream`) and on the harness
//!     thread (drop the bus). Its stream holds only a `broadcast::Receiver`,
//!     which needs no runtime to be polled or dropped: no timer, no reactor
//!     registration. Its drain is also run to its END before the test
//!     returns. So there is no runtime whose shutdown a live drain could
//!     outlast;
//!   - `a_harness_self_check_is_not_reported_as_a_production_panic` and
//!     `a_late_panic_report_is_reported_as_the_panic` have none either. Their
//!     streams need no runtime (a `futures` channel, a `poll_fn`), and every
//!     drain they spawn has ended in its captured panic before the test
//!     returns;
//!   - `harness_thread_has_no_tokio_context` builds its runtime only for its
//!     last step and has no `Drain`: its executor tasks report once and
//!     finish. `a_teardown_or_lock_wait_that_expires_fails_the_test` has no
//!     `Drain` at all.
//!
//! ## Non-vacuity contract (bug-log #66 / story 2-63 AC3)
//!
//! Every wait is bounded, and its expiry FAILS the test: no accepting timeout
//! anywhere, with ONE documented exception. A wait for a report, a producer
//! step or a helper thread is a `std::sync::mpsc::Receiver::recv_timeout`.
//! The two waits that are not a report are bounded too: the timing lock is
//! taken within [`TIMING_WAIT`] (std's `RwLock` has no timed acquire, so it is
//! a deadline over a 1 ms retry), and the producer runtime must shut down
//! within [`SHUTDOWN_WAIT`] (a shutdown that hit its bound means a blocking
//! task outlived the test). `a_teardown_or_lock_wait_that_expires_fails_the_test`
//! forces both expiries. On a test that is ALREADY failing, the two teardown
//! waits (`Drain::drop`, `ProducerRuntime::drop`) print a note instead of
//! failing a second time, because a second panic would abort the run and hide
//! the first. The exception: self-test shape (g) in
//! `a_reactor_dependent_stream_fails_here_instead_of_passing` is the file's
//! one negative-control wait: a stream that buffers until its channel closes
//! must stay silent for 1 s while its sender lives, so there the expiry is
//! the expected outcome. It is bounded by its positive half, which follows
//! at once: the sender drops, and the same `next_item` must then get every
//! buffered item, in order, followed by the END. So the silence it accepts is
//! proven to be buffering, not a reader that cannot see items.
//!
//! A panic in `stream()` (on the stand-in) or while polled (on the pool) is
//! captured with `catch_unwind` and reported with its site and message,
//! which fails the test. A captured panic whose payload starts with
//! `HARNESS:` is a harness self-check ([`harness_check`]: the stream count of
//! a batch, a premise that is true by construction). [`interpret`] reports it
//! as a HARNESS DEFECT, never as "`Recipe::stream` PANICKED ... (no tokio
//! context)", so a broken harness cannot pass for a production failure. The
//! reverse holds too: a fact that PRODUCTION code produces (how many recipes
//! `ui::live::subscription` / `trail_mirror_subscription` return, their iced
//! ids) is never a harness self-check. Those are plain asserts on the test
//! thread, made before anything is streamed, whose message names both causes
//! (production changed, or the test's table is stale), so a production
//! regression cannot read as "the harness is out of date" (B-P35..B-P37).
//! `a_harness_self_check_is_not_reported_as_a_production_panic` proves the
//! label two-sided, at both sites (the stand-in and the executor).
//!
//! A report that may carry a captured panic gets [`PANIC_REPORT_WAIT`], 45 s:
//! a panic report is sent only after the panic hook ran, and under CI's
//! `RUST_BACKTRACE=1` the hook symbolizes a backtrace, which on a cold Windows
//! PDB may outlast 5 s. The build and the first poll get it up front. A LATER
//! poll is read with its shorter wait, and [`recv_report`] adds the 45 s only
//! when that wait expires while the drain is still INSIDE a poll or its drop
//! (a panic there may still be in its hook): a `Panicked` report then reads as
//! the panic, and anything else as a poll that blocked an executor thread for
//! longer than the wait, which fails. A wait that expires while the drain is
//! parked fails at once. `a_late_panic_report_is_reported_as_the_panic` proves
//! both halves. The budget only detects hangs, and its expiry still fails. A
//! task that vanishes without reporting (`Disconnected`) fails too. So does a
//! drain still running 5 s after its `Drain` cancelled it, when the test is
//! otherwise passing.
//!
//! ## Timing isolation
//!
//! libtest runs this binary's tests on parallel threads. Three of them assert
//! REAL-TIME bounds of their own. The two tick tests,
//! `server_time_stream_ticks_off_runtime` and
//! `toast_dismiss_stream_ticks_off_runtime`, bound each tick (see the cadence
//! paragraph below). The self-test `a_late_panic_report_is_reported_as_the_panic`
//! needs its test thread to start its 100 ms `SHORT` wait within 900 ms
//! (`HOLD`, 1 s, minus `SHORT`) of the blocking poll signalling that it began
//! (the poll sleeps `HOLD` from that signal). Five tests panic on purpose,
//! inside a `catch_unwind`:
//! `a_reactor_dependent_stream_fails_here_instead_of_passing`,
//! `a_harness_self_check_is_not_reported_as_a_production_panic`,
//! `a_late_panic_report_is_reported_as_the_panic` and
//! `a_teardown_or_lock_wait_that_expires_fails_the_test` several times each,
//! and `harness_thread_has_no_tokio_context` twice
//! (`catch_unwind(tokio::runtime::Handle::current)`, on two threads). Under
//! CI's `RUST_BACKTRACE=1` every one of those panics runs the hook, which
//! symbolizes a backtrace: a burst of CPU and disk work (a cold PDB on the
//! 4-vCPU Windows leg) at an unpredictable moment. A test with a real-time
//! bound that runs at the same time absorbs it as delay. For a tick test that
//! is a late tick. For the self-test it is a false RED (never a false green):
//! if its thread is descheduled past the 900 ms, bar 2's late item arrives
//! inside the short wait, `next_item` returns normally, and
//! `harness_failure`'s `expect_err` fails (round 6's falsifier, REASONED, not
//! measured).
//!
//! So one `static` [`RwLock`] (`TIMING`) serializes them, by one rule: a test
//! with a real-time bound of its own holds it for WRITING for its whole
//! duration ([`timing_exclusive`]); EVERY other test holds it for READING
//! ([`timing_shared`]). A real-time bound of its own is one that a scheduling
//! delay can push the test across: a tick window, or a wait that must start
//! before a deadline (the self-test's `SHORT` wait). Bounded waits whose
//! outcome no delay short of a hang can change do not count, although every
//! test has them: a hang detector such as `WAIT`, a wait whose expiry is the
//! expected outcome (shape (g), whose sender lives throughout the wait; the
//! forced expiries of `a_teardown_or_lock_wait_that_expires_fails_the_test`,
//! whose lock stays held and whose blocking task waits up to `WAIT` for its
//! release), and a lock wait on a free lock, which takes it at the first
//! try. The writers are exactly three: the two tick tests AND
//! `a_late_panic_report_is_reported_as_the_panic`. A new test must take one
//! of the two as its first statement, and the write lock exactly when it has
//! such a bound. Readers still run alongside each other, so a reader gets no
//! protection from another reader's panic: round 6 measured two readers
//! overlapping ("What has been measured" below), which is why the self-test
//! is a writer. A writer shares the binary with no other test, so never with
//! another test's panic in flight. Each guard is the test's first local, so
//! it drops last, after the drains and the runtime. Both accessors are
//! poison-tolerant (a poisoned lock counts as free): a failing writer poisons
//! the lock, and the rest must still run and report their own verdicts. Both
//! are bounded by [`TIMING_WAIT`] (non-vacuity contract above). They retry
//! `try_write` / `try_read`, so a waiting writer gets the lock once no reader
//! holds it; readers are not held back meanwhile, which costs throughput
//! only. The cost is the writers' own run time, during which nothing else in
//! this binary runs: about 3 s for the tick tests (two 1 s ticks, two 500 ms
//! ticks; 3.01 s together in round 6's lock measurement) and about 2 s for
//! the self-test (its two 1 s `HOLD`s; 2.01 s alone there). The lock reaches
//! only this binary: other test binaries are separate processes.
//!
//! Every test asserts the exact relayed message list and the stream END (or,
//! for the two always-on tick streams, each tick's time window and that the
//! stream is still live after its ticks). What `stream()` ITSELF must do is
//! read on the event-loop stand-in right after it returned, before the drain
//! is even spawned (`Built::observed_after_stream`): for take-recipes the
//! shared cell holds `None` (B-P18); for TrailMirror its receiver is the
//! channel's only one (B-P12). `ActivityRecipe` exposes no receiver count, so
//! `activity_recipe_subscribes_before_its_first_poll` starts AND drops an
//! activity at that same point, which publishes `Start` and `End(Success)`.
//! The first poll must yield the `Start` (B-P19). The drain reads on after a
//! first-poll item, so the next poll must relay the buffered `End`, and then
//! the stream must end with the bus.
//! `a_reactor_dependent_stream_fails_here_instead_of_passing` proves the
//! harness turns each REACTOR, CONTEXT and BUFFERING failure shape the probes
//! create (B-P1..B-P11, B-P17, B-P20..B-P25) into a test failure that carries
//! its cause, and a batch's leak into one that names the right recipe. It runs
//! those shapes on this tokio version (1.52.3), so it does not rely on
//! describing them. The other probe shapes (cross-talk B-P32/B-P33, stamps
//! B-P13/B-P15/B-P26/B-P27, take and subscribe timing B-P12/B-P18/B-P19, the
//! production facts B-P35..B-P37) fail at direct asserts in the tests
//! themselves, with no self-test of their own: their evidence is their probe.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4; Wave B ids)
//!
//! One edit each to production code, located by symbol and quoted line text
//! (B-P22 is one span that carries B-P20's and B-P21's edits together).
//! Procedure:
//!
//! 1. snapshot the file (`cp -p <file> <snap>`) and apply the edit;
//! 2. run `cargo test -p ui --test recipe_off_runtime_poll` and the sibling in
//!    the sibling column (column 6: a `--test` target, or `--lib` where the
//!    row says "the lib's"). The named test must go RED at its designed
//!    assertion. For B-P1..B-P9 the named sibling stays GREEN, because
//!    `#[tokio::test]` supplies the very context the mutation needs. That
//!    asymmetry is this file's reason to exist. Each row says whether its
//!    sibling column was MEASURED or is REASONED from the sibling's source;
//! 3. restore from the snapshot, THEN `touch <file>`. A `cp -p` restore keeps
//!    the old mtime, so cargo re-runs the MUTANT binary;
//! 4. re-run. The GREEN counts only if that run shows `Compiling ui` and the
//!    file's SHA-256 equals the pre-probe one.
//!
//! Where each RED fails (inside `expect_parked` unless the row says otherwise):
//!
//! - B-P1/B-P2 build their interval inside `stream()`, so they fail with
//!   "`Recipe::stream` PANICKED on the event-loop stand-in thread (no tokio context): there is no reactor running ...".
//!   In the cockpit that panic is on iced's event-loop thread, inside `runtime.track`;
//! - B-P3..B-P9, B-P20 and B-P23..B-P25 build their timeout at the first poll,
//!   so they fail with
//!   "the stream PANICKED on iced's executor (no tokio context): there is no reactor running ...";
//! - B-P10, B-P11, B-P21 and B-P22 fail at the "`stream()` must leave NO tokio context entered"
//!   premise. Since round 6 no drain is spawned once it fails, so the log no longer shows a second,
//!   consequential panic after the cause (round 5 measured B-P22's "no reactor running" at
//!   `training_log.rs` on a pool worker there; round 6 measured one printed panic in this file's
//!   run for each of the four);
//! - B-P12..B-P19, B-P26..B-P29, B-P34, B-P39 and B-P40 fail at the assertion their row names, in
//!   the test body;
//! - B-P32 and B-P33 fail at a "CROSS-TALK" report that names both channels: the relay's own and the
//!   one whose event it yielded;
//! - B-P35, B-P36 and B-P37 fail at a PRODUCTION-fact assert on the test thread, before anything is
//!   streamed: "IDENTITY: ..." (B-P35), the bus recipe count (B-P36), the trail recipe count (B-P37).
//!   Never at a "HARNESS DEFECT" label;
//! - B-P30, B-P31 and B-P38 do NOT fail here, BY DESIGN. B-P30 and B-P31 are slower intervals inside
//!   this file's real-time window (see the cadence paragraph below); B-P38 ends server time after
//!   four ticks, and this file takes two plus one poll. Their RED is the paused-clock sibling's;
//! - B-P41 failed NOWHERE in round 6. It is the toast analogue of B-P38, and neither target that
//!   called `toast_dismiss_stream_impl` then (this file, the pre-E1a `toast_dismiss_recipe_stream`)
//!   pinned its ticker past its fourth tick: a measured survivor, routed to story 2-63 Wave E1 (its
//!   row). This file is still GREEN under it by design. Since Wave E1a, `toast_dismiss_recipe_stream`
//!   is RED under it, at "toast: the stream is still live after tick 4" (MEASURED by the E1a
//!   falsifier, as ST-OWN-1).
//!
//! Sibling cells describe the siblings' revisions at round 6. Later waves changed the siblings:
//! Wave E1a (2026-11-13) rewrote all seven v0.2 recipe-stream siblings; Waves C1, C2 and E1b
//! (2026-11-14) added `subscription_batch_inclusion` (which builds, hashes and streams the batch
//! `ui::live::subscription_batch` builds) and the two E1b recipe tests. So a sibling cell is
//! round-6 history unless its row says "Since E1a", "Since E1b" or "Since C1/C2". The rows whose
//! sibling state a later wave is known to have changed say so, each with that wave's own
//! measurement or reasoning, labelled as such: since E1a, B-P12, B-P18, B-P21, B-P22, B-P28,
//! B-P37, B-P38, B-P39 and B-P41; since E1b, B-P28, B-P29, B-P34, B-P39 and B-P40 (B-P38 leaves
//! the E1b test green); since C1/C2, B-P12, B-P17, B-P18, B-P35, B-P36 and B-P37. That list is
//! what the waves' falsifiers and reviewers found, not a re-derivation of every row.
//!
//! The last column, "round 6 (file SHA bc9133fb...)", is what round 6's sweep MEASURED (workflow
//! `wf_f002d295-ed8`, step `falsify-r6`, 2026-11-12) against this file at SHA-256
//! `bc9133fb728248d7f6f1c86a5c67e482cd5fb986cc85075e49fb0f0428a9e5e9` and
//! `server_time_recipe_stream.rs` at
//! `d132975dfb77acb9a2ff13f5e382587bd6616e71174bf57a24423e7bf36653c3`. Each cell names the targets
//! that ran ("this file only" when the sibling was not run), the tests that went red and whether
//! at the designed assertion, and cites its log: `r5x2-B-P<n>.summary` in the Wave B scratchpad
//! (one per probe, with each target's RESULT line and its panic text; B-P20..B-P22's lib runs
//! appear there as `LIB RESULT`, their full logs in `r5x2-B-P<n>-lib.log`). All 41 summaries show
//! the probe applied, `ui` recompiled, a byte-identical restore, and both files' SHA-256 prefixes
//! (16 hex digits) unchanged; the full SHA-256s above are from the falsifier's result
//! (`file_sha256`, `sibling_sha256`). A sibling cell that says "MEASURED in round 6" is from the
//! same run. Rows B-P35..B-P38 are new in round 6; B-P39..B-P41 are the sweep's own extras,
//! measured in the same run. The close-out edit that filled in this column changed this module
//! doc, the timing-lock doc comments and one lock call (see "Timing isolation"), after the sweep:
//! the sweep did not run against it.
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED (this file) | sibling that stays GREEN | round 6 (file SHA bc9133fb...) |
//! |---|---|---|---|---|---|---|
//! | B-P1 | `ui::live::server_time_stream_impl` (`crates/ui/src/live.rs`) | `let _guard = rt_handle.enter();` (the line above `tokio::time::interval(Duration::from_secs(1))`) | delete the line | `server_time_stream_ticks_off_runtime` (AC4-designated: makes the K8 claim in `server_time_recipe_stream.rs` falsifiable for the first time) | `server_time_recipe_stream` (measured in rounds 1, 2 and 6) | MEASURED, with `server_time_recipe_stream`: this file 17/18, `server_time_stream_ticks_off_runtime` RED at its designed assertion (`Recipe::stream` PANICKED on the event-loop stand-in, "no reactor running", tokio `interval.rs:138`); `server_time_recipe_stream` 4/4 GREEN, so the asymmetry holds (`r5x2-B-P1.summary`) |
//! | B-P2 | `ui::live::toast_dismiss_stream_impl` (`crates/ui/src/live.rs`) | `let _guard = rt_handle.enter();` (the line above `tokio::time::interval(Duration::from_millis(500))`) | delete the line | `toast_dismiss_stream_ticks_off_runtime` (AC4-designated) | `toast_dismiss_recipe_stream` (measured in rounds 1 and 2) | MEASURED, this file only: 17/18, `toast_dismiss_stream_ticks_off_runtime` RED at its designed assertion (`Recipe::stream` PANICKED on the event-loop stand-in, "no reactor running") (`r5x2-B-P2.summary`) |
//! | B-P3 | `ui::live::forward_plan_stream_impl` | `while let Some(plan) = rx.recv().await {` | → `while let Ok(Some(plan)) = tokio::time::timeout(std::time::Duration::from_secs(60), rx.recv()).await {` | `forward_plan_recipe_drains_off_runtime` | `forward_narration_relay` (measured in rounds 1 and 2) | MEASURED, this file only: 17/18, `forward_plan_recipe_drains_off_runtime` RED at its designed assertion (the stream PANICKED on iced's executor, "no reactor running", `live.rs:936` on a pool worker) (`r5x2-B-P3.summary`) |
//! | B-P4 | `ui::live::narration_outcome_stream_impl` | `while let Some(outcome) = rx.recv().await {` | → `while let Ok(Some(outcome)) = tokio::time::timeout(std::time::Duration::from_secs(60), rx.recv()).await {` | `narration_outcome_recipe_drains_off_runtime` | `forward_narration_relay` (measured in rounds 1 and 2) | MEASURED, this file only: 17/18, `narration_outcome_recipe_drains_off_runtime` RED at its designed assertion (the executor panic, `live.rs:1002`) (`r5x2-B-P4.summary`) |
//! | B-P5 | `ui::live::bakeoff_progress_stream_impl` | `while let Some(progress) = rx.recv().await {` (the line above `yield Message::BakeoffProgress(progress);`) | → `while let Ok(Some(progress)) = tokio::time::timeout(std::time::Duration::from_secs(60), rx.recv()).await {` | `bakeoff_progress_recipe_drains_off_runtime` | `bakeoff_progress_relay` (measured in round 1) | MEASURED, this file only: 17/18, `bakeoff_progress_recipe_drains_off_runtime` RED at its designed assertion (the executor panic, `live.rs:1107`) (`r5x2-B-P5.summary`) |
//! | B-P6 | `ui::live::sweep_progress_stream_impl` | `while let Some(progress) = rx.recv().await {` (the line above `yield Message::SweepProgress(progress);`) | same wrap | `sweep_progress_recipe_drains_off_runtime` | `sweep_progress_relay` (measured in round 1) | MEASURED, this file only: 17/18, `sweep_progress_recipe_drains_off_runtime` RED at its designed assertion (the executor panic, `live.rs:1176`) (`r5x2-B-P6.summary`) |
//! | B-P7 | `ui::lab::progress::stream_impl` (`crates/ui/src/lab/progress.rs`) | `while let Some(progress) = rx.recv().await {` (the line above `yield Message::LabRunProgress(progress);`) | same wrap | `lab_progress_recipe_drains_off_runtime` | `lab_progress_recipe_stream` (measured in round 1) | MEASURED, this file only: 17/18, `lab_progress_recipe_drains_off_runtime` RED at its designed assertion (the executor panic, `progress.rs:102`) (`r5x2-B-P7.summary`) |
//! | B-P8 | `ui::live::trail_mirror_stream_impl` | `match rx.recv().await {` (the line above `Ok(tick) => yield Message::TrailMirrorTick(TrailMirrorUiTick::from(tick)),`) | → `match tokio::time::timeout(std::time::Duration::from_secs(60), rx.recv()).await.unwrap_or(Err(tokio::sync::broadcast::error::RecvError::Closed)) {` | `trail_mirror_recipe_drains_off_runtime` | `trail_mirror_recipe_stream` (measured in round 1) | MEASURED, this file only: 17/18, `trail_mirror_recipe_drains_off_runtime` RED at its designed assertion (the executor panic, `live.rs:614`) (`r5x2-B-P8.summary`) |
//! | B-P9 | `ui::live::activity_stream_impl` | `match rx.recv().await {` (the line above `Ok(event) => yield Message::ActivityEventReceived(event),`) | same wrap as B-P8 | TWO tests, both at the executor panic: `activity_recipe_drains_off_runtime` and `activity_recipe_subscribes_before_its_first_poll` (measured, round 3) | `activity_recipe_stream` (measured in round 1) | MEASURED, this file only: 16/18, both designed tests RED at the executor panic (`live.rs:749`) (`r5x2-B-P9.summary`) |
//! | B-P10 | `ui::live::server_time_stream_impl` | `let _guard = rt_handle.enter();` (the line above `tokio::time::interval(Duration::from_secs(1))`) | → `std::mem::forget(rt_handle.enter());` (leak the guard) | `server_time_stream_ticks_off_runtime`, at `expect_parked`'s "`stream()` must leave NO tokio context entered" premise | none: `server_time_recipe_stream` goes RED too, 4/4, through tokio's out-of-order `EnterGuard` drop panic (measured in rounds 1 and 6) | MEASURED, with `server_time_recipe_stream`: this file 17/18, `server_time_stream_ticks_off_runtime` RED at the after-`stream()` PREMISE ("No drain was spawned for this stream"), the only panic printed in this file's run; `server_time_recipe_stream` 0/4, every test at tokio's "`EnterGuard` values dropped out of order" (`r5x2-B-P10.summary`) |
//! | B-P11 | `<ForwardPlanRecipe as Recipe>::stream` | `let _guard = self.rt_handle.enter();` (in the block that ends just above `forward_plan_stream_impl(rx_opt)`) | → `std::mem::forget(self.rt_handle.enter());` | `forward_plan_recipe_drains_off_runtime`, at the same premise | none: `forward_narration_relay::forward_plan_recipe_stream_takes_receiver_once_and_relays` goes RED too (measured in round 1, same cause) | MEASURED, this file only: 17/18, `forward_plan_recipe_drains_off_runtime` RED at the after-`stream()` PREMISE, the only panic printed in the run (`r5x2-B-P11.summary`) |
//! | B-P12 | `<TrailMirrorRecipe as Recipe>::stream` (`crates/ui/src/live.rs`) | `let rx = self.handle.tick_tx.subscribe();` and the next line `trail_mirror_stream_impl(rx)` | subscribe lazily, in the body: replace both lines with `let handle = self.handle; Box::pin(async_stream::stream! { let mut inner = trail_mirror_stream_impl(handle.tick_tx.subscribe()); while let Some(msg) = futures::StreamExt::next(&mut inner).await { yield msg; } })` | `trail_mirror_recipe_drains_off_runtime`, at "`Recipe::stream` subscribed EAGERLY" (`Some(0)` ≠ `Some(1)`) | `trail_mirror_recipe_stream` (measured, round 3; pre-E1a revision). Since E1a: its T-TM-1d, `trail_mirror_recipe_stream_end_to_end`, goes RED too, at "trail recipe: `Recipe::stream` subscribed EAGERLY to the handle's own sender" (REASONED in that file's doc, not measured). Since C1/C2 (REASONED): `subscription_batch_inclusion`'s `batched_trail_recipe_relays` goes RED too, at "trail: the batched unit subscribed to THIS ctx's sender" (the send right after `stream()` finds no receiver) | MEASURED, this file only: 17/18, `trail_mirror_recipe_drains_off_runtime` RED at "`Recipe::stream` subscribed EAGERLY" (left `Some(0)`, right `Some(1)`) (`r5x2-B-P12.summary`) |
//! | B-P13 | `ui::live::server_time_stream_impl` | `yield Message::ServerTimeTick(Timestamp::now());` | → `yield Message::ServerTimeTick(Timestamp::new(Timestamp::now().inner() - Duration::from_secs(1)));` (the clock shows a tick-stale time) | `server_time_stream_ticks_off_runtime`, at "the payload must be stamped when it was yielded" (the sample at yield is about 1 s past the stamp; the bound is half a period) | none, since round 6: `server_time_recipe_stream` was measured GREEN under it (rounds 3 and 5), because its cadence test checked only that the stamps do not decrease. Round 6 gave that test a two-sided freshness check, and round 6 MEASURED `server_time_stream_impl_emits_at_1_hz_cadence` RED at ServerTimeTick #1, at "the payload must be stamped when it was yielded" (1.000003s past the stamp; the bound is 500 ms). Its other three tests were MEASURED GREEN | MEASURED, with `server_time_recipe_stream`: this file 17/18, RED at its designed assertion (1.00001s past the stamp, bound 500ms); `server_time_recipe_stream` 3/4 as in the sibling column (`r5x2-B-P13.summary`) |
//! | B-P14 | `ui::live::server_time_stream_impl` | `tokio::time::interval(Duration::from_secs(1))` | → `tokio::time::interval(Duration::from_millis(100))` | `server_time_stream_ticks_off_runtime`, at "yielded ... after `stream()` began, expected no earlier than 1s" | none, since round 5: `server_time_recipe_stream` was measured GREEN under it in round 3 (0.40 s), because its cadence test asserted only that the stamps do not decrease, a vacuity in THAT file. Round 5 rewrote `server_time_stream_impl_emits_at_1_hz_cadence` on a paused clock (every gap exactly 1 s), and round 6 MEASURED it RED at that gap assertion (left `[100ms, 100ms, 100ms, 100ms]`); the other three tests there were MEASURED GREEN | MEASURED, with `server_time_recipe_stream`: this file 17/18, RED at its designed assertion ("yielded 102.50325ms after `stream()` began, expected no earlier than 1s"); `server_time_recipe_stream` 3/4 as in the sibling column (`r5x2-B-P14.summary`) |
//! | B-P15 | `ui::live::toast_dismiss_stream_impl` | `yield Message::ToastTick(Instant::now());` | → `yield Message::ToastTick(Instant::now() - Duration::from_millis(500));` (a tick-stale stamp) | `toast_dismiss_stream_ticks_off_runtime`, at "the payload must be stamped when it was yielded" (the sample is about 500 ms past the stamp; the bound is half a period, 250 ms). Round 3 measured it red at the stamp's cadence bound, which ran first then | `toast_dismiss_recipe_stream` (measured, round 3) | MEASURED, this file only: 17/18, RED at the half-period bound, its designed assertion (500.005292ms past the stamp, bound 250ms): the site round 4 left unmeasured (`r5x2-B-P15.summary`) |
//! | B-P16 | `ui::live::toast_dismiss_stream_impl` | `tokio::time::interval(Duration::from_millis(500))` | → `tokio::time::interval(Duration::from_millis(50))` | `toast_dismiss_stream_ticks_off_runtime`, at "stamped ... after `stream()` began, expected no earlier than 500ms" (stamped at about 50 ms) | none: `toast_dismiss_recipe_stream` goes RED too, 3/3 (measured, round 3) | MEASURED, this file only: 17/18, RED at "stamped 51.5745ms after `stream()` began, expected no earlier than 500ms" (`r5x2-B-P16.summary`) |
//! | B-P17 | `ui::live::bakeoff_progress_stream_impl` | `while let Some(progress) = rx.recv().await {` and the next two lines (`yield Message::BakeoffProgress(progress);`, `}`) | buffer until the channel closes: push each item into a `Vec` inside the loop, yield them all after it | `bakeoff_progress_recipe_drains_off_runtime`, at the first event's `next_item` ("nothing reported within 5s: the stream hung"): it was sent with a sender alive, and is relayed only on close | none: `bakeoff_progress_relay` goes RED too, 4/6, at `bakeoff_progress_recipe_stream_end_to_end` and `bakeoff_progress_relay_yields_message_and_populates_state` (measured, round 3). Since C1/C2 (REASONED): `subscription_batch_inclusion` goes RED too, in `batched_bakeoff_progress_recipe_relays` at "bakeoff_progress: NO item within 2s" and in both both-cells tests at "both cells, salts 7/7: bake-off unit: NO item within 2s" (and the same at 7/8) | MEASURED, this file only: 17/18 (binary 8.02 s), RED at the first event's `next_item`, "nothing reported within 5s: the stream hung" (`r5x2-B-P17.summary`) |
//! | B-P18 | `<LabProgressRecipe as Recipe>::stream` (`crates/ui/src/lab/progress.rs`) | the `let rx_opt = {` block (`let _guard = self.rt_handle.enter();` ... `.take()`) and `stream_impl(rx_opt)` | take lazily: `let cell = std::sync::Arc::clone(&self.rx);` and return a `stream!` that takes from `cell` at its first poll, then relays `stream_impl(rx_opt)` | `lab_progress_recipe_drains_off_runtime`, at "`Recipe::stream` took the receiver" (`Some(0)` ≠ `Some(1)`) | `lab_progress_recipe_stream` (measured, round 3, pre-E1a revision: its end-to-end test then read the cell only after the first message, when the body has taken it). Since E1a: RED, MEASURED by the E1a falsifier as E1-P10 (the same edit), at `lab_progress_recipe_stream_end_to_end`'s "lab progress recipe: `Recipe::stream` took the receiver before the first poll". Since C1/C2 (REASONED): `subscription_batch_inclusion`'s `batched_lab_progress_recipe_relays` goes RED too, at "lab_progress: the batched unit took THIS ctx's receiver" | MEASURED, this file only: 17/18, RED at "`Recipe::stream` took the receiver" (left `Some(0)`, right `Some(1)`) (`r5x2-B-P18.summary`) |
//! | B-P19 | `<ActivityRecipe as Recipe>::stream` | `let rx = self.bus.activity().subscribe();` and the next line `activity_stream_impl(rx)` | subscribe lazily: `let bus = std::sync::Arc::clone(&self.bus);` and a `stream!` that subscribes at its first poll, drops `bus`, then relays `activity_stream_impl(rx)` | `activity_recipe_subscribes_before_its_first_poll`, at "the first poll found NOTHING" (both events published between `stream()` and the first poll are lost) | `activity_recipe_drains_off_runtime` here, by design (its first send follows the first poll; measured GREEN in rounds 3 and 6), and `activity_recipe_stream` (it subscribes for itself; measured, round 3) | MEASURED, this file only: 17/18, `activity_recipe_subscribes_before_its_first_poll` RED at "the first poll found NOTHING"; `activity_recipe_drains_off_runtime` GREEN, by design (`r5x2-B-P19.summary`) |
//! | B-P20 | `ui::lab::training_log::stream_impl` (`crates/ui/src/lab/training_log.rs`) | `Box::pin(lines.map(` ... `))`, its last line | → an `unfold` whose step awaits `tokio::time::timeout(std::time::Duration::from_secs(60), lines.next())` | `training_log_recipe_drains_off_runtime` | `training_log_recipe_harness` (`#[tokio::test]` over `stream_impl`; measured, round 3). The lib's `recipe_stream_drains_without_a_tokio_runtime` goes RED too (its polling thread has no context either): MEASURED in round 6, its first measurement (`--lib lab::training_log`: 2 passed, 1 failed, "no reactor running" at `training_log.rs:140`) | MEASURED, with `--lib lab::training_log`: this file 17/18, RED at the executor panic ("no reactor running", `training_log.rs:140`); the lib RED as in the sibling column (`r5x2-B-P20.summary`, `r5x2-B-P20-lib.log`) |
//! | B-P21 | `<TrainingLogRecipe as Recipe>::stream` | `let _guard = self.rt_handle.enter();` | → `std::mem::forget(self.rt_handle.enter());` | `training_log_recipe_drains_off_runtime`, at the after-`stream()` premise | the lib's `recipe_stream_drains_without_a_tokio_runtime`, MEASURED GREEN in round 6 (`--lib lab::training_log`, 3/3): it checks the premise only before `stream()`, and its thread holds no outer guard for tokio's drop-order check to trip on. Since E1a: `training_log_recipe_harness` is RED under it, MEASURED (orchestrator hand probe, 2026-11-13), in `training_log_recipe_stream_takes_receiver_once` only (3 passed, 1 failed), at tokio's own "`EnterGuard` values dropped out of order", not at a designed assertion | MEASURED, with `--lib lab::training_log`: this file 17/18, RED at the after-`stream()` PREMISE, the only panic printed in the run; the lib 3/3 GREEN, so of the two targets run only this file catches it (`r5x2-B-P21.summary`, `r5x2-B-P21-lib.log`) |
//! | B-P22 | `TrainingLogRecipe`: from `stream()`'s guard line through `stream_impl`'s last line (one span) | B-P21's line and B-P20's line | B-P20 AND B-P21 at once: the review's scenario, where leaking the guard "fixes" B-P20's red in the lib's test | `training_log_recipe_drains_off_runtime`, at the after-`stream()` premise | `training_log_recipe_harness` (measured, round 3; pre-E1a revision), and the lib's `recipe_stream_drains_without_a_tokio_runtime`, MEASURED GREEN in round 6 (`--lib lab::training_log`, 3/3): the leaked context feeds the timeout on its one polling thread. Since E1a: `training_log_recipe_harness` is RED under it, MEASURED (orchestrator hand probe, 2026-11-13), in `training_log_recipe_stream_takes_receiver_once` only (3 passed, 1 failed), at tokio's own "`EnterGuard` values dropped out of order", not at a designed assertion | MEASURED, with `--lib lab::training_log`: this file 17/18, RED at the after-`stream()` PREMISE; one printed panic in the log and no "no reactor running", so round 5's consequential pool-worker panic is gone; the lib 3/3 GREEN (`r5x2-B-P22.summary`, `r5x2-B-P22-lib.log`) |
//! | B-P23 | `ui::live::stream_fills` | `match rx.recv().await {` (the line above `Ok(fill) => yield Message::FillReceived(fill_to_view(&fill)),`) | same wrap as B-P8 | `bus_recipes_relay_off_runtime`, at bus recipe #1 (Fills) | `live_subscription`, `live_subscription_full_bus` (`#[tokio::test]` over `stream_fills`; measured, round 3) | MEASURED, this file only: 17/18, RED at bus recipe #1 (Fills), the executor panic (`live.rs:166`) (`r5x2-B-P23.summary`) |
//! | B-P24 | `ui::live::stream_ticks` | `match rx.recv().await {` (the line above `Ok(tick) => yield Message::TickReceived(tick),`) | same wrap as B-P8 | `bus_recipes_relay_off_runtime`, at bus recipe #4 (Ticks) | `live_subscription_full_bus` (measured, round 3) | MEASURED, this file only: 17/18, RED at bus recipe #4 (Ticks), the executor panic (`live.rs:239`) (`r5x2-B-P24.summary`) |
//! | B-P25 | `ui::live::stream_risk_telemetry` | `match rx.recv().await {` (the line above `Ok(event) => yield Message::RiskStateRefreshed(risk_telemetry_to_state(&event)),`) | same wrap as B-P8 | `bus_recipes_relay_off_runtime`, at bus recipe #11 (RiskTelemetry) | `risk_telemetry_subscription` (measured, round 3) | MEASURED, this file only: 17/18, RED at bus recipe #11 (RiskTelemetry), the executor panic (`live.rs:445`) (`r5x2-B-P25.summary`) |
//! | B-P26 | `ui::live::toast_dismiss_stream_impl` | `yield Message::ToastTick(Instant::now());` | → `yield Message::ToastTick(Instant::now() + Duration::from_millis(500));` (a future stamp) | `toast_dismiss_stream_ticks_off_runtime`, at "the payload is stamped AFTER it was yielded (a future instant)" | `toast_dismiss_recipe_stream` (measured, round 3) | MEASURED, this file only: 17/18, RED at "the payload is stamped AFTER it was yielded (a future instant)" (499.995875ms ahead) (`r5x2-B-P26.summary`) |
//! | B-P27 | `ui::live::server_time_stream_impl` | `yield Message::ServerTimeTick(Timestamp::now());` | → `yield Message::ServerTimeTick(Timestamp::new(Timestamp::now().inner() + Duration::from_secs(1)));` (a future stamp) | `server_time_stream_ticks_off_runtime`, at "the payload is stamped AFTER it was yielded (a future time)" | none, since round 6: `server_time_recipe_stream` was measured GREEN under it (rounds 3 and 5), as for B-P13. With its round-6 freshness check, round 6 MEASURED `server_time_stream_impl_emits_at_1_hz_cadence` RED at ServerTimeTick #1, at "the payload is stamped AFTER it was yielded (a future time)" (999.998ms ahead). Its other three tests were MEASURED GREEN | MEASURED, with `server_time_recipe_stream`: this file 17/18, RED at its designed assertion (999.991ms ahead); `server_time_recipe_stream` 3/4 as in the sibling column (`r5x2-B-P27.summary`) |
//! | B-P28 | `ui::live::server_time_stream_impl` | `tokio::time::interval(Duration::from_secs(1))` | → `tokio::time::interval(Duration::from_secs(2))` (the clock ticks at half speed; round 3's survivor X-1) | `server_time_stream_ticks_off_runtime`, ServerTimeTick #1, at "yielded ... after `stream()` began, expected before 2s" (yielded at about 2 s; measured, round 4) | none: `server_time_recipe_stream` went RED too, 4/4 (measured, round 4, on its pre-round-5 revision: each test's first tick has a 1.5 s timeout). Round 6 MEASURED it RED again, 4/4 (0 of 4 passed), on its round-6 revision (pre-E1a): the cadence test at its paused-clock gap assertion (left `[2s, 2s, 2s, 2s]`), the other three at their 1.5 s real-time timeouts. Since E1a (REASONED, not measured): E1a deleted T-ST-1d and moved T-ST-1c onto a paused clock, where it is expected red at tick 1's 1.5 s timeout, and E1b's `server_time_recipe_stream_yields_tick` at "server time recipe: the first ServerTimeTick arrives within 1.5 s (paused clock)" | MEASURED, with `server_time_recipe_stream`: this file 17/18, RED at ServerTimeTick #1, "yielded 2.003272916s after `stream()` began, expected before 2s"; `server_time_recipe_stream` 0/4 as in the sibling column (`r5x2-B-P28.summary`) |
//! | B-P29 | `ui::live::toast_dismiss_stream_impl` | `tokio::time::interval(Duration::from_millis(500))` | → `tokio::time::interval(Duration::from_millis(1000))` (toasts age at half speed; round 3's survivor X-2) | `toast_dismiss_stream_ticks_off_runtime`, ToastTick #1, at "yielded ... after `stream()` began, expected before 1s" (yielded at about 1 s; measured, round 4) | none: `toast_dismiss_recipe_stream` went RED too, 3/3 (measured, round 4: it expects one tick per 500 ms of paused-clock advance). Since E1b (REASONED): its `toast_dismiss_recipe_stream_yields_tick` is RED too, at "toast recipe: the first ToastTick arrives within 750 ms (paused clock)" (the first tick is due at 1 s) | MEASURED, this file only: 17/18, RED at ToastTick #1, "yielded 1.00259s after `stream()` began, expected before 1s" (`r5x2-B-P29.summary`) |
//! | B-P30 | `ui::live::server_time_stream_impl` | `tokio::time::interval(Duration::from_secs(1))` | → `tokio::time::interval(Duration::from_millis(1400))` (1.4 x the period, inside this file's window) | none: GREEN BY DESIGN. 1.4 s lies inside the window `[1 s, 1.5 s)` (tick 1 at about 1.4 s, before 2 s; tick 2 at about 2.8 s, before 3 s) | none: `server_time_recipe_stream::server_time_stream_impl_emits_at_1_hz_cadence` goes RED at its paused-clock gap assertion (left `[1.4s, 1.4s, 1.4s, 1.4s]`), MEASURED in round 6. Its other three tests were MEASURED GREEN (each waits 1.5 s of real time for a tick: 100 ms of margin) | MEASURED, with `server_time_recipe_stream`: this file GREEN 18/18 by design (binary 5.82 s); `server_time_recipe_stream` 3/4, RED at its designed assertion, as in the sibling column (`r5x2-B-P30.summary`) |
//! | B-P31 | `ui::live::toast_dismiss_stream_impl` | `tokio::time::interval(Duration::from_millis(500))` | → `tokio::time::interval(Duration::from_millis(700))` (1.4 x the period, inside this file's window) | none: GREEN BY DESIGN. 700 ms lies inside `[500 ms, 750 ms)` (tick 1 at about 700 ms, before 1 s; tick 2 at about 1.4 s, before 1.5 s). It goes red here only if tick 2 is delivered more than 100 ms late | none: `toast_dismiss_recipe_stream` goes RED, 3/3 (0 of 3 passed), MEASURED in round 6 at its count asserts: three 500 ms advances of its paused clock yield two ticks (at 700 ms and 1.4 s), not three ("got 2", in two tests), and two advances yield one, not two ("got 1") | MEASURED, with `toast_dismiss_recipe_stream`: this file GREEN 18/18 by design (binary 5.42 s: tick 2 was not delivered more than 100 ms late); the sibling 0/3, RED at its designed assertions, as in the sibling column (`r5x2-B-P31.summary`) |
//! | B-P32 | `<BusRecipe as Recipe>::stream` (`crates/ui/src/live.rs`, its `match channel`) | `Channel::Positions => Box::pin(stream_positions(&bus)),` | → `Channel::Positions => Box::pin(stream_fills(&bus)),` (the Positions relay subscribes to the fills receiver) | `bus_recipes_relay_off_runtime`, at "CROSS-TALK: the Positions relay yielded `FillReceived ...`", which names bus recipe #2 (Positions) and the Fills channel (turn #1's publish). Where the item lands decides which check reports it: the sweep after turn #1, turn #2's sweep before its own publish, or turn #2's wait for its own event. It is the same CROSS-TALK report each time | `live_subscription_full_bus`, MEASURED GREEN 2/2 in round 6: it calls the `stream_*` fns directly, never `BusRecipe::stream`, and no other `ui` target builds `ui::live::subscription` | MEASURED, with `live_subscription_full_bus`: this file 17/18, RED at "CROSS-TALK" for bus recipe #2 (Positions), naming the Fills event of turn #1, reported after turn #1; the sibling 2/2 GREEN (`r5x2-B-P32.summary`) |
//! | B-P33 | `<BusRecipe as Recipe>::stream` (as B-P32) | `Channel::Fills => Box::pin(stream_fills(&bus)),` | → `Channel::Fills => Box::pin(futures::stream::select(stream_fills(&bus), stream_ticks(&bus))),` (the Fills relay ALSO relays ticks) | `bus_recipes_relay_off_runtime`, at "CROSS-TALK: the Fills relay yielded `TickReceived 4242 ...`", which names bus recipe #1 (Fills) and the Ticks channel (turn #4's publish): at the sweep after turn #4 or a later one, the check before the bus drops, or the Fills tail. The pre-round-5 revision is REASONED to have gone RED only at the Fills tail after the bus dropped, with no word of cross-talk | `live_subscription_full_bus`, MEASURED GREEN 2/2 in round 6, as for B-P32 | MEASURED, with `live_subscription_full_bus`: this file 17/18, RED at "CROSS-TALK" for bus recipe #1 (Fills), naming `TickReceived 4242 Price(40003)` of turn #4, reported after turn #4; the sibling 2/2 GREEN (`r5x2-B-P33.summary`) |
//! | B-P34 | `ui::live::server_time_stream_impl` | `interval.tick().await;`, the line under `// arrives ~1 s after subscription, not immediately at boot.` | delete the line (the immediate first tick is no longer skipped) | `server_time_stream_ticks_off_runtime`, ServerTimeTick #1, at "yielded ... after `stream()` began, expected no earlier than 1s" (yielded about 1 ms in). If the immediate tick completes inside the first poll instead, at `expect_parked`'s "the first poll yielded ServerTimeTick(..) before anything was produced" | none: `server_time_recipe_stream::server_time_stream_impl_emits_at_1_hz_cadence` goes RED at its gap assertion (left `[0ns, 1s, 1s, 1s]`: first gap 0, not 1 s), MEASURED in round 6. Its other three tests were MEASURED GREEN. Since E1b (MEASURED, 2026-11-14, by the falsifier of workflow `wf_be1acf7b-3e7`: `server_time_recipe_stream` 2 of 4 red, T-ST-1b and this test; `subscription_batch_inclusion` 25/25): `server_time_recipe_stream_yields_tick` is RED too, at "server time recipe: `ServerTimeRecipe::stream` returns a live stream: it yielded with no time passing" (the unskipped immediate tick completes inside its first poll); `subscription_batch_inclusion`'s `batched_server_time_recipe_relays` checks only its first item's variant and stays green | MEASURED, with `server_time_recipe_stream`: this file 17/18, RED at the primary site, "yielded 1.184ms after `stream()` began, expected no earlier than 1s"; `server_time_recipe_stream` 3/4 as in the sibling column (`r5x2-B-P34.summary`) |
//! | B-P35 | `<BusRecipe as Recipe>::hash` (`crates/ui/src/live.rs`) | `self.channel.hash(state);` (the line under `TypeId::of::<Self>().hash(state);`) | delete the line (all eleven bus recipes hash to one iced id) | `bus_recipes_relay_off_runtime`, at "IDENTITY: bus recipe #2 (Positions) has the same iced id as bus recipe #1 (Fills)", on the test thread before anything is streamed. In the cockpit iced would stream only the Fills relay | `live_subscription_full_bus`, MEASURED GREEN 2/2 in round 6: it calls the `stream_*` fns directly, and no `ui` target hashed a `BusRecipe` then. Since C1/C2: `subscription_batch_inclusion` hashes every `BusRecipe` (`id_of`) and is RED under this edit, MEASURED as C2-P6 (2026-11-14, `wf_a82b74dd-7bb` falsifier): `batched_bus_recipe_relays`, at "bus/mode: exactly one batched unit carries the reference id" (11 hits), and `recipe_ids_are_pairwise_distinct`, at "all 22 units of the all-Some batch have distinct ids (every salt 0)" (12) | MEASURED, with `live_subscription_full_bus`: this file 17/18, RED at the test-thread "IDENTITY: bus recipe #2 (Positions) has the same iced id as bus recipe #1 (Fills)", not a HARNESS DEFECT; the sibling 2/2 GREEN (`r5x2-B-P35.summary`) |
//! | B-P36 | `ui::live::subscription` | `subscription_for(Channel::MarketHealth, Arc::clone(&bus)),` | delete the line (the cockpit loses its market-health relay) | `bus_recipes_relay_off_runtime`, at "`ui::live::subscription` batched 10 recipes, but this test's table `BUS_BATCH` lists 11 channels", on the test thread. Not a HARNESS DEFECT, whose message used to blame the table | `live_subscription_full_bus`, MEASURED GREEN 2/2 in round 6, as for B-P35. Since C1/C2: `subscription_batch_inclusion` is RED under this edit, MEASURED as C1-P8 (the Wave C1 falsifier, 2026-11-14): `subscription_batch_units_match_descriptor`, at "ui::live::subscription batches one unit per bus channel" (10, wanted 11); under the C2 tests its three identity tests go red too, at their 22-unit premise (reasoned) | MEASURED, with `live_subscription_full_bus`: this file 17/18, RED at the test-thread count assert (left 10, right 11), not a HARNESS DEFECT; the sibling 2/2 GREEN (`r5x2-B-P36.summary`) |
//! | B-P37 | `ui::live::trail_mirror_subscription` | `iced::advanced::subscription::from_recipe(TrailMirrorRecipe { handle })` | → `let _unarmed = TrailMirrorRecipe { handle };` then `iced::Subscription::none()` (the trail-mirror relay is never batched) | `trail_mirror_recipe_drains_off_runtime`, at "`ui::live::trail_mirror_subscription` built 0 recipes, expected exactly one", on the test thread. Not a HARNESS DEFECT | `trail_mirror_recipe_stream` (it then drove `trail_mirror_stream_impl`, never the constructor; pre-E1a revision) and `trail_mirror_subscription_handle_gating` (it tested the descriptor fn only; DELETED in Wave C1, folded into `subscription_batch_inclusion.rs`), both MEASURED GREEN in round 6 (4/4, 2/2). Since E1a (REASONED): `trail_mirror_recipe_stream`'s T-TM-1d, `trail_mirror_recipe_stream_end_to_end`, is RED, at "trail recipe: trail_mirror_subscription batches exactly one recipe" (it now goes through the constructor). Since C1/C2 (REASONED): `subscription_batch_inclusion` is RED too: `batched_trail_recipe_relays` while building its reference, at "trail_mirror_subscription builds one recipe", and C2-P14's other reds (`subscription_batch_units_match_descriptor` at "units for ctx `trail`", the three identity tests at their 22-unit premise) | MEASURED, with both siblings: this file 17/18, RED at the test-thread count assert (left 0, right 1), not a HARNESS DEFECT; the siblings 4/4 and 2/2 GREEN (`r5x2-B-P37.summary`) |
//! | B-P38 | `ui::live::server_time_stream_impl` | `loop {` (the line above `interval.tick().await;` and `yield Message::ServerTimeTick(Timestamp::now());`) | → `for _ in 0..4 {` (the status-bar clock stops after four ticks) | none: GREEN BY DESIGN. This file takes two ticks plus one poll, which still finds the stream live | none: `server_time_recipe_stream::server_time_stream_impl_emits_at_1_hz_cadence` goes RED at "the stream ENDED before ServerTimeTick #5", MEASURED in round 6. Its other three tests were MEASURED GREEN (pre-E1a revision: T-ST-1c then accepted a timeout after its third tick). Since E1a (REASONED, not measured): E1a deleted T-ST-1d, and its T-ST-1c, 7,200 paused ticks, is expected red too, at "server time: the stream ENDED before tick 5"; E1b's recipe test takes one tick and stays green | MEASURED, with `server_time_recipe_stream`: this file GREEN 18/18 by design (two ticks plus one poll); `server_time_recipe_stream` 3/4, RED at its designed assertion, as in the sibling column (`r5x2-B-P38.summary`) |
//! | B-P39 | `ui::live::server_time_stream_impl` | `tokio::time::interval(Duration::from_secs(1))` | → `tokio::time::interval(Duration::from_millis(1600))` (1.6 x the period: the slow side's k = 2 bound, which B-P28 (red at k = 1) and B-P30 (inside the window) do not reach) | `server_time_stream_ticks_off_runtime`, ServerTimeTick #2, at "yielded ... after `stream()` began, expected before 3s" (tick 2's upper bound, k = 2) | none: `server_time_recipe_stream` goes RED, 4/4 (0 of 4 passed), MEASURED in round 6 on its pre-E1a revision: the cadence test at its paused-clock gap assertion (left `[1.6s, 1.6s, 1.6s, 1.6s]`), the other three at their 1.5 s real-time timeouts. Since E1a (REASONED, not measured): T-ST-1d is deleted, T-ST-1c (paused) is expected red at tick 1's 1.5 s timeout, and E1b's recipe test at "server time recipe: the first ServerTimeTick arrives within 1.5 s (paused clock)" | MEASURED (a round-6 falsifier extra), with `server_time_recipe_stream`: this file 17/18, RED at ServerTimeTick #2, "yielded 3.201341208s after `stream()` began, expected before 3s", tick 2's upper bound (k = 2); `server_time_recipe_stream` 0/4 as in the sibling column (`r5x2-B-P39.summary`) |
//! | B-P40 | `ui::live::toast_dismiss_stream_impl` | `tokio::time::interval(Duration::from_millis(500))` | → `tokio::time::interval(Duration::from_millis(800))` (1.6 x the period: toast's k = 2 bound, as B-P39 for server time) | `toast_dismiss_stream_ticks_off_runtime`, ToastTick #2, at "yielded ... after `stream()` began, expected before 1.5s" (tick 2's upper bound, k = 2) | none: `toast_dismiss_recipe_stream` goes RED, 3/3 (0 of 3 passed), MEASURED in round 6 at its count asserts ("got 1" in each). Since E1b (REASONED): its `toast_dismiss_recipe_stream_yields_tick` is RED too, at "toast recipe: the first ToastTick arrives within 750 ms (paused clock)" (the first tick is due at 800 ms) | MEASURED (a round-6 falsifier extra), with `toast_dismiss_recipe_stream`: this file 17/18, RED at ToastTick #2, "yielded 1.602633583s after `stream()` began, expected before 1.5s" (k = 2); the sibling 0/3 (`r5x2-B-P40.summary`) |
//! | B-P41 | `ui::live::toast_dismiss_stream_impl` | `loop {` (the line above `interval.tick().await;` and `yield Message::ToastTick(Instant::now());`) | → `for _ in 0..4 {` (the auto-dismiss ticker stops after four ticks: the toast analogue of B-P38) | none, and none designed: this file takes two ticks plus one poll, which still finds the stream live | none went RED in round 6: `toast_dismiss_recipe_stream` 3/3 GREEN, MEASURED on its pre-E1a revision (`toast_dismiss_stream_remains_open` then accepted tick 4 or an empty channel after its fourth advance). Since E1a: RED, MEASURED by the E1a falsifier (as ST-OWN-1), at `toast_dismiss_stream_remains_open`'s "toast: the stream is still live after tick 4": the survivor is killed | MEASURED SURVIVOR (a round-6 falsifier extra): this file 18/18 GREEN and `toast_dismiss_recipe_stream` 3/3 GREEN (cargo exit 0). The toast ticker's always-on property past its 4th tick is pinned by neither target that calls `toast_dismiss_stream_impl` (by grep, these two are the only `ui` targets that do). Routed to story 2-63 Wave E1, which owns `toast_dismiss_recipe_stream.rs`, as a paused-clock liveness test there (many ticks, then `Pending`, in the style of `server_time_recipe_stream`'s T-ST-1b, which B-P38 measured RED) (`r5x2-B-P41.summary`) |
//!
//! ### What has been measured, and against what
//!
//! - Round 1 (Wave B falsifier, 2026-11-11) applied B-P1..B-P11 one at a time
//!   to a revision of this file that built the stream inside the pool task.
//!   Each turned its named test red. B-P10 and B-P11 failed at the
//!   after-`stream()` premise, and they also turned the siblings named in their
//!   rows red, through tokio's out-of-order `EnterGuard` drop panic.
//! - Round 2 built the stream on the event-loop stand-in and added
//!   B-P12..B-P16. B-P1..B-P4 re-measured red, only in this file, siblings
//!   green; the round was stopped at B-P5 on purpose, because a review was
//!   already rewriting the file.
//! - Round 3 (2026-11-11/12) measured B-P1..B-P27 against the revision that
//!   added turn-taking with a parked consumer, the facts read right after
//!   `stream()`, TrainingLog, the bus batch and the freshness bounds. All 27
//!   went red at their designed assertion. B-P1..B-P11 were run against this
//!   file only. B-P12..B-P27 were also run against the eight siblings of the
//!   B-P1..B-P9 column (plus `training_log_recipe_harness`, `live_subscription`,
//!   `live_subscription_full_bus` and `risk_telemetry_subscription` where their
//!   rows name them). The siblings went red only where a row says so. Two
//!   extra probes SURVIVED, 14/14 green: X-1 (server time at a 2 s period) and
//!   X-2 (toast at 1000 ms). Cadence was bounded only from below, and the only
//!   upper bound was the per-tick wait (3 s / 2 s).
//! - Round 4 answered that measurement. It added the cadence UPPER bound (X-1
//!   and X-2 became B-P28 and B-P29), the explicit half-period staleness bound
//!   for toast (B-P15 now fails there), iced's cancellable drain with its
//!   teardown, and the `End` relay after the eager-subscribe race. It also
//!   added the teardown self-test,
//!   `a_dropped_drain_stops_before_the_runtime_shuts_down`. Its re-falsifier
//!   MEASURED B-P28 and B-P29 against that revision: this file red at the
//!   designed assertion, `server_time_recipe_stream` red 4/4 under B-P28,
//!   `toast_dismiss_recipe_stream` red 3/3 under B-P29. B-P15's new failure
//!   site was not measured. The teardown WAS measured, by transient edits to this file only,
//!   each restored byte-identically. A `Drain::drop` that skips its wait turns
//!   the self-test red at its flag check. One that cancels only after the wait
//!   turns it red inside `drop` ("still running 5s after it was cancelled"),
//!   and `a_reactor_dependent_stream_fails_here_instead_of_passing` too, at the
//!   drain over `pending()` that only a cancel can stop. Forcing both tick
//!   tests red at their cadence upper bound printed each real cause and NO
//!   "being shutdown" panic. Round 3's logs show one after each of the seven
//!   tick-test failures that left a drain running (B-P10, B-P13..B-P16,
//!   B-P26, B-P27).
//! - Round 5 answered round 4's re-falsifier. The slow side of the cadence
//!   moved to a paused clock in `server_time_recipe_stream` (its cadence test
//!   pins every gap at exactly 1 s), and this file states its real-time
//!   window exactly (the cadence paragraph below). It also added the timing
//!   lock, the two-sided shape (g), the `HARNESS:` label with its self-test,
//!   and the bus cross-talk checks, and declared B-P30..B-P34. Its
//!   re-falsifier (`falsify-r5`) swept all 34 rows over the round-5 revision.
//!   That record is not entered in the table: its last column is round 6's
//!   sweep (below).
//! - Round 6 (the revision at SHA-256 `bc9133fb...`) answers that re-falsifier
//!   and a read-only review.
//!   It changed the shared harness once more: the after-`stream()` premise is
//!   sampled per recipe and a stream that fails it is not spawned; a later
//!   poll's wait gets the panic-report budget while the drain is inside a
//!   poll ([`recv_report`]); the timing lock and the runtime shutdown are
//!   bounded, failing waits; the recipe counts and the bus ids are production
//!   asserts on the test thread. It adds three self-test parts (shape (h), the
//!   executor-site label halves, the two new self-tests), declares B-P35..B-P38,
//!   and gives `server_time_recipe_stream`'s cadence test freshness and an
//!   hour of liveness, which moves the sibling column of B-P13 and B-P27. It
//!   reasoned that no designed failure site of B-P1..B-P34 moves (a one-recipe
//!   test's premise is the old batch sample AND the new per-recipe one, a
//!   parked drain's wait still expires at once, and the pre-checks pass on
//!   unmutated production). Round 6's sweep measured that: next bullet.
//! - Round 6's sweep (`falsify-r6`, workflow `wf_f002d295-ed8`, 2026-11-12)
//!   applied every table row and three extras, one at a time, to production,
//!   with this file at SHA-256 `bc9133fb...` and `server_time_recipe_stream.rs`
//!   at `d132975d...`; every summary shows both unchanged after the restore.
//!   Of the 38 designated probes, 35 turned this file red at their designed
//!   assertion (B-P1..B-P29, B-P32..B-P37). The other three (B-P30, B-P31,
//!   B-P38) left it GREEN 18/18 by design and turned their paused-clock
//!   sibling red at its designed assertion. No designated probe survived.
//!   Every sibling it ran came out as its row stated, so the sibling cells
//!   of B-P13, B-P14, B-P27, B-P28, B-P30..B-P38 and the lib claims of
//!   B-P20..B-P22 now read MEASURED. The lib's
//!   `recipe_stream_drains_without_a_tokio_runtime` went RED under B-P20 and
//!   stayed GREEN under B-P21 and B-P22: of the two targets run, only this
//!   file catches a leaked guard. B-P15 failed at the half-period bound (the
//!   site round 4 left unmeasured). B-P10, B-P11, B-P21 and B-P22 each printed
//!   exactly one panic in this file's run, and no log of the sweep (41 probe
//!   logs, 3 lib logs, 10 flake logs, the baseline and the final run) contains
//!   "being shutdown". Two extras went red at tick 2's upper bound (k = 2),
//!   which no table row had reached: B-P39 (server time at 1600 ms) and B-P40
//!   (toast at 800 ms). The third, B-P41 (toast's `loop` cut to four ticks),
//!   SURVIVED: this file 18/18, `toast_dismiss_recipe_stream` 3/3. It is
//!   routed to story 2-63 Wave E1 (its row), whose E1a falsifier later
//!   MEASURED it red in `toast_dismiss_recipe_stream` (as ST-OWN-1). Each
//!   result cites its `r5x2-B-P<n>.summary`.
//!
//!   Flake check, unmutated, after B-P1..B-P38 and before the three extras
//!   (`r5x2-flake.out`): this file 5/5 GREEN, 18/18 each, in 5.01 to 5.02 s;
//!   `server_time_recipe_stream` 5/5 GREEN, 4/4 each, in 3.20 s. Run 1 of
//!   each recompiled `ui`. The baseline before the probes and the final run
//!   after the extras were green too, 18/18 and 4/4 (`r5x2-baseline.log`,
//!   `r5x2-final.log`). In all five runs of this file the two tick tests
//!   finished last, right after
//!   `a_late_panic_report_is_reported_as_the_panic`
//!   (`r5x2-flake-recipe_off_runtime_poll-<n>.log`, n = 1..5).
//!
//!   Timing lock, measured with filtered runs of this binary while
//!   `a_late_panic_report_is_reported_as_the_panic` still took the READ lock
//!   (`r5x2-rwlock-measure.out`): `server_time_stream_ticks_off_runtime`
//!   alone 2.01 s and that self-test alone 2.01 s, both together 4.02 s (a
//!   writer and a reader ran one after the other); the self-test with
//!   `a_reactor_dependent_stream_fails_here_instead_of_passing`, two readers,
//!   2.01 s (they overlapped: shape (g) alone makes that test wait at least
//!   1 s, so the two one after the other would take at least 3.01 s); the two
//!   tick tests, two writers, 3.01 s (one after the other). So the lock
//!   serializes as documented, and readers overlap. That left the self-test's
//!   own real-time bound unprotected from the other readers' deliberate
//!   panics, a false RED that load can cause (the falsifier's finding,
//!   REASONED, not measured). The close-out after the sweep therefore moved
//!   the self-test to the WRITE lock ("Timing isolation"). That change is NOT
//!   measured. REASONED: it only serializes more, at the cost of the
//!   self-test's own run time.
//! - The sibling column for B-P1..B-P9 was measured against the eight siblings
//!   it names, and only those. These other ui targets reference the same
//!   recipes and were run under no probe: `activity_tape_event_storm`,
//!   `cockpit_audit_aggregator_boot`, `cockpit_live_lab_run_smoke`,
//!   `trail_mirror_subscription_handle_gating` (deleted in story 2-63 Wave
//!   C1), `subscription_batch_inclusion` (story 2-63 Waves C1/C2, after round
//!   6: no Wave B probe ran against it, though B-P36 and B-P35 were later
//!   measured red there as C1-P8 and C2-P6), and the `ui` lib unit tests.
//!   "Only this file catches it" is therefore claimed against those eight
//!   siblings, not against the whole crate.
//!
//! The always-on tick bounds are DERIVED from production's documented periods
//! ("once per second", "every 500 ms"), never measured off the subject. Both
//! tick tests make the same four checks, in the same order (see the comment
//! above `assert_stamped_at_yield`):
//!
//! - FRESHNESS, two-sided. Each stamp is compared with a sample taken on the
//!   same worker right after the poll that yielded it (`Report::Item`'s
//!   `yielded_at` and `wall_yielded_at`). The stamp may be neither later than
//!   that sample nor half a period or more older. The window between stamp
//!   and sample is one return from `poll_next`: round 3's messages put it at
//!   about 3.5 to 36 µs. The bounds assume the thread is not descheduled for
//!   half a period inside that window and, for server time's wall clock, that
//!   the clock does not step backward inside it. Neither is plausible at
//!   these sizes.
//!
//!   DESIGN LIMIT: a stamp that is stale by LESS than half a period passes (a
//!   body that stamps with a time read up to `P / 2` before it yields, say).
//!   That is a stated limit, not an oversight. No realistic edit produces it:
//!   the plausible ones stamp a whole tick stale (B-P13, B-P15) or in the
//!   future (B-P26, B-P27), and those fail. A tighter bound would cost load
//!   tolerance, because the bound would then sit closer to a descheduling
//!   the box may really do.
//! - CADENCE, on the monotonic clock: tick `k` (k = 1, 2) must lie in
//!   `[k * P, (k + 1) * P)` after `stream()` began. The lower edge is hard:
//!   tokio never completes a tick before its deadline, so load can only make
//!   a tick later. The upper edge is the load-tolerant one: a single tick has
//!   to be a whole period late (1 s or 500 ms) to fail it.
//!
//!   EXACTLY what that pins, for an interval of period `P'`: tick `k` comes
//!   at `k * P' + d_k`, where `d_k` is the delay from `stream()` beginning to
//!   the interval's creation plus the delivery of tick `k`. Both checks pass
//!   iff `P - d_k / k <= P' < (k + 1) / k * P - d_k / k` for k = 1 and k = 2.
//!   With no delay that is the window `P' ∈ [P, 1.5 * P)`: `[1 s, 1.5 s)` for
//!   server time, `[500 ms, 750 ms)` for toast. A delay moves both edges DOWN
//!   by at most `d_k`: it can let through an interval faster by less than the
//!   delay, and it narrows the slow side. Any interval of `1.5 * P` or slower
//!   fails at tick 2 (of `2 * P` or slower already at tick 1), and
//!   deterministically, since such an interval cannot complete the tick in
//!   time (measured in round 6: at tick 2 for `1.6 * P` by B-P39 and B-P40,
//!   at tick 1 for `2 * P` by B-P28 and B-P29; the `1.5 * P` edge itself was
//!   not probed). An interval inside the window passes BY DESIGN: a 1.4 s
//!   server-time interval (B-P30) and a 700 ms toast interval (B-P31) are
//!   green here.
//!
//!   The EXACT period is pinned on a PAUSED tokio clock, by the siblings:
//!   - `server_time_recipe_stream::server_time_stream_impl_emits_at_1_hz_cadence`
//!     (`#[tokio::test(start_paused = true)]`): every gap, from `stream()` to
//!     tick 1 and then tick to tick, read with `tokio::time::Instant`, must be
//!     exactly 1 s. That fails for 100 ms (B-P14), 1.4 s (B-P30) and 2 s
//!     (B-P28) alike, and for a deleted skip of the immediate tick (B-P34).
//!     Since round 6 it also checks each stamp's freshness, two-sided
//!     (B-P13, B-P27), and takes an hour of paused ticks before it requires
//!     the stream still live (B-P38). It is the only test there that pins the
//!     period: in round 6 (that file's pre-E1a revision) the other three
//!     waited up to 1.5 s of real time per tick, so they accepted any interval
//!     under 1.5 s (rounds 5 and 6 measured them GREEN under B-P30 and B-P34,
//!     round 6 also under B-P38), by design. Since Wave E1a, T-ST-1d is
//!     deleted and T-ST-1c waits up to 1.5 s of PAUSED time for each of 7,200
//!     ticks; since Wave E1b, `server_time_recipe_stream_yields_tick` adds the
//!     Recipe path. Each still accepts any interval under 1.5 s, by design;
//!   - `toast_dismiss_recipe_stream` (`start_paused = true`). In round 6
//!     (its pre-E1a revision): three 500 ms advances had to yield exactly
//!     three ticks, two advances exactly two. That fails for EVERY period
//!     above 500 ms (B-P29, B-P31), with no slack. Read alone it still
//!     accepted any period in `(375 ms, 500 ms]` (three ticks still fit in
//!     1.5 s), which this file's lower edge rejects, so the two together
//!     pinned toast at 500 ms, to within the delivery delay `d_k`. Its stamp
//!     check then, `toast_tick_instants_are_monotone`'s `t2 >= t1` (that test
//!     is DELETED since Wave E1a), could not fail: the payload is a real
//!     `std::time::Instant`, monotone by definition, and on the paused clock
//!     two stamps are microseconds apart in real time (round 5, B-P31: two
//!     stamps 700 ms apart in paused time were 24.4 µs apart). Only its count
//!     asserts had teeth, so toast's stamp freshness (B-P15, B-P26) is pinned
//!     by this file alone, and still is: E1a's version checks only that each
//!     message IS a `ToastTick`. Its `toast_dismiss_stream_remains_open` then
//!     also accepted an empty channel after its fourth advance, so toast's
//!     "never ends" was pinned only to about its fourth tick (this file: its
//!     second, plus one poll). Round 6 measured that gap: a ticker cut to four
//!     ticks (B-P41) passed both files. Since Wave E1a that sibling pins the
//!     period EXACTLY on its own (`toast_period_is_exactly_500ms`: nothing
//!     after 499 ms and exactly one tick at 500 ms, in each of three periods)
//!     and requires tick 4 and then one tick per advance for 7,200 ticks; the
//!     E1a falsifier MEASURED B-P41 red there (as ST-OWN-1).
//!
//!   This split is deliberate (story 2-63, round-5 decision 1): tightening the
//!   real-time slack here would trade load tolerance on the 4-vCPU Windows
//!   leg for drifts that no unit slip produces, while a paused clock pins the
//!   period with no slack at all.
//!
//! The one difference is the payload's clock. Toast's stamp is an `Instant`,
//! so its lower cadence bound is checked on the stamp itself. Server time's
//! stamp is wall-clock time, which is not monotonic, so its cadence is checked
//! on the monotonic sample, which freshness ties to the stamp.
//!
//! The per-tick wait is the general `WAIT` (5 s), a hang detector only. It is
//! no longer what bounds cadence from above.
//!
//! ## Not covered here
//!
//! - The `ServerTimeRecipe::stream` / `ToastDismissRecipe::stream` bodies,
//!   OFF-runtime: covered here through their seams only (above). Since Wave
//!   C1 both recipes are `pub` in `ui::live`, and the bodies are streamed
//!   ON a tokio runtime by `server_time_recipe_stream_yields_tick`,
//!   `toast_dismiss_recipe_stream_yields_tick` (Wave E1b) and
//!   `subscription_batch_inclusion.rs`'s `batched_server_time_recipe_relays`
//!   and `batched_toast_dismiss_recipe_relays` (Wave C2, on the batch
//!   `subscription_batch` builds from a TEST ctx). (Before E1a,
//!   `server_time_recipe_stream`'s T-ST-1d, now deleted, called the seam
//!   exactly as T-ST-1a does, although its header claimed a real
//!   `ServerTimeRecipe`.)
//! - Recipe IDENTITY across updates: a recipe kept alive, or restarted, when
//!   iced rebuilds the batch, and the salts of the take-once recipes. Pinned
//!   since story 2-63 Wave C2 (its rows MEASURED 2026-11-14, `wf_a82b74dd-7bb` falsifier) by
//!   `subscription_batch_inclusion.rs`, on the batch `ui::live::subscription_batch`
//!   builds from a TEST `SubscriptionCtx`: `recipe_ids_are_frame_stable`,
//!   `salted_recipe_ids_change_with_salt`, `recipe_ids_are_pairwise_distinct`
//!   and `forward_plan_stays_batched_after_receiver_taken`. The bin's ctx
//!   literal, which picks each cell and salt in production, stays checked by
//!   nothing: a swapped salt there (that file's C1-RES2) leaves every target
//!   green. Within ONE batch, the bus ids are checked here too (B-P35).
//! - The `RecvError::Lagged` arms (the `stream_*` relays, `trail_mirror_stream_impl`,
//!   `activity_stream_impl`): no test here sends faster than a relay reads,
//!   so none of them runs. A `Lagged` arm turned into `break` passes here; the
//!   lag arms are the S1 waves' scope.
//! - Toast's always-on property past its fourth tick, HERE. This file takes
//!   two ticks plus one poll, so B-P41 (`loop` cut to four ticks) passes it by
//!   design. In round 6 it survived both targets that called
//!   `toast_dismiss_stream_impl`, because the pre-E1a
//!   `toast_dismiss_recipe_stream` accepted an empty channel after its fourth
//!   advance. Closed by Wave E1a in that file: `toast_dismiss_stream_remains_open`
//!   now requires one tick per advance for 7,200 ticks, and the E1a falsifier
//!   MEASURED B-P41 red there (as ST-OWN-1).
//! - `BusRecipe::stream`'s EAGER subscribe. `EventBus` exposes no receiver
//!   count, and here every bus stream is first polled before anything is
//!   published. A `stream()` that deferred its `stream_*` call into the body
//!   and kept the bus for it goes red here anyway, at the `Arc::strong_count`
//!   check right after the batch is built (and the bus would never close); one
//!   that dropped the bus right after a deferred subscribe would not. The
//!   `stream_*` fns subscribe before they return, which
//!   `live_subscription_full_bus` relies on for six of them by publishing
//!   before their first poll.
//! - The producers are test tasks on a real runtime, not the agent supervisor,
//!   `run_bakeoff` or the trainer's reader tasks. This file pins the consumer
//!   half of the hop.
//!
//! Gated on `live` (the only build where the recipes exist), NOT on
//! `target_os`: no pixels, deterministic on every OS.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self as std_mpsc, RecvTimeoutError, TryRecvError};
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use iced::advanced::subscription::{EventStream, Hasher, Recipe, into_recipes};
use rust_decimal_macros::dec;
use smol_str::SmolStr;

use agent::EventBus;
use agent::activity::{ActivityEvent, ActivityKind};
use agent::config::BusConfig;
use backtest::progress::{BakeoffProgress, BakeoffProgressSender, Progress, bakeoff_progress_pair};
use reflection::trail_mirror::{TrailMirrorHandle, TrailMirrorTick};
use trading_core::{
    Bar, FeeTier, Fill, FillId, Liquidity, MarketHealth, Money, OrderId, PnlSnapshot, Position,
    Price, Quantity, RiskTelemetry, Side, StrategyId, StrategyLoadError, StrategyLoaded,
    StrategySwapped, Symbol, Tick, Timeframe, Timestamp, Usdt, Venue,
};
use ui::lab::progress::LabProgressRecipe;
use ui::lab::trainer::TrainingLogLine;
use ui::lab::training_log::TrainingLogRecipe;
use ui::live::{
    ActivityRecipe, BakeoffProgressRecipe, ForwardPlanRecipe, NarrationOutcomeRecipe,
    SweepProgressRecipe,
};
use ui::state::{Message, TrailMirrorUiTick};

/// The bound on every wait for a report, unless a test names another one.
const WAIT: Duration = Duration::from_secs(5);

/// The bound on a report that may carry a captured panic: the build
/// (`stream()` on the stand-in), the first poll, and any later wait that
/// expires while the drain is still INSIDE a poll or its drop ([`recv_report`]).
/// Such a report is sent only after the panic hook ran, and under
/// `RUST_BACKTRACE=1` (CI) the hook symbolizes a backtrace first. The bound
/// only detects hangs: its expiry still fails the test.
const PANIC_REPORT_WAIT: Duration = Duration::from_secs(45);

/// The bound on the producer runtime's shutdown ([`ProducerRuntime`]'s
/// `Drop`). Every producer step has finished by then, so only a blocking task
/// that outlived the test can make it expire, and its expiry fails the test.
const SHUTDOWN_WAIT: Duration = Duration::from_secs(2);

/// The bound on taking the timing lock ([`timing_exclusive`],
/// [`timing_shared`]). A holder releases it within its own bounded waits, so
/// this bound only turns a holder that hung anyway into a failure here
/// instead of a hang. Generous on purpose: a FAILING holder may spend
/// [`PANIC_REPORT_WAIT`] and its teardown waits first.
const TIMING_WAIT: Duration = Duration::from_secs(300);

/// `server_time_stream_impl`'s documented period: "Emits
/// `Message::ServerTimeTick(Timestamp::now())` once per second."
const SERVER_TIME_PERIOD: Duration = Duration::from_secs(1);

/// `toast_dismiss_stream_impl`'s documented period: "Emits
/// `Message::ToastTick(Instant::now())` every 500 ms."
const TOAST_PERIOD: Duration = Duration::from_millis(500);

/// The payload prefix of every HARNESS self-check's panic. [`interpret`]
/// reports such a panic as a harness defect, never as a production
/// `Recipe::stream` / executor panic.
const HARNESS: &str = "HARNESS:";

/// A harness self-check: something true by construction of the harness. When
/// it fails, the HARNESS is broken, and the payload says so. A fact that
/// production code produces (how many recipes a production `Subscription`
/// holds, their ids) is NEVER checked with it: those are plain asserts, on the
/// test thread, whose message names both possible causes.
#[track_caller]
fn harness_check(holds: bool, what: impl FnOnce() -> String) {
    if !holds {
        panic!("{HARNESS} {}", what());
    }
}

// ── Timing isolation ──────────────────────────────────────────────────────────

/// Keeps the three tests with a real-time bound of their own (the two tick
/// tests and `a_late_panic_report_is_reported_as_the_panic`) apart from every
/// other test in this binary (libtest runs a binary's tests on parallel
/// threads). See the module doc, "Timing isolation".
static TIMING: RwLock<()> = RwLock::new(());

/// For the three tests with a real-time bound of their own (the two tick
/// tests and `a_late_panic_report_is_reported_as_the_panic`), held for the
/// whole test: no other test of this binary runs meanwhile. Poison-tolerant,
/// because a failed writer poisons the lock and the others must still run and
/// report their own verdicts. Bounded by [`TIMING_WAIT`]: its expiry fails the
/// test.
fn timing_exclusive() -> RwLockWriteGuard<'static, ()> {
    acquire_within("exclusive", TIMING_WAIT, || try_exclusive(&TIMING))
}

/// For every other test, held for the whole test: these run alongside each
/// other, never alongside a writer. Bounded like [`timing_exclusive`].
fn timing_shared() -> RwLockReadGuard<'static, ()> {
    acquire_within("shared", TIMING_WAIT, || try_shared(&TIMING))
}

/// A write guard if one is free now; a poisoned lock counts as free.
fn try_exclusive(lock: &RwLock<()>) -> Option<RwLockWriteGuard<'_, ()>> {
    match lock.try_write() {
        Ok(guard) => Some(guard),
        Err(TryLockError::Poisoned(poisoned)) => Some(poisoned.into_inner()),
        Err(TryLockError::WouldBlock) => None,
    }
}

/// A read guard if one is free now; a poisoned lock counts as free.
fn try_shared(lock: &RwLock<()>) -> Option<RwLockReadGuard<'_, ()>> {
    match lock.try_read() {
        Ok(guard) => Some(guard),
        Err(TryLockError::Poisoned(poisoned)) => Some(poisoned.into_inner()),
        Err(TryLockError::WouldBlock) => None,
    }
}

/// Retry `try_take` until it hands back a guard, for at most `bound`. Its
/// expiry FAILS the test: std's `RwLock` has no timed acquire, so the bound
/// is a deadline over a 1 ms retry.
fn acquire_within<G>(what: &str, bound: Duration, mut try_take: impl FnMut() -> Option<G>) -> G {
    let deadline = Instant::now() + bound;
    loop {
        if let Some(guard) = try_take() {
            return guard;
        }
        assert!(
            Instant::now() < deadline,
            "timing lock ({what}): not taken within {bound:?}. Another test of this binary held it \
             that long, so that test hung: its failure is the cause, this one follows from it"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

// ── The two sides: iced's executor and the producer runtime ───────────────────

/// iced's own default executor, built with the same trait call iced makes.
///
/// The return type is the COMPILE-TIME pin described in the module doc: this
/// type-checks only while `iced::executor::Default` IS
/// `futures::executor::ThreadPool`.
fn iced_executor() -> futures::executor::ThreadPool {
    let executor: iced::executor::Default = <iced::executor::Default as iced::Executor>::new()
        .expect("iced's default executor (the futures ThreadPool) builds");
    executor
}

/// `true` iff the CURRENT thread has no tokio runtime context.
fn no_tokio_context() -> bool {
    tokio::runtime::Handle::try_current().is_err()
}

/// The producer side: a multi-thread runtime, like the agent runtime the
/// cockpit builds. The harness thread never enters it. Tasks are scheduled
/// with `Runtime::spawn`, and `Drop` shuts it down on a helper thread, so not
/// even the shutdown sets a context on the harness thread.
struct ProducerRuntime {
    rt: Option<tokio::runtime::Runtime>,
    /// The bound on the shutdown in `Drop`: [`SHUTDOWN_WAIT`], or a shorter
    /// one for the self-test that proves its expiry fails.
    shutdown_within: Duration,
}

impl ProducerRuntime {
    fn new() -> Self {
        Self::with_shutdown_bound(SHUTDOWN_WAIT)
    }

    fn with_shutdown_bound(shutdown_within: Duration) -> Self {
        harness_check(no_tokio_context(), || {
            "premise: the harness thread starts with no tokio context".to_owned()
        });
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("the producer runtime builds");
        Self {
            rt: Some(rt),
            shutdown_within,
        }
    }

    fn rt(&self) -> &tokio::runtime::Runtime {
        self.rt.as_ref().expect("the runtime is alive until drop")
    }

    /// The runtime `Handle`, what the cockpit hands every recipe.
    fn handle(&self) -> tokio::runtime::Handle {
        self.rt().handle().clone()
    }

    /// Run ONE producer step as a task on the runtime (`Runtime::spawn`),
    /// where production's senders live, and wait (bounded) for its checked
    /// result. `Err` means a send failed and FAILS the test. So do a timeout
    /// and a panic (`Disconnected`). What the step hands back (an activity
    /// handle, the expected relay of a publish) comes back to the harness.
    fn produce<T, F>(&self, what: &str, step: F) -> T
    where
        T: Send + 'static,
        F: Future<Output = Result<T, String>> + Send + 'static,
    {
        let (done_tx, done_rx) = std_mpsc::channel();
        let _detached = self.rt().spawn(async move {
            let _ = done_tx.send(step.await);
        });
        match done_rx.recv_timeout(WAIT) {
            Ok(Ok(value)) => value,
            Ok(Err(e)) => panic!("{what}: a producer send FAILED: {e}"),
            Err(RecvTimeoutError::Timeout) => {
                panic!("{what}: the producer did not finish within {WAIT:?}")
            }
            Err(RecvTimeoutError::Disconnected) => panic!("{what}: the producer task panicked"),
        }
    }
}

/// The shutdown is a bounded wait like every other: it must finish within
/// `shutdown_within`, and a shutdown that hit that bound (a blocking task
/// still running), hung past it, or panicked FAILS the test
/// (`a_teardown_or_lock_wait_that_expires_fails_the_test`). On a test that is
/// already failing it only prints a note, because a second panic would abort
/// the run and hide the first.
impl Drop for ProducerRuntime {
    fn drop(&mut self) {
        let Some(rt) = self.rt.take() else {
            return;
        };
        let bound = self.shutdown_within;
        let (done_tx, done_rx) = std_mpsc::channel();
        let _detached = std::thread::spawn(move || {
            let started = Instant::now();
            rt.shutdown_timeout(bound);
            let _ = done_tx.send(started.elapsed());
        });
        let failure = match done_rx.recv_timeout(bound + WAIT) {
            Ok(took) if took < bound => return,
            Ok(took) => format!(
                "the producer runtime's shutdown hit its {bound:?} bound (it returned after \
                 {took:?}): a blocking task on it was still running, so it outlived the test"
            ),
            Err(RecvTimeoutError::Timeout) => format!(
                "the producer runtime did not finish shutting down within {:?}",
                bound + WAIT
            ),
            Err(RecvTimeoutError::Disconnected) => {
                "the producer runtime's shutdown panicked".to_owned()
            }
        };
        if std::thread::panicking() {
            eprintln!(
                "teardown, after the failure above: {failure}. This follows from that failure; it \
                 is not its cause"
            );
        } else {
            panic!("teardown: {failure}");
        }
    }
}

// ── Building on the event-loop stand-in, draining on the pool ─────────────────

/// What `make_stream` hands back for one recipe: the stream, optionally ONE
/// fact the test reads right after `Recipe::stream` returned, on the same
/// thread, BEFORE the first poll (the drain is not even spawned yet), and the
/// premise sampled at that same moment.
///
/// Make it right after `stream()` returns, with [`Built::from`] or
/// [`Built::observing`]: both sample the premise for THIS recipe, so in a
/// batch a context leaked by recipe #k is reported against recipe #k, not
/// against the batch's first recipe.
struct Built {
    stream: BoxStream<'static, Message>,
    observed_after_stream: Option<usize>,
    /// `no_tokio_context()` on the event-loop stand-in, sampled when this
    /// `Built` was made.
    no_tokio_after_stream: bool,
}

impl Built {
    /// `stream` just returned from `Recipe::stream`; `observed` is the fact
    /// the test read right after it.
    fn observing(stream: BoxStream<'static, Message>, observed: usize) -> Self {
        Self {
            stream,
            observed_after_stream: Some(observed),
            no_tokio_after_stream: no_tokio_context(),
        }
    }
}

impl From<BoxStream<'static, Message>> for Built {
    fn from(stream: BoxStream<'static, Message>) -> Self {
        Self {
            stream,
            observed_after_stream: None,
            no_tokio_after_stream: no_tokio_context(),
        }
    }
}

/// Where a captured panic happened.
#[derive(Debug, Clone, Copy)]
enum Site {
    /// In `make_stream` (`Recipe::stream`), on the event-loop stand-in.
    RecipeStream,
    /// In the drain task (a poll, or the drop), on iced's executor.
    Executor,
}

/// The first poll, made with the task's own waker before anything is produced.
#[derive(Debug)]
enum FirstPoll {
    Pending,
    Ended,
    Item(Box<Message>),
}

/// What the two threads report, in order: `StreamBuilt` (event-loop
/// stand-in), then `Parked`, then `Waiting` / `Item` interleaved, then
/// `Ended` (or `StoppedLive` for a capped always-on stream), then `Exit` (the
/// pool). A captured panic or a premise violation is the last report. A drain
/// cancelled by its [`Drain`] stops with no further report.
#[derive(Debug)]
enum Report {
    StreamBuilt {
        no_tokio_before_stream: bool,
        no_tokio_after_stream: bool,
        observed_after_stream: Option<usize>,
        before_stream: Instant,
    },
    Parked {
        first_poll: FirstPoll,
    },
    /// A poll returned `Pending`, after the stream registered its waker: the
    /// consumer is parked on its source with nothing buffered. Sent from
    /// inside that poll, so the next item can reach it only through a wake.
    Waiting,
    Item {
        msg: Box<Message>,
        /// Sampled on the worker right after the poll that returned the item.
        yielded_at: Instant,
        /// The wall clock, sampled next to `yielded_at`.
        wall_yielded_at: Timestamp,
    },
    Ended,
    StoppedLive,
    /// The pool worker already had a tokio context when poll `poll` began.
    ContextBeforePoll {
        poll: usize,
    },
    /// Poll `poll` left a tokio context entered on the worker that made it.
    ContextLeaked {
        poll: usize,
        outcome: &'static str,
    },
    Exit {
        no_tokio_after_drop: bool,
    },
    Panicked {
        site: Site,
        msg: String,
    },
}

fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_owned()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_owned()
    }
}

/// The harness's handle on ONE drain: its report channel (what every
/// `expect_*` reads, through `Deref`) and the two ends of its teardown.
///
/// Dropping it CANCELS the drain the way iced does when a subscription goes
/// away: `Tracker::update` (`subscription/tracker.rs`) keeps a
/// `futures::channel::oneshot::Sender<()>` per execution and runs every drain
/// as `select(&mut canceled, stream.next())`, so dropping that sender ends the
/// drain and drops its stream. Then it waits, bounded, until the drain task
/// is gone. Every test that HAS a [`ProducerRuntime`] declares it BEFORE its
/// drains, and locals drop in reverse, so this runs while the runtime is still
/// up, on a failing test's unwind too (the module doc's teardown bullet names
/// the tests without one and why that is safe). So no drain outlives its test, and
/// none polls a timer of a runtime that is shutting down. That poll used to panic
/// ("A Tokio 1.x context was found, but it is being shutdown",
/// `tokio/src/runtime/time/entry.rs`) and the panic hook printed it after the
/// real cause of a failure, where it read like the cause.
struct Drain {
    reports: std_mpsc::Receiver<Report>,
    /// iced's `_cancel`: dropping it resolves the drain's `canceled`.
    cancel: Option<futures::channel::oneshot::Sender<()>>,
    /// Never receives: it reports `Disconnected` once the drain task has
    /// finished and dropped its stream (or the stand-in never spawned it).
    gone: std_mpsc::Receiver<Infallible>,
    /// Set while the drain is INSIDE a `poll_next` or the stream's drop, and
    /// left set by a panic there. [`recv_report`] reads it when a wait
    /// expires: a drain still inside a poll may be running a panic hook.
    in_poll: Arc<AtomicBool>,
}

impl std::ops::Deref for Drain {
    type Target = std_mpsc::Receiver<Report>;

    fn deref(&self) -> &Self::Target {
        &self.reports
    }
}

impl Drop for Drain {
    fn drop(&mut self) {
        drop(self.cancel.take());
        match self.gone.recv_timeout(WAIT) {
            Err(RecvTimeoutError::Disconnected) => {}
            Ok(never) => match never {},
            // Already failing: a second panic here would abort the process and
            // hide the first one, so only say what follows.
            Err(RecvTimeoutError::Timeout) if std::thread::panicking() => eprintln!(
                "teardown, after the failure above: a drain was still running {WAIT:?} after it \
                 was cancelled. Anything it prints from here on follows from that failure; it is \
                 not its cause"
            ),
            Err(RecvTimeoutError::Timeout) => panic!(
                "teardown: a drain was still running {WAIT:?} after it was cancelled (dropping its \
                 stream hung, or the stand-in never returned from `stream()`)"
            ),
        }
    }
}

/// The drain-side ends of one [`Drain`]: they move to the stand-in, then into
/// the drain task.
struct DrainEnds {
    tx: std_mpsc::Sender<Report>,
    canceled: futures::channel::oneshot::Receiver<()>,
    alive: std_mpsc::Sender<Infallible>,
    in_poll: Arc<AtomicBool>,
}

/// Mirror `Runtime::track` for a whole batch: on a fresh std thread standing
/// in for iced's event loop, run `make_streams` (every recipe's `stream()`)
/// inside ONE `iced::Executor::enter`, check the premise there before and
/// after, and only then spawn each drain onto iced's executor
/// ([`spawn_drain`]). Each stream reports over its own [`Drain`], in the order
/// `make_streams` returned them.
///
/// One step of `Tracker::update` is NOT mirrored: it hashes every recipe and
/// skips one whose id is already tracked, which also dedups within one batch.
/// Here every recipe `make_streams` builds is streamed. That is faithful only
/// where the ids are distinct: trivially for one recipe on a fresh tracker,
/// and for the bus batch because `bus_recipes_relay_off_runtime` asserts it
/// first ([`tracker_ids`]). Identity across updates (a recipe kept or
/// restarted when the batch is rebuilt, the salts) is story 2-63 Wave C2's,
/// pinned in `subscription_batch_inclusion.rs` on a batch built from a TEST
/// ctx (the bin's ctx literal, a swapped salt included, stays checked by
/// nothing).
///
/// The after-`stream()` premise is sampled per recipe ([`Built`]), and once
/// more after the whole batch, which counts against the LAST recipe only (it
/// covers what ran on the stand-in after that recipe's `stream()`). A stream
/// whose premise failed is NOT spawned: its test fails at that premise, and a
/// drain would only print a second, consequential panic after the cause
/// (B-P22's "no reactor running" on a pool worker, measured in round 5).
///
/// `cap`: `None` drains to the stream END. `Some(n)` takes `n` items from an
/// always-on stream, then reports `StoppedLive` if one more poll does not end
/// it, or `Ended` if it does.
fn poll_batch_on_iced_executor<F>(
    executor: &futures::executor::ThreadPool,
    count: usize,
    cap: Option<usize>,
    make_streams: F,
) -> Vec<Drain>
where
    F: FnOnce() -> Vec<Built> + Send + 'static,
{
    let (ends, drains): (Vec<_>, Vec<_>) = (0..count)
        .map(|_| {
            let (tx, reports) = std_mpsc::channel();
            let (cancel, canceled) = futures::channel::oneshot::channel();
            let (alive, gone) = std_mpsc::channel();
            let in_poll = Arc::new(AtomicBool::new(false));
            (
                DrainEnds {
                    tx,
                    canceled,
                    alive,
                    in_poll: Arc::clone(&in_poll),
                },
                Drain {
                    reports,
                    cancel: Some(cancel),
                    gone,
                    in_poll,
                },
            )
        })
        .unzip();
    let executor = executor.clone();
    let _event_loop = std::thread::Builder::new()
        .name("iced-event-loop-stand-in".to_owned())
        .spawn(move || {
            let no_tokio_before_stream = no_tokio_context();
            let before_stream = Instant::now();
            let built = std::panic::catch_unwind(AssertUnwindSafe(|| -> Vec<Built> {
                let streams = iced::Executor::enter(&executor, make_streams);
                harness_check(streams.len() == count, || {
                    format!(
                        "`make_streams` built {} streams for {count} report channels",
                        streams.len()
                    )
                });
                streams
            }));
            let no_tokio_after_batch = no_tokio_context();
            let streams = match built {
                Ok(streams) => streams,
                Err(payload) => {
                    let msg = panic_text(payload.as_ref());
                    for end in &ends {
                        let _ = end.tx.send(Report::Panicked {
                            site: Site::RecipeStream,
                            msg: msg.clone(),
                        });
                    }
                    return;
                }
            };
            for (i, (built, end)) in streams.into_iter().zip(ends).enumerate() {
                let no_tokio_after_stream =
                    built.no_tokio_after_stream && (i + 1 < count || no_tokio_after_batch);
                let report = Report::StreamBuilt {
                    no_tokio_before_stream,
                    no_tokio_after_stream,
                    observed_after_stream: built.observed_after_stream,
                    before_stream,
                };
                let premise_held = no_tokio_before_stream && no_tokio_after_stream;
                if end.tx.send(report).is_ok() && premise_held {
                    spawn_drain(&executor, cap, built.stream, end);
                }
                // Otherwise the stream and its ends drop here, on the stand-in:
                // the report channel closes and `Drain::drop` returns at once.
            }
        })
        .expect("the event-loop stand-in thread starts");
    drains
}

/// [`poll_batch_on_iced_executor`] for one recipe.
fn poll_on_iced_executor<F, S>(
    executor: &futures::executor::ThreadPool,
    cap: Option<usize>,
    make_stream: F,
) -> Drain
where
    F: FnOnce() -> S + Send + 'static,
    S: Into<Built>,
{
    poll_batch_on_iced_executor(executor, 1, cap, move || vec![make_stream().into()])
        .pop()
        .expect("one report channel")
}

/// One `poll_next`, with the premise checked on the SAME thread immediately
/// before and after it, inside the same `poll` call, so no wake can move the
/// task to another worker in between. A violation ends the drain. `in_poll`
/// is set for exactly the `poll_next` call; a panic inside it leaves it set.
fn poll_watched(
    stream: &mut BoxStream<'static, Message>,
    polls: &mut usize,
    in_poll: &AtomicBool,
    cx: &mut Context<'_>,
) -> Result<Poll<Option<Message>>, Report> {
    *polls += 1;
    let poll = *polls;
    if !no_tokio_context() {
        return Err(Report::ContextBeforePoll { poll });
    }
    in_poll.store(true, Ordering::SeqCst);
    let polled = stream.poll_next_unpin(cx);
    in_poll.store(false, Ordering::SeqCst);
    if no_tokio_context() {
        Ok(polled)
    } else {
        let outcome = match polled {
            Poll::Pending => "Pending",
            Poll::Ready(Some(_)) => "an item",
            Poll::Ready(None) => "the end",
        };
        Err(Report::ContextLeaked { poll, outcome })
    }
}

/// Exactly ONE watched poll, with the task's own waker.
async fn poll_once_watched(
    stream: &mut BoxStream<'static, Message>,
    polls: &mut usize,
    in_poll: &AtomicBool,
) -> Result<Poll<Option<Message>>, Report> {
    futures::future::poll_fn(|cx| Poll::Ready(poll_watched(stream, polls, in_poll, cx))).await
}

/// Watched polls until the stream yields or ends. Every poll that returns
/// `Pending` reports `Waiting` from inside that poll, after the stream
/// registered its waker, so the harness can send exactly when the consumer is
/// parked.
async fn next_watched(
    stream: &mut BoxStream<'static, Message>,
    polls: &mut usize,
    in_poll: &AtomicBool,
    tx: &std_mpsc::Sender<Report>,
) -> Result<Option<Message>, Report> {
    futures::future::poll_fn(|cx| match poll_watched(stream, polls, in_poll, cx) {
        Ok(Poll::Pending) => {
            let _ = tx.send(Report::Waiting);
            Poll::Pending
        }
        Ok(Poll::Ready(item)) => Poll::Ready(Ok(item)),
        Err(premise) => Poll::Ready(Err(premise)),
    })
    .await
}

/// Spawn the drain, the one future iced puts on its executor, and report
/// every step of it. Like iced's, it ends early when it is CANCELLED (its
/// [`Drain`] dropped): it then drops the stream on the worker and finishes,
/// without a report.
///
/// A first poll that yields an item does not end the drain: it reads on, so a
/// test that published before the first poll sees the rest relayed too.
///
/// The cancel branch drops the stream OUTSIDE the `catch_unwind`, so a `Drop`
/// that panics there is only printed by the hook, not reported. In a passing
/// run no production stream reaches that branch: every drain has run to its
/// `Exit` (through `Ended`, or `StoppedLive` for a capped always-on stream),
/// inside the capture, before its `Drain` drops. Only a test that already
/// failed, or the teardown self-test (whose stream is the test's own), drops
/// a live drain.
fn spawn_drain(
    executor: &futures::executor::ThreadPool,
    cap: Option<usize>,
    mut stream: BoxStream<'static, Message>,
    ends: DrainEnds,
) {
    let DrainEnds {
        tx,
        canceled,
        alive,
        in_poll,
    } = ends;
    let on_panic = tx.clone();
    let task = async move {
        let mut polls = 0usize;
        let first_poll = match poll_once_watched(&mut stream, &mut polls, &in_poll).await {
            Ok(Poll::Pending) => FirstPoll::Pending,
            Ok(Poll::Ready(None)) => FirstPoll::Ended,
            Ok(Poll::Ready(Some(msg))) => FirstPoll::Item(Box::new(msg)),
            Err(premise) => {
                let _ = tx.send(premise);
                return;
            }
        };
        let live = !matches!(first_poll, FirstPoll::Ended);
        let mut relayed = usize::from(matches!(first_poll, FirstPoll::Item(_)));
        if tx.send(Report::Parked { first_poll }).is_err() {
            return; // the harness has already failed
        }
        if live {
            loop {
                let report = if cap == Some(relayed) {
                    match poll_once_watched(&mut stream, &mut polls, &in_poll).await {
                        Ok(Poll::Ready(None)) => Report::Ended,
                        Ok(_) => Report::StoppedLive,
                        Err(premise) => {
                            let _ = tx.send(premise);
                            return;
                        }
                    }
                } else {
                    match next_watched(&mut stream, &mut polls, &in_poll, &tx).await {
                        Ok(Some(msg)) => {
                            relayed += 1;
                            Report::Item {
                                msg: Box::new(msg),
                                yielded_at: Instant::now(),
                                wall_yielded_at: Timestamp::now(),
                            }
                        }
                        Ok(None) => Report::Ended,
                        Err(premise) => {
                            let _ = tx.send(premise);
                            return;
                        }
                    }
                };
                let more = matches!(report, Report::Item { .. });
                if tx.send(report).is_err() || !more {
                    break;
                }
            }
        }
        in_poll.store(true, Ordering::SeqCst);
        drop(stream);
        in_poll.store(false, Ordering::SeqCst);
        let _ = tx.send(Report::Exit {
            no_tokio_after_drop: no_tokio_context(),
        });
    };
    iced::Executor::spawn(executor, async move {
        let task = AssertUnwindSafe(task).catch_unwind().boxed();
        match futures::future::select(canceled, task).await {
            // Cancelled: the stream, inside `task`, drops HERE, on the worker,
            // while the producer runtime is still up (see `Drain`).
            futures::future::Either::Left((_, task)) => drop(task),
            futures::future::Either::Right((Err(payload), _)) => {
                let _ = on_panic.send(Report::Panicked {
                    site: Site::Executor,
                    msg: panic_text(payload.as_ref()),
                });
            }
            futures::future::Either::Right((Ok(()), _)) => {}
        }
        // Last, after the stream is gone: `Drain::drop` waits for exactly this.
        drop(alive);
    });
}

// ── Reading the reports ───────────────────────────────────────────────────────

/// One received report, or the failure it stands for. A panic, a premise
/// violation, a timeout and a vanished task all FAIL the test, each with its
/// own message.
///
/// A captured panic whose payload starts with [`HARNESS`] is a harness
/// self-check ([`harness_check`]) that failed inside the capture. It is
/// reported as a HARNESS DEFECT, not as `Recipe::stream` or the stream
/// panicking (`a_harness_self_check_is_not_reported_as_a_production_panic`).
fn interpret(got: Result<Report, RecvTimeoutError>, within: Duration, what: &str) -> Report {
    match got {
        Ok(Report::Panicked { site, msg }) if msg.starts_with(HARNESS) => {
            let place = match site {
                Site::RecipeStream => "on the event-loop stand-in, while the recipes were built",
                Site::Executor => "on iced's executor, inside the drain",
            };
            panic!(
                "{what}: HARNESS DEFECT, not a production failure: a harness self-check failed \
                 {place}: {msg}"
            )
        }
        Ok(Report::Panicked {
            site: Site::RecipeStream,
            msg,
        }) => panic!(
            "{what}: `Recipe::stream` PANICKED on the event-loop stand-in thread (no tokio context): {msg}"
        ),
        Ok(Report::Panicked {
            site: Site::Executor,
            msg,
        }) => {
            panic!("{what}: the stream PANICKED on iced's executor (no tokio context): {msg}")
        }
        Ok(Report::ContextLeaked { poll, outcome }) => panic!(
            "{what}: PREMISE: poll #{poll} (it returned {outcome}) left a tokio context entered on \
             the executor worker that made it: from then on the stream would run inside the runtime"
        ),
        Ok(Report::ContextBeforePoll { poll }) => panic!(
            "{what}: PREMISE: the executor worker already had a tokio context when poll #{poll} began"
        ),
        Ok(report) => report,
        Err(RecvTimeoutError::Timeout) => {
            panic!("{what}: nothing reported within {within:?}: the stream hung")
        }
        Err(RecvTimeoutError::Disconnected) => panic!(
            "{what}: a harness thread ended without reporting (it panicked outside the capture, or was dropped)"
        ),
    }
}

/// The next report within `left` (`within` is the whole budget, for the
/// message), through [`interpret`].
///
/// A wait that expires while the drain is INSIDE a poll or its drop
/// ([`Drain::in_poll`]) is not yet a verdict: a poll that panicked is running
/// the panic hook, which under CI's `RUST_BACKTRACE=1` may outlast `within`
/// (a cold Windows PDB). So up to [`PANIC_REPORT_WAIT`] more is spent on that
/// one report. A `Panicked` report is then reported as itself; anything else
/// means one poll ran longer than `within`, blocking an executor thread,
/// which FAILS as such. A wait that expires while the drain is PARKED (a
/// stream that buffers, B-P17) fails at once, as before.
/// `a_late_panic_report_is_reported_as_the_panic` proves both halves.
fn recv_report(drain: &Drain, left: Duration, within: Duration, what: &str) -> Report {
    let got = match drain.reports.recv_timeout(left) {
        Err(RecvTimeoutError::Timeout) if drain.in_poll.load(Ordering::SeqCst) => {
            match drain.reports.recv_timeout(PANIC_REPORT_WAIT) {
                Ok(Report::Panicked { site, msg }) => {
                    let late = format!(
                        "{msg} [reported after the {within:?} wait had expired, while that poll \
                         was still running: its panic hook ran long]"
                    );
                    Ok(Report::Panicked { site, msg: late })
                }
                Ok(other) => panic!(
                    "{what}: nothing reported within {within:?}, and at that moment the drain was \
                     still INSIDE a poll (or the stream's drop): one poll ran longer than \
                     {within:?}, blocking an executor thread. It reported only afterwards: \
                     {other:?}"
                ),
                Err(RecvTimeoutError::Timeout) => panic!(
                    "{what}: nothing reported within {within:?}: the stream hung INSIDE a poll (or \
                     its drop), blocking an executor thread, and no panic report came within a \
                     further {PANIC_REPORT_WAIT:?}"
                ),
                Err(RecvTimeoutError::Disconnected) => Err(RecvTimeoutError::Disconnected),
            }
        }
        other => other,
    };
    interpret(got, within, what)
}

/// The next report within `within`.
fn next_report(drain: &Drain, within: Duration, what: &str) -> Report {
    recv_report(drain, within, within, what)
}

/// When `stream()` ran, and what the test read right after it, as seen on the
/// event-loop stand-in.
struct BuildFacts {
    observed_after_stream: Option<usize>,
    before_stream: Instant,
}

/// The first report: the premise held on the event-loop stand-in before and
/// after `stream()`.
///
/// The two checks are not alike. BEFORE `stream()` is a HARNESS self-check,
/// true by construction: the stand-in is a freshly spawned std thread, so it
/// fails only if the harness itself broke, never because of production code.
/// AFTER `stream()` is the one about production: a `stream()` that leaks its
/// `EnterGuard` fails it (B-P10, B-P11, B-P21, B-P22). It is sampled for THIS
/// recipe ([`Built`]), and no drain was spawned for a stream that fails it.
fn expect_stream_built(reports: &Drain, what: &str) -> BuildFacts {
    match next_report(reports, PANIC_REPORT_WAIT, what) {
        Report::StreamBuilt {
            no_tokio_before_stream,
            no_tokio_after_stream,
            observed_after_stream,
            before_stream,
        } => {
            harness_check(no_tokio_before_stream, || {
                format!(
                    "{what}: premise (true by construction, so the harness itself is broken, not \
                     production code): `stream()` must be called with NO tokio context, as on \
                     iced's event-loop thread, and the stand-in is a fresh std thread"
                )
            });
            assert!(
                no_tokio_after_stream,
                "{what}: PREMISE: `stream()` must leave NO tokio context entered on the thread that \
                 called it. On iced's event-loop thread a leaked EnterGuard gives every LATER \
                 `stream()` call a context, which would hide a missing guard in another recipe. \
                 (Sampled on the event-loop stand-in right after THIS recipe's `stream()` returned; \
                 for a batch's last recipe, also after the batch's remaining steps there. No drain \
                 was spawned for this stream.)"
            );
            BuildFacts {
                observed_after_stream,
                before_stream,
            }
        }
        other => panic!("{what}: expected the StreamBuilt report first, got {other:?}"),
    }
}

/// The first two reports: the stream was built with the premise holding, and
/// the first poll (premise held around it too) found it LIVE with nothing
/// buffered.
fn expect_parked(reports: &Drain, what: &str) -> BuildFacts {
    let facts = expect_stream_built(reports, what);
    match next_report(reports, PANIC_REPORT_WAIT, what) {
        Report::Parked { first_poll } => match first_poll {
            FirstPoll::Pending => {}
            FirstPoll::Ended => panic!(
                "{what}: the stream ENDED on its first poll (expected it live on its source)"
            ),
            FirstPoll::Item(msg) => {
                panic!("{what}: the first poll yielded {msg:?} before anything was produced")
            }
        },
        other => panic!("{what}: expected the Parked report, got {other:?}"),
    }
    facts
}

/// The first two reports, for a test that published AFTER `stream()` returned
/// and BEFORE the drain was spawned: the build held the premise, and the first
/// poll must yield that event. Returns it.
fn expect_first_poll_item(reports: &Drain, what: &str) -> Message {
    expect_stream_built(reports, what);
    match next_report(reports, PANIC_REPORT_WAIT, what) {
        Report::Parked {
            first_poll: FirstPoll::Item(msg),
        } => *msg,
        Report::Parked {
            first_poll: FirstPoll::Pending,
        } => panic!(
            "{what}: the first poll found NOTHING: the event published after `stream()` returned, \
             before the first poll, was LOST, so `Recipe::stream` did not subscribe EAGERLY \
             (production: subscribe \"before the stream is first polled\" to avoid the \
             publish-before-subscribe race)"
        ),
        Report::Parked {
            first_poll: FirstPoll::Ended,
        } => panic!("{what}: the stream ENDED on its first poll"),
        other => panic!("{what}: expected the Parked report, got {other:?}"),
    }
}

/// The consumer polled again after its last report and found nothing: it is
/// PARKED on its source, so whatever is sent next reaches it through a wake.
fn expect_waiting(reports: &Drain, what: &str) {
    match next_report(reports, WAIT, what) {
        Report::Waiting => {}
        Report::Item { msg, .. } => panic!(
            "{what}: the stream yielded {msg:?} with nothing sent since its last item (expected it \
             parked on its source)"
        ),
        other => panic!("{what}: expected the stream parked again on its source, got {other:?}"),
    }
}

/// A relayed message, with what the worker sampled right after the poll that
/// returned it.
struct Yielded {
    msg: Message,
    at: Instant,
    wall_at: Timestamp,
}

/// The next relayed message within `within` (in total), skipping `Waiting`
/// reports: a wake that brought no item yet, or a consumer parked again before
/// an always-on stream's next tick.
fn next_item(reports: &Drain, within: Duration, what: &str) -> Yielded {
    let deadline = Instant::now() + within;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match recv_report(reports, left, within, what) {
            Report::Waiting => {}
            Report::Item {
                msg,
                yielded_at,
                wall_yielded_at,
            } => {
                return Yielded {
                    msg: *msg,
                    at: yielded_at,
                    wall_at: wall_yielded_at,
                };
            }
            other => panic!("{what}: expected a relayed message, got {other:?}"),
        }
    }
}

/// Every relayed message until the stream END (within `WAIT` in total), then
/// the clean exit.
fn drain_to_end(reports: &Drain, what: &str) -> Vec<Message> {
    let deadline = Instant::now() + WAIT;
    let mut got = Vec::new();
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match recv_report(reports, left, WAIT, what) {
            Report::Waiting => {}
            Report::Item { msg, .. } => {
                got.push(*msg);
                assert!(got.len() <= 64, "{what}: no stream end after 64 messages");
            }
            Report::Ended => break,
            other => panic!("{what}: expected a message or the stream end, got {other:?}"),
        }
    }
    expect_clean_exit(reports, what);
    got
}

/// Drop the LAST sender, from a task on the runtime and only now that every
/// item was observed, then drain the stream to its END. Returns what it
/// relayed after the drop (its closing message, where it has one).
fn close_and_drain<F>(
    producer: &ProducerRuntime,
    reports: &Drain,
    what: &str,
    drop_last_sender: F,
) -> Vec<Message>
where
    F: Future<Output = Result<(), String>> + Send + 'static,
{
    expect_waiting(reports, &format!("{what}, before its last sender drops"));
    producer.produce(&format!("{what}: drop the last sender"), drop_last_sender);
    drain_to_end(reports, what)
}

/// The capped always-on stream was still live after its items.
fn expect_still_live(reports: &Drain, what: &str) {
    match next_report(reports, WAIT, what) {
        Report::StoppedLive => {}
        Report::Ended => panic!("{what}: an always-on stream ENDED"),
        other => panic!("{what}: expected the stream still live, got {other:?}"),
    }
    expect_clean_exit(reports, what);
}

/// `Exit` with the premise holding after the stream was dropped on the
/// worker, and then nothing more: both harness threads are done, so every
/// sender is gone.
fn expect_clean_exit(reports: &Drain, what: &str) {
    match next_report(reports, WAIT, what) {
        Report::Exit {
            no_tokio_after_drop,
        } => assert!(
            no_tokio_after_drop,
            "{what}: PREMISE: dropping the stream left a tokio context on the executor worker"
        ),
        other => panic!("{what}: expected the task's Exit, got {other:?}"),
    }
    match reports.recv_timeout(WAIT) {
        Err(RecvTimeoutError::Disconnected) => {}
        other => panic!("{what}: the executor task reported after its Exit: {other:?}"),
    }
}

/// Run a harness step that MUST fail, and return its failure text.
fn harness_failure(step: impl FnOnce()) -> String {
    let verdict = std::panic::catch_unwind(AssertUnwindSafe(step));
    let payload =
        verdict.expect_err("the harness FAILED the test (it did not pass the probe shape)");
    panic_text(payload.as_ref())
}

// ── Builders and projections ──────────────────────────────────────────────────

/// The runtime event stream a `Recipe::stream` receives. Every recipe here
/// ignores it, so an empty one is faithful.
fn empty_event_stream() -> EventStream {
    Box::pin(futures::stream::empty::<iced::advanced::subscription::Event>())
}

/// The id iced gives each recipe, computed exactly as `Tracker::update` does
/// (`iced_futures-0.14.0/src/subscription/tracker.rs`): a fresh
/// `Hasher::default()`, the recipe's own `hash`, then `finish()`. Tracker
/// streams a recipe only if its id is not tracked yet, and it inserts each id
/// as it goes, so two recipes of ONE batch with the same id are streamed
/// once. Nothing here is streamed: the recipes drop on return.
fn tracker_ids(recipes: Vec<Box<dyn Recipe<Output = Message>>>) -> Vec<u64> {
    recipes
        .iter()
        .map(|recipe| {
            let mut hasher = Hasher::default();
            recipe.hash(&mut hasher);
            std::hash::Hasher::finish(&hasher)
        })
        .collect()
}

/// `1` iff the shared receiver cell is EMPTY (the recipe took the receiver),
/// `0` while it still holds it. Read right after `stream()` returned.
fn cell_taken<T>(cell: &Mutex<Option<T>>) -> usize {
    usize::from(cell.lock().unwrap().is_none())
}

/// The take-once check, from the fact read right after `stream()` returned.
fn assert_taken(built: &BuildFacts, what: &str) {
    assert_eq!(
        built.observed_after_stream,
        Some(1),
        "{what}: `Recipe::stream` took the receiver: right after it returned, on the event-loop \
         stand-in and before the drain was spawned, the cell holds None (Some(1)). Some(0) means \
         the take moved into the stream body"
    );
}

/// A deterministic agent-side `ForwardPlan` with the strategy id varied, so
/// relayed plans can be told apart.
fn agent_forward_plan_for(strategy: &str) -> agent::ForwardPlan {
    agent::config::ForwardPlan {
        strategy: StrategyId::new(strategy),
        symbol: Symbol::new("BTCUSDT"),
        stance: agent::config::PlanStance::Long,
        latest_signal: Some(agent::config::PlanSignal::Buy),
        rule: agent::config::PlanRuleKind::SmaCross {
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

fn bakeoff_progress(done: u16, total: u16, id: &str) -> BakeoffProgress {
    BakeoffProgress {
        done,
        total,
        current_id: SmolStr::new(id),
    }
}

/// A CHECKED send through the production pair's inner sender.
/// `BakeoffProgressSender::try_send` swallows a full or closed channel by
/// design.
fn send_progress_checked(tx: &BakeoffProgressSender, p: BakeoffProgress) -> Result<(), String> {
    tx.0.as_ref()
        .ok_or_else(|| "bakeoff_progress_pair built a disabled sender".to_owned())?
        .try_send(p)
        .map_err(|e| format!("the progress event did not enter the channel: {e}"))
}

fn sender_is_closed(tx: &BakeoffProgressSender) -> bool {
    tx.0.as_ref()
        .expect("bakeoff_progress_pair builds an enabled sender")
        .is_closed()
}

fn plan_strategy(msg: &Message) -> String {
    match msg {
        Message::ForwardPlanReceived(view) => view.strategy.to_string(),
        other => panic!("expected Message::ForwardPlanReceived, got {other:?}"),
    }
}

fn narration(msg: &Message) -> ui::leaderboard::NarrationOutcome {
    match msg {
        Message::BakeoffNarrationCompleted(outcome) => outcome.clone(),
        other => panic!("expected Message::BakeoffNarrationCompleted, got {other:?}"),
    }
}

fn bakeoff_fields(msg: &Message) -> (u16, u16, String) {
    match msg {
        Message::BakeoffProgress(p) => (p.done, p.total, p.current_id.to_string()),
        other => panic!("expected Message::BakeoffProgress, got {other:?}"),
    }
}

fn sweep_fields(msg: &Message) -> (u16, u16, String) {
    match msg {
        Message::SweepProgress(p) => (p.done, p.total, p.current_id.to_string()),
        other => panic!("expected Message::SweepProgress, got {other:?}"),
    }
}

/// A relayed lab-progress message, projected to something comparable
/// (`Progress` has no `PartialEq`).
#[derive(Debug, PartialEq, Eq)]
enum LabMsg {
    Progress {
        current_bar: usize,
        total_bars: usize,
        elapsed_ms: u64,
    },
    Done,
}

fn lab_msg(msg: &Message) -> LabMsg {
    match msg {
        Message::LabRunProgress(p) => LabMsg::Progress {
            current_bar: p.current_bar,
            total_bars: p.total_bars,
            elapsed_ms: p.elapsed_ms,
        },
        Message::LabRunProgressDone => LabMsg::Done,
        other => panic!("expected LabRunProgress / LabRunProgressDone, got {other:?}"),
    }
}

fn training_line(msg: &Message) -> String {
    match msg {
        Message::TrainingLogLine(text) => text.to_string(),
        other => panic!("expected Message::TrainingLogLine, got {other:?}"),
    }
}

fn trail_updated_id(msg: &Message) -> String {
    match msg {
        Message::TrailMirrorTick(TrailMirrorUiTick::TrailUpdated(id)) => id.to_string(),
        other => panic!("expected Message::TrailMirrorTick(TrailUpdated), got {other:?}"),
    }
}

/// `(id, kind, label, phase)` of an activity event, with the phase as its
/// `Debug` text (`ActivityPhase` has no `PartialEq`).
fn activity_fields(event: &ActivityEvent) -> (u64, ActivityKind, String, String) {
    (
        event.id.0,
        event.kind,
        event.label.clone(),
        format!("{:?}", event.phase),
    )
}

fn activity_event(msg: &Message) -> &ActivityEvent {
    match msg {
        Message::ActivityEventReceived(event) => event,
        other => panic!("expected Message::ActivityEventReceived, got {other:?}"),
    }
}

// ── The bus batch: channels, events, projections ──────────────────────────────

/// The eleven always-on relays, in the order `ui::live::subscription` batches
/// them (`crates/ui/src/live.rs`). iced keeps that order
/// (`Subscription::batch` flat-maps its inputs in order, and `into_recipes`
/// returns that `Vec`), so recipe #i relays `BUS_BATCH[i]`. A reorder in
/// production turns `bus_recipes_relay_off_runtime` red at the first relay
/// that receives nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BusChannel {
    Fills,
    Positions,
    Pnl,
    Ticks,
    Bars,
    Mode,
    StrategyLoaded,
    StrategySwapped,
    StrategyError,
    MarketHealth,
    RiskTelemetry,
}

const BUS_BATCH: [BusChannel; 11] = [
    BusChannel::Fills,
    BusChannel::Positions,
    BusChannel::Pnl,
    BusChannel::Ticks,
    BusChannel::Bars,
    BusChannel::Mode,
    BusChannel::StrategyLoaded,
    BusChannel::StrategySwapped,
    BusChannel::StrategyError,
    BusChannel::MarketHealth,
    BusChannel::RiskTelemetry,
];

fn bus_ts(offset_secs: i64) -> Timestamp {
    Timestamp::new(
        time::OffsetDateTime::from_unix_timestamp(1_700_000_000 + offset_secs)
            .expect("a valid unix time"),
    )
}

fn bus_fill() -> Fill {
    Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new("BTCUSDT"),
        side: Side::Buy,
        qty: Quantity::new(dec!(0.1)).unwrap(),
        price: Price::new(dec!(40_001)).unwrap(),
        fee: Money::from_decimal(dec!(1.6)),
        fee_tier: FeeTier::Taker,
        venue_ts: bus_ts(1),
        local_ts: bus_ts(1),
        liquidity: Liquidity::Taker,
        transaction_id: None,
    }
}

fn bus_position() -> Position {
    Position {
        symbol: Symbol::new("BTCUSDT"),
        base_qty: dec!(0.25),
        cost_basis: Money::from_decimal(dec!(10_000)),
        last_mark: Price::new(dec!(40_050)).unwrap(),
        realized_pnl: Money::from_decimal(dec!(5)),
        unrealized_pnl: Money::from_decimal(dec!(7.5)),
    }
}

fn bus_pnl() -> PnlSnapshot {
    PnlSnapshot {
        cash: Money::from_decimal(dec!(90_000)),
        unrealized: Money::from_decimal(dec!(250)),
        realized: Money::from_decimal(dec!(-120.50)),
        total_equity: Money::from_decimal(dec!(90_129.50)),
        daily_return: Money::from_decimal(dec!(129.50)),
        as_of: bus_ts(2),
        bar_ts: None,
    }
}

fn bus_tick() -> Tick {
    Tick {
        symbol: Symbol::new("BTCUSDT"),
        venue_ts: bus_ts(3),
        local_recv_ts: bus_ts(3),
        price: Price::new(dec!(40_003)).unwrap(),
        qty: Quantity::new(dec!(1)).unwrap(),
        side: Side::Buy,
        trade_id: 4_242,
        venue: Venue::Binance,
    }
}

fn bus_bar() -> Bar {
    let close = Price::new(dec!(50_004)).unwrap();
    Bar {
        symbol: Symbol::new("BTCUSDT"),
        tf: Timeframe::OneMinute,
        open_ts: bus_ts(0),
        close_ts: bus_ts(60),
        open: close,
        high: close,
        low: close,
        close,
        volume: Quantity::new(dec!(1)).unwrap(),
        trade_count: 1,
        local_recv_ts: bus_ts(60),
        venue: Venue::Binance,
    }
}

fn bus_risk_telemetry() -> RiskTelemetry {
    let key = (Venue::Binance, Symbol::new("BTCUSDT"));
    RiskTelemetry {
        per_symbol_exposure: HashMap::from([(key.clone(), dec!(50))]),
        per_symbol_caps: HashMap::from([(key, dec!(100))]),
        daily_loss_used_pct: dec!(20),
        daily_loss_cap_pct: dec!(100),
        heartbeat_age_ms: 250,
        heartbeat_timeout_ms: 30_000,
    }
}

/// The event a CONTROL subscriber (subscribed just before the publish) got:
/// `EventBus::publish_*` discards the send's result, so this is the check
/// that the event entered the channel.
fn entered<T: Clone>(
    mut control: tokio::sync::broadcast::Receiver<T>,
    channel: BusChannel,
) -> Result<T, String> {
    control
        .try_recv()
        .map_err(|e| format!("{channel:?}: the published event did not enter the channel ({e:?})"))
}

/// Publish ONE event on `channel`, checked, and return what its relay must
/// yield for it, projected with [`bus_msg`] from the event the control got.
fn publish_on(bus: &EventBus, channel: BusChannel) -> Result<Vec<String>, String> {
    Ok(match channel {
        BusChannel::Fills => {
            let control = bus.fills();
            bus.publish_fill(bus_fill());
            let f = entered(control, channel)?;
            vec![format!(
                "FillReceived {:?} {:?} {:?} {:?}",
                f.symbol, f.side, f.price, f.qty
            )]
        }
        BusChannel::Positions => {
            let control = bus.positions();
            bus.publish_position(bus_position());
            let p = entered(control, channel)?;
            vec![format!(
                "PositionsRefreshed {:?}",
                vec![(p.symbol, p.base_qty)]
            )]
        }
        BusChannel::Pnl => {
            let control = bus.pnl();
            bus.publish_pnl(bus_pnl());
            let s = entered(control, channel)?;
            vec![format!("PnlRefreshed {:?} {:?}", s.total_equity, s.as_of)]
        }
        BusChannel::Ticks => {
            let control = bus.ticks();
            bus.publish_tick(bus_tick());
            let t = entered(control, channel)?;
            vec![format!("TickReceived {} {:?}", t.trade_id, t.price)]
        }
        BusChannel::Bars => {
            let control = bus.bars();
            bus.publish_bar(bus_bar());
            let b = entered(control, channel)?;
            vec![
                format!("BarReceived {:?} {:?}", b.close_ts, b.close),
                format!("BarClose {:?}", b.close_ts),
            ]
        }
        BusChannel::Mode => {
            let control = bus.mode();
            bus.publish_mode(agent::AgentMode::Halted {
                reason: "off-runtime halt".to_owned(),
            });
            match entered(control, channel)? {
                agent::AgentMode::Halted { reason } => {
                    vec![format!("AgentHaltedExternally {reason}")]
                }
                agent::AgentMode::Running => {
                    return Err(
                        "Mode: the control got Running, not the Halted published".to_owned()
                    );
                }
            }
        }
        BusChannel::StrategyLoaded => {
            let control = bus.strategy_loaded();
            bus.publish_strategy_loaded(StrategyLoaded {
                id: StrategyId::new("v0.sma"),
                hash: [0xAA; 32],
                source_path: SmolStr::new("config/strategies/v0.sma.toml"),
                ts: bus_ts(7),
            });
            let e = entered(control, channel)?;
            vec![format!("StrategyLoaded {:?} {:?}", e.id, e.source_path)]
        }
        BusChannel::StrategySwapped => {
            let control = bus.strategy_swapped();
            bus.publish_strategy_swapped(StrategySwapped {
                id: StrategyId::new("v0.5.rsi"),
                old_hash: [0xAA; 32],
                new_hash: [0xBB; 32],
                source_path: SmolStr::new("config/strategies/v0.5.rsi.toml"),
                ts: bus_ts(8),
            });
            let e = entered(control, channel)?;
            vec![format!("StrategySwapped {:?} {:?}", e.id, e.source_path)]
        }
        BusChannel::StrategyError => {
            let control = bus.strategy_error();
            bus.publish_strategy_error(StrategyLoadError {
                source_path: SmolStr::new("config/strategies/bad.toml"),
                strategy_id: None,
                error_code: SmolStr::new("toml_parse"),
                error_summary: SmolStr::new("unexpected token at line 3"),
                ts: bus_ts(9),
            });
            let e = entered(control, channel)?;
            vec![format!(
                "StrategyLoadError {:?} {:?}",
                e.error_code, e.source_path
            )]
        }
        BusChannel::MarketHealth => {
            let control = bus.market_health();
            bus.publish_market_health(MarketHealth::Stale {
                venue: Venue::Binance,
                last_tick_ts: bus_ts(10),
                threshold_secs: 30,
            });
            let e = entered(control, channel)?;
            vec![format!("MarketHealthUpdated {e:?}")]
        }
        BusChannel::RiskTelemetry => {
            let control = bus.risk_telemetry();
            bus.publish_risk_telemetry(bus_risk_telemetry());
            let t = entered(control, channel)?;
            vec![format!(
                "RiskStateRefreshed {} {} {:?}",
                t.heartbeat_age_ms, t.heartbeat_timeout_ms, t.daily_loss_used_pct
            )]
        }
    })
}

/// What a bus relay must yield once the bus dropped, before its END: the
/// panel-error message of the channels that have one (`live.rs`, each
/// `Err(RecvError::Closed)` arm), nothing for the four that only stop.
fn bus_after_close(channel: BusChannel) -> Vec<String> {
    let closed = ui::strings::CONNECTION_CHANNEL_CLOSED;
    match channel {
        BusChannel::Fills => vec![format!("TapeError {closed}")],
        BusChannel::Positions => vec![format!("PositionsError {closed}")],
        BusChannel::Pnl => vec![format!("PnlError {closed}")],
        BusChannel::Mode => vec![format!("AgentHaltedExternally {closed}")],
        BusChannel::StrategyLoaded | BusChannel::StrategySwapped | BusChannel::StrategyError => {
            vec![format!("StrategiesError {closed}")]
        }
        BusChannel::Ticks
        | BusChannel::Bars
        | BusChannel::MarketHealth
        | BusChannel::RiskTelemetry => Vec::new(),
    }
}

/// A relayed bus message, projected to the fields that tell it apart.
fn bus_msg(msg: &Message) -> String {
    match msg {
        Message::FillReceived(v) => format!(
            "FillReceived {:?} {:?} {:?} {:?}",
            v.symbol, v.side, v.price, v.qty
        ),
        Message::TapeError(e) => format!("TapeError {e}"),
        Message::PositionsRefreshed(views) => format!(
            "PositionsRefreshed {:?}",
            views
                .iter()
                .map(|v| (v.symbol.clone(), v.base_qty))
                .collect::<Vec<_>>()
        ),
        Message::PositionsError(e) => format!("PositionsError {e}"),
        Message::PnlRefreshed(s) => format!("PnlRefreshed {:?} {:?}", s.total_equity, s.as_of),
        Message::PnlError(e) => format!("PnlError {e}"),
        Message::TickReceived(t) => format!("TickReceived {} {:?}", t.trade_id, t.price),
        Message::BarReceived(b) => format!("BarReceived {:?} {:?}", b.close_ts, b.close),
        Message::BarClose(ts) => format!("BarClose {ts:?}"),
        Message::AgentModeChanged(mode) => format!("AgentModeChanged {mode:?}"),
        Message::AgentHaltedExternally(reason) => format!("AgentHaltedExternally {reason}"),
        Message::StrategyLoaded(e) => format!("StrategyLoaded {:?} {:?}", e.id, e.source_path),
        Message::StrategySwapped(e) => format!("StrategySwapped {:?} {:?}", e.id, e.source_path),
        Message::StrategyLoadError(e) => {
            format!("StrategyLoadError {:?} {:?}", e.error_code, e.source_path)
        }
        Message::StrategiesError(e) => format!("StrategiesError {e}"),
        Message::MarketHealthUpdated(e) => format!("MarketHealthUpdated {e:?}"),
        Message::RiskStateRefreshed(s) => format!(
            "RiskStateRefreshed {} {} {:?}",
            s.heartbeat_age_ms, s.heartbeat_timeout_ms, s.daily_loss_used_pct
        ),
        other => format!("UNEXPECTED {other:?}"),
    }
}

// ── Premise controls ──────────────────────────────────────────────────────────

/// **The premise, stand-alone and two-sided.** No tokio context on the libtest
/// thread, on a fresh std thread, or in a task on iced's executor. Then the
/// SAME detector reports a context inside `Handle::enter()` on that executor,
/// and none again once the guard drops. A detector that always said "no
/// context" would fail the second half.
#[test]
fn harness_thread_has_no_tokio_context() {
    let _timing = timing_shared();
    // 1) The libtest thread this test runs on.
    assert!(
        no_tokio_context(),
        "the harness thread has no tokio context"
    );
    assert!(
        std::panic::catch_unwind(tokio::runtime::Handle::current).is_err(),
        "Handle::current() panics on the harness thread"
    );

    // 2) A fresh std thread, read over a channel (bounded) instead of `join`.
    let (tx, rx) = std_mpsc::channel();
    let _fresh = std::thread::spawn(move || {
        let _ = tx.send((
            no_tokio_context(),
            std::panic::catch_unwind(tokio::runtime::Handle::current).is_err(),
        ));
    });
    let fresh = rx
        .recv_timeout(WAIT)
        .expect("the fresh thread reported within 5 s");
    assert_eq!(
        fresh,
        (true, true),
        "a fresh std thread has no tokio context, and Handle::current() panics there"
    );

    // 3) A task on iced's executor.
    let executor = iced_executor();
    let (tx, rx) = std_mpsc::channel();
    iced::Executor::spawn(&executor, async move {
        let _ = tx.send(no_tokio_context());
    });
    assert!(
        rx.recv_timeout(WAIT)
            .expect("the executor task reported within 5 s"),
        "a task on iced's executor has no tokio context"
    );

    // 4) Two-sided: the detector SEES a context where one exists.
    let producer = ProducerRuntime::new();
    let handle = producer.handle();
    let (tx, rx) = std_mpsc::channel();
    iced::Executor::spawn(&executor, async move {
        let inside_guard = {
            let _guard = handle.enter();
            !no_tokio_context()
        };
        let _ = tx.send((inside_guard, no_tokio_context()));
    });
    assert_eq!(
        rx.recv_timeout(WAIT)
            .expect("the entered executor task reported within 5 s"),
        (true, true),
        "inside handle.enter() the detector reports a context; after the guard drops, none"
    );
    assert!(
        no_tokio_context(),
        "the harness thread still has no tokio context"
    );
}

/// **The harness cannot pass a probe's failure shape.** Through the same
/// `poll_on_iced_executor` + `expect_parked` path every recipe test uses:
///
/// - (a) the B-P3..B-P9 shape: a `tokio::time::timeout` around a real tokio
///   `mpsc` receive, created when the stream is polled. It PANICS on the pool;
/// - (b) the B-P1/B-P2 shape: a `tokio::time::interval` created in `stream()`
///   with no runtime entered. It PANICS on the event-loop stand-in;
/// - (c) the other side of (b): the same interval, built inside
///   `handle.enter()`, ticks on the executor;
/// - (g) the B-P17 shape: a stream that buffers until its channel closes.
///   With two sends while the sender stays alive, its first item never comes,
///   so the turn-taking wait fails instead of passing on the close wake. This
///   is the file's one NEGATIVE-CONTROL wait (its expiry is the expected
///   outcome), and it is two-sided: after the 1 s silence the sender drops,
///   and the same `next_item` must then get both buffered items, in order,
///   followed by the END. So the silence was buffering, not a broken reader;
/// - (d) the B-P10/B-P11 shape: a `stream()` that leaks its `EnterGuard`
///   fails at the after-`stream()` premise, and no drain is spawned for it;
/// - (h) the same leak in a BATCH, by its second recipe: the clean first
///   recipe parks, and the failure names recipe #2, because the premise is
///   sampled per recipe;
/// - (e) a stream BODY that leaks a context at its first poll, then builds a
///   `tokio::time::timeout` inside it. Without the per-poll premise that
///   drain would run inside the runtime and pass;
/// - (f) a body that leaks at a LATER poll, after relaying one item cleanly.
///   This is the leak a single end-of-drain sample can miss, because the pool
///   may run the last poll on another worker.
///
/// Each failing shape becomes a test failure that carries its cause: not a
/// hang, not a pass. (a) and (b) go through `expect_parked`'s
/// `PANIC_REPORT_WAIT`, which also has to cover the panic hook. (d) and (h)
/// leak on their own stand-in threads; (e) and (f) leak on pool workers, so
/// each gets an executor of its own, dropped right after.
#[test]
fn a_reactor_dependent_stream_fails_here_instead_of_passing() {
    let _timing = timing_shared();
    let producer = ProducerRuntime::new();
    let executor = iced_executor();

    // (a) the timeout-wrapped receive, built at the first poll.
    let (tx, rx) = tokio::sync::mpsc::channel::<Progress>(8);
    let reports = poll_on_iced_executor(&executor, None, move || {
        futures::stream::unfold(rx, |mut rx| async move {
            match tokio::time::timeout(Duration::from_secs(60), rx.recv()).await {
                Ok(Some(p)) => Some((Message::LabRunProgress(p), rx)),
                _ => None,
            }
        })
        .boxed()
    });
    let failure = harness_failure(|| {
        expect_parked(&reports, "probe shape (a)");
    });
    assert!(
        failure.contains("the stream PANICKED on iced's executor")
            && failure.contains("no reactor running"),
        "(a) the failure names the executor panic and its cause: {failure}"
    );
    drop(tx);

    // (b) the interval built in `stream()` outside the runtime guard.
    let reports = poll_on_iced_executor(&executor, Some(1), || {
        let interval = tokio::time::interval(Duration::from_millis(50));
        futures::stream::unfold(interval, |mut interval| async move {
            interval.tick().await;
            Some((Message::LabRunProgressDone, interval))
        })
        .boxed()
    });
    let failure = harness_failure(|| {
        expect_parked(&reports, "probe shape (b)");
    });
    assert!(
        failure.contains("`Recipe::stream` PANICKED on the event-loop stand-in thread")
            && failure.contains("no reactor running"),
        "(b) the failure names the `stream()` panic and its cause: {failure}"
    );

    // (c) the same interval, built inside the guard, ticks off-runtime.
    let handle = producer.handle();
    let reports = poll_on_iced_executor(&executor, Some(1), move || {
        let interval = {
            let _guard = handle.enter();
            let start = tokio::time::Instant::now() + Duration::from_millis(50);
            tokio::time::interval_at(start, Duration::from_millis(50))
        };
        futures::stream::unfold(interval, |mut interval| async move {
            interval.tick().await;
            Some((Message::LabRunProgressDone, interval))
        })
        .boxed()
    });
    expect_parked(&reports, "(c) the guarded interval");
    let tick = next_item(&reports, WAIT, "(c) the guarded interval's tick");
    assert!(
        matches!(tick.msg, Message::LabRunProgressDone),
        "(c) got {:?}",
        tick.msg
    );
    expect_still_live(&reports, "(c) the guarded interval");

    // (g) a stream that relays only once its channel has closed. This is the
    // file's ONE negative-control wait: its expiry is the expected outcome.
    // It is bounded by its positive half, which follows it: once the sender
    // drops, every buffered item must arrive, in order, and then the END.
    let (feed, source) = futures::channel::mpsc::unbounded::<usize>();
    let reports = poll_on_iced_executor(&executor, None, move || {
        futures::stream::once(source.collect::<Vec<usize>>())
            .flat_map(futures::stream::iter)
            .map(|current_bar| {
                Message::LabRunProgress(Progress {
                    current_bar,
                    total_bars: 2,
                    elapsed_ms: 0,
                })
            })
            .boxed()
    });
    expect_parked(&reports, "probe shape (g)");
    expect_waiting(&reports, "probe shape (g)");
    feed.unbounded_send(1).expect("bar 1 enters the feed");
    feed.unbounded_send(2).expect("bar 2 enters the feed");
    let failure = harness_failure(|| {
        next_item(
            &reports,
            Duration::from_secs(1),
            "probe shape (g), bars 1 and 2, sent with the sender still alive",
        );
    });
    assert!(
        failure.contains("nothing reported within 1s: the stream hung"),
        "(g) the failure names the items that never came while the sender lived: {failure}"
    );
    // The positive half. The silence above was BUFFERING, not a reader that
    // sees nothing: with the sender gone, the SAME reader, `next_item`, gets
    // both bars, in send order, and then the stream ENDS with nothing more
    // and exits cleanly.
    drop(feed);
    let relayed: Vec<LabMsg> = (1..=2)
        .map(|n| {
            let what = format!("probe shape (g), buffered bar {n}, after its sender dropped");
            lab_msg(&next_item(&reports, WAIT, &what).msg)
        })
        .collect();
    let bar = |current_bar| LabMsg::Progress {
        current_bar,
        total_bars: 2,
        elapsed_ms: 0,
    };
    assert_eq!(
        relayed,
        [bar(1), bar(2)],
        "(g) once the sender dropped, the buffered bars arrived in send order"
    );
    let tail = drain_to_end(&reports, "probe shape (g), after its buffered bars");
    assert!(
        tail.is_empty(),
        "(g) after the buffered bars, the END and nothing else: {tail:?}"
    );

    // (d) the B-P10/B-P11 shape: a `stream()` that LEAKS its runtime guard.
    // The stream itself is `pending()`, so only the premise can fire.
    let handle = producer.handle();
    let reports = poll_on_iced_executor(&executor, None, move || {
        std::mem::forget(handle.enter());
        futures::stream::pending::<Message>().boxed()
    });
    let failure = harness_failure(|| {
        expect_parked(&reports, "probe shape (d)");
    });
    assert!(
        failure.contains("`stream()` must leave NO tokio context entered"),
        "(d) the failure names the leaked-guard premise: {failure}"
    );
    assert!(
        matches!(
            reports.recv_timeout(WAIT),
            Err(RecvTimeoutError::Disconnected)
        ),
        "(d) no drain was spawned for the stream whose premise failed: its report channel closed \
         right after StreamBuilt"
    );

    // (h) a BATCH whose second recipe leaks its guard. The premise is sampled
    // per recipe: the clean first recipe parks, and the failure names the
    // second, the one that leaked, not the batch's first.
    let handle = producer.handle();
    let mut batch = poll_batch_on_iced_executor(&executor, 2, None, move || {
        let clean = Built::from(futures::stream::pending::<Message>().boxed());
        std::mem::forget(handle.enter());
        let leaking = Built::from(futures::stream::pending::<Message>().boxed());
        vec![clean, leaking]
    });
    let leaking = batch.pop().expect("two report channels");
    let clean = batch.pop().expect("two report channels");
    expect_parked(&clean, "probe shape (h), recipe #1");
    let failure = harness_failure(|| {
        expect_parked(&leaking, "probe shape (h), recipe #2");
    });
    assert!(
        failure.starts_with(
            "probe shape (h), recipe #2: PREMISE: `stream()` must leave NO tokio context entered"
        ),
        "(h) the failure names the recipe that leaked, #2: {failure}"
    );
    assert!(
        matches!(
            leaking.recv_timeout(WAIT),
            Err(RecvTimeoutError::Disconnected)
        ),
        "(h) no drain was spawned for recipe #2"
    );
    drop(clean);
    drop(leaking);
    drop(executor);

    // (e) the body leaks at its FIRST poll, then builds a timeout that works
    // only because of the leak.
    let leaky = iced_executor();
    let handle = producer.handle();
    let (tx, rx) = tokio::sync::mpsc::channel::<Progress>(8);
    let reports = poll_on_iced_executor(&leaky, None, move || {
        futures::stream::unfold((rx, Some(handle)), |(mut rx, handle)| async move {
            if let Some(handle) = handle {
                std::mem::forget(handle.enter());
            }
            match tokio::time::timeout(Duration::from_secs(60), rx.recv()).await {
                Ok(Some(p)) => Some((Message::LabRunProgress(p), (rx, None))),
                _ => None,
            }
        })
        .boxed()
    });
    let failure = harness_failure(|| {
        expect_parked(&reports, "probe shape (e)");
    });
    assert!(
        failure.contains("poll #1 (it returned Pending) left a tokio context entered"),
        "(e) the failure names the leaking first poll: {failure}"
    );
    drop(tx);
    drop(leaky);

    // (f) the body leaks at a LATER poll, the one that yields bar 2.
    let leaky = iced_executor();
    let handle = producer.handle();
    let (feed, source) = futures::channel::mpsc::unbounded::<usize>();
    let reports = poll_on_iced_executor(&leaky, None, move || {
        source
            .map(move |current_bar| {
                if current_bar == 2 {
                    std::mem::forget(handle.enter());
                }
                Message::LabRunProgress(Progress {
                    current_bar,
                    total_bars: 2,
                    elapsed_ms: 0,
                })
            })
            .boxed()
    });
    expect_parked(&reports, "probe shape (f)");
    feed.unbounded_send(1).expect("bar 1 enters the feed");
    let bar_1 = next_item(&reports, WAIT, "probe shape (f), bar 1");
    assert_eq!(
        lab_msg(&bar_1.msg),
        LabMsg::Progress {
            current_bar: 1,
            total_bars: 2,
            elapsed_ms: 0
        },
        "(f) the clean poll relays bar 1"
    );
    feed.unbounded_send(2).expect("bar 2 enters the feed");
    let failure = harness_failure(|| {
        next_item(&reports, WAIT, "probe shape (f), bar 2");
    });
    assert!(
        failure.contains("(it returned an item) left a tokio context entered")
            && !failure.contains("poll #1 ("),
        "(f) the failure names a LATER poll, the one that yielded the leaking item: {failure}"
    );
    drop(feed);
    drop(leaky);
}

/// **A harness defect is never reported as a production panic.** Two-sided
/// at EACH of the two sites, and through the same
/// `poll_batch_on_iced_executor` + `expect_parked` path every recipe test
/// uses, so at each site only the payload can tell the two apart:
///
/// - on the event-loop stand-in, a real harness self-check fails: the batch
///   hands back one stream fewer than it has report channels. The failure
///   must read HARNESS DEFECT with the `HARNESS:` payload, and must NOT read
///   as `Recipe::stream` panicking;
/// - on the stand-in, a production-shaped panic: `stream()` itself panics.
///   The failure must read as `Recipe::stream` PANICKED (no tokio context),
///   and must NOT read HARNESS;
/// - on iced's executor, a self-check that fails inside a poll must read
///   HARNESS DEFECT "on iced's executor, inside the drain", and a plain panic
///   in a poll must read as the stream PANICKED there, never HARNESS.
///
/// A label keyed on the site instead of the payload fails one half or the
/// other, at either site.
#[test]
fn a_harness_self_check_is_not_reported_as_a_production_panic() {
    let _timing = timing_shared();
    let executor = iced_executor();

    // The harness side: no stream for the one report channel.
    let reports = poll_batch_on_iced_executor(&executor, 1, None, Vec::<Built>::new)
        .pop()
        .expect("one report channel");
    let failure = harness_failure(|| {
        expect_parked(&reports, "a batch one stream short");
    });
    assert!(
        failure.contains("HARNESS DEFECT, not a production failure")
            && failure.contains("HARNESS: `make_streams` built 0 streams for 1 report channels")
            && !failure.contains("PANICKED")
            && !failure.contains("no tokio context"),
        "a failed harness self-check reads as a harness defect, never as a production panic: \
         {failure}"
    );

    // The production side, same site: `stream()` itself panics.
    let reports = poll_on_iced_executor(&executor, None, || -> BoxStream<'static, Message> {
        panic!("a production-shaped panic in stream()")
    });
    let failure = harness_failure(|| {
        expect_parked(&reports, "a panicking stream()");
    });
    assert!(
        failure.contains(
            "`Recipe::stream` PANICKED on the event-loop stand-in thread (no tokio context): \
             a production-shaped panic in stream()"
        ) && !failure.contains("HARNESS"),
        "a production-shaped panic reads as `Recipe::stream` panicking, never as a harness \
         defect: {failure}"
    );

    // The OTHER site, iced's executor. No harness self-check runs inside a
    // drain today, so this half is what keeps that branch of the label
    // tested: a self-check that fails inside a poll...
    let reports = poll_on_iced_executor(&executor, None, || {
        futures::stream::poll_fn(|_| -> Poll<Option<Message>> {
            harness_check(false, || "a self-check that fails inside a poll".to_owned());
            Poll::Pending
        })
        .boxed()
    });
    let failure = harness_failure(|| {
        expect_parked(&reports, "a self-check failing in a poll");
    });
    assert!(
        failure.contains(
            "HARNESS DEFECT, not a production failure: a harness self-check failed on iced's \
             executor, inside the drain: HARNESS: a self-check that fails inside a poll"
        ) && !failure.contains("PANICKED"),
        "a harness self-check failing in a poll reads as a harness defect on the executor: \
         {failure}"
    );

    // ...and a production-shaped panic in a poll, at the same site.
    let reports = poll_on_iced_executor(&executor, None, || {
        futures::stream::poll_fn(|_| -> Poll<Option<Message>> {
            panic!("a production-shaped panic in a poll")
        })
        .boxed()
    });
    let failure = harness_failure(|| {
        expect_parked(&reports, "a panicking poll");
    });
    assert!(
        failure.contains(
            "the stream PANICKED on iced's executor (no tokio context): a production-shaped panic \
             in a poll"
        ) && !failure.contains("HARNESS"),
        "a production-shaped panic in a poll reads as the stream panicking, never as a harness \
         defect: {failure}"
    );
}

/// **The teardown leaves no drain behind.** Dropping a [`Drain`] cancels a
/// LIVE drain, as iced's `_cancel` does, and returns only once the drain's
/// stream is dropped, while the producer runtime is still up. The stream holds
/// a guarded tokio interval (first tick an hour away) and a flag its drop
/// sets. Two-sided: the flag is CLEAR while the drain is parked, and SET the
/// moment `drop` returns. A `Drop` that did not wait fails the second check.
/// One that did not cancel fails inside `drop` ("still running ... after it
/// was cancelled"). Before this teardown, a failing test's drain lived on, and
/// when its runtime shut down its next poll panicked inside tokio's timer. The
/// hook printed that panic after the failure's real cause.
#[test]
fn a_dropped_drain_stops_before_the_runtime_shuts_down() {
    struct SetOnDrop(Arc<AtomicBool>);
    impl Drop for SetOnDrop {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    let _timing = timing_shared();
    let producer = ProducerRuntime::new();
    let executor = iced_executor();
    let dropped = Arc::new(AtomicBool::new(false));
    let flag = SetOnDrop(Arc::clone(&dropped));
    let handle = producer.handle();
    let reports = poll_on_iced_executor(&executor, None, move || {
        let interval = {
            let _guard = handle.enter();
            let hour = Duration::from_secs(3600);
            tokio::time::interval_at(tokio::time::Instant::now() + hour, hour)
        };
        futures::stream::unfold((interval, flag), |(mut interval, flag)| async move {
            interval.tick().await;
            Some((Message::LabRunProgressDone, (interval, flag)))
        })
        .boxed()
    });
    expect_parked(&reports, "a live drain");
    expect_waiting(&reports, "a live drain");
    assert!(
        !dropped.load(Ordering::SeqCst),
        "the parked drain still holds its stream"
    );
    drop(reports);
    assert!(
        dropped.load(Ordering::SeqCst),
        "dropping the Drain cancelled the drain and returned only after its stream was dropped"
    );
    drop(producer);
}

/// **A panic at a LATER poll is reported as the panic, however long its hook
/// runs, and a poll that blocks is not accepted.** The two reports that wait
/// [`PANIC_REPORT_WAIT`] up front (the build, the first poll) cover a panic
/// only there; a later poll is read with a shorter wait. [`recv_report`]
/// closes that gap. Each blocking poll below signals that it began, so its
/// drain is INSIDE the poll when the test's short wait expires, and then
/// stays there for `HOLD`, standing in for a slow panic hook. Two-sided:
///
/// - bar 2's poll blocks, then YIELDS: the expired wait must fail as "one
///   poll ran longer than ..., blocking an executor thread", with the late
///   item named. A grace that accepted any late report would pass it;
/// - bar 3's poll blocks, then PANICS: the failure must be the panic itself,
///   marked as reported after the wait had expired. Without the grace it
///   would read "nothing reported within 100ms: the stream hung", the
///   buffering diagnosis of B-P17, which is wrong here.
///
/// A wait that expires while the drain is PARKED still fails at once: shape
/// (g) of `a_reactor_dependent_stream_fails_here_instead_of_passing`.
#[test]
fn a_late_panic_report_is_reported_as_the_panic() {
    /// The wait that must expire while the poll is still running.
    const SHORT: Duration = Duration::from_millis(100);
    /// How long each blocking poll runs after it signalled: 900 ms of slack
    /// over `SHORT` for the test thread to start its wait.
    const HOLD: Duration = Duration::from_secs(1);

    let _timing = timing_exclusive();
    let executor = iced_executor();
    let (feed, source) = futures::channel::mpsc::unbounded::<usize>();
    let (entered_tx, entered_rx) = std_mpsc::channel::<usize>();
    let reports = poll_on_iced_executor(&executor, None, move || {
        source
            .map(move |current_bar| {
                if current_bar >= 2 {
                    let _ = entered_tx.send(current_bar);
                    std::thread::sleep(HOLD);
                }
                if current_bar == 3 {
                    panic!("a panic at a later poll, the one that relays bar 3");
                }
                Message::LabRunProgress(Progress {
                    current_bar,
                    total_bars: 3,
                    elapsed_ms: 0,
                })
            })
            .boxed()
    });
    expect_parked(&reports, "a late panic");
    feed.unbounded_send(1).expect("bar 1 enters the feed");
    let bar_1 = next_item(&reports, WAIT, "a late panic, bar 1 (a plain poll)");
    assert!(
        matches!(
            bar_1.msg,
            Message::LabRunProgress(Progress { current_bar: 1, .. })
        ),
        "bar 1 is relayed by a plain poll: {:?}",
        bar_1.msg
    );

    // Bar 2: the poll blocks past the wait, then yields.
    feed.unbounded_send(2).expect("bar 2 enters the feed");
    assert_eq!(
        entered_rx.recv_timeout(WAIT),
        Ok(2),
        "the poll that relays bar 2 began"
    );
    let failure = harness_failure(|| {
        next_item(&reports, SHORT, "a late panic, bar 2 (a poll that blocks)");
    });
    assert!(
        failure.contains("one poll ran longer than 100ms, blocking an executor thread")
            && failure.contains("current_bar: 2"),
        "a poll that blocks past the wait fails as one, naming its late item: {failure}"
    );

    // Bar 3: the poll blocks past the wait, then panics.
    feed.unbounded_send(3).expect("bar 3 enters the feed");
    assert_eq!(
        entered_rx.recv_timeout(WAIT),
        Ok(3),
        "the poll that relays bar 3 began"
    );
    let failure = harness_failure(|| {
        next_item(&reports, SHORT, "a late panic, bar 3 (a poll that panics)");
    });
    assert!(
        failure.contains("the stream PANICKED on iced's executor (no tokio context)")
            && failure.contains("a panic at a later poll, the one that relays bar 3")
            && failure.contains("reported after the 100ms wait had expired")
            && !failure.contains("the stream hung"),
        "a panic whose report comes after the wait expired is reported as the panic: {failure}"
    );
    drop(feed);
}

/// **The teardown and lock waits fail when they expire.** Every wait in this
/// file is bounded, including the two that are not a report: the producer
/// runtime's shutdown and the timing lock. Here each one's expiry is forced:
///
/// - a runtime whose shutdown bound is 100 ms, with a blocking task still
///   running on it: dropping it must FAIL, naming the bound;
/// - a LOCAL lock (never `TIMING` itself) held for writing: taking it shared
///   within 50 ms must FAIL; once it is released, the same call takes it.
///
/// The passing side of the shutdown is every other test's teardown.
#[test]
fn a_teardown_or_lock_wait_that_expires_fails_the_test() {
    let _timing = timing_shared();

    let producer = ProducerRuntime::with_shutdown_bound(Duration::from_millis(100));
    let (started_tx, started_rx) = std_mpsc::channel();
    let (release_tx, release_rx) = std_mpsc::channel::<()>();
    drop(producer.rt().spawn_blocking(move || {
        let _ = started_tx.send(());
        let _ = release_rx.recv_timeout(WAIT);
    }));
    started_rx
        .recv_timeout(WAIT)
        .expect("the blocking task started");
    let failure = harness_failure(move || drop(producer));
    assert!(
        failure.starts_with("teardown: the producer runtime's shutdown hit its 100ms bound"),
        "a shutdown that hit its bound fails the test: {failure}"
    );
    drop(release_tx); // the detached blocking task returns at once

    let local = RwLock::new(());
    let held = try_exclusive(&local).expect("a fresh lock is free");
    let failure = harness_failure(|| {
        drop(acquire_within("probe", Duration::from_millis(50), || {
            try_shared(&local)
        }));
    });
    assert!(
        failure.starts_with("timing lock (probe): not taken within 50ms"),
        "a lock that stays taken past the bound fails the test: {failure}"
    );
    drop(held);
    drop(acquire_within("probe", Duration::from_millis(50), || {
        try_shared(&local)
    }));
}

// ── The always-on tick streams (AC4-designated: B-P1, B-P2) ───────────────────
//
// Both tick tests make the same four checks per tick `k` (k = 1, 2), in the
// same order, every bound DERIVED from the documented period `P`:
//
// 1. freshness, future side: the stamp is not later than the sample taken on
//    the same worker right after the poll that yielded it;
// 2. freshness, stale side: that sample is less than `P / 2` past the stamp;
// 3. cadence, lower: no earlier than `k * P` after `stream()` began;
// 4. cadence, upper: the tick is yielded before `(k + 1) * P` after `stream()`
//    began.
//
// One difference is forced by the payload's clock. Toast's stamp is an
// `Instant`, the clock the interval runs on, so (3) is checked on the stamp
// itself. Server time's stamp is wall-clock time, which is not monotonic, so
// (3) is checked on the monotonic sample, and (1)-(2) tie the stamp to that
// sample on the wall clock, across the one `poll_next` return between them.
//
// What (3) and (4) pin together is the window `[P, 1.5 * P)` for the
// interval's period, not the period itself (module doc, the cadence
// paragraph). The exact period is pinned on a paused clock by
// `server_time_recipe_stream` and `toast_dismiss_recipe_stream`. Check (2)
// passes a stamp stale by less than `P / 2`: a stated design limit.
//
// Both tick tests hold the timing lock for WRITING ([`timing_exclusive`]), so
// no other test of this binary, and no deliberate panic, runs beside them.

/// Checks (1) and (2): `stamp_vs_sample` is `Ok(age)` when the stamp is not
/// later than the sample taken right after the poll that yielded it, and
/// `Err(ahead)` when it is. `future` names the clock in the message. An age
/// under `P / 2` passes, by design (the module doc's FRESHNESS limit).
fn assert_stamped_at_yield(
    what: &str,
    period: Duration,
    stamp_vs_sample: Result<Duration, Duration>,
    future: &str,
    shown: &str,
) {
    let half = period / 2;
    match stamp_vs_sample {
        Err(ahead) => panic!(
            "{what}: the payload is stamped AFTER it was yielded ({future}): the stamp is \
             {ahead:?} past the sample taken right after the poll that yielded it ({shown})"
        ),
        Ok(age) => assert!(
            age < half,
            "{what}: the payload must be stamped when it was yielded: the sample taken right after \
             the poll that yielded it is {age:?} past the stamp, half a period ({half:?}) or more \
             ({shown})"
        ),
    }
}

/// Check (3): `at` (`verb`: "stamped" or "yielded"), a time on the monotonic
/// clock, is no earlier than `k` periods after `stream()` began. The interval
/// is built inside `stream()`, after `before_stream`, and tokio never completes
/// a tick before its deadline, so load can only make `at` later: this bound
/// cannot go red on a slow box. A FASTER interval fails it (B-P14, B-P16).
fn assert_no_earlier_than_k_periods(
    what: &str,
    verb: &str,
    k: u32,
    period: Duration,
    before_stream: Instant,
    at: Instant,
) {
    let after_stream = at.saturating_duration_since(before_stream);
    let earliest = period * k;
    assert!(
        after_stream >= earliest,
        "{what}: {verb} {after_stream:?} after `stream()` began, expected no earlier than \
         {earliest:?} (one {period:?} period per tick, the immediate first tick skipped)"
    );
}

/// Check (4): tick `k` is yielded before `k + 1` periods after `stream()`
/// began. A SLOWER interval fails it: one of `2 * P` or more at tick 1, one of
/// `1.5 * P` or more at tick 2, and deterministically, since such an interval
/// cannot complete tick `k` before `before_stream + k * (its period)` (B-P28,
/// B-P29). The slack is one whole period (1 s for server time, 500 ms for
/// toast) for building the stream plus delivering the tick, so load has to
/// delay a single tick by that much to turn it red. The interval's deadlines
/// are fixed at creation (`MissedTickBehavior::Burst`), so a late tick 1 does
/// not push tick 2 later. A period slower by less than half a period passes,
/// BY DESIGN (1.4 s for server time, B-P30; 700 ms for toast, B-P31): that
/// side is the paused-clock siblings'.
fn assert_before_k_plus_one_periods(
    what: &str,
    k: u32,
    period: Duration,
    before_stream: Instant,
    yielded_at: Instant,
) {
    let after_stream = yielded_at.saturating_duration_since(before_stream);
    let latest = period * (k + 1);
    assert!(
        after_stream < latest,
        "{what}: yielded {after_stream:?} after `stream()` began, expected before {latest:?} \
         (tick {k} is due at {k} x {period:?}; one more period is slack): the interval is SLOWER \
         than its documented period"
    );
}

/// **B-P1 (AC4-designated), B-P10, B-P13, B-P14, B-P27, B-P28, B-P34.**
/// `server_time_stream_impl(&handle)`, the whole body of
/// `ui::live::ServerTimeRecipe::stream` (the bin's until story 2-63 Wave C1),
/// is built on the event-loop stand-in and ticks
/// on iced's executor. Each of ticks 1 and 2 passes the four checks above
/// with `P` = 1 s (the interval's immediate first tick is skipped, so tick 1
/// is one full period in): its wall-clock stamp is the time it was yielded,
/// and it is yielded in `[k, k + 1)` periods. Then the stream is still live.
/// That pins the period to the window `[1 s, 1.5 s)`, so a 1.4 s interval
/// (B-P30) is GREEN here by design and RED in `server_time_recipe_stream`.
/// Deleting its `let _guard = rt_handle.enter();` makes the interval panic
/// off-runtime. Under `#[tokio::test]` (`server_time_recipe_stream.rs`) that
/// deletion is invisible, so this test is what makes its K8 claim falsifiable.
#[test]
fn server_time_stream_ticks_off_runtime() {
    let _timing = timing_exclusive();
    let producer = ProducerRuntime::new();
    let handle = producer.handle();
    let executor = iced_executor();

    let reports = poll_on_iced_executor(&executor, Some(2), move || {
        ui::live::server_time_stream_impl(&handle)
    });
    let built = expect_parked(&reports, "server time");

    for k in 1..=2u32 {
        let what = format!("ServerTimeTick #{k}");
        let got = next_item(&reports, WAIT, &what);
        let stamp = match got.msg {
            Message::ServerTimeTick(ts) => ts,
            other => panic!("{what}: expected Message::ServerTimeTick, got {other:?}"),
        };
        let sample_minus_stamp = got.wall_at.inner() - stamp.inner();
        let stamp_vs_sample = if sample_minus_stamp.is_negative() {
            Err(sample_minus_stamp.unsigned_abs())
        } else {
            Ok(sample_minus_stamp.unsigned_abs())
        };
        assert_stamped_at_yield(
            &what,
            SERVER_TIME_PERIOD,
            stamp_vs_sample,
            "a future time",
            &format!(
                "stamp {stamp:?}, wall clock at the sample {:?}",
                got.wall_at
            ),
        );
        assert_no_earlier_than_k_periods(
            &what,
            "yielded",
            k,
            SERVER_TIME_PERIOD,
            built.before_stream,
            got.at,
        );
        assert_before_k_plus_one_periods(&what, k, SERVER_TIME_PERIOD, built.before_stream, got.at);
    }
    expect_still_live(&reports, "server time (always-on)");
}

/// **B-P2 (AC4-designated), B-P15, B-P16, B-P26, B-P29.**
/// `toast_dismiss_stream_impl(&handle)`, the whole body of
/// `ui::live::ToastDismissRecipe::stream` (the bin's until story 2-63 Wave C1),
/// is built on the event-loop stand-in and ticks
/// on iced's executor. Each of ticks 1 and 2 passes the four checks above
/// with `P` = 500 ms: its `Instant` is the time it was yielded, it is stamped
/// no earlier than `k` periods in, and yielded before `k + 1`. Then the stream
/// is still live. That pins the period to the window `[500 ms, 750 ms)`, so a
/// 700 ms interval (B-P31) is GREEN here by design and RED in
/// `toast_dismiss_recipe_stream`.
#[test]
fn toast_dismiss_stream_ticks_off_runtime() {
    let _timing = timing_exclusive();
    let producer = ProducerRuntime::new();
    let handle = producer.handle();
    let executor = iced_executor();

    let reports = poll_on_iced_executor(&executor, Some(2), move || {
        ui::live::toast_dismiss_stream_impl(&handle)
    });
    let built = expect_parked(&reports, "toast dismiss");

    for k in 1..=2u32 {
        let what = format!("ToastTick #{k}");
        let got = next_item(&reports, WAIT, &what);
        let stamp = match got.msg {
            Message::ToastTick(at) => at,
            other => panic!("{what}: expected Message::ToastTick, got {other:?}"),
        };
        let stamp_vs_sample = if stamp <= got.at {
            Ok(got.at - stamp)
        } else {
            Err(stamp - got.at)
        };
        assert_stamped_at_yield(
            &what,
            TOAST_PERIOD,
            stamp_vs_sample,
            "a future instant",
            "both on the monotonic clock",
        );
        assert_no_earlier_than_k_periods(
            &what,
            "stamped",
            k,
            TOAST_PERIOD,
            built.before_stream,
            stamp,
        );
        assert_before_k_plus_one_periods(&what, k, TOAST_PERIOD, built.before_stream, got.at);
    }
    expect_still_live(&reports, "toast dismiss (always-on)");
}

// ── The take-once mpsc recipes ────────────────────────────────────────────────

/// **B-P3, B-P11.** `ForwardPlanRecipe::stream` runs on the event-loop
/// stand-in and takes the receiver there (read right after it returned); its
/// stream, drained on iced's executor, stays live on it. Three plans, sent
/// from the runtime with production's `try_send` (checked) on production's
/// depth-4 channel, one per turn, each relayed before the next is sent. The
/// stream then ends when the last sender drops.
#[test]
fn forward_plan_recipe_drains_off_runtime() {
    let _timing = timing_shared();
    let producer = ProducerRuntime::new();
    let executor = iced_executor();
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::ForwardPlan>(4);
    let cell = Arc::new(Mutex::new(Some(rx)));
    let recipe = ForwardPlanRecipe {
        rt_handle: producer.handle(),
        rx: Arc::clone(&cell),
    };

    let reports = poll_on_iced_executor(&executor, None, move || {
        let stream = Box::new(recipe).stream(empty_event_stream());
        Built::observing(stream, cell_taken(&cell))
    });
    let built = expect_parked(&reports, "forward plan");
    assert_taken(&built, "forward plan");
    assert!(
        !tx.is_closed(),
        "the receiver it took is alive inside the parked stream"
    );

    for id in ["v0.sma", "v0.5.rsi", "v0.macd"] {
        let what = format!("forward plan {id}");
        expect_waiting(&reports, &what);
        let sender = tx.clone();
        producer.produce(&what, async move {
            sender
                .try_send(agent_forward_plan_for(id))
                .map_err(|e| format!("plan {id}: {e}"))
        });
        let got = next_item(&reports, WAIT, &format!("{what}, sent with a sender alive"));
        assert_eq!(
            plan_strategy(&got.msg),
            id,
            "{what}: relayed before the next send, in send order"
        );
    }
    let tail = close_and_drain(&producer, &reports, "forward plan", async move {
        drop(tx);
        Ok(())
    });
    assert!(
        tail.is_empty(),
        "forward plan: after the last sender dropped, the end and nothing else: {tail:?}"
    );
}

/// **B-P4.** `NarrationOutcomeRecipe::stream` on the event-loop stand-in
/// (taking the receiver there), drained on iced's executor. Three outcomes,
/// sent from the runtime with production's `send().await` (checked) on its
/// depth-4 channel, one per turn, each relayed mapped before the next is sent.
/// The stream then ends.
#[test]
fn narration_outcome_recipe_drains_off_runtime() {
    use ui::leaderboard::NarrationOutcome as UiOutcome;

    let _timing = timing_shared();
    let producer = ProducerRuntime::new();
    let executor = iced_executor();
    let (tx, rx) = tokio::sync::mpsc::channel::<agent::NarrationOutcome>(4);
    let cell = Arc::new(Mutex::new(Some(rx)));
    let recipe = NarrationOutcomeRecipe {
        rt_handle: producer.handle(),
        rx: Arc::clone(&cell),
    };

    let reports = poll_on_iced_executor(&executor, None, move || {
        let stream = Box::new(recipe).stream(empty_event_stream());
        Built::observing(stream, cell_taken(&cell))
    });
    let built = expect_parked(&reports, "narration outcome");
    assert_taken(&built, "narration outcome");
    assert!(
        !tx.is_closed(),
        "the receiver it took is alive inside the parked stream"
    );

    let first = "Buy-and-hold won on this era.";
    let third = "No active edge survived the gate.";
    let turns = [
        (
            agent::NarrationOutcome::Ready(SmolStr::new(first)),
            UiOutcome::Ready(SmolStr::new(first)),
        ),
        (agent::NarrationOutcome::FellBack, UiOutcome::FellBack),
        (
            agent::NarrationOutcome::Ready(SmolStr::new(third)),
            UiOutcome::Ready(SmolStr::new(third)),
        ),
    ];
    for (n, (outcome, expected)) in turns.into_iter().enumerate() {
        let what = format!("narration outcome {}", n + 1);
        expect_waiting(&reports, &what);
        let sender = tx.clone();
        producer.produce(&what, async move {
            sender
                .send(outcome)
                .await
                .map_err(|e| format!("narration outcome: {e}"))
        });
        let got = next_item(&reports, WAIT, &format!("{what}, sent with a sender alive"));
        assert_eq!(
            narration(&got.msg),
            expected,
            "{what}: relayed once, mapped, before the next send"
        );
    }
    let tail = close_and_drain(&producer, &reports, "narration outcome", async move {
        drop(tx);
        Ok(())
    });
    assert!(
        tail.is_empty(),
        "narration outcome: after the last sender dropped, the end and nothing else: {tail:?}"
    );
}

/// **B-P5, B-P17.** `BakeoffProgressRecipe::stream` (salt 1) on the event-loop
/// stand-in, drained on iced's executor, over the production
/// `bakeoff_progress_pair`. Three candidate events, sent CHECKED through the
/// pair's inner sender, one per turn, each arriving as
/// `Message::BakeoffProgress` with every field before the next is sent: the
/// progress bar moves while the run is still going. The stream then ends.
#[test]
fn bakeoff_progress_recipe_drains_off_runtime() {
    let _timing = timing_shared();
    let producer = ProducerRuntime::new();
    let executor = iced_executor();
    let (tx, rx) = bakeoff_progress_pair();
    let cell = Arc::new(Mutex::new(Some(rx)));
    let recipe = BakeoffProgressRecipe {
        rt_handle: producer.handle(),
        rx: Arc::clone(&cell),
        salt: 1,
    };

    let reports = poll_on_iced_executor(&executor, None, move || {
        let stream = Box::new(recipe).stream(empty_event_stream());
        Built::observing(stream, cell_taken(&cell))
    });
    let built = expect_parked(&reports, "bakeoff progress");
    assert_taken(&built, "bakeoff progress");
    assert!(
        !sender_is_closed(&tx),
        "the receiver it took is alive inside the parked stream"
    );

    for (done, id) in [(0, "v0.buyhold"), (1, "v0.sma"), (2, "v0.5.macd")] {
        let what = format!("bakeoff progress {done} of 3 ({id})");
        expect_waiting(&reports, &what);
        let sender = tx.clone();
        producer.produce(&what, async move {
            send_progress_checked(&sender, bakeoff_progress(done, 3, id))
        });
        let got = next_item(&reports, WAIT, &format!("{what}, sent with a sender alive"));
        assert_eq!(
            bakeoff_fields(&got.msg),
            (done, 3, id.to_owned()),
            "{what}: relayed as BakeoffProgress before the next send"
        );
    }
    let tail = close_and_drain(&producer, &reports, "bakeoff progress", async move {
        drop(tx);
        Ok(())
    });
    assert!(
        tail.is_empty(),
        "bakeoff progress: after the last sender dropped, the end and nothing else: {tail:?}"
    );
}

/// **B-P6.** `SweepProgressRecipe::stream` (salt 1) on the event-loop
/// stand-in, drained on iced's executor: the same hop as the bake-off, one
/// event per turn, but every event must arrive as `Message::SweepProgress`,
/// never as the bake-off sibling's variant.
#[test]
fn sweep_progress_recipe_drains_off_runtime() {
    let _timing = timing_shared();
    let producer = ProducerRuntime::new();
    let executor = iced_executor();
    let (tx, rx) = bakeoff_progress_pair();
    let cell = Arc::new(Mutex::new(Some(rx)));
    let recipe = SweepProgressRecipe {
        rt_handle: producer.handle(),
        rx: Arc::clone(&cell),
        salt: 1,
    };

    let reports = poll_on_iced_executor(&executor, None, move || {
        let stream = Box::new(recipe).stream(empty_event_stream());
        Built::observing(stream, cell_taken(&cell))
    });
    let built = expect_parked(&reports, "sweep progress");
    assert_taken(&built, "sweep progress");
    assert!(
        !sender_is_closed(&tx),
        "the receiver it took is alive inside the parked stream"
    );

    for (done, id) in [
        (0, "fast=10, slow=30"),
        (1, "fast=15, slow=40"),
        (2, "fast=20, slow=50"),
    ] {
        let what = format!("sweep progress {done} of 3 ({id})");
        expect_waiting(&reports, &what);
        let sender = tx.clone();
        producer.produce(&what, async move {
            send_progress_checked(&sender, bakeoff_progress(done, 3, id))
        });
        let got = next_item(&reports, WAIT, &format!("{what}, sent with a sender alive"));
        assert_eq!(
            sweep_fields(&got.msg),
            (done, 3, id.to_owned()),
            "{what}: relayed as SweepProgress before the next send"
        );
    }
    let tail = close_and_drain(&producer, &reports, "sweep progress", async move {
        drop(tx);
        Ok(())
    });
    assert!(
        tail.is_empty(),
        "sweep progress: after the last sender dropped, the end and nothing else: {tail:?}"
    );
}

/// **B-P7, B-P18.** `LabProgressRecipe::stream` (salt 1) on the event-loop
/// stand-in (taking the receiver there), drained on iced's executor. Three bar
/// events, one per turn, each relayed before the next is sent; then
/// `LabRunProgressDone` when the last sender drops, then the end.
///
/// The channel is a raw `tokio::sync::mpsc::channel(8)`, the same type and
/// capacity `progress_pair()` builds, because `ProgressSender`'s inner sender
/// is private and its `try_send` swallows a failed send. The checked
/// `send().await` is the only way to prove each event entered the channel.
#[test]
fn lab_progress_recipe_drains_off_runtime() {
    let _timing = timing_shared();
    let producer = ProducerRuntime::new();
    let executor = iced_executor();
    let (tx, rx) = tokio::sync::mpsc::channel::<Progress>(8);
    let cell = Arc::new(Mutex::new(Some(rx)));
    let recipe = LabProgressRecipe {
        rt_handle: producer.handle(),
        rx: Arc::clone(&cell),
        salt: 1,
    };

    let reports = poll_on_iced_executor(&executor, None, move || {
        let stream = Box::new(recipe).stream(empty_event_stream());
        Built::observing(stream, cell_taken(&cell))
    });
    let built = expect_parked(&reports, "lab progress");
    assert_taken(&built, "lab progress");
    assert!(
        !tx.is_closed(),
        "the receiver it took is alive inside the parked stream"
    );

    for (current_bar, elapsed_ms) in [(100, 1_500), (360, 4_000), (719, 9_250)] {
        let what = format!("lab progress bar {current_bar}");
        expect_waiting(&reports, &what);
        let sender = tx.clone();
        producer.produce(&what, async move {
            sender
                .send(Progress {
                    current_bar,
                    total_bars: 720,
                    elapsed_ms,
                })
                .await
                .map_err(|e| format!("lab progress: {e}"))
        });
        let got = next_item(&reports, WAIT, &format!("{what}, sent with a sender alive"));
        assert_eq!(
            lab_msg(&got.msg),
            LabMsg::Progress {
                current_bar,
                total_bars: 720,
                elapsed_ms,
            },
            "{what}: relayed before the next send"
        );
    }
    let tail: Vec<LabMsg> = close_and_drain(&producer, &reports, "lab progress", async move {
        drop(tx);
        Ok(())
    })
    .iter()
    .map(lab_msg)
    .collect();
    assert_eq!(
        tail,
        [LabMsg::Done],
        "lab progress: after the last sender dropped, LabRunProgressDone, then the end"
    );
}

/// **B-P20, B-P21, B-P22.** `TrainingLogRecipe::stream` (salt 1) on the
/// event-loop stand-in, taking the std receiver there (read right after it
/// returned) and starting its bridge thread; the line stream is drained on
/// iced's executor. The channel is production's: a std `sync_channel(256)`
/// (`cockpit_live.rs`), written by tasks on the runtime like the trainer's
/// stdout/stderr readers (`trainer::spawn_training_run`). Three lines, one per
/// turn, sent CHECKED (`SyncSender::send` returns the failure), each relayed
/// before the next is sent. The stream then ends when the last sender drops.
///
/// The lib's own `recipe_stream_drains_without_a_tokio_runtime` builds and
/// polls on one thread and checks the premise only before `stream()`; this
/// one separates the two threads and checks after `stream()` and around every
/// poll, which is what B-P21/B-P22 need.
#[test]
fn training_log_recipe_drains_off_runtime() {
    let _timing = timing_shared();
    let producer = ProducerRuntime::new();
    let executor = iced_executor();
    let (tx, rx) = std::sync::mpsc::sync_channel::<TrainingLogLine>(256);
    let cell = Arc::new(Mutex::new(Some(rx)));
    let recipe = TrainingLogRecipe {
        rt_handle: producer.handle(),
        rx: Arc::clone(&cell),
        salt: 1,
    };

    let reports = poll_on_iced_executor(&executor, None, move || {
        let stream = Box::new(recipe).stream(empty_event_stream());
        Built::observing(stream, cell_taken(&cell))
    });
    let built = expect_parked(&reports, "training log");
    assert_taken(&built, "training log");

    for (text, is_stderr) in [
        ("epoch 1/3 loss=0.91", false),
        ("warning: learning rate clipped", true),
        ("epoch 3/3 loss=0.42", false),
    ] {
        let what = format!("training log line {text:?}");
        expect_waiting(&reports, &what);
        let sender = tx.clone();
        producer.produce(&what, async move {
            sender
                .send(TrainingLogLine {
                    text: SmolStr::new(text),
                    is_stderr,
                })
                .map_err(|e| format!("training log line {text:?}: {e}"))
        });
        let got = next_item(&reports, WAIT, &format!("{what}, sent with a sender alive"));
        assert_eq!(
            training_line(&got.msg),
            text,
            "{what}: relayed before the next send"
        );
    }
    let tail = close_and_drain(&producer, &reports, "training log", async move {
        drop(tx);
        Ok(())
    });
    assert!(
        tail.is_empty(),
        "training log: after the last sender dropped, the end and nothing else: {tail:?}"
    );
}

// ── The broadcast recipes ─────────────────────────────────────────────────────

/// **B-P8, B-P12, B-P37.** The trail-mirror recipe, built by its production
/// constructor `trail_mirror_subscription` (the struct is private) and
/// unpacked with iced's `into_recipes`, all on the event-loop stand-in, as
/// iced does. First, on the test thread, the same constructor must yield
/// exactly ONE recipe: a production fact, asserted as one (B-P37: a
/// constructor that returns `Subscription::none()` fails there, not as a
/// harness defect). `stream()` subscribes EAGERLY: the receiver count is read on
/// that thread right after `stream()` returned, before the drain is even
/// spawned, and its receiver is the only one. Three ticks, one per turn, each
/// sent CHECKED to exactly that receiver from the runtime and relayed before
/// the next. The stream then ends when the last sender drops.
#[test]
fn trail_mirror_recipe_drains_off_runtime() {
    let _timing = timing_shared();
    let producer = ProducerRuntime::new();
    let executor = iced_executor();
    let (tick_tx, initial_rx) = tokio::sync::broadcast::channel::<TrailMirrorTick>(16);
    drop(initial_rx);
    let (req_tx, _req_rx) = tokio::sync::mpsc::channel(8);
    let handle = TrailMirrorHandle {
        req_tx,
        tick_tx: tick_tx.clone(),
    };
    // A sender clone, only to read the count; it drops when `make_stream` returns.
    let count_probe = tick_tx.clone();

    // A PRODUCTION fact, checked on the test thread before anything is
    // streamed, so a failure reads as what it is: the constructor's batch.
    let recipes = tracker_ids(into_recipes(ui::live::trail_mirror_subscription(
        handle.clone(),
    )));
    assert_eq!(
        recipes.len(),
        1,
        "`ui::live::trail_mirror_subscription` built {} recipes, expected exactly one. Either \
         production changed (the cockpit lost its trail-mirror relay, or it now has several), or \
         this test's expectation is stale: read `live.rs` before editing the test",
        recipes.len()
    );

    let reports = poll_on_iced_executor(&executor, None, move || {
        let mut recipes = into_recipes(ui::live::trail_mirror_subscription(handle));
        harness_check(recipes.len() == 1, || {
            format!(
                "the trail-mirror constructor built {} recipes here, but one on the test thread",
                recipes.len()
            )
        });
        let stream = recipes
            .pop()
            .unwrap_or_else(|| panic!("{HARNESS} the one trail-mirror recipe"))
            .stream(empty_event_stream());
        Built::observing(stream, count_probe.receiver_count())
    });
    let built = expect_parked(&reports, "trail mirror");
    assert_eq!(
        built.observed_after_stream,
        Some(1),
        "`Recipe::stream` subscribed EAGERLY: right after it returned, before the first poll, its \
         receiver was the only one (production: subscribe \"before the stream is first polled\" \
         to avoid the publish-before-subscribe race)"
    );

    for id in ["audit-a", "audit-b", "audit-c"] {
        let what = format!("trail tick {id}");
        expect_waiting(&reports, &what);
        let sender = tick_tx.clone();
        producer.produce(&what, async move {
            let reached = sender
                .send(TrailMirrorTick::TrailUpdated(id.to_owned()))
                .map_err(|_| format!("tick {id}: no receiver"))?;
            if reached == 1 {
                Ok(())
            } else {
                Err(format!(
                    "tick {id} reached {reached} receivers, expected exactly the recipe's"
                ))
            }
        });
        let got = next_item(&reports, WAIT, &format!("{what}, sent with a sender alive"));
        assert_eq!(
            trail_updated_id(&got.msg),
            id,
            "{what}: relayed before the next send"
        );
    }
    let tail = close_and_drain(&producer, &reports, "trail mirror", async move {
        drop(tick_tx);
        Ok(())
    });
    assert!(
        tail.is_empty(),
        "trail mirror: after the last sender dropped, the end and nothing else: {tail:?}"
    );
}

/// One turn of the activity test: wait until the stream is parked, run ONE
/// producer step on the runtime, check the CONTROL subscriber got exactly one
/// event from it (the production send discards its result), and require the
/// recipe to relay that same event before the next step. Returns what the
/// step handed back and the event.
fn activity_turn<T, F>(
    producer: &ProducerRuntime,
    reports: &Drain,
    control: &mut tokio::sync::broadcast::Receiver<ActivityEvent>,
    what: &str,
    step: F,
) -> (T, (u64, ActivityKind, String, String))
where
    T: Send + 'static,
    F: Future<Output = Result<T, String>> + Send + 'static,
{
    expect_waiting(reports, what);
    let handed_back = producer.produce(what, step);
    let sent = control
        .try_recv()
        .unwrap_or_else(|e| panic!("{what}: control: the event did not enter the channel ({e:?})"));
    assert!(
        matches!(
            control.try_recv(),
            Err(tokio::sync::broadcast::error::TryRecvError::Empty)
        ),
        "{what}: control: the step emitted exactly one event"
    );
    let got = next_item(reports, WAIT, &format!("{what}, sent with a sender alive"));
    assert_eq!(
        activity_fields(activity_event(&got.msg)),
        activity_fields(&sent),
        "{what}: the recipe relayed the event the control got, before the next step"
    );
    (handed_back, activity_fields(&sent))
}

/// **B-P9.** `ActivityRecipe::stream` on the event-loop stand-in, drained on
/// iced's executor, over a real `EventBus`. Two activities are run on the
/// runtime through the production API, one step per turn:
/// `ActivitySender::start` (emits `Start`), then `fail`/`cancel` and the
/// handle's drop (emits `End`). Those sends discard their result
/// (`let _ = self.0.send(...)` in `agent/src/activity.rs`), so a CONTROL
/// subscriber checks each step's one event. The stream relays each event
/// before the next step, in order, then ends once the bus and every sender are
/// dropped; the control then reads `Closed`.
#[test]
fn activity_recipe_drains_off_runtime() {
    let _timing = timing_shared();
    let producer = ProducerRuntime::new();
    let executor = iced_executor();
    let bus = Arc::new(EventBus::new(&BusConfig::default()));
    let mut control = bus.activity().subscribe();
    let sender = bus.activity();
    let recipe = ActivityRecipe {
        bus: Arc::clone(&bus),
    };

    let reports = poll_on_iced_executor(&executor, None, move || {
        Box::new(recipe).stream(empty_event_stream())
    });
    expect_parked(&reports, "activity");

    let starter = sender.clone();
    let (run, lab_start) = activity_turn(
        &producer,
        &reports,
        &mut control,
        "activity: start the lab run",
        async move { Ok(starter.start(ActivityKind::LabRun, "off-runtime lab run")) },
    );
    let ((), lab_end) = activity_turn(
        &producer,
        &reports,
        &mut control,
        "activity: fail and drop the lab run",
        async move {
            run.fail("probe failure");
            drop(run);
            Ok(())
        },
    );
    let starter = sender.clone();
    let (preload, preload_start) = activity_turn(
        &producer,
        &reports,
        &mut control,
        "activity: start the preload",
        async move { Ok(starter.start(ActivityKind::YahooPreload, "off-runtime preload")) },
    );
    let ((), preload_end) = activity_turn(
        &producer,
        &reports,
        &mut control,
        "activity: cancel and drop the preload",
        async move {
            preload.cancel();
            drop(preload);
            Ok(())
        },
    );

    let sent = [lab_start, lab_end, preload_start, preload_end];
    let without_id: Vec<(ActivityKind, String, String)> = sent
        .iter()
        .map(|(_, kind, label, phase)| (*kind, label.clone(), phase.clone()))
        .collect();
    let expected = vec![
        (
            ActivityKind::LabRun,
            "off-runtime lab run".to_owned(),
            "Start { total_units: None }".to_owned(),
        ),
        (
            ActivityKind::LabRun,
            "off-runtime lab run".to_owned(),
            "End(Failed(\"probe failure\"))".to_owned(),
        ),
        (
            ActivityKind::YahooPreload,
            "off-runtime preload".to_owned(),
            "Start { total_units: None }".to_owned(),
        ),
        (
            ActivityKind::YahooPreload,
            "off-runtime preload".to_owned(),
            "End(Cancelled)".to_owned(),
        ),
    ];
    assert_eq!(
        without_id, expected,
        "control: the four production events, one per step"
    );
    assert!(
        sent[0].0 == sent[1].0 && sent[2].0 == sent[3].0 && sent[0].0 != sent[2].0,
        "control: one id per activity, distinct across the two: {sent:?}"
    );

    let tail = close_and_drain(&producer, &reports, "activity", async move {
        drop(sender);
        drop(bus);
        Ok(())
    });
    assert!(
        tail.is_empty(),
        "activity: after the bus and every sender dropped, the end and nothing else: {tail:?}"
    );
    assert!(
        matches!(
            control.try_recv(),
            Err(tokio::sync::broadcast::error::TryRecvError::Closed)
        ),
        "control: the channel closed once the bus and every sender dropped"
    );
}

/// **B-P19.** `ActivityRecipe::stream` subscribes EAGERLY, before the stream
/// is first polled (its comment: "to avoid the publish-before-subscribe
/// race"). `ActivitySender` exposes no receiver count, so the race itself is
/// staged: an activity is started AND its handle dropped on the event-loop
/// stand-in right after `stream()` returned, before the drain is spawned. That
/// publishes two events, `Start` and the drop's `End(Success)`
/// (`ActivityHandle::drop`). A subscribe moved into the body would subscribe
/// at the first poll and lose both. Here the first poll must yield the
/// `Start`, the next poll the buffered `End`, in that order and exactly as a
/// CONTROL subscriber got them. Then the stream parks with nothing more, and it
/// ends once the bus is dropped. `activity_recipe_drains_off_runtime` cannot
/// see the race, because its first send follows the first poll.
#[test]
fn activity_recipe_subscribes_before_its_first_poll() {
    let _timing = timing_shared();
    let executor = iced_executor();
    let bus = Arc::new(EventBus::new(&BusConfig::default()));
    let mut control = bus.activity().subscribe();
    let sender = bus.activity();
    let recipe = ActivityRecipe {
        bus: Arc::clone(&bus),
    };
    let label = "started after stream() returned, before the first poll";

    // `sender` moves into the closure and drops with it, so once the stream
    // is built the test's `bus` is the activity channel's last sender.
    let reports = poll_on_iced_executor(&executor, None, move || {
        let stream = Box::new(recipe).stream(empty_event_stream());
        drop(sender.start(ActivityKind::LabRun, label));
        Built::from(stream)
    });
    let what = "activity, two events between stream() and the first poll";
    let first = expect_first_poll_item(&reports, what);
    let start = control
        .try_recv()
        .expect("control: the early Start entered the channel");
    let end = control
        .try_recv()
        .expect("control: the handle's drop published its End");
    assert!(
        matches!(
            control.try_recv(),
            Err(tokio::sync::broadcast::error::TryRecvError::Empty)
        ),
        "control: starting and dropping one activity published exactly two events"
    );
    let early = |phase: &str| (ActivityKind::LabRun, label.to_owned(), phase.to_owned());
    let without_id = |e: &ActivityEvent| {
        let (_, kind, label, phase) = activity_fields(e);
        (kind, label, phase)
    };
    assert_eq!(
        [without_id(&start), without_id(&end)],
        [early("Start { total_units: None }"), early("End(Success)")],
        "control: the early Start, then the drop's End(Success)"
    );
    assert_eq!(
        start.id.0, end.id.0,
        "control: both events carry the one activity's id"
    );
    assert_eq!(
        activity_fields(activity_event(&first)),
        activity_fields(&start),
        "{what}: the first poll yielded the first event published before it, the Start"
    );
    let second = next_item(
        &reports,
        WAIT,
        &format!("{what}, the second (End), buffered before the first poll as well"),
    );
    assert_eq!(
        activity_fields(activity_event(&second.msg)),
        activity_fields(&end),
        "{what}: the eagerly subscribed stream relayed the End next, the one the control got"
    );

    expect_waiting(&reports, &format!("{what}, after both events"));
    drop(bus);
    let tail = drain_to_end(&reports, what);
    assert!(
        tail.is_empty(),
        "{what}: after the bus dropped, the end and nothing else: {tail:?}"
    );
    assert!(
        matches!(
            control.try_recv(),
            Err(tokio::sync::broadcast::error::TryRecvError::Closed)
        ),
        "control: the channel closed once the bus dropped"
    );
}

// ── The always-on bus relays (`ui::live::subscription`) ───────────────────────

/// One relay of the bus batch, as `bus_recipes_relay_off_runtime` tracks it.
struct BusRelay {
    drain: Drain,
    channel: BusChannel,
    label: String,
    /// The last report read from it was `Waiting`: it is parked on its source
    /// with nothing buffered, so the next item reaches it only through a wake.
    parked: bool,
}

/// Every publish so far, in turn order: its channel, and what that channel's
/// relay had to yield for it (projected with [`bus_msg`]).
type Published = Vec<(BusChannel, Vec<String>)>;

/// The turn (from 0) and channel of the publish that produced `got`, if a
/// channel OTHER than `own` produced it.
fn foreign_publish(
    own: BusChannel,
    got: &str,
    published: &Published,
) -> Option<(usize, BusChannel)> {
    published
        .iter()
        .enumerate()
        .find(|(_, (channel, wants))| *channel != own && wants.iter().any(|w| w == got))
        .map(|(turn, (channel, _))| (turn, *channel))
}

/// `relay` yielded `got` (projected) where none of its own events was due.
/// Fails, naming the relay's channel AND the channel whose publish the item
/// came from.
fn cross_talk(relay: &BusRelay, got: &str, published: &Published, when: &str) -> ! {
    let own = relay.channel;
    match foreign_publish(own, got, published) {
        Some((turn, source)) => panic!(
            "{}: CROSS-TALK: the {own:?} relay yielded `{got}` {when}. That is the {source:?} \
             channel's event (the publish of turn #{}), not one of the {own:?} channel's: the \
             {own:?} relay receives the {source:?} channel",
            relay.label,
            turn + 1
        ),
        None => {
            let duplicate = published
                .iter()
                .any(|(channel, wants)| *channel == own && wants.iter().any(|w| w == got));
            let cause = if got.starts_with("UNEXPECTED ") {
                "a message of a variant that `bus_msg` does not project: no bus relay should yield \
                 it, so its source cannot be named here"
            } else if duplicate {
                "a DUPLICATE of an event of its own channel that it has already relayed"
            } else {
                "an event that no publish so far produced (not an event of another channel that \
                 has published)"
            };
            panic!(
                "{}: the {own:?} relay yielded `{got}` {when}, where none of its own events was \
                 due: {cause}",
                relay.label
            )
        }
    }
}

/// A report read while `relay` should be parked: `Waiting` marks it parked;
/// an `Item` is cross-talk (nothing of its own is due `when`); anything else
/// fails.
fn absorb_while_parked(relay: &mut BusRelay, report: Report, published: &Published, when: &str) {
    match report {
        Report::Waiting => relay.parked = true,
        Report::Item { msg, .. } => cross_talk(relay, &bus_msg(&msg), published, when),
        other => panic!(
            "{}: expected it parked on its source {when}, got {other:?}",
            relay.label
        ),
    }
}

/// Read every report ALREADY queued for `relay`, without waiting, through
/// [`interpret`] (a panic or premise report fails there) and
/// [`absorb_while_parked`].
fn sweep_bus_relay(relay: &mut BusRelay, published: &Published, when: &str) {
    loop {
        let got = match relay.drain.try_recv() {
            Ok(report) => Ok(report),
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(RecvTimeoutError::Disconnected),
        };
        let report = interpret(got, WAIT, &relay.label);
        absorb_while_parked(relay, report, published, when);
    }
}

/// `relay` is PARKED on its source with nothing buffered: sweep what is
/// queued, and if no `Waiting` was seen since its last item, wait (bounded)
/// for one.
fn expect_bus_relay_parked(relay: &mut BusRelay, published: &Published, when: &str) {
    sweep_bus_relay(relay, published, when);
    if !relay.parked {
        let report = next_report(&relay.drain, WAIT, &format!("{}, {when}", relay.label));
        absorb_while_parked(relay, report, published, when);
    }
}

/// **B-P23, B-P24, B-P25, B-P32, B-P33, B-P35, B-P36.** The whole bus batch,
/// as the cockpit builds it: `ui::live::subscription(bus)` over a real
/// `agent::EventBus`, unpacked with iced's `into_recipes`, all eleven
/// `BusRecipe::stream` calls inside ONE `iced::Executor::enter` on the
/// event-loop stand-in, then eleven drains on iced's executor.
///
/// Before that, on the test thread, two PRODUCTION facts about the same
/// batch, each a plain assert whose message names both possible causes: it
/// holds exactly the eleven relays of `BUS_BATCH` (B-P36: a deleted relay
/// fails here, not as a harness defect), and the eleven ids iced's Tracker
/// computes ([`tracker_ids`]) are pairwise distinct, so iced would stream
/// every one (B-P35: a `BusRecipe::hash` that ignores the channel fails here;
/// iced would stream only Fills).
///
/// Each relay is first parked. Then, one channel per turn:
///
/// 1. that relay's queue is drained first. Anything it yielded BEFORE its own
///    channel's publish is CROSS-TALK, reported with both channels: its own,
///    and the one whose publish the item came from. Then it must be parked;
/// 2. ONE event is published from the runtime, checked by a control
///    subscriber;
/// 3. that relay must yield it (bars: `BarReceived` then `BarClose`) before
///    the next channel's publish. Another channel's event in its place is
///    cross-talk too;
/// 4. the OTHER ten relays' queues are swept: an item in any of them is
///    cross-talk (or a duplicate), since no publish of theirs is due.
///
/// So a queued `Waiting` can no longer stand in for a fresh one with a
/// foreign item behind it. Right after the batch is built, the test holds
/// the only handle to the bus: every `stream()` let go of it, as its comment
/// says. Dropping that handle closes every channel: each relay yields its
/// closing message, where it has one, then ends. A foreign event in that tail
/// is reported as cross-talk too, so one that arrives after every sweep is
/// still named. `EventBus` exposes no receiver count, so cross-talk is
/// detected by what the relays YIELD, not by what they subscribe.
///
/// The sweeps do not wait (`try_recv`), so WHICH check names a foreign item
/// depends on when it lands (round 5 measured B-P32 at "before its own
/// channel's publish", where the emulation had hit "after turn #1"). Whether
/// it is named does not: every yielded item is read by one of the sweeps, the
/// wait for the relay's own event, the check before the bus drops, or the
/// tail. The projection [`bus_msg`] compares a subset of fields (a fill's fee,
/// ids and timestamps are not projected), so "the exact relayed list" here
/// means the projected strings; the full mapping is the siblings'.
#[test]
fn bus_recipes_relay_off_runtime() {
    let _timing = timing_shared();
    let producer = ProducerRuntime::new();
    let executor = iced_executor();
    let bus = Arc::new(EventBus::new(&BusConfig::default()));

    // PRODUCTION facts, checked on the test thread over a batch built the
    // same way, before anything is streamed: how many relays the cockpit
    // batches, and that iced's Tracker would stream every one of them (its
    // identity step, which the harness does not mirror: pairwise-distinct
    // ids). These are plain asserts, never harness self-checks: a failure
    // here is a production regression until shown otherwise.
    let ids = tracker_ids(into_recipes(ui::live::subscription(Arc::clone(&bus))));
    assert_eq!(
        ids.len(),
        BUS_BATCH.len(),
        "`ui::live::subscription` batched {} recipes, but this test's table `BUS_BATCH` lists {} \
         channels. Either production changed (a relay the cockpit no longer gets, or a new one \
         with no test here), or the table is stale: read `ui::live::subscription` in `live.rs` \
         BEFORE editing `BUS_BATCH`",
        ids.len(),
        BUS_BATCH.len()
    );
    for (i, id) in ids.iter().enumerate() {
        if let Some(j) = ids[..i].iter().position(|earlier| earlier == id) {
            panic!(
                "IDENTITY: bus recipe #{} ({:?}) has the same iced id as bus recipe #{} ({:?}), \
                 {id:#018x} (`Hasher::default()` + `Recipe::hash`, as `Tracker::update` computes \
                 it). iced streams only the first recipe of an id, so the cockpit would never get \
                 the {:?} relay: `BusRecipe::hash` must tell the channels apart",
                i + 1,
                BUS_BATCH[i],
                j + 1,
                BUS_BATCH[j],
                BUS_BATCH[i]
            );
        }
    }

    let for_recipes = Arc::clone(&bus);
    let drains = poll_batch_on_iced_executor(&executor, BUS_BATCH.len(), None, move || {
        into_recipes(ui::live::subscription(for_recipes))
            .into_iter()
            .map(|recipe| Built::from(recipe.stream(empty_event_stream())))
            .collect()
    });
    let mut relays: Vec<BusRelay> = drains
        .into_iter()
        .zip(BUS_BATCH)
        .enumerate()
        .map(|(i, (drain, channel))| BusRelay {
            drain,
            channel,
            label: format!(
                "bus recipe #{} ({channel:?}, by the batch order of `ui::live::subscription`)",
                i + 1
            ),
            parked: false,
        })
        .collect();
    for relay in &relays {
        expect_parked(&relay.drain, &relay.label);
    }
    assert_eq!(
        Arc::strong_count(&bus),
        1,
        "every `BusRecipe::stream` let go of the bus: its stream \"only captures `rx`, never \
         `bus`\" (live.rs), so dropping the test's handle can close the channels"
    );

    let mut published: Published = Vec::new();
    for turn in 0..relays.len() {
        let channel = relays[turn].channel;
        let what = relays[turn].label.clone();
        expect_bus_relay_parked(
            &mut relays[turn],
            &published,
            "before its own channel's publish",
        );

        let publisher = Arc::clone(&bus);
        let expected = producer.produce(&what, async move { publish_on(&publisher, channel) });
        published.push((channel, expected.clone()));
        relays[turn].parked = false;
        for want in &expected {
            let got = next_item(
                &relays[turn].drain,
                WAIT,
                &format!("{what}: one event published, the bus still alive"),
            );
            let got = bus_msg(&got.msg);
            if foreign_publish(channel, &got, &published).is_some() {
                cross_talk(
                    &relays[turn],
                    &got,
                    &published,
                    "in its own turn, in place of its own event",
                );
            }
            assert_eq!(
                got, *want,
                "{what}: relayed before the next channel's publish"
            );
        }

        let when = format!("after turn #{} (one publish, on {channel:?})", turn + 1);
        for (other, relay) in relays.iter_mut().enumerate() {
            if other != turn {
                sweep_bus_relay(relay, &published, &when);
            }
        }
    }

    for relay in &mut relays {
        expect_bus_relay_parked(relay, &published, "before the bus drops");
    }
    producer.produce("bus: drop the last EventBus handle", async move {
        drop(bus);
        Ok(())
    });
    for relay in &relays {
        let tail: Vec<String> = drain_to_end(&relay.drain, &relay.label)
            .iter()
            .map(bus_msg)
            .collect();
        for got in &tail {
            if foreign_publish(relay.channel, got, &published).is_some() {
                cross_talk(relay, got, &published, "after the bus dropped");
            }
        }
        assert_eq!(
            tail,
            bus_after_close(relay.channel),
            "{}: after the bus dropped, its closing message (if it has one), then the end",
            relay.label
        );
    }
}
