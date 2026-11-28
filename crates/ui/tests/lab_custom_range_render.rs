//! bug-log `#230` — the Lab's Custom range editor at the rendered-PIXEL layer (AD-10).
//!
//! The fix (ADR-0105) makes one per-bound parse, `lab::runner::custom_bound_ms`, the only
//! reader of a Custom bound, and classifies each keystroke in `update` against the live range:
//! a new pair whose two bounds parse COMMITS (it becomes `range`), any other new pair is a
//! DRAFT that only the fields show. These frames pin what the operator sees of that:
//!
//! - the editor colours a bound by the run's own parse (F1-F3b): an RFC 3339 bound is not red
//!   (HEAD drew it red), `2024-02-30` is (HEAD drew it valid);
//! - an uncommitted edit draws the typed text (F4), through the one `date_range::view` call
//!   `screens::lab::view` makes with the draft;
//! - a partial bound keeps the last run drawn (F6): the KPI strip and the Run button's
//!   "Re-run" stay until a pair commits (HEAD cleared them on every keystroke);
//! - the Yahoo cadence badge reads an RFC 3339 Custom range (F7; HEAD fell back to
//!   now − 90 d for both bounds and drew "1m");
//! - a whole RFC 3339 bound, offset included, is visible in its field (F8; bug-log `#230`
//!   review: the 120 px field clipped a bound after about 15 characters, so a valid bound and
//!   one without its offset looked the same but for the border). The fields are 210 px
//!   (`widgets::date_range`'s `DATE_FIELD_WIDTH_PX`).
//!
//! ## The frames
//!
//! Base build: `test_support::charts_screen_cockpit()`, `Screen::Lab`, `v0.sma` × Binance
//! `BTCUSDT`, Synthetic unless stated, and `lab_report_roots = Vec::new()` so no report walk
//! touches the box. Every frame is built by a closure through
//! `test_support::program_from_build` (the cockpit is built inside `boot`; the run mirrors are
//! not cloned, `lab/state.rs`'s `impl Clone`, so a `program_from_cockpit` frame would lose F6's
//! last run: harness probe H1b) and rendered with `iced_test::screenshot` at 1600×900, scale
//! 1.0, Dark, after `ui::force_chart_utc_for_tests()`. A keystroke is the production message
//! `Message::LabCustomRangeEdited` through the production `ui::state::update`.
//!
//! ## The bands
//!
//! Measured by the `#[ignore]` `measure` test (macOS dev box, 2026-11-28, at HEAD `ecfb1bed`
//! with no fix, re-measured identical with the fix, and re-measured with the 210 px fields of
//! the review round: the rows are unchanged, the badge moved right; ADR-0092's rule after
//! bug-log `#96`: a band is stated with the render it came from). One-px rows, x ≥ the 180 px
//! sidebar:
//!
//! ```text
//! two-hint frame (start "2024-1-1", end "2024-2-1"; Synthetic)
//! y 175..201   ACCENT    the range chips (the Custom chip's selected border)
//! y 210..236   DOWN_500  the two red field borders
//! y 247..254   DOWN_500  hint 1, "Invalid date (use YYYY-MM-DD or RFC 3339 with an offset)"
//! y 270..277   DOWN_500  hint 2
//! y 294..324   ACCENT    the Run button
//! y 375..      DOWN_500  the Buys/Sells card's red figures (chrome), the first chrome red
//! F6 base frame (a committed, valid pair and a last run; no hint)
//! y 249..281   ACCENT    the "Re-run" button
//! F7 frames (Yahoo): the cadence badge chip spans x 653..680, y 167..188 (x 606..636 with
//!   the 120 px fields of HEAD and the first round)
//! start field: left border x 196, the 210 px field to x 405; border rows 210 and 236
//! the widest 25-char runs: `2024-01-01T01:00:00+01:00` ends 181 px from the left border,
//!   `0000-00-00T00:00:00+00:00` 190 px (measured at a scratch 400 px width, identical at 210)
//! ```
//!
//! - the EDITOR band: rows `[165, 288)`, x from the sidebar to the right edge. It holds the
//!   chip row, the field row and both hint rows; its lower edge is 11 rows under hint 2 and
//!   87 rows above the first chrome `DOWN_500` row of the tallest (two-hint) frame. In a frame
//!   with fewer hints the band also holds the Run button, which draws no `DOWN_500`.
//! - the RUN band: rows `[242, 620)`, x from the sidebar: from under the field row (it ends
//!   at 236) through the Run row, the Buys/Sells card, the KPI strip and the chart strip.
//! - the BADGE rect: x `[649, 693)`, y `[163, 193)`, the badge chip with margin. The badge is
//!   top-aligned in its `Row` (`screens/lab.rs`, no `align_y`), so a hint under the picker does
//!   not move it (HEAD's F7 frame A, two hints, measured at the then rect); it follows the
//!   picker column's width, which the field row sets since the fields are 210 px.
//! - the START-field TEXT INTERIOR (F8): x `[201, 401)`, rows `[213, 234)`, inside the border,
//!   its antialiased corners and the 5 px padding, so a border colour change is outside it.
//!
//! **Colour.** `DOWN_500` and `ACCENT` are the theme tokens at Dark, matched within ±24 per
//! channel (`lab_stop_render.rs`'s classifier).
//!
//! ## Measured
//!
//! Red first, at HEAD `ecfb1bed` (Step 0 of the #230 fix session; `range` assigned directly,
//! F6's keystrokes as HEAD's `LabSelectRange(Custom{..})`):
//!
//! The fixed column is the review round's tree (210 px fields and the longer hint, so more red
//! pixels per hint and per border than the first round's 324). F8 did not exist at HEAD; under
//! probe P26 (the fields back to 120 px) it is red at its 25th-character clause (0 px), while
//! its `Z` clause read 32 px there: the narrower field's red right border (F2b) falls inside the
//! interior rect, which is drawn for the 210 px field. The 25th-character clause is the one
//! that cannot be met by a border.
//!
//! | test | HEAD | fixed |
//! |---|---|---|
//! | F1 | RED: editor `DOWN_500` = 648 (both RFC 3339 bounds drawn red, two hints) | 0 |
//! | F2 | green: 324 | 568 |
//! | F2b | green: 648 | 568 (the end bound now parses) |
//! | F3a | RED: 0 (`2024-02-30` drawn valid) | 568 |
//! | F3b | green: 0 | 0 |
//! | F4 | not run (needs the new message) | diff(A, B) = 0; diff(A, C) = 5818 px; down(A) = 568, down(C) = 0 |
//! | F5 | green | green |
//! | F6 | RED: run band A ≠ C (the keystroke cleared the last run) | A vs C 0 px; D vs C 7711 px |
//! | F7 | RED: badge A ≠ B (A fell back to now − 90 d both ends: "1m") | A vs B 0 px ("1h"); B vs C 163 px ("1m") |
//! | F8 | n/a (new) | the `Z`: 44 px; the 25th character: 66 px; identical builds: 0; ink 863 px |
//!
//! ## Tests
//!
//! | test | what it pins |
//! |---|---|
//! | `an_rfc3339_custom_range_draws_no_invalid_mark` (F1) | `("2024-01-01T00:00:00Z", "2024-02-01T00:00:00Z")`: editor `DOWN_500` = 0 |
//! | `a_malformed_bound_draws_the_invalid_mark` (F2) | the detector control: start `2024-1-1`: editor `DOWN_500` > 0 |
//! | `an_rfc3339_bound_without_an_offset_draws_the_invalid_mark` (F2b) | F1 minus the start's `Z`: > 0 (F1's #161 partner: only the offset differs) |
//! | `a_calendar_invalid_bound_draws_the_invalid_mark` (F3a) | `("2024-02-30", "2024-03-01")`: > 0 |
//! | `a_leap_day_bound_draws_no_invalid_mark` (F3b) | `("2024-02-29", "2024-03-01")`: = 0 (F3a's #161 partner) |
//! | `an_uncommitted_edit_draws_the_typed_text` (F4) | A = committed `(2024-01-15, 2024-02-01)` + the keystroke `("2024-01-1", "2024-02-01")`, B = `range` assigned `("2024-01-1", "2024-02-01")` (what HEAD drew), C = A without the keystroke: editor band A = B byte-identical, A ≠ C, down(A) > 0, down(C) = 0 |
//! | `the_same_custom_state_renders_identically` (F5) | the jitter control: two F1 builds, the whole frame byte-identical |
//! | `a_partial_custom_bound_keeps_the_last_run_drawn` (F6) | base = committed `(2024-01-15, 2024-02-01)` + a last run keyed to its tuple. C = base; A = base + keystroke `("", "2024-02-01")` (a draft that draws no hint, so the layout matches C); D = base + keystroke `("2024-01-16", "2024-02-01")` (a commit, the negative control): run band A = C byte-identical, D ≠ C. A = C doubles as this populated state's jitter control (the mirror holds no clock: `generated_at` is `UNIX_EPOCH` and nothing draws it) |
//! | `the_cadence_badge_reads_an_rfc3339_custom_range` (F7) | Yahoo. A = `("2024-01-01T12:00:00Z", "2024-01-31T00:00:00Z")`, B = `("2024-01-01", "2024-01-31")`, C = `("2024-01-01", "2024-01-03")`: badge A = B ("1h"), B ≠ C ("1m"). No `now` reaches these frames: every bound parses |
//! | `the_whole_rfc3339_bound_is_visible_in_its_field` (F8) | inside the start field's text interior: F1 vs F2b (only the trailing `Z`) differ, `2024-01-01T01:00:00+01:00` vs `…+01:01` (both valid, only the 25th character) differ, two identical builds do not (the control), and the interior holds glyph ink |
//! | `measure` (`#[ignore]`) | prints the `DOWN_500`/`ACCENT` row profile the bands were read from, F6's run-band diff, F7's badge chip and the widest 25-char runs' glyph extent, and writes the PNGs |
//!
//! The T-T4 probes that redden these tests (P2, P2e, P3, P-trunc, P21, P22, P4, P6, P11m, P16,
//! P23, P26, H1b, H2) are recorded with their measured red sets in bug-log `#230`.
//!
//! ## macOS gate (ADR-0057 § D2)
//!
//! Real-renderer pixel assertions are macOS-canonical. `scripts/ci_test_target_counts.py`
//! declares this target at 11 on macOS (10 tests + the ignored `measure`, which `--list`
//! includes) and 0 on Windows.
//!
//! PNGs: set `UI_DUMP_PNG` to a directory (else the OS temp dir is used):
//! `lab_custom_230_<frame>.png`.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::path::PathBuf;
use std::time::Duration;

