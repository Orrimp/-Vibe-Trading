# Flaky charts visual-regression gate: root-cause and fix

**Date:** 2026-06-15 (observed 2026-06-13, fixed 2026-06-15/16)
**Author:** developer agent
**Symptom:** `charts_screen_dark_floor`, `charts_screen_dark_typical`,
`charts_screen_dark_operator` in `crates/ui/tests/visual_snapshots.rs` fail
intermittently in the full `cargo test -p ui` run but always pass in isolation
(`cargo test -p ui --test visual_snapshots charts_screen_dark`).

---

## Root cause

### The data race

`std::env::set_var` is **not thread-safe** (it is `unsafe` in Rust edition 2024
precisely for this reason). All five snapshot test binaries in `crates/ui/tests/`
that render chart widgets called:

```rust
// OLD — DATA RACE
unsafe { std::env::set_var(ui::strings::CHART_FORCE_UTC_ENV, "1") };
```

at the **top of each render helper** (inside individual test functions), while in
`crates/ui/src/widgets/chart.rs` the production `#[cfg(not(test))]` branch of
`local_offset_or_utc()` reads the same environment variable:

```rust
if std::env::var_os(crate::strings::CHART_FORCE_UTC_ENV).is_some() {
    return time::UtcOffset::UTC;
}
```

When `cargo test -p ui` runs the `visual_snapshots` binary (51 tests), Rust's
test harness spawns all non-`#[ignore]` tests **in parallel threads**. In that
regime:

- Thread A calls `std::env::set_var("UI_CHART_FORCE_UTC", "1")` (write to the
  process environment block)
- Thread B (in the chart rendering pipeline) calls `std::env::var_os(...)`
  (read from the same environment block)

This is a data race on the underlying C `environ` array — undefined behaviour.
Depending on thread scheduling, Thread B may:

1. Read before Thread A writes → sees no env var → calls
   `time::UtcOffset::current_local_offset()` → renders x-axis labels in the
   local timezone (CEST / CET / etc.) instead of UTC → bytes differ from the
   committed UTC baseline → mismatch.
2. Read a partially-written value → undefined behaviour.
3. Race at the OS level on macOS, where `setenv(3)` is guarded by a non-recursive
   mutex that can deadlock when mixed with `getenv(3)` from another thread.

### Why it passes in isolation

`cargo test -p ui --test visual_snapshots` runs ONLY the `visual_snapshots`
binary with no thread competition from other binaries. Within that binary, the
three `charts_screen_dark_*` tests still race with each other and with the 48
other tests — but the window is tiny and the scheduling is benign in most runs.
The failure artifacts (2026-06-13, 2026-06-15) prove the race IS hit during the
full suite run where OS scheduler load is higher.

### Secondary non-determinism: `ThrottledSpinner`

`crates/ui/src/widgets/throttled_spinner.rs` initialises `SpinnerState` with
`last_update: Instant::now()` and advances `state.t` based on elapsed wall time
in the `RedrawRequested` handler. When `Loading` panels render spinners, the
spinner angle is time-dependent. The `charts_screen_dark_*` fixture uses
`PanelState::Ready(...)` for all panels, so this is NOT the primary failure mode
for those three tests — but it explains why the `#[ignore]`d shell-composition
tests are non-deterministic.

---

## Fix

### 1. Thread-safe flag in the chart widget

`crates/ui/src/widgets/chart.rs` — added a `static AtomicBool` flag that
replaces the env-var read:

```rust
static FORCE_UTC_ATOMIC: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

pub fn force_chart_utc_for_tests() {
    FORCE_UTC_ATOMIC.store(true, std::sync::atomic::Ordering::SeqCst);
}
```

`local_offset_or_utc()` (non-test branch) checks the atomic flag first:

```rust
if FORCE_UTC_ATOMIC.load(std::sync::atomic::Ordering::SeqCst) {
    return time::UtcOffset::UTC;
}
// Legacy env-var path kept as fallback for external-process / CI overrides
if std::env::var_os(crate::strings::CHART_FORCE_UTC_ENV).is_some() {
    return time::UtcOffset::UTC;
}
```

`SeqCst` ordering ensures any thread that called `store` before another thread
calls `load` will see the updated value — precisely the visibility guarantee
needed here.

### 2. Public re-export from `ui` crate

`crates/ui/src/lib.rs` re-exports the function:

```rust
pub use widgets::chart::force_chart_utc_for_tests;
```

### 3. `Once`-protected call-site in each test binary

Every test binary (and bench) that previously called `unsafe { set_var }` now
uses a process-wide `std::sync::Once` so the atomic store happens exactly once,
regardless of parallel test threads:

```rust
static INIT_UTC: std::sync::Once = std::sync::Once::new();
// ...
INIT_UTC.call_once(ui::force_chart_utc_for_tests);
```

Files changed:
- `crates/ui/tests/visual_snapshots.rs` — `run_slot()`
- `crates/ui/tests/fixtures/viewport_matrix.rs` — `snapshot_widget_at_slot()`
- `crates/ui/tests/render_snapshots.rs` — `run_panel_slot_legacy()` + new `force_utc_once()`
- `crates/ui/tests/render_timing_probe.rs` — `time_render()`
- `crates/ui/tests/live_equity_render.rs` — `render_live()`, `render_overlay()`, `render_compare_screen()`
- `crates/ui/benches/cockpit_render.rs` — `bench_render()`, `emulator_construct_only()`
- `crates/ui/benches/chart_build_probe.rs` — `measure_split()`

---

## Secondary: deprecated Screen variant renames

Three test files used `Screen::Audit` (deprecated since 0.2.0; alias for
`Screen::Trail`) and one file used `Screen::Home` (deprecated; alias for
`Screen::Live`). Renamed to the canonical variants so `cargo clippy --tests -p
ui --all-features -- -D warnings` is clean:

- `crates/ui/tests/audit_row_opens_modal.rs`: `Screen::Audit` → `Screen::Trail`
- `crates/ui/tests/audit_filter_chip_emits_filter_changed.rs`: `Screen::Audit` → `Screen::Trail`
- `crates/ui/tests/home_strategies_row_cross_link.rs`: `Screen::Home` → `Screen::Live`

---

## Verification

5 consecutive `cargo test -p ui` runs — all passed (0 failures) with the three
previously flaky tests green on every run. Clippy clean.

```
Run 1: 51 passed; 0 failed  (visual_snapshots binary)
Run 2: 51 passed; 0 failed
Run 3: 51 passed; 0 failed
Run 4: 51 passed; 0 failed
Run 5: 51 passed; 0 failed
```

---

## Lessons

1. `std::env::set_var` is `unsafe` in edition 2024 for a reason — if you write
   "SAFETY: single-threaded..." the comment must be TRUE. Rust's parallel test
   harness makes it false in most integration-test binaries.
2. Fixes that look safe in isolation can be races at the binary level. The correct
   pattern is an in-process `AtomicBool` (or `OnceLock`) for test flags, not
   environment variables.
3. The `#[cfg(test)]` cfg gate on `local_offset_or_utc()` is NOT sufficient for
   integration tests — integration test binaries link against the library compiled
   WITHOUT `cfg(test)`. The non-test branch must be made deterministic via a
   mechanism accessible from integration test code.
