//! bug-log `#215` — a Stop during a Yahoo preload, at the rendered-PIXEL layer
//! (AD-10): the Lab's Run row draws "Cancelled — Run again" and no red, and the
//! status bar draws no red `YahooPreload` row.
//!
//! **Scope of the status-bar claim.** The frames hold only the activity events
//! the production ARM sends (`YahooPreload`). In the shipped cockpit every Lab
//! run also puts the binary's `LabRun` activity on the tape, and on a Stop only
//! `cockpit_live.rs`'s `LabRunStopRequested` branch (`handle.cancel()`) keeps
//! that row from ending `Failed`. The binary's `AppState` cannot be built in a
//! test (ADR-0048 K3 #6), so that branch is NOT in these frames and is unpinned
//! (bug-log `#215` "Not done", `#224`).
//!
//! ## What this file drives
//!
//! Each frame's state comes from the PRODUCTION arm, not a hand-built string:
//! `tests/yahoo_arm_support/mod.rs`'s `start_arm` runs
//! `ui::lab::runner::run_yahoo_preload_arm` off-runtime (how iced polls the
//! Lab Task), and its `done.result` and `run.activity()` events are fed through
//! the production `ui::state::update` arms on a Lab cockpit
//! (`charts_screen_cockpit`, `Screen::Lab`, strategy `v0.sma`, pair
//! Binance `BTCUSDT`): `LabRunRequested`, (`LabRunStopRequested` for a Stop),
//! each event as `ActivityEventReceived`, then `LabRunCompleted(Err(e))` — the
//! message `spawn_lab_run`'s `.await?` posts for an arm error. The frame is the
//! full shell (`shell::view` → `screens::lab::view` and `widgets::status_bar`)
//! through `iced_test::screenshot` at 1600×900, scale 1.0, Dark.
//!
//! **Why `program_from_build`, not `program_from_cockpit`.**
//! `program_from_cockpit` boots from `cockpit.clone()`, and `LabState::clone`
//! resets `last_run_error`, `last_run_notice` and `last_run_cancelled`, so a
//! run outcome seeded on the cockpit never reaches the frame: the Run row would
//! draw Idle "Run" whatever the outcome (H-P1 below measures that). The
//! activity tape derives `Clone`, so only the Run row would be lost.
//! `program_from_build` runs the build closure inside `boot`.
//!
//! **The outcomes.**
//! - Stop: a pre-cancelled run (`Spec { pre_cancelled: true, .. }`, a 5 s
//!   120-bar mock), so the arm's cancel branch wins its first `select!`.
//! - Fail, the negative control: a mock preload that fails with the untagged
//!   [`FAILURE`] after 10 ms.
//! - Idle, the label REFERENCE: the same Lab cockpit with no run dispatched,
//!   so its Run row is the enabled 3-glyph "Run" button whatever any run
//!   outcome does.
//!
//! **The bands** (measured by the `#[ignore]` `measure` test on 2026-11-24,
//! macOS dev box, at HEAD `28a71fa1` plus `program_from_build` and this file,
//! with no fix; ADR-0092's rule after bug-log `#96`: a band is stated with the
//! render it came from). The 10-px profile, x ≥ 180, then per-row reads of the
//! PNGs:
//!
//! ```text
//! y  170..210  ACCENT 74/18/18/64   the range chips' selected border (rows 175..201)
//! y  210..250  ACCENT 344/563/562/288, DOWN_500 8/140 (rows 217..226)
//!              the Run button (rows 214..244) and the red "⚠ …" line
//! y  290..340  DOWN_500 604         the Buys/Sells card's red figures (chrome)
//! y  550..590  DOWN_500 150         the chart's sell markers (chrome)
//! y  770..810  DOWN_500 840         the volume bars (chrome)
//! y  876..900  DOWN_500 69          the status bar's red "Loading data" row
//! ```
//!
//! - the Run row: rows `[206, 252)` (the Run button's rows 214..244 with
//!   margin, clear of the chips above and the card below), from the end of
//!   the 180 px sidebar to the right edge;
//! - the status bar: its 24 px content row, rows `[H − 24, H)`, from the end of
//!   the sidebar (`activity_tape_failed_hold_render.rs`'s band).
//!
//! The Lab chrome draws `DOWN_500` elsewhere (`lab_session_restore_render.rs`
//! records down = 1594 below its band), so red is counted only inside a band.
//!
//! **Colour.** `DOWN_500` and `ACCENT` are the theme tokens at Dark, matched
//! within ±24 per channel (`lab_session_restore_render.rs`'s classifier).
//!
//! **The label clause.** The Run button's enabled background is `ACCENT`, so
//! the Run row's `ACCENT` count grows with the label's laid-out width (about
//! 100 px per glyph here). R1 requires the Stop frame's count to be at least
//! [`LABEL_FACTOR`] (2) × the Idle frame's: measured, "Run" is 1531, "Retry"
//! 1757 (1.15×), "Re-run" 2009 (1.31×) and "Cancelled — Run again" 4558
//! (2.98×). Only a label of about twice "Run"'s footprint clears the gap, so a
//! Stop that draws any of the short labels — "Re-run" included — is red. The
//! reference depends on no run outcome, so a regression on the FAILURE path
//! cannot turn R1 red and be blamed on the Stop (bug-log `#189`).
//!
//! **The clock.** A failed tape row draws its wall-clock elapsed time
//! (`widgets/activity_tape.rs`'s CLOCK-OK block names this file): the R2 frame,
//! and the Stop frame before the `#215` fix, hold such a row, stamped by
//! `ActivityTape::apply`'s `Instant::now()` when `boot` runs. So no frame is
//! stored as a baseline, R2 is only counted in colour, and only R3 compares
//! bytes, between two Stop frames; after the fix those hold no tape row. At
//! HEAD (the red-first measurement) R3 compared two held rows, each "<1s"
//! because each frame renders within a second of its `apply`: a measurement
//! with a 1 s flake window, recorded as such.
//!
//! **Measured** (macOS dev box, 2026-11-24; the floors below are fixed
//! literals, half the Fail frame's counts, never recomputed at run time):
//!
//! | frame | run-row `DOWN_500` | status-bar `DOWN_500` | run-row `ACCENT` |
//! |---|---|---|---|
//! | Stop at HEAD `28a71fa1` (no fix: "Retry" + "⚠ operator cancelled during preload" + a red tape row) | 148 | 69 | 1757 |
//! | Fail (HEAD and fixed alike) | 135 | 69 | 1757 |
//! | Stop, fixed ("Cancelled — Run again", no red line, no tape row) | 0 | 0 | 4558 |
//! | Idle, the reference ("Run", no run dispatched) | 0 | 0 | 1531 |
//! | Stop under probe T215-P14 ("Re-run" drawn for `Cancelled`) | 0 | 0 | 2009 |
//!
//! Red first: at HEAD R1 failed on all three of its first-form clauses at once
//! (`[RED]` ×3; the label clause then compared against the Fail frame, 1757 vs
//! 1757); R2 and R3 were green (R3 inside its 1 s window). The review's label
//! clause (≥ 2 × Idle) is red on HEAD's measured 1757 as well (< 3062; derived
//! from the counts above, not re-run at HEAD). After the fix R1-R3 are green.
//!
//! ## Tests
//!
//! | test | what it pins |
//! |---|---|
//! | `a_stop_draws_cancelled_and_no_red` (R1) | Stop frame: run-row `DOWN_500` = 0, status-bar `DOWN_500` = 0 (no red `YahooPreload` row), and run-row `ACCENT` ≥ [`LABEL_FACTOR`] × the Idle reference's (the 21-glyph Cancelled label, not a short one); the reference itself ≥ [`IDLE_ACCENT_FLOOR`]. All four counts are computed before the one assert, so a red names every clause and its frame |
//! | `a_failed_preload_draws_red_in_the_run_row_and_the_status_bar` (R2) | the NEGATIVE CONTROL: Fail frame: `DOWN_500` ≥ its floor in both bands, and run-row `ACCENT` ≥ its floor (the band holds the Run button). Proves the detector, the bands and the harness can see red and the button |
//! | `the_same_stop_renders_identically` (R3) | the JITTER CONTROL: two independent Stop builds give byte-identical run-row and status-bar bands |
//! | `measure` (`#[ignore]`) | prints the per-band profile the bands and floors were read from, and writes the PNGs |
//!
//! ## T-T4 probes (MEASURED 2026-11-24, one edit each; bug-log `#215` has the full table)
//!
//! | probe | one edit | red here |
//! |---|---|---|
//! | T215-P1 | `runner.rs`: `h.cancel();` → `h.fail("operator cancelled");` | R1 at its status-bar clause alone (69) |
//! | T215-P2 | `runner.rs`: `h.cancel();` → `let _ = &h;` | none, by design: an `End(Success)` row is dropped like `Cancelled`, so the frame cannot tell them apart (both are silent to the operator) |
//! | T215-P3 | `runner.rs`: the arm's `cancelled_message(..)` return → `SmolStr::new(..)` | R1 at run-row `DOWN_500` (148) and the label clause (1757 < 2 × 1531) |
//! | T215-P4 | `runner.rs` `classify`: the `CANCELLED_TAG` branch never matches | R1 at run-row `DOWN_500` (234: the tag bytes draw too) and the label clause (1757) |
//! | T215-P6 | `state.rs`: `last_run_cancelled = true;` → `= false;` | R1 at the label clause alone (1531, Idle "Run") |
//! | T215-P7 | `screens/lab.rs`: `.with_cancelled(model.lab_state.last_run_cancelled)` removed | R1 at the label clause alone (1531): render-only wiring, which is why this file exists |
//! | T215-P8 | `run_button.rs` `with_cancelled`: `if last_run_cancelled` → `if false` | R1 at the label clause alone (1531) |
//! | T215-P11 | `runner.rs`: the arm's classified-error `Err(e)` → `Err(preload_notice::cancelled_message(&e))` | R2 alone ("run-row DOWN_500 = 0, floor 67"); R1 stays green (its reference is the Idle frame, which no run outcome touches) |
//! | T215-P14 | `run_button.rs` `view`: `RunState::Cancelled => LAB_RUN_BUTTON_CANCELLED,` → `RunState::Cancelled => LAB_RUN_BUTTON_COMPLETED,` ("Re-run") | R1 at the label clause alone (2009 < 3062). GREEN under R1's first form (vs the Fail frame's 1757): the review's finding |
//! | H-P1 | here, `render_built`: `program_from_build(build)` → `ui::test_support::program_from_cockpit(build())` (the clone trap) | R1 at the label clause (1531: the Stop frame falls back to Idle "Run") and R2 at run-row `DOWN_500` (0) |
//! | H-P2 | here, R3: `let second = render(&run_arm(Outcome::Stop));` → `Outcome::Fail` | R3 at "byte-identical in the run row band" |
//!
//! T215-P5, P9, P10, P12 and P13 have no red here by design (an engine Stop, a
//! stale flag across runs, an engine error, `Running` and the session log are
//! not in these frames).
//!
//! ## macOS + `live` gate (ADR-0057 § D2)
//!
//! Real-renderer pixel assertions are macOS-canonical, and the arm is compiled
//! under `live`. `scripts/ci_test_target_counts.py` declares this target at 4
//! (`--list` includes the ignored `measure`) on macOS and 0 on Windows.
//!
//! PNGs (with `UI_DUMP_PNG` set, or always from `measure`):
//! `/tmp/lab_stop_215_{stop,fail,idle}.png`.