use smol_str::SmolStr;
use trading_core::{StrategyId, Symbol, Venue};
use ui::lab::equity_loader::LabTuple;
use ui::lab::state::{DateRange, LabDataSource};
use ui::state::{Cockpit, Message, Screen, update};
use ui::test_support::program_from_build;

const W: u32 = 1600;
const H: u32 = 900;

/// The editor band's rows (module doc).
const EDITOR_TOP: u32 = 165;
const EDITOR_BOTTOM: u32 = 288;

/// The run band's rows (module doc).
const RUN_TOP: u32 = 242;
const RUN_BOTTOM: u32 = 620;

/// The cadence badge rect (module doc).
const BADGE_TOP: u32 = 163;
const BADGE_BOTTOM: u32 = 193;
const BADGE_X0: u32 = 649;
const BADGE_X1: u32 = 693;

/// The start field's left border column (every frame: the picker column starts here).
const START_FIELD_X0: u32 = 196;

/// The start field's text interior (module doc): x `[x0, x1)`, rows `[top, bottom)`.
const START_TEXT: (u32, u32, u32, u32) = (201, START_TEXT_X1, 213, 234);
const START_TEXT_X1: u32 = 401;

/// F8's 25-character bounds: both valid, differing only in the 25th character.
const WIDE_A: &str = "2024-01-01T01:00:00+01:00";
const WIDE_B: &str = "2024-01-01T01:00:00+01:01";

