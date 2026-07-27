//! Group-B proactive pixel-layer render audit (ui-debugger sweep, 2026-06-18).
//!
//! NOT a regression of a reported bug — a forward sweep over the strategy /
//! model / run surfaces to catch latent "blank panel / no graph / placeholder
//! where data should be" bugs BEFORE the operator hits them. Each screen is
//! rendered HEADLESS through the REAL shell (`program_from_cockpit` →
//! `shell::view`) at 1920×1080 in its POPULATED, data-bearing state, the RGBA
//! readback is dumped to `/tmp/ui-audit/group-b/<screen>-<state>.png`, and a
//! NEGATIVE CONTROL (cold/empty) render is dumped alongside so "the data drew"
//! is provable by contrast.
//!
//! Screens (Group B):
//!   - strategies        → `screens::strategy_registry::view` (the real route)
//!   - strategy_registry → same route (card stack)
//!   - models            → `screens::models::view` (checkpoint rows)
//!   - lab               → `screens::lab::view` (run chart)
//!   - memory            → `screens::memory::view` (lesson cards)
//!
//! macOS-gated (ADR-0057 D2) — cosmic-text rasterisation is per-OS; pixel
//! assertions are macOS-canonical. Compiles to nothing elsewhere.

#![cfg(target_os = "macos")]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
#![allow(non_snake_case)]

use std::time::Duration;

use smol_str::SmolStr;
use ui::state::{Cockpit, PanelState, Screen};
use ui::test_support::{charts_screen_cockpit, program_from_cockpit};
use ui::theme::{ThemeMode, color};

const W: u32 = 1920;
const H: u32 = 1080;

/// Render the full cockpit shell for `cockpit` and return `(w, h, rgba)`.
fn render(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
    ui::force_chart_utc_for_tests();
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;
    let shot = iced_test::screenshot(&program, &theme, (W, H), 1.0, Duration::ZERO);
    (shot.size.width, shot.size.height, shot.rgba.to_vec())
}

/// Save the RGBA buffer as a PNG for human (and Read-tool) inspection.
fn save(name: &str, w: u32, h: u32, rgba: &[u8]) {
    // The debug-dump dir is NOT guaranteed to exist (macOS purges /tmp);
    // without this, `img.save` panics and takes all five audit tests —
    // and, via cargo's fail-fast, the alphabetically-later ui suite —
    // down with it (2026-07-26 story 1-10 review, found during gate re-run).
    let dir = "/tmp/ui-audit/group-b";
    std::fs::create_dir_all(dir).expect("create audit dump dir");
    let path = format!("{dir}/{name}.png");
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.to_vec()) {
        img.save(&path).expect("png save");
    }
    eprintln!("[audit] wrote {path}");
}

// ── pixel classifiers ─────────────────────────────────────────────────────

fn rgb_at(rgba: &[u8], w: u32, x: u32, y: u32) -> (i32, i32, i32) {
    let idx = ((y as usize * w as usize) + x as usize) * 4;
    (
        i32::from(rgba[idx]),
        i32::from(rgba[idx + 1]),
        i32::from(rgba[idx + 2]),
    )
}

/// `ACCENT` (dark theme) reference rgb 0-255.
fn accent_rgb() -> (i32, i32, i32) {
    let c = color::ACCENT.current(ThemeMode::Dark);
    (
        (c.r * 255.0).round() as i32,
        (c.g * 255.0).round() as i32,
        (c.b * 255.0).round() as i32,
    )
}

fn dist2(p: (i32, i32, i32), q: (i32, i32, i32)) -> i32 {
    let (dr, dg, db) = (p.0 - q.0, p.1 - q.1, p.2 - q.2);
    dr * dr + dg * dg + db * db
}

/// Count `ACCENT`-coloured pixels in the screen body (x > 12% to skip the
/// sidebar's active-item ACCENT highlight). Tolerance r² = 1200 absorbs AA.
fn accent_body_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    let target = accent_rgb();
    let x0 = (w as f32 * 0.12) as u32;
    let mut n = 0u64;
    for y in 0..h {
        for x in x0..w {
            if dist2(rgb_at(rgba, w, x, y), target) <= 1200 {
                n += 1;
            }
        }
    }
    n
}