#![cfg(all(target_os = "macos", feature = "live"))]
#![allow(clippy::expect_used, clippy::unwrap_used)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

#[path = "yahoo_arm_support/mod.rs"]
mod support;

use std::time::Duration;

use agent::activity::ActivityEvent;
use smol_str::SmolStr;
use support::{Behavior, Spec, start_arm};
use trading_core::{StrategyId, Symbol, Venue};
use ui::state::{Cockpit, Message, Screen, update};
use ui::test_support::program_from_build;

const W: u32 = 1600;
const H: u32 = 900;

/// The status bar's content row height (T1508; `status_bar.rs`'s `BAR_HEIGHT`).
const BAR_H: u32 = 24;

/// The Run row's band (see the module doc for the measurement).
const RUN_TOP: u32 = 206;
const RUN_BOTTOM: u32 = 252;

/// The untagged preload failure of the Fail outcome (the negative control).
const FAILURE: &str = "network error: connection refused";

/// R1's label clause: the Stop frame's Run-row `ACCENT` must be at least this
/// many times the Idle reference's ("Run"). Measured ratios: "Retry" 1.15×,
/// "Re-run" 1.31×, "Cancelled — Run again" 2.98× (module doc).
const LABEL_FACTOR: u64 = 2;

/// The Idle reference's Run-row `ACCENT` floor: 1531 measured ("Run"), halved.
/// Below it the reference no longer draws the enabled button, and the label
/// clause would compare against nothing.
const IDLE_ACCENT_FLOOR: u64 = 765;

