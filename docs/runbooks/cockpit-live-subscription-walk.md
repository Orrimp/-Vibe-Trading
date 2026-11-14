# Runbook: walk `cockpit_live`'s subscription batch by hand

**Why this exists (story 2-63 K3 #6).** Since story 2-63 Wave C1, the shipped cockpit
(`cockpit_live`) builds its whole subscription batch with one library function,
`ui::live::subscription_batch`. That batch is every background stream that feeds the window: bus
data, the clock, progress bars, narration, the plan, the trail, toasts, and the `Esc` key for the
tape-row modal. `crates/ui/tests/subscription_batch_inclusion.rs` tests that function directly.

No automated check reads the struct literal through which the bin hands its live state to that
function (`ui::live::SubscriptionCtx { .. }` inside `fn subscription(&self)` in
`crates/ui/src/bin/cockpit_live.rs`), for two reasons:

- `AppState` cannot be built in a test.
- `cockpit-smoke` boots only the fixtures `cockpit` bin, never `cockpit_live`, and presses nothing.

The literal still compiles when it is wrong. The bake-off and sweep receivers have the same type,
the four per-run "salt" numbers are interchangeable, and any field can be passed as `None`. Probes
C1-RES1 (`forward_plan: None`) and C1-RES2 (a swapped salt) measured exactly this and left every
test green.

**What the walk can and cannot see.**

- **It catches** a field passed as `None`, and a field wired to the wrong cell. In each case a
  screen below stays empty or frozen, and that is C1-RES1's class.
- **It cannot catch** a swapped salt (C1-RES2). The bin clears each take-once cell when its run
  ends, so iced drops the recipe's id, and the next run streams afresh whatever its salt is.
  Nothing checks the salts, this walk included.

Paper/sim only: research mode replays history and places no real orders. It writes no durable
equity rows.

## Command

Run this from a terminal on the Mac, not inside a Claude session:

```sh
cd /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading
SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.sdk \
  cargo run -p ui --bin cockpit_live -- --mode research 2>&1 | tee /tmp/cockpit_live_walk.log
```

The `SDKROOT=` prefix is required, because the default SDK on this machine cannot link
([`sdk-linker-bump-for-arm64e-x1.md`](sdk-linker-bump-for-arm64e-x1.md)).

## Steps

Each step names, in brackets, the stream it proves. Do them in order, and note which steps pass and
which fail.

1. **[ServerTime] The clock ticks.** Watch the status bar's server-time field for 5 seconds. It
   advances once a second.
2. **[BakeoffProgress] The bake-off bar advances.**
   - Open the Leaderboard and click **"Run bake-off"**. The progress bar fills step by step. A
     spinner that never turns into a bar is a failure.
   - When the bake-off finishes, click **"Run bake-off"** again. The bar must advance again. This
     shows that the new run's cell is batched after the old one was cleared. It does not test the
     salt.
3. **[NarrationOutcome] Explain resolves.** On the finished bake-off, click **"Explain in plain
   language"**. Within about 30 seconds the panel shows text: an explanation, or the templated
   fallback when the LLM is disabled (the default). It must not spin indefinitely.
4. **[SweepProgress] The sweep bar advances.**
   - Open Tune (Calibrate) and click **"Run sweep"**. The sweep progress bar fills step by step.
     The Leaderboard's bar must NOT move instead.
   - When the sweep finishes, click **"Run sweep"** again. The bar must advance again, as in
     step 2.
5. **[ForwardPlan] The Plan screen fills.**
   - Click **"Promote"** on a non-fragile Tune row, or finish a bake-off that recommends a row.
     Open the Plan screen. It shows a plan, not its empty state.
   - Promote again, or run another bake-off. The plan updates. This shows that the plan stream
     stays subscribed after its first message.
6. **[LabProgress, TrainingLog] The Lab streams.**
   - Open the Lab and click **"Run"**. Its progress bar advances.
   - If a training run is available on this machine, start it. Its log lines stream into the
     panel. If no training run is available, record this half as "not exercised".
7. **[Trail] A decision trail loads.** Open the Trail screen and select a row from its list. The
   trail view replaces its loading placeholder with the reconstructed trail. A placeholder that
   never resolves is a failure. If the list has no rows in this mode, record "not exercised".
8. **[ModalEsc] Esc closes the tape-row modal.**
   - Open the Live screen and wait until the tape shows fills (research mode replays them).
   - Click a tape row: the audit modal opens.
   - Press `Esc`: it closes.
   - Press `Esc` again: nothing happens.
9. **[ToastDismiss] Toasts dismiss themselves.** Any toast that appears during the walk disappears
   on its own after a few seconds. "Export this plan" on the Plan screen is one way to raise one.
10. **[Bus] The data feed is live.** During the walk, the Live screen's tape, positions and P&L
    panels leave their loading state, and the agent mode shows the running agent, not "halted".

## Timing

- **Build.** 1 to 5 minutes on a warm `target/`. The first launch of a freshly linked binary can
  also sit about 2 minutes in macOS's first-launch check before the window appears. That delay is
  the OS, not the cockpit. If the build takes more than 2 minutes, watch it from a second terminal:

  ```sh
  watch -n 15 'tail -3 /tmp/cockpit_live_walk.log'
  ```

- **The walk.** About 10 minutes. Most of that is the bake-off and sweep runs.

## Expected result

- All 10 steps behave as described. Step 6's training half and step 7 may be "not exercised", and
  then they are recorded as such, not as passes.
- After you close the window, this prints `0`:

  ```sh
  grep -c 'panicked at' /tmp/cockpit_live_walk.log
  ```

## Failure diagnosis

1. **Run the automated tests first.** Expect 2 to 5 minutes, which includes macOS's first-launch
   check of the fresh test binary:

   ```sh
   cd /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading
   SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.sdk \
     cargo test -p ui --test subscription_batch_inclusion 2>&1 | tee /tmp/sbi_walk.log
   ```

   ```sh
   watch -n 15 'tail -3 /tmp/sbi_walk.log'
   ```

   - **Green while a step failed:** the defect is almost certainly in the bin's ctx literal,
     `ui::live::SubscriptionCtx { .. }` inside `fn subscription(&self)` in
     `crates/ui/src/bin/cockpit_live.rs`.
   - **Red:** the defect is in the library. Read the failing assertion's message.
2. **Map the failed step to its field in that literal:**
   - **Step 2 frozen:** `bakeoff_progress:`. It must read `self.bakeoff_progress_rx`.
   - **Step 3 spins:** `narration_outcome:` (`self.narration_outcome_rx`).
   - **Step 4 frozen, or the Leaderboard bar moves instead:** `sweep_progress:`. It must read
     `self.sweep_progress_rx`, not `self.bakeoff_progress_rx`.
   - **Step 5, Plan empty:** `forward_plan:`. It must be `self.plan_rx.as_ref()`, not `None`. This is
     probe C1-RES1.
   - **Step 6:** `lab_progress:` (`self.lab_progress_rx`) or `training_log:` (`self.training_log_rx`).
   - **Step 7, placeholder never resolves:** `trail:` (`self.trail_mirror_handle.as_ref()`).
   - **Step 8, Esc does nothing:** `tape_audit_modal_open:` (`self.cockpit.tape_audit_modal.is_some()`).
   - **Step 1, 9 or 10:** these are always-on streams built inside the library, with no field in the
     literal. Suspect `ui::live::subscription_batch` itself, and redo diagnosis step 1.
3. **Any `panicked at` in the log:** send the log file. Its tail names the source line.

## Cleanup

- Close the cockpit window. If `cargo run` is still attached, stop it with Ctrl-C.
- Remove the logs: `rm -f /tmp/cockpit_live_walk.log /tmp/sbi_walk.log`.
- Confirm the walk left nothing behind: `git status --short` must show no new files beyond those
  already there before you started.