/// Count "foreground" pixels in a rectangular region: glyph text, chart
/// strokes, accent fills — anything markedly BRIGHTER than the dark cockpit
/// chrome. All Lumen dark surfaces (`CANVAS`/`PANEL`/`PANEL_RAISED`/
/// `PANEL_SUNKEN`) have max-channel < ~46; FG text + chart ink + accent chips
/// all clear ~90. A luminance gate at 70 cleanly separates content from every
/// dark background AND from the card-row `PANEL_RAISED` fills (which the
/// earlier hue-distance classifier wrongly counted as "must-exclude chrome",
/// hiding the text that sits ON those rows). This is the right "is there
/// rendered content here" probe for non-ACCENT card/list/chart regions.
fn content_pixels_in(w: u32, rgba: &[u8], x0: u32, y0: u32, x1: u32, y1: u32) -> u64 {
    let mut n = 0u64;
    for y in y0..y1 {
        for x in x0..x1 {
            let (r, g, b) = rgb_at(rgba, w, x, y);
            // Rec.601-ish luma; integer math. Dark chrome lands < ~46.
            let luma = (r * 2 + g * 5 + b) / 8;
            if luma > 70 {
                n += 1;
            }
        }
    }
    n
}

// ── state builders ─────────────────────────────────────────────────────────

/// Strategies / strategy_registry POPULATED — `Screen::Strategies` routes to
/// `strategy_registry::view`. `fake_cockpit_v15a_pairs_steady_state` seeds
/// `strategies = Ready(rows)` + recent events, so the card stack populates.
fn strategies_populated() -> Cockpit {
    let mut c = ui::fixtures::fake_cockpit_v15a_pairs_steady_state();
    c.current_screen = Screen::Strategies;
    c
}

/// Strategies NEGATIVE control — empty registry.
fn strategies_empty() -> Cockpit {
    let mut c = ui::fixtures::fake_cockpit_v15a_pairs_steady_state();
    c.current_screen = Screen::Strategies;
    c.strategies = PanelState::Empty;
    c
}

/// Models POPULATED — 2 TCN checkpoint rows (mirrors the live workstation
/// state; same shape as `tests/fixtures::models__steady_state_2_checkpoints`).
fn models_populated() -> Cockpit {
    use ui::models::state::{CheckpointMeta, ModelFamily, ModelStatus, ModelsScreenState};

    let checkpoints = vec![
        CheckpointMeta {
            model_revision: SmolStr::new(
                "d1c3696d1f2a8e3b5c7d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b",
            ),
            family: ModelFamily::Tcn,
            data_span_start: SmolStr::new("2023-01-01"),
            data_span_end: SmolStr::new("2024-12-31"),
            interval: SmolStr::new("1h"),
            symbols_count: 10,
            final_val_loss: 0.0312,
            final_train_loss: 0.0287,
            sigma_train: 0.085,
            weights_sha256: SmolStr::new("d1c3696d"),
            file_size_bytes: 855,
            status: ModelStatus::Staged,
            source_path: std::path::PathBuf::from(
                "crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d.metadata.json",
            ),
        },
        CheckpointMeta {
            model_revision: SmolStr::new(
                "3fabcabe4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f",
            ),
            family: ModelFamily::Tcn,
            data_span_start: SmolStr::new("2023-06-01"),
            data_span_end: SmolStr::new("2025-02-28"),
            interval: SmolStr::new("1h"),
            symbols_count: 10,
            final_val_loss: 0.0298,
            final_train_loss: 0.0271,
            sigma_train: 0.079,
            weights_sha256: SmolStr::new("3fabcabe"),
            file_size_bytes: 852,
            status: ModelStatus::Staged,
            source_path: std::path::PathBuf::from(
                "crates/forecast/checkpoints/anchors/tcn-bs2-3fabcabe.metadata.json",
            ),
        },
    ];

    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = Screen::Models;
    cockpit.models_screen_state = ModelsScreenState {
        checkpoints,
        last_indexed: Some(SmolStr::new("2026-05-20T10:00:00Z")),
        ..ModelsScreenState::default()
    };
    cockpit
}

/// Models NEGATIVE control — cold boot, no checkpoints (empty state).
fn models_empty() -> Cockpit {
    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = Screen::Models;
    cockpit.models_screen_state = Default::default();
    cockpit
}