/// F1's RFC 3339 pair.
const F1_START: &str = "2024-01-01T00:00:00Z";
const F1_END: &str = "2024-02-01T00:00:00Z";

fn custom(start: &str, end: &str) -> DateRange {
    DateRange::Custom {
        start_raw: SmolStr::new(start),
        end_raw: SmolStr::new(end),
    }
}

/// One keystroke in a Custom field: the production message, both texts as shown.
fn keystroke(start: &str, end: &str) -> Message {
    Message::LabCustomRangeEdited {
        start_raw: SmolStr::new(start),
        end_raw: SmolStr::new(end),
    }
}

/// The base Lab over `range` (module doc), Synthetic.
fn lab(range: DateRange) -> Cockpit {
    let mut c = ui::test_support::charts_screen_cockpit();
    c.current_screen = Screen::Lab;
    c.lab_state.strategy = Some(StrategyId::new("v0.sma"));
    c.lab_state.pair = Some((Venue::Binance, Symbol::new("BTCUSDT")));
    c.lab_report_roots = Vec::new();
    c.lab_state.range = range;
    c
}

/// The base Lab over `range`, Yahoo (assigned directly: `LabSelectDataSource` would probe the
/// box's Yahoo cache into the frame).
fn lab_yahoo(range: DateRange) -> Cockpit {
    let mut c = lab(range);
    c.lab_state.data_source = LabDataSource::YahooCache;
    c
}

