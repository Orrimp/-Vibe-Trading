//! GROUP C pixel-layer render audit (ui-debugger sweep, 2026-06-18).
//!
//! NOT a shipped guard yet — a proactive audit harness that renders the
//! POPULATED + cold (negative-control) state of every Group C screen
//! (home / control / risk / audit / trail / settings / debug) through the
//! REAL cockpit shell, writes each frame to `/tmp/ui-audit/group-c/`, and
//! counts coarse hue-pixels so a human (the ui-debugger) can `Read` each PNG.
//!
//! Routing reality (see `crate::shell::screen_body`):
//!   - risk    → Screen::Settings + settings_active_tab = Risk
//!   - control → Screen::Settings + settings_active_tab = Control
//!   - debug   → Screen::Settings + settings_active_tab = Debug
//!   - audit   → Screen::Trail (list mode = audit::view delegation)
//!   - trail   → Screen::Trail (trail mode, reconstructed_trail populated)
//!   - settings→ the rollup chrome itself (tab strip + active body)
//!   - home    → home::view (dead-routed in shell today, but a Group C file;
//!     rendered directly via a bespoke one-screen program)
//!
//! macOS-gated for pixel determinism (ADR-0057 / cosmic-text per-OS).

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::time::Duration;

use ui::state::{
    AuditScreenState, Cockpit, ExecutionMode, MarketHealthState, PanelState, Screen, SettingsTab,
};
use ui::test_support::program_from_cockpit;

// ─── render helper ───────────────────────────────────────────────────────

fn render_rgba(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
    ui::force_chart_utc_for_tests();
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;
    let shot = iced_test::screenshot(&program, &theme, (1920, 1080), 1.0, Duration::ZERO);
    (shot.size.width, shot.size.height, shot.rgba.to_vec())
}

fn save(name: &str, w: u32, h: u32, rgba: &[u8]) -> String {
    let path = format!("/tmp/ui-audit/group-c/{name}.png");
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.to_vec()) {
        let _ = img.save(&path);
    }
    path
}

/// Count "ink" pixels: anything meaningfully brighter than the darkest
/// chrome. The cockpit panel background is ~#1C2127 (luma ~33). Text +
/// bars + chips are clearly above. A populated data screen has FAR more
/// ink than a cold/empty one. Skips the left sidebar rail (x < 220) so the
/// always-present nav chrome doesn't dominate the body delta.
fn ink_pixels_body(w: u32, h: u32, rgba: &[u8]) -> u64 {
    let x0 = 220u32.min(w);
    let mut hits = 0u64;
    for y in 0..h {
        for x in x0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let r = i32::from(rgba[idx]);
            let g = i32::from(rgba[idx + 1]);
            let b = i32::from(rgba[idx + 2]);
            // luma-ish; panel base ~33, raised panels ~40. Count > 70 as ink.
            let luma = (r * 2 + g * 5 + b) / 8;
            if luma > 70 {
                hits += 1;
            }
        }
    }
    hits
}

/// Count saturated GREEN/teal pixels in the body (threshold bars in the
/// safe band, the trail `ACCENT` chevrons, sparkline). Distinguishes a
/// populated risk/threshold screen from a flat empty one.
fn green_pixels_body(w: u32, h: u32, rgba: &[u8]) -> u64 {
    let x0 = 220u32.min(w);
    let mut hits = 0u64;
    for y in 0..h {
        for x in x0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let r = i32::from(rgba[idx]);
            let g = i32::from(rgba[idx + 1]);
            let b = i32::from(rgba[idx + 2]);
            let teal = g > 110 && b > 100 && (g - b).abs() < 45 && (g - r) > 20;
            let sage = g > 90 && g > r + 14 && g > b + 14 && (40..150).contains(&r);
            if teal || sage {
                hits += 1;
            }
        }
    }
    hits
}

/// Count saturated RED pixels (DOWN_500 over-cap threshold bar, danger).
fn red_pixels_body(w: u32, h: u32, rgba: &[u8]) -> u64 {
    let x0 = 220u32.min(w);
    let mut hits = 0u64;
    for y in 0..h {
        for x in x0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let r = i32::from(rgba[idx]);
            let g = i32::from(rgba[idx + 1]);
            let b = i32::from(rgba[idx + 2]);
            if r > 130 && (r - g) > 45 && (r - b) > 35 {
                hits += 1;
            }
        }
    }
    hits
}