/// Which production arm run a frame is built from.
#[derive(Clone, Copy, Debug)]
enum Outcome {
    /// The operator's Stop: a pre-cancelled 5 s preload.
    Stop,
    /// A hard preload failure, [`FAILURE`].
    Fail,
}

/// What the production arm returned, and the activity events it sent.
#[derive(Clone, Debug)]
struct ArmOutcome {
    outcome: Outcome,
    error: SmolStr,
    events: Vec<ActivityEvent>,
}

/// Run the PRODUCTION `run_yahoo_preload_arm` for `outcome` and collect its
/// error and its activity events.
fn run_arm(outcome: Outcome) -> ArmOutcome {
    let spec = match outcome {
        Outcome::Stop => Spec {
            pre_cancelled: true,
            with_activity: true,
            ..Spec::new(5_000, Behavior::Bars(120))
        },
        Outcome::Fail => Spec {
            with_activity: true,
            ..Spec::new(10, Behavior::Fail(FAILURE))
        },
    };
    let mut run = start_arm(spec);
    let done = run
        .wait_done(Duration::from_secs(3))
        .expect("HARNESS: the arm must return within 3 s");
    let error = done
        .result
        .expect_err("HARNESS: a Stop and a failed preload both end the arm in Err");
    let events = run.activity();
    assert_eq!(
        events.len(),
        2,
        "HARNESS: the arm sends exactly one activity Start and one End; got {events:?}"
    );
    ArmOutcome {
        outcome,
        error,
        events,
    }
}