/// Memory POPULATED — 5 lesson cards (mix Win/Loss/Scratch).
fn memory_populated() -> Cockpit {
    use ui::memory::state::{LessonCardCard, MemoryScreenState};

    let cards = vec![
        LessonCardCard {
            card_id: SmolStr::new("card_e"),
            symbol_or_pair: SmolStr::new("BTCUSDT"),
            closed_at: SmolStr::new("2026-01-05T12:00:00Z"),
            strategy_id: SmolStr::new("v1.momentum"),
            signed_pnl_display: SmolStr::new("+85.00 USDT"),
            outcome_class: SmolStr::new("Win"),
            note: Some(SmolStr::new("Trend continuation confirmed.")),
            close_transaction_id: Some(SmolStr::new("tx-e001")),
        },
        LessonCardCard {
            card_id: SmolStr::new("card_d"),
            symbol_or_pair: SmolStr::new("ETHUSDT"),
            closed_at: SmolStr::new("2026-01-04T09:30:00Z"),
            strategy_id: SmolStr::new("v1.momentum"),
            signed_pnl_display: SmolStr::new("-23.50 USDT"),
            outcome_class: SmolStr::new("Loss"),
            note: None,
            close_transaction_id: None,
        },
        LessonCardCard {
            card_id: SmolStr::new("card_c"),
            symbol_or_pair: SmolStr::new("SOLUSDT"),
            closed_at: SmolStr::new("2026-01-03T15:00:00Z"),
            strategy_id: SmolStr::new("sma_crossover"),
            signed_pnl_display: SmolStr::new("+2.10 USDT"),
            outcome_class: SmolStr::new("Scratch"),
            note: None,
            close_transaction_id: None,
        },
        LessonCardCard {
            card_id: SmolStr::new("card_b"),
            symbol_or_pair: SmolStr::new("BTCUSDT"),
            closed_at: SmolStr::new("2026-01-02T08:00:00Z"),
            strategy_id: SmolStr::new("v1.momentum"),
            signed_pnl_display: SmolStr::new("+140.00 USDT"),
            outcome_class: SmolStr::new("Win"),
            note: Some(SmolStr::new("Double top breakout.")),
            close_transaction_id: None,
        },
        LessonCardCard {
            card_id: SmolStr::new("card_a"),
            symbol_or_pair: SmolStr::new("ETHUSDT"),
            closed_at: SmolStr::new("2026-01-01T06:00:00Z"),
            strategy_id: SmolStr::new("sma_crossover"),
            signed_pnl_display: SmolStr::new("-11.00 USDT"),
            outcome_class: SmolStr::new("Loss"),
            note: None,
            close_transaction_id: None,
        },
    ];

    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = Screen::Memory;
    cockpit.memory_screen_state = MemoryScreenState {
        cache: cards,
        last_indexed: Some(SmolStr::new("2026-01-05T12:01:00Z")),
        ..MemoryScreenState::default()
    };
    cockpit
}

/// Memory POPULATED + drawer open on the first card.
fn memory_drawer_open() -> Cockpit {
    use ui::memory::state::MemoryScreenState;
    let mut cockpit = memory_populated();
    cockpit.memory_screen_state = MemoryScreenState {
        drawer_open: Some(SmolStr::new("card_e")),
        ..cockpit.memory_screen_state
    };
    cockpit
}

/// Memory NEGATIVE control — empty cache.
fn memory_empty() -> Cockpit {
    use ui::memory::state::MemoryScreenState;
    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = Screen::Memory;
    cockpit.memory_screen_state = MemoryScreenState::default();
    cockpit
}

/// Lab POPULATED — Charts/Lab screen with 60 synthetic bars + 4 fill markers
/// for the active BTCUSDT pair, so the chart canvas paints a price series +
/// buy/sell triangles. `charts_screen_cockpit` already seeds this.
fn lab_populated() -> Cockpit {
    let mut c = charts_screen_cockpit();
    c.current_screen = Screen::Lab;
    c
}