// ─── populated-state cockpit builders (per screen) ───────────────────────

/// Risk screen, FULLY populated: Settings rollup pinned to the Risk tab
/// with a Ready `risk_state` (3 exposure bars across green/warn/red bands +
/// daily-loss + kill-threshold bars).
fn cockpit_risk_populated() -> Cockpit {
    let mut c = ui::fixtures::fake_cockpit_ready();
    c.current_screen = Screen::Settings;
    c.settings_active_tab = SettingsTab::Risk;
    c.risk_state = PanelState::Ready(ui::fixtures::fake_risk_state());
    c
}

/// Risk screen NEGATIVE CONTROL: same route but risk_state Loading (the
/// cold-boot default) → the loading spinner / no bars.
fn cockpit_risk_cold() -> Cockpit {
    let mut c = ui::fixtures::fake_cockpit_ready();
    c.current_screen = Screen::Settings;
    c.settings_active_tab = SettingsTab::Risk;
    c.risk_state = PanelState::Loading;
    c
}

/// Control screen, populated: Settings rollup on the Control tab with Ready
/// risk_state (drives the daily-loss + max-position mirror rows) and Ready
/// pnl (drives the sign-coloured used-today row). ExecutionMode::Supervised
/// so the active segment is the middle one (proves the active highlight is
/// not always-leftmost).
fn cockpit_control_populated() -> Cockpit {
    let mut c = ui::fixtures::fake_cockpit_ready();
    c.current_screen = Screen::Settings;
    c.settings_active_tab = SettingsTab::Control;
    c.risk_state = PanelState::Ready(ui::fixtures::fake_risk_state());
    c.execution_mode = ExecutionMode::Supervised;
    c
}

/// Control screen NEGATIVE CONTROL: risk_state Loading → mirror rows show
/// placeholder dashes; ExecutionMode default Observe.
fn cockpit_control_cold() -> Cockpit {
    let mut c = ui::fixtures::fake_cockpit_ready();
    c.current_screen = Screen::Settings;
    c.settings_active_tab = SettingsTab::Control;
    c.risk_state = PanelState::Loading;
    c.execution_mode = ExecutionMode::Observe;
    c
}

/// Debug screen, populated: Settings rollup on the Debug tab with market
/// health (multi-venue), a server-time tick, recent tick + latency known.
fn cockpit_debug_populated() -> Cockpit {
    let mut c = ui::fixtures::fake_cockpit_ready();
    c.current_screen = Screen::Settings;
    c.settings_active_tab = SettingsTab::Debug;
    // Multi-venue market health so the per-venue rows have >1 entry.
    c.market_health
        .insert(trading_core::Venue::Binance, MarketHealthState::Fresh);
    c.market_health
        .insert(trading_core::Venue::Coinbase, MarketHealthState::Stale);
    c.market_health
        .insert(trading_core::Venue::Kraken, MarketHealthState::Fresh);
    // Server time + tick recency drive the server-time row + last_tick age.
    c.server_time_now = Some(ui::fixtures::fixed_ts(42));
    c.last_tick_ts = Some(ui::fixtures::fixed_ts(40));
    c.latency = ui::state::Latency::Known { ms: 120 };
    c
}

/// Debug screen NEGATIVE CONTROL: no market health, no server time, latency
/// unknown → "— UTC", "last_tick —", "Unknown" latency.
fn cockpit_debug_cold() -> Cockpit {
    let mut c = ui::fixtures::fake_cockpit_ready();
    c.current_screen = Screen::Settings;
    c.settings_active_tab = SettingsTab::Debug;
    c.market_health.clear();
    c.server_time_now = None;
    c.last_tick_ts = None;
    c.latency = ui::state::Latency::Unknown;
    c
}