/// The Lab cockpit after `arm`, built through the production `update` arms.
fn lab_after(arm: &ArmOutcome) -> Cockpit {
    let mut c = ui::test_support::charts_screen_cockpit();
    c.current_screen = Screen::Lab;
    c.lab_state.strategy = Some(StrategyId::new("v0.sma"));
    c.lab_state.pair = Some((Venue::Binance, Symbol::new("BTCUSDT")));
    update(&mut c, Message::LabRunRequested);
    if matches!(arm.outcome, Outcome::Stop) {
        update(&mut c, Message::LabRunStopRequested);
    }
    for ev in &arm.events {
        update(&mut c, Message::ActivityEventReceived(ev.clone()));
    }
    update(&mut c, Message::LabRunCompleted(Err(arm.error.clone())));
    c
}

/// The label reference: the same Lab cockpit with no run dispatched.
fn lab_idle() -> Cockpit {
    let mut c = ui::test_support::charts_screen_cockpit();
    c.current_screen = Screen::Lab;
    c.lab_state.strategy = Some(StrategyId::new("v0.sma"));
    c.lab_state.pair = Some((Venue::Binance, Symbol::new("BTCUSDT")));
    c
}

/// Render the Lab after `arm` (a fresh screenshot per call).
fn render(arm: &ArmOutcome) -> Vec<u8> {
    let arm = arm.clone();
    render_built(move || lab_after(&arm))
}

/// Render the Idle reference (a fresh screenshot per call).
fn render_idle() -> Vec<u8> {
    render_built(lab_idle)
}

/// Render the cockpit `build` returns, built inside `boot`.
fn render_built(build: impl Fn() -> Cockpit + 'static) -> Vec<u8> {
    ui::force_chart_utc_for_tests();
    let program = program_from_build(build);
    let shot = iced_test::screenshot(&program, &iced::Theme::Dark, (W, H), 1.0, Duration::ZERO);
    assert!(
        shot.size.width == W && shot.size.height == H,
        "HARNESS: frame is {}×{}, expected {W}×{H} at scale 1.0",
        shot.size.width,
        shot.size.height
    );
    shot.rgba.to_vec()
}

fn dump_png(rgba: &[u8], path: &str) {
    if let Some(img) = image::RgbaImage::from_raw(W, H, rgba.to_vec()) {
        let _ = img.save(path);
    }
}

fn maybe_dump(rgba: &[u8], path: &str) {
    if std::env::var_os("UI_DUMP_PNG").is_some() {
        dump_png(rgba, path);
    }
}