/// Lab NEGATIVE control — Lab screen with NO bars and NO active symbol, so the
/// chart canvas paints its empty placeholder.
fn lab_empty() -> Cockpit {
    let mut c = ui::fixtures::fake_cockpit_ready();
    c.current_screen = Screen::Lab;
    c.selected_symbol = None;
    c.universe = Vec::new();
    c.chart_markers = PanelState::Empty;
    c
}

// ── tests: one positive + reuse negative control per screen ────────────────

/// **strategies / strategy_registry** — the populated card stack must paint
/// visible content (strategy id headers, status pills, body rows). The ACCENT
/// status pill on every card is the discriminator: a populated registry paints
/// many ACCENT body pixels; the empty registry paints near-none.
#[test]
fn strategies_registry_populated_paints_cards() {
    let (w, h, pop) = render(strategies_populated());
    save("strategies-populated", w, h, &pop);
    let (w2, h2, empty) = render(strategies_empty());
    save("strategies-empty", w2, h2, &empty);

    // The populated card paints the ACCENT "shipped" status-pill underline +
    // teal pill text (the only body ACCENT source on this screen). The empty
    // registry paints a muted placeholder with ZERO body ACCENT. Measured
    // (macOS, 1920×1080): populated ≈ 3091 ACCENT px, empty ≈ 0.
    let pop_accent = accent_body_pixels(w, h, &pop);
    let empty_accent = accent_body_pixels(w2, h2, &empty);
    // Card body content (id header + universe/anchor/last-run rows + button)
    // — measured ≈ 4690 populated.
    let pop_card = content_pixels_in(w, &pop, (w as f32 * 0.12) as u32, 40, w, 280);

    assert!(
        pop_accent > 1500,
        "populated strategy registry must paint the ACCENT 'shipped' status \
         pill on the card (expected >1500 ACCENT px, got {pop_accent}; PNG \
         /tmp/ui-audit/group-b/strategies-populated.png)"
    );
    assert!(
        pop_card > 2500,
        "populated strategy registry card must paint its body rows (id, \
         universe, anchor, last-run, button) — expected >2500 fg px, got \
         {pop_card}; PNG /tmp/ui-audit/group-b/strategies-populated.png"
    );
    assert!(
        pop_accent > empty_accent + 1000,
        "populated registry ({pop_accent} ACCENT px) must paint clearly more \
         than the empty registry ({empty_accent} px) — proving the card drew, \
         not chrome"
    );
}

/// **models** — the populated checkpoint list must paint visible rows. The
/// family cell ("TCN") is `ACCENT`-coloured on every checkpoint row, so a
/// populated list paints many ACCENT body px while the empty state (only the
/// muted "No checkpoints" text + disabled chips) paints far fewer.
#[test]
fn models_populated_paints_checkpoint_rows() {
    let (w, h, pop) = render(models_populated());
    save("models-populated", w, h, &pop);
    let (w2, h2, empty) = render(models_empty());
    save("models-empty", w2, h2, &empty);

    // NOTE: `accent_body_pixels` is NOT a discriminator here — the ACCENT "TCN"
    // family + "staged" status TOOLBAR chips render in BOTH states (≈2160 px
    // each), dwarfing the small per-row ACCENT family cells. The correct probe
    // is the CHECKPOINT-ROW band directly under the toolbar (y≈45-115): the two
    // rows' text glyphs (TCN/rev/span/pill/size) are foreground content the
    // empty "No models loaded yet" placeholder lacks. Measured: populated ≈
    // 4419 fg px, empty ≈ 1593 (toolbar bleed) — a 2.8× separation.
    let pop_rows = content_pixels_in(w, &pop, (w as f32 * 0.12) as u32, 45, w, 115);
    let empty_rows = content_pixels_in(w2, &empty, (w2 as f32 * 0.12) as u32, 45, w2, 115);

    assert!(
        pop_rows > 3000,
        "populated models screen must paint visible checkpoint-row content in \
         the row band (expected >3000 fg px, got {pop_rows}; PNG \
         /tmp/ui-audit/group-b/models-populated.png) — if this drops to ~the \
         empty baseline the rows stopped rendering despite a non-empty \
         checkpoint list"
    );
    assert!(
        pop_rows > empty_rows * 2,
        "populated models row band ({pop_rows} fg px) must far exceed the empty \
         state ({empty_rows} px, toolbar-only) — the 2 checkpoint rows drew"
    );
}