/// Audit screen, populated: Screen::Trail list mode (delegates to
/// audit::view) with 30 Ready journal rows + a non-zero total so the
/// pagination header reads "Showing 1–30 of 30" and the table fills.
fn cockpit_audit_populated() -> Cockpit {
    let mut c = ui::fixtures::fake_cockpit_ready();
    c.current_screen = Screen::Trail;
    c.trail_screen_state = Default::default(); // list mode (no selection)
    c.audit_screen_state = AuditScreenState {
        rows: PanelState::Ready(ui::fixtures::fake_journal_rows(30)),
        total_count: Some(30),
        ..Default::default()
    };
    c
}

/// Audit screen NEGATIVE CONTROL: rows Empty → "No matching journal
/// entries" muted body, "Showing 0–0 of 0".
fn cockpit_audit_cold() -> Cockpit {
    let mut c = ui::fixtures::fake_cockpit_ready();
    c.current_screen = Screen::Trail;
    c.trail_screen_state = Default::default();
    c.audit_screen_state = AuditScreenState {
        rows: PanelState::Empty,
        total_count: Some(0),
        ..Default::default()
    };
    c
}

/// Settings rollup chrome itself, populated: pinned to Risk tab with Ready
/// risk so the tab strip + an active populated body render together.
fn cockpit_settings_populated() -> Cockpit {
    // Same as risk-populated; the point is the tab strip is visible above
    // the body. We additionally render the Debug tab variant separately.
    cockpit_risk_populated()
}

// ─── the audit renders (one #[test] per screen, populated + control) ─────

#[test]
fn audit_render_risk() {
    let (w, h, rgba) = render_rgba(cockpit_risk_populated());
    let path = save("risk-populated", w, h, &rgba);
    let ink = ink_pixels_body(w, h, &rgba);
    let green = green_pixels_body(w, h, &rgba);
    let red = red_pixels_body(w, h, &rgba);
    eprintln!("[risk-populated] {path} ink={ink} green={green} red={red}");

    let (w2, h2, rgba2) = render_rgba(cockpit_risk_cold());
    let path2 = save("risk-cold", w2, h2, &rgba2);
    let ink2 = ink_pixels_body(w2, h2, &rgba2);
    let green2 = green_pixels_body(w2, h2, &rgba2);
    eprintln!("[risk-cold] {path2} ink={ink2} green={green2}");

    // Populated risk MUST paint threshold bars (green safe band + a red
    // over-cap bar from the 95% SOLUSDT exposure) and far more ink than the
    // loading control.
    assert!(
        green > 400,
        "risk populated must paint green threshold-bar fill (got {green})"
    );
    assert!(
        red > 80,
        "risk populated must paint the red over-cap bar (95% SOLUSDT) (got {red})"
    );
    assert!(
        ink > ink2 + 2000,
        "risk populated ({ink}) must have far more ink than cold loading ({ink2})"
    );
}

#[test]
fn audit_render_control() {
    let (w, h, rgba) = render_rgba(cockpit_control_populated());
    let path = save("control-populated", w, h, &rgba);
    let ink = ink_pixels_body(w, h, &rgba);
    let green = green_pixels_body(w, h, &rgba);
    eprintln!("[control-populated] {path} ink={ink} green={green}");

    let (w2, h2, rgba2) = render_rgba(cockpit_control_cold());
    let path2 = save("control-cold", w2, h2, &rgba2);
    let ink2 = ink_pixels_body(w2, h2, &rgba2);
    eprintln!("[control-cold] {path2} ink={ink2}");

    // Both states have the mode segment + kill action, so ink is similar;
    // the populated one has real limit values (not dashes). Just assert it
    // paints a substantial panel (mode buttons + 3 rows + kill).
    assert!(
        ink > 6000,
        "control panel must paint a populated HumanControl panel (got ink={ink})"
    );
}

#[test]
fn audit_render_debug() {
    let (w, h, rgba) = render_rgba(cockpit_debug_populated());
    let path = save("debug-populated", w, h, &rgba);
    let ink = ink_pixels_body(w, h, &rgba);
    eprintln!("[debug-populated] {path} ink={ink}");

    let (w2, h2, rgba2) = render_rgba(cockpit_debug_cold());
    let path2 = save("debug-cold", w2, h2, &rgba2);
    let ink2 = ink_pixels_body(w2, h2, &rgba2);
    eprintln!("[debug-cold] {path2} ink={ink2}");

    // Populated debug shows 3 venue rows + server time + last_tick ages →
    // more ink than the cold control (which shows 0 venue rows, "— UTC").
    assert!(
        ink > ink2 + 600,
        "debug populated ({ink}) must have more ink than cold ({ink2})"
    );
}