/// A rectangle of the frame: rows `[top, bottom)`, columns `[x0, W)`.
#[derive(Clone, Copy, Debug)]
struct Band {
    top: u32,
    bottom: u32,
    x0: u32,
}

/// The Run row, from the end of the sidebar.
fn run_row() -> Band {
    Band {
        top: RUN_TOP,
        bottom: RUN_BOTTOM,
        x0: ui::theme::layout::SIDEBAR_WIDTH_PX as u32,
    }
}

/// The status bar's 24 px content row, from the end of the sidebar.
fn status_bar() -> Band {
    Band {
        top: H - BAR_H,
        bottom: H,
        x0: ui::theme::layout::SIDEBAR_WIDTH_PX as u32,
    }
}

/// The band's RGBA bytes.
fn bytes(rgba: &[u8], band: Band) -> Vec<u8> {
    let mut out = Vec::new();
    for y in band.top..band.bottom {
        let row = (y * W) as usize * 4;
        out.extend_from_slice(&rgba[row + band.x0 as usize * 4..row + W as usize * 4]);
    }
    out
}

/// The token's Dark value as 8-bit RGB.
fn rgb(token: ui::theme::color::ModeColor) -> [i32; 3] {
    let c = token.current(ui::theme::ThemeMode::Dark);
    [
        (c.r * 255.0).round() as i32,
        (c.g * 255.0).round() as i32,
        (c.b * 255.0).round() as i32,
    ]
}

/// Pixels within ±24 per channel of `token` inside `band`.
fn count(rgba: &[u8], band: Band, token: ui::theme::color::ModeColor) -> u64 {
    let target = rgb(token);
    bytes(rgba, band)
        .chunks_exact(4)
        .filter(|p| (0..3).all(|k| (i32::from(p[k]) - target[k]).abs() <= 24))
        .count() as u64
}

fn down(rgba: &[u8], band: Band) -> u64 {
    count(rgba, band, ui::theme::color::DOWN_500)
}

fn accent(rgba: &[u8], band: Band) -> u64 {
    count(rgba, band, ui::theme::color::ACCENT)
}

/// R1 — a Stop draws "Cancelled — Run again" and no red in the Run row, and no
/// red `YahooPreload` row in the status bar.
#[test]
fn a_stop_draws_cancelled_and_no_red() {
    let stop = render(&run_arm(Outcome::Stop));
    let idle = render_idle();
    maybe_dump(&stop, "/tmp/lab_stop_215_stop.png");
    maybe_dump(&idle, "/tmp/lab_stop_215_idle.png");
    let run_down = down(&stop, run_row());
    let bar_down = down(&stop, status_bar());
    let run_accent = accent(&stop, run_row());
    let idle_accent = accent(&idle, run_row());
    let mark = |ok: bool| if ok { "ok" } else { "RED" };
    let (a, b) = (run_down == 0, bar_down == 0);
    let c = run_accent >= LABEL_FACTOR * idle_accent;
    let d = idle_accent >= IDLE_ACCENT_FLOOR;
    eprintln!(
        "R1: run-row DOWN_500 = {run_down}, status-bar DOWN_500 = {bar_down}, \
         run-row ACCENT = {run_accent} (Idle reference: {idle_accent})"
    );
    assert!(
        a && b && c && d,
        "R1: the Stop frame must draw no red and the Cancelled label: \
         [{}] Stop frame run-row DOWN_500 = {run_down} (want 0: no red run error); \
         [{}] Stop frame status-bar DOWN_500 = {bar_down} (want 0: no red YahooPreload \
         row; the binary's LabRun row is not in this frame); \
         [{}] Stop frame run-row ACCENT = {run_accent} >= {LABEL_FACTOR} x the Idle \
         reference's {idle_accent} (want the 21-glyph \"Cancelled — Run again\" button; \
         \"Run\", \"Retry\" and \"Re-run\" stay below); \
         [{}] REFERENCE: the Idle frame (no run) run-row ACCENT = {idle_accent} >= \
         {IDLE_ACCENT_FLOOR} (the reference draws the enabled Run button). \
         PNGs with UI_DUMP_PNG: /tmp/lab_stop_215_stop.png, /tmp/lab_stop_215_idle.png",
        mark(a),
        mark(b),
        mark(c),
        mark(d)
    );
}