/// **memory** — the populated card list must paint visible rows. Each card row
/// has a `PANEL_RAISED` background and FG-1 text; the empty state paints only
/// the muted placeholder. Assert the card-list band has content + the Win-card
/// ACCENT outcome badges paint.
#[test]
fn memory_populated_paints_lesson_cards() {
    let (w, h, pop) = render(memory_populated());
    save("memory-populated", w, h, &pop);
    let (w2, h2, empty) = render(memory_empty());
    save("memory-empty", w2, h2, &empty);

    // Card-list band: below the toolbar (~y 50) down to y 280 covers all 5
    // compact rows. Measured: populated ≈ 5533 fg px, empty ≈ 1046 (toolbar +
    // placeholder) — a 5.3× separation.
    let pop_cards = content_pixels_in(w, &pop, (w as f32 * 0.12) as u32, 50, w, 280);
    let empty_cards = content_pixels_in(w2, &empty, (w2 as f32 * 0.12) as u32, 50, w2, 280);

    assert!(
        pop_cards > 3500,
        "populated memory screen must paint visible lesson-card rows (expected \
         >3500 fg px in the card band, got {pop_cards}; PNG \
         /tmp/ui-audit/group-b/memory-populated.png)"
    );
    assert!(
        pop_cards > empty_cards * 2,
        "populated memory card band ({pop_cards} fg px) must far exceed the \
         empty state ({empty_cards} px) — the 5 cards drew, not chrome"
    );
}

/// **memory drawer** — opening a card's side-drawer must paint the drawer in
/// the right half of the body without collapsing the card list.
#[test]
fn memory_drawer_open_paints_right_pane() {
    let (w, h, drawer) = render(memory_drawer_open());
    save("memory-drawer-open", w, h, &drawer);

    // Right rail (x > 82%) should carry drawer content: the "Memory Entry"
    // header, Symbol/Closed/Strategy/P&L/Outcome key-values, the Lesson body,
    // and "View in Trail" / "Close" links. Measured ≈ 3057 fg px.
    let right_pane = content_pixels_in(w, &drawer, (w as f32 * 0.82) as u32, 40, w, 320);
    assert!(
        right_pane > 1800,
        "memory drawer-open must paint the side-drawer content in the right \
         rail (expected >1800 fg px, got {right_pane}; PNG \
         /tmp/ui-audit/group-b/memory-drawer-open.png)"
    );
}

/// **lab** — the Lab/Charts screen with a seeded bar series + fill markers must
/// paint a non-trivial chart (price line + candles + triangles). Assert the
/// chart band has substantial content and far exceeds the empty-chart control.
#[test]
fn lab_populated_paints_chart() {
    let (w, h, pop) = render(lab_populated());
    save("lab-populated", w, h, &pop);
    let (w2, h2, empty) = render(lab_empty());
    save("lab-empty", w2, h2, &empty);

    // Chart sits in the middle of the body. Sample a generous central band
    // (x 15%..98%, y 30%..72%) — the candles/price line/markers are non-chrome.
    let x0 = (w as f32 * 0.15) as u32;
    let x1 = (w as f32 * 0.98) as u32;
    let y0 = (h as f32 * 0.30) as u32;
    let y1 = (h as f32 * 0.72) as u32;
    // Measured: populated chart ≈ 5356 fg px (candle/line series + buy/sell
    // triangles + legend + axis labels), empty chart ≈ 1000 ("No data" label +
    // axis chrome) — a 5.4× separation.
    let pop_chart = content_pixels_in(w, &pop, x0, y0, x1, y1);
    let empty_chart = content_pixels_in(w2, &empty, x0, y0, x1, y1);

    assert!(
        pop_chart > 3500,
        "populated Lab chart must paint a visible price series + markers \
         (expected >3500 fg px in the chart band, got {pop_chart}; PNG \
         /tmp/ui-audit/group-b/lab-populated.png) — if this collapses toward \
         the empty baseline the chart stopped rendering bars/markers"
    );
    assert!(
        pop_chart > empty_chart * 2,
        "populated Lab chart ({pop_chart} fg px) must far exceed the empty \
         'No data' chart ({empty_chart} px) — the bars/markers drew"
    );
}