/// `c` with a last run keyed to its own tuple, so the Lab draws it (KPI strip, "Re-run").
fn with_last_run(mut c: Cockpit) -> Cockpit {
    let tuple = LabTuple::new(
        &StrategyId::new("v0.sma"),
        Venue::Binance,
        &Symbol::new("BTCUSDT"),
        c.lab_state.range.clone(),
        c.lab_state.data_source,
    );
    let (mut last, _) = ui::fixtures::fake_run_report_mirror_pair();
    last.tuple = tuple;
    c.lab_state.last_run_report = Some(last);
    c
}

/// F6's base: a committed, valid pair and a last run.
fn f6_base() -> Cockpit {
    with_last_run(lab(custom("2024-01-15", "2024-02-01")))
}

/// `build` then one keystroke through the production `update`.
fn typed(build: fn() -> Cockpit, start: &'static str, end: &'static str) -> impl Fn() -> Cockpit {
    move || {
        let mut c = build();
        update(&mut c, keystroke(start, end));
        c
    }
}

/// Render the cockpit `build` returns, built inside `boot` (a fresh screenshot per call).
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

fn dump_dir() -> PathBuf {
    std::env::var_os("UI_DUMP_PNG")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(std::env::temp_dir)
}

fn dump_png(rgba: &[u8], frame: &str) {
    let path = dump_dir().join(format!("lab_custom_230_{frame}.png"));
    if let Some(img) = image::RgbaImage::from_raw(W, H, rgba.to_vec()) {
        let _ = img.save(&path);
        eprintln!("PNG {}", path.display());
    }
}

fn maybe_dump(rgba: &[u8], frame: &str) {
    if std::env::var_os("UI_DUMP_PNG").is_some() {
        dump_png(rgba, frame);
    }
}

/// A rectangle of the frame: rows `[top, bottom)`, columns `[x0, x1)`.
#[derive(Clone, Copy, Debug)]
struct Band {
    top: u32,
    bottom: u32,
    x0: u32,
    x1: u32,
}

fn sidebar() -> u32 {
    ui::theme::layout::SIDEBAR_WIDTH_PX as u32
}

fn editor() -> Band {
    Band {
        top: EDITOR_TOP,
        bottom: EDITOR_BOTTOM,
        x0: sidebar(),
        x1: W,
    }
}

fn run_band() -> Band {
    Band {
        top: RUN_TOP,
        bottom: RUN_BOTTOM,
        x0: sidebar(),
        x1: W,
    }
}

fn badge() -> Band {
    Band {
        top: BADGE_TOP,
        bottom: BADGE_BOTTOM,
        x0: BADGE_X0,
        x1: BADGE_X1,
    }
}

fn start_text() -> Band {
    Band {
        x0: START_TEXT.0,
        x1: START_TEXT.1,
        top: START_TEXT.2,
        bottom: START_TEXT.3,
    }
}

/// Pixels in `band` NOT within ±6 per channel of `token` (glyph ink over a flat fill).
fn count_not(rgba: &[u8], band: Band, token: ui::theme::color::ModeColor) -> u64 {
    let target = rgb(token);
    bytes(rgba, band)
        .chunks_exact(4)
        .filter(|p| !(0..3).all(|k| (i32::from(p[k]) - target[k]).abs() <= 6))
        .count() as u64
}

fn whole() -> Band {
    Band {
        top: 0,
        bottom: H,
        x0: 0,
        x1: W,
    }
}

/// The band's RGBA bytes.
fn bytes(rgba: &[u8], band: Band) -> Vec<u8> {
    let mut out = Vec::new();
    for y in band.top..band.bottom {
        let row = (y * W) as usize * 4;
        out.extend_from_slice(&rgba[row + band.x0 as usize * 4..row + band.x1 as usize * 4]);
    }
    out
}