/// R2 — the negative control: a failed preload draws red in the Run row and in
/// the status bar, and the Run row band holds the (enabled, `ACCENT`) button.
#[test]
fn a_failed_preload_draws_red_in_the_run_row_and_the_status_bar() {
    let fail = render(&run_arm(Outcome::Fail));
    maybe_dump(&fail, "/tmp/lab_stop_215_fail.png");
    let run_down = down(&fail, run_row());
    let bar_down = down(&fail, status_bar());
    let run_accent = accent(&fail, run_row());
    eprintln!(
        "R2: run-row DOWN_500 = {run_down}, status-bar DOWN_500 = {bar_down}, \
         run-row ACCENT = {run_accent}"
    );
    assert!(
        run_down >= RUN_DOWN_FLOOR,
        "R2: a failed preload draws its red run error: run-row DOWN_500 = {run_down}, \
         floor {RUN_DOWN_FLOOR}"
    );
    assert!(
        bar_down >= BAR_DOWN_FLOOR,
        "R2: a failed preload draws a red activity row: status-bar DOWN_500 = {bar_down}, \
         floor {BAR_DOWN_FLOOR}"
    );
    assert!(
        run_accent >= ACCENT_FLOOR,
        "R2: the Run row band holds the enabled Run button: run-row ACCENT = {run_accent}, \
         floor {ACCENT_FLOOR}"
    );
}

/// R3 — the jitter control: two independent Stop builds render the same bytes
/// in both bands, so R1's counts are a property of the state, not of the run.
#[test]
fn the_same_stop_renders_identically() {
    let first = render(&run_arm(Outcome::Stop));
    let second = render(&run_arm(Outcome::Stop));
    for (name, band) in [("run row", run_row()), ("status bar", status_bar())] {
        let (a, b) = (bytes(&first, band), bytes(&second, band));
        let differing = a
            .chunks_exact(4)
            .zip(b.chunks_exact(4))
            .filter(|(p, q)| p != q)
            .count();
        assert_eq!(
            differing, 0,
            "R3: two independent Stop frames must be byte-identical in the {name} band"
        );
    }
}

/// `DOWN_500` floor of the Fail frame's Run row: 135 measured, halved.
const RUN_DOWN_FLOOR: u64 = 67;
/// `DOWN_500` floor of the Fail frame's status bar: 69 measured, halved.
const BAR_DOWN_FLOOR: u64 = 34;
/// `ACCENT` floor of the Fail frame's Run row: 1757 measured ("Retry"), halved.
const ACCENT_FLOOR: u64 = 878;

/// Prints the per-10-px-band profile (`DOWN_500`, `ACCENT`) of the Stop and
/// Fail frames, which the bands and floors above were read from, and the Idle
/// reference's Run-row `ACCENT`, and writes the three PNGs.
#[test]
#[ignore = "measurement: run by hand to re-derive the bands (ADR-0092)"]
fn measure() {
    let stop = render(&run_arm(Outcome::Stop));
    let fail = render(&run_arm(Outcome::Fail));
    let idle = render_idle();
    dump_png(&stop, "/tmp/lab_stop_215_stop.png");
    dump_png(&fail, "/tmp/lab_stop_215_fail.png");
    dump_png(&idle, "/tmp/lab_stop_215_idle.png");
    eprintln!(
        "run row: stop ACCENT={} fail ACCENT={} idle ACCENT={}",
        accent(&stop, run_row()),
        accent(&fail, run_row()),
        accent(&idle, run_row())
    );
    let x0 = ui::theme::layout::SIDEBAR_WIDTH_PX as u32;
    for y in (0..H).step_by(10) {
        let band = Band {
            top: y,
            bottom: (y + 10).min(H),
            x0,
        };
        let row = [
            down(&stop, band),
            accent(&stop, band),
            down(&fail, band),
            accent(&fail, band),
        ];
        if row.iter().any(|&n| n > 0) {
            eprintln!(
                "y {:>4}..{:<4} stop: down={:<6} accent={:<6} fail: down={:<6} accent={:<6}",
                band.top, band.bottom, row[0], row[1], row[2], row[3]
            );
        }
    }
    let bar = status_bar();
    eprintln!(
        "status bar {}..{}: stop down={} fail down={}",
        bar.top,
        bar.bottom,
        down(&stop, bar),
        down(&fail, bar)
    );
}