#[test]
fn audit_render_audit() {
    let (w, h, rgba) = render_rgba(cockpit_audit_populated());
    let path = save("audit-populated", w, h, &rgba);
    let ink = ink_pixels_body(w, h, &rgba);
    eprintln!("[audit-populated] {path} ink={ink}");

    let (w2, h2, rgba2) = render_rgba(cockpit_audit_cold());
    let path2 = save("audit-cold", w2, h2, &rgba2);
    let ink2 = ink_pixels_body(w2, h2, &rgba2);
    eprintln!("[audit-cold] {path2} ink={ink2}");

    // 30 rows of table text vs a single muted "no match" line.
    assert!(
        ink > ink2 + 5000,
        "audit populated 30-row table ({ink}) must dwarf the empty state ({ink2})"
    );
}

#[test]
fn audit_render_trail() {
    use smol_str::SmolStr;
    use ui::state::{ReconstructedTrailUi, TrailScreenState, TrailStageUi};
    use ui::widgets::trail_node::{TrailNode, TrailNodeKind};

    // Trail mode populated — replicate tests/fixtures trail_side_drawer_open.
    let mut c = ui::fixtures::fake_cockpit_ready();
    c.current_screen = Screen::Trail;
    let fill = TrailStageUi {
        timestamp: Some("12:34:56.789".to_string()),
        actor: Some("strategy:sma_crossover".to_string()),
        headline: Some("Buy 0.05 BTCUSDT @ 42000.00".to_string()),
        raw_payload: Some(r#"{"fill_id":"abc123"}"#.to_string()),
    };
    let signal = TrailStageUi {
        timestamp: Some("12:34:55.123".to_string()),
        actor: Some("strategy:sma_crossover".to_string()),
        headline: Some("Buy signal triggered (SMA crossover)".to_string()),
        raw_payload: Some(r#"{"signal_id":"sig001"}"#.to_string()),
    };
    let forecast = TrailStageUi {
        timestamp: Some("12:34:54.001".to_string()),
        actor: Some("tcn:abc12345".to_string()),
        headline: Some("Bullish p=0.72 horizon=15m".to_string()),
        raw_payload: Some(r#"{"forecast_id":"fc001"}"#.to_string()),
    };
    let nodes = vec![
        TrailNode {
            kind: TrailNodeKind::Forecast,
            timestamp: forecast.timestamp.clone(),
            actor: forecast.actor.clone(),
            headline: forecast.headline.clone(),
        },
        TrailNode {
            kind: TrailNodeKind::LlmDebate,
            timestamp: None,
            actor: None,
            headline: None,
        },
        TrailNode {
            kind: TrailNodeKind::Signal,
            timestamp: signal.timestamp.clone(),
            actor: signal.actor.clone(),
            headline: signal.headline.clone(),
        },
        TrailNode {
            kind: TrailNodeKind::Fill,
            timestamp: fill.timestamp.clone(),
            actor: fill.actor.clone(),
            headline: fill.headline.clone(),
        },
    ];
    let trail = ReconstructedTrailUi {
        audit_id: SmolStr::new("fixture-audit-id-001"),
        fill,
        signal,
        forecast,
        debate: TrailStageUi::default(),
        nodes,
    };
    c.trail_screen_state = TrailScreenState {
        selected_audit_id: Some(SmolStr::new("fixture-audit-id-001")),
        drawer_selected_node: Some(TrailNodeKind::Forecast),
        reconstructed_trail: Some(trail),
        pending_trail_audit_id: None,
    };

    let (w, h, rgba) = render_rgba(c);
    let path = save("trail-populated", w, h, &rgba);
    let ink = ink_pixels_body(w, h, &rgba);
    let green = green_pixels_body(w, h, &rgba);
    eprintln!("[trail-populated] {path} ink={ink} green={green}");

    assert!(
        ink > 4000,
        "trail mode must paint 4 node cards + drawer + breadcrumb (got ink={ink})"
    );
}

#[test]
fn audit_render_settings() {
    let (w, h, rgba) = render_rgba(cockpit_settings_populated());
    let path = save("settings-rollup", w, h, &rgba);
    let ink = ink_pixels_body(w, h, &rgba);
    eprintln!("[settings-rollup] {path} ink={ink}");
    // The settings rollup chrome (tab strip) sits above a populated Risk
    // body — well over 6000 ink.
    assert!(
        ink > 6000,
        "settings rollup must paint tab strip + body (got {ink})"
    );
}

#[test]
fn audit_render_home() {
    // Home is dead-routed in the shell (Screen::Home → live::view), but
    // home::view is still a Group C file. Render it DIRECTLY via a bespoke
    // one-screen program so the 2×2 panel grid is exercised. Use the
    // FULLY-populated cockpit (`fake_cockpit_with_strategies` keeps fills +
    // positions + pnl AND seeds the strategies table Ready) so all four
    // grid quadrants carry data, not just three (the plain `fake_cockpit_ready`
    // leaves `strategies` Loading by construction — correct-by-data, not a bug).
    let mut c = ui::fixtures::fake_cockpit_with_strategies();
    // Ensure pnl/positions/tape are also present for the other 3 quadrants.
    c.pnl = PanelState::Ready(ui::fixtures::fake_pnl_positive());
    c.positions = PanelState::Ready(ui::fixtures::fake_positions());
    c.tape = PanelState::Ready(ui::fixtures::fake_fill_feed(8).into_iter().collect());
    let (w, h, rgba) = render_home_direct(c);
    let path = save("home-populated", w, h, &rgba);
    let ink = ink_pixels_body(w, h, &rgba);
    eprintln!("[home-populated] {path} ink={ink}");
    assert!(
        ink > 6000,
        "home 2x2 grid (pnl+positions+strategies+tape) must paint (got {ink})"
    );
}

/// Longest continuous vertical run (in px) of "lighter-than-panel" pixels in
/// a column, scanned across the body. An iced `Scrollable` paints a vertical
/// scrollbar TRACK — a tall, thin, uniformly-lighter-than-the-`PANEL`-bg
/// column. The audit table's panel bg is ~#1C2127 (luma ~33); the scrollbar
/// track sits clearly above. A tall page WITHOUT a scroll container has no
/// such column (rows just clip at the viewport). Returns `(maxrun, x)`.
fn longest_light_vrun(w: u32, h: u32, rgba: &[u8]) -> (u32, u32) {
    let x0 = 240u32.min(w);
    let y_lo = 150u32;
    let y_hi = h.saturating_sub(40);
    let mut best = (0u32, 0u32);
    for x in x0..w.saturating_sub(4) {
        let mut run = 0u32;
        let mut maxrun = 0u32;
        for y in y_lo..y_hi {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let r = i32::from(rgba[idx]);
            let g = i32::from(rgba[idx + 1]);
            let b = i32::from(rgba[idx + 2]);
            let luma = (r * 2 + g * 5 + b) / 8;
            if (50..120).contains(&luma) {
                run += 1;
                maxrun = maxrun.max(run);
            } else {
                run = 0;
            }
        }
        if maxrun > best.0 {
            best = (maxrun, x);
        }
    }
    best
}

/// **DURABLE GUARD (ui-debugger Group C fix).** A full `AUDIT_PAGE_SIZE`
/// (= 250) journal page is far taller than any viewport. Before the fix the
/// audit table was a bare `Column` with NO scroll container, so a 60-row page
/// clipped at ~row 13 and the rest were unreachable (proven at the pixel
/// layer). The fix wraps the table region in a `Scrollable` in
/// `screens::audit::view`. This guard renders a 60-row page and asserts the
/// vertical scrollbar TRACK paints (a tall light column) — i.e. the table is
/// scroll-contained, not clipped. Also serves Trail list mode (it delegates
/// to `audit::view`).
#[test]
fn audit_table_is_scroll_contained_on_tall_page() {
    let mut tall = ui::fixtures::fake_cockpit_ready();
    tall.current_screen = Screen::Trail;
    tall.trail_screen_state = Default::default();
    tall.audit_screen_state = AuditScreenState {
        rows: PanelState::Ready(ui::fixtures::fake_journal_rows(60)),
        total_count: Some(60),
        ..Default::default()
    };
    let (w, h, rgba) = render_rgba(tall);
    save("audit-60rows-overflow", w, h, &rgba);
    let (maxrun, x) = longest_light_vrun(w, h, &rgba);
    eprintln!("[audit-60rows] scrollbar track: maxrun={maxrun}px at x={x}");

    // A scrollbar track runs most of the table height (>500px on a 1080 frame
    // with a ~64px status bar + ~110px filter/pagination header). A clipped
    // bare Column has no such uniform tall light column.
    assert!(
        maxrun > 500,
        "audit table must be scroll-contained on a 60-row page — expected a \
         vertical scrollbar track >500px tall, got {maxrun}px at x={x}. If \
         this fails the table reverted to a bare Column and rows past the \
         viewport are unreachable. PNG: /tmp/ui-audit/group-c/audit-60rows-overflow.png"
    );

    // Negative control: a 3-row page fits entirely → NO scrollbar track
    // (the scrollable collapses its bar when content fits). Proves the guard
    // discriminates the overflow case rather than always passing.
    let mut short = ui::fixtures::fake_cockpit_ready();
    short.current_screen = Screen::Trail;
    short.trail_screen_state = Default::default();
    short.audit_screen_state = AuditScreenState {
        rows: PanelState::Ready(ui::fixtures::fake_journal_rows(3)),
        total_count: Some(3),
        ..Default::default()
    };
    let (w2, h2, rgba2) = render_rgba(short);
    save("audit-3rows-fits", w2, h2, &rgba2);
    let (maxrun2, _x2) = longest_light_vrun(w2, h2, &rgba2);
    eprintln!("[audit-3rows] scrollbar track: maxrun={maxrun2}px (should be short — content fits)");
    assert!(
        maxrun2 < maxrun,
        "a 3-row page that fits must show a shorter/absent scrollbar track \
         ({maxrun2}px) than the 60-row overflow page ({maxrun}px) — proving \
         the scrollbar only appears when rows overflow."
    );
}

// Bespoke one-screen program rendering `screens::home::view` directly.
fn render_home_direct(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
    ui::force_chart_utc_for_tests();
    let program = home_program(cockpit);
    let theme = iced::Theme::Dark;
    let shot = iced_test::screenshot(&program, &theme, (1920, 1080), 1.0, Duration::ZERO);
    (shot.size.width, shot.size.height, shot.rgba.to_vec())
}

fn home_program(
    cockpit: Cockpit,
) -> iced::Application<
    impl iced::Program<State = HomeApp, Message = ui::state::Message, Theme = iced::Theme>,
> {
    let boot = move || {
        (
            HomeApp {
                cockpit: cockpit.clone(),
            },
            iced::Task::none(),
        )
    };
    iced::application(boot, HomeApp::update, HomeApp::view)
        .title(HomeApp::title)
        .theme(HomeApp::theme)
}

struct HomeApp {
    cockpit: Cockpit,
}

impl Default for HomeApp {
    fn default() -> Self {
        Self {
            cockpit: ui::fixtures::fake_cockpit_ready(),
        }
    }
}

impl HomeApp {
    fn title(&self) -> String {
        "home-audit".to_string()
    }
    fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }
    fn update(&mut self, msg: ui::state::Message) -> iced::Task<ui::state::Message> {
        ui::state::update(&mut self.cockpit, msg);
        iced::Task::none()
    }
    fn view(&self) -> iced::Element<'_, ui::state::Message> {
        use iced::Length;
        use iced::widget::{Container, container};
        let body = ui::screens::home::view(&self.cockpit, ui::theme::ThemeMode::Dark);
        Container::new(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_t: &iced::Theme| container::Style {
                background: Some(
                    ui::theme::color::PANEL
                        .current(ui::theme::ThemeMode::Dark)
                        .into(),
                ),
                ..Default::default()
            })
            .into()
    }
}