/// Pixels that differ between two frames inside `band`.
fn differing(a: &[u8], b: &[u8], band: Band) -> usize {
    bytes(a, band)
        .chunks_exact(4)
        .zip(bytes(b, band).chunks_exact(4))
        .filter(|(p, q)| p != q)
        .count()
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

/// The editor band's `DOWN_500` of a Synthetic Lab over `(start, end)`.
fn editor_down(start: &'static str, end: &'static str, frame: &str) -> u64 {
    let f = render_built(move || lab(custom(start, end)));
    maybe_dump(&f, frame);
    let d = down(&f, editor());
    eprintln!("{frame}: ({start:?}, {end:?}) editor-band DOWN_500 = {d}");
    d
}

/// F1 — an RFC 3339 Custom range draws no invalid mark (HEAD drew both bounds red).
#[test]
fn an_rfc3339_custom_range_draws_no_invalid_mark() {
    let d = editor_down(F1_START, F1_END, "f1");
    assert_eq!(
        d, 0,
        "F1: an RFC 3339 pair is valid to the run's parse, so the editor draws no red: \
         editor-band DOWN_500 = {d} (want 0; HEAD drew 648)"
    );
}

/// F2 — the detector control: a malformed bound draws the invalid mark.
#[test]
fn a_malformed_bound_draws_the_invalid_mark() {
    let d = editor_down("2024-1-1", "2024-02-01", "f2");
    assert!(
        d > 0,
        "F2: a malformed start draws a red border and hint: editor-band DOWN_500 = {d} (want > 0)"
    );
}

/// F2b — F1 minus the start's `Z`: an RFC 3339 bound without an offset is invalid (F1's #161
/// partner: the two frames differ only in the offset and expect opposite outcomes).
#[test]
fn an_rfc3339_bound_without_an_offset_draws_the_invalid_mark() {
    let d = editor_down("2024-01-01T00:00:00", F1_END, "f2b");
    assert!(
        d > 0,
        "F2b: a bound with no offset does not parse, so it is red: editor-band DOWN_500 = {d} \
         (want > 0)"
    );
}

/// F3a — a calendar-invalid bound draws the invalid mark (HEAD's shape check drew it valid).
#[test]
fn a_calendar_invalid_bound_draws_the_invalid_mark() {
    let d = editor_down("2024-02-30", "2024-03-01", "f3a");
    assert!(
        d > 0,
        "F3a: 2024-02-30 is refused by the run's parse, so it is red: editor-band DOWN_500 = {d} \
         (want > 0; HEAD drew 0)"
    );
}

/// F3b — a leap day draws no invalid mark (F3a's #161 partner).
#[test]
fn a_leap_day_bound_draws_no_invalid_mark() {
    let d = editor_down("2024-02-29", "2024-03-01", "f3b");
    assert_eq!(
        d, 0,
        "F3b: 2024-02-29 parses, so the editor draws no red: editor-band DOWN_500 = {d} (want 0)"
    );
}

/// F4 — an uncommitted edit draws the typed text: the editor band of the draft frame is the
/// frame of that text committed (what HEAD drew), and not the committed pair's.
#[test]
fn an_uncommitted_edit_draws_the_typed_text() {
    fn committed() -> Cockpit {
        lab(custom("2024-01-15", "2024-02-01"))
    }
    let a = render_built(typed(committed, "2024-01-1", "2024-02-01"));
    let b = render_built(|| lab(custom("2024-01-1", "2024-02-01")));
    let c = render_built(committed);
    maybe_dump(&a, "f4_a");
    maybe_dump(&b, "f4_b");
    maybe_dump(&c, "f4_c");
    let ab = differing(&a, &b, editor());
    let ac = differing(&a, &c, editor());
    let (down_a, down_c) = (down(&a, editor()), down(&c, editor()));
    eprintln!("F4: diff(A,B) = {ab}, diff(A,C) = {ac}, down(A) = {down_a}, down(C) = {down_c}");
    assert!(
        ab == 0 && ac > 0 && down_a > 0 && down_c == 0,
        "F4: the draft frame draws the typed text: editor-band differing px A vs B = {ab} \
         (want 0: the draft draws as that text committed), A vs C = {ac} (want > 0: not the \
         committed pair), DOWN_500 A = {down_a} (want > 0: the partial bound is red), C = \
         {down_c} (want 0)"
    );
}

/// F5 — the jitter control: two independent F1 builds render the same bytes.
#[test]
fn the_same_custom_state_renders_identically() {
    let a = render_built(|| lab(custom(F1_START, F1_END)));
    let b = render_built(|| lab(custom(F1_START, F1_END)));
    let n = differing(&a, &b, whole());
    assert_eq!(
        n, 0,
        "F5: two F1 builds must be byte-identical: {n} px differ"
    );
}

/// F6 — a partial bound keeps the last run drawn; a commit (the negative control) does not.
#[test]
fn a_partial_custom_bound_keeps_the_last_run_drawn() {
    let c = render_built(f6_base);
    let a = render_built(typed(f6_base, "", "2024-02-01"));
    let d = render_built(typed(f6_base, "2024-01-16", "2024-02-01"));
    maybe_dump(&c, "f6_c");
    maybe_dump(&a, "f6_a");
    maybe_dump(&d, "f6_d");
    let ac = differing(&a, &c, run_band());
    let dc = differing(&d, &c, run_band());
    let (accent_c, accent_d) = (accent(&c, run_band()), accent(&d, run_band()));
    eprintln!("F6: run band A vs C = {ac}, D vs C = {dc}; ACCENT C = {accent_c}, D = {accent_d}");
    assert!(
        ac == 0 && dc > 0,
        "F6: run-band differing px A (a draft keystroke) vs C (no keystroke) = {ac} (want 0: \
         the last run stays drawn); D (a committing keystroke) vs C = {dc} (want > 0: the \
         negative control, a commit clears the last run). HEAD measured A vs C > 0"
    );
}

/// F7 — the Yahoo cadence badge reads an RFC 3339 Custom range by the run's parse.
#[test]
fn the_cadence_badge_reads_an_rfc3339_custom_range() {
    let a = render_built(|| lab_yahoo(custom("2024-01-01T12:00:00Z", "2024-01-31T00:00:00Z")));
    let b = render_built(|| lab_yahoo(custom("2024-01-01", "2024-01-31")));
    let c = render_built(|| lab_yahoo(custom("2024-01-01", "2024-01-03")));
    maybe_dump(&a, "f7_a");
    maybe_dump(&b, "f7_b");
    maybe_dump(&c, "f7_c");
    let ab = differing(&a, &b, badge());
    let bc = differing(&b, &c, badge());
    eprintln!("F7: badge A vs B = {ab}, B vs C = {bc}");
    assert!(
        ab == 0 && bc > 0,
        "F7: badge-rect differing px A (29.5 days with a time) vs B (30 days, \"1h\") = {ab} \
         (want 0); B vs C (2 days, \"1m\") = {bc} (want > 0: the rect holds the badge). HEAD \
         drew A as \"1m\""
    );
}

/// F8 (bug-log `#230` review) — the whole RFC 3339 bound is visible in its field: frames that
/// differ only in a bound's LAST character differ inside the start field's text interior (not
/// only in its border or a hint). At the old 120 px width the field clipped a bound after about
/// 15 characters, so (a) F1 and F2b (only the trailing `Z` differs) and (b) two valid 25-char
/// bounds that differ only in their 25th character drew byte-identical interiors.
#[test]
fn the_whole_rfc3339_bound_is_visible_in_its_field() {
    let f1 = render_built(|| lab(custom(F1_START, F1_END)));
    let f2b = render_built(|| lab(custom("2024-01-01T00:00:00", F1_END)));
    let wide_a = render_built(|| lab(custom(WIDE_A, "2024-02-01")));
    let wide_b = render_built(|| lab(custom(WIDE_B, "2024-02-01")));
    let wide_a2 = render_built(|| lab(custom(WIDE_A, "2024-02-01")));
    maybe_dump(&wide_a, "f8_wide_a");
    maybe_dump(&wide_b, "f8_wide_b");
    let z = differing(&f1, &f2b, start_text());
    let last = differing(&wide_a, &wide_b, start_text());
    let same = differing(&wide_a, &wide_a2, start_text());
    let ink = count_not(&wide_a, start_text(), ui::theme::color::PANEL);
    eprintln!("F8: Z {z} px, 25th char {last} px, identical {same} px, ink {ink} px");
    assert!(
        z > 0 && last > 0 && same == 0 && ink > 0,
        "F8: inside the start field's text interior {START_TEXT:?}: F1 vs F2b (the trailing Z) \
         differ by {z} px (want > 0); {WIDE_A:?} vs {WIDE_B:?} (the 25th character) by {last} \
         px (want > 0); two identical builds by {same} px (want 0, the control); the interior \
         holds {ink} px of glyph ink (want > 0)"
    );
}

/// Prints the row profiles the bands were read from, F6's run-band diff and F7's badge chip,
/// and writes the PNGs.
#[test]
#[ignore = "measurement: run by hand to re-derive the bands (ADR-0092)"]
fn measure() {
    let x0 = sidebar();
    let two_hint = render_built(|| lab(custom("2024-1-1", "2024-2-1")));
    let f1 = render_built(|| lab(custom(F1_START, F1_END)));
    let f2 = render_built(|| lab(custom("2024-1-1", "2024-02-01")));
    let f6c = render_built(f6_base);
    let f6d = render_built(typed(f6_base, "2024-01-16", "2024-02-01"));
    let f7b = render_built(|| lab_yahoo(custom("2024-01-01", "2024-01-31")));
    for (name, f) in [
        ("measure_two_hint", &two_hint),
        ("measure_f1", &f1),
        ("measure_f2", &f2),
        ("measure_f6c", &f6c),
        ("measure_f6d", &f6d),
        ("measure_f7b", &f7b),
    ] {
        dump_png(f, name);
    }
    for (name, f) in [("two_hint", &two_hint), ("f1", &f1), ("f2", &f2)] {
        for y in 160..400 {
            let row = Band {
                top: y,
                bottom: y + 1,
                x0,
                x1: W,
            };
            let (d, a) = (down(f, row), accent(f, row));
            if d > 0 || a > 0 {
                eprintln!("{name} row {y}: DOWN_500={d} ACCENT={a}");
            }
        }
    }
    for y in 236..300 {
        let row = Band {
            top: y,
            bottom: y + 1,
            x0,
            x1: W,
        };
        let a = accent(&f6c, row);
        if a > 0 {
            eprintln!("f6c row {y}: ACCENT={a}");
        }
    }
    eprintln!(
        "f6: run band [{RUN_TOP}, {RUN_BOTTOM}) D vs C differing = {}",
        differing(&f6d, &f6c, run_band())
    );
    let panel = rgb(ui::theme::color::PANEL_RAISED);
    let border = rgb(ui::theme::color::BORDER_1);
    let mut bbox: Option<(u32, u32, u32, u32)> = None;
    for y in 150..198 {
        for x in 600..760 {
            let i = ((y * W + x) * 4) as usize;
            let p = &f7b[i..i + 4];
            let near = |t: [i32; 3]| (0..3).all(|k| (i32::from(p[k]) - t[k]).abs() <= 3);
            if near(panel) || near(border) {
                bbox = Some(match bbox {
                    None => (x, y, x, y),
                    Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x), d.max(y)),
                });
            }
        }
    }
    eprintln!("f7: the badge chip (x0, y0, x1, y1) = {bbox:?}");
    // The widest RFC 3339 bounds' glyph extent inside the start field's text interior. While a
    // run fits, this equals its unclipped extent (measured with the field at 400 px: 181-190 px
    // from the left border); a run that does not fit ends at the interior's edge
    // (`START_TEXT_X1`), which is how a too-narrow `widgets::date_range` width shows here.
    let border = rgb(ui::theme::color::BORDER_1);
    let panel_bg = rgb(ui::theme::color::PANEL);
    let near = |f: &[u8], x: u32, y: u32, t: [i32; 3], tol: i32| {
        let i = ((y * W + x) * 4) as usize;
        (0..3).all(|k| (i32::from(f[i + k]) - t[k]).abs() <= tol)
    };
    for bound in [
        "2024-01-01T01:00:00+01:00",
        "8888-88-88T88:88:88+88:88",
        "0000-00-00T00:00:00+00:00",
        "4444-44-44T44:44:44-44:44",
    ] {
        let wide = render_built(move || lab(custom(bound, "2024-02-01")));
        let left = START_FIELD_X0;
        let red = rgb(ui::theme::color::DOWN_500);
        let mut glyph_right = left;
        for y in 212..234 {
            for x in left + 2..START_TEXT_X1 {
                if !near(&wide, x, y, panel_bg, 6)
                    && !near(&wide, x, y, border, 6)
                    && !near(&wide, x, y, red, 6)
                {
                    glyph_right = glyph_right.max(x);
                }
            }
        }
        eprintln!(
            "wide {bound:?}: field left border x {left}; glyphs end at x {glyph_right} \
             ({} px from the left border)",
            glyph_right - left
        );
    }
}
