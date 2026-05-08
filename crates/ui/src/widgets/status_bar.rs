//! Status bar widget — bottom chrome strip (T1508).
//!
//! Renders a single fixed-height row with six fields:
//! connection state + coloured dot, latency, account label, server time,
//! CPU placeholder, and app version.
//!
//! **Zero string literals** — all copy from [`crate::strings`].
//! **Zero hex colours** — all tokens from [`crate::theme`].
//!
//! Layout spec (from `lumen-phase-1-foundation.md` T1508):
//! - `Row` with `align_y(Center)`, `padding(0, 12)`,
//!   `height(Length::Fixed(24.0))`, `background: PANEL`,
//!   `border-top: 1 px BORDER_1`, text size `text::MICRO`, text colour
//!   `FG_3`. Spacing between items = `space::L` (16 px).

use iced::widget::{Container, Row, Space, Text};
use iced::{Alignment, Border, Length};

use crate::state::{Cockpit, Latency, MarketHealthState};
use crate::strings::{
    STATUS_BAR_CONNECTED, STATUS_BAR_CPU_PLACEHOLDER, STATUS_BAR_DISCONNECTED,
    STATUS_BAR_LATENCY_LABEL, STATUS_BAR_MS, STATUS_BAR_NO_LATENCY, STATUS_BAR_NO_SERVER_TIME,
    STATUS_BAR_RECONNECTING, STATUS_BAR_SERVER_LABEL, STATUS_BAR_UTC_SUFFIX, STATUS_BAR_VERSION,
};
use crate::theme::{color, color_for_latency_ms, radius, space, text, ThemeMode};

/// Height of the status bar in logical pixels.
const BAR_HEIGHT: f32 = 24.0;

/// Diameter of the connection-status dot in logical pixels.
const DOT_SIZE: f32 = 6.0;

/// The Lumen cold-start theme. Status bar always uses dark mode in Phase 1
/// (same convention as all other widgets in the cockpit).
const MODE: ThemeMode = ThemeMode::Dark;

/// Render the status bar.
///
/// Called by the cockpit shell (T1509 wires this into both bins' `view`).
/// The function is pure — it reads only `&Cockpit` and emits an
/// `Element<Message>`.
// `cast_possible_truncation`: space::* constants are u32 with bounded values 0..64;
// cast to u16 padding is safe.
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn view(cockpit: &Cockpit) -> crate::Element<'_> {
    let fg3 = color::FG_3.current(MODE);

    // ── Connection field ────────────────────────────────────────────────────
    let (dot_color, connection_text) = connection_state(cockpit);
    let dot = Container::new(
        Space::new()
            .width(Length::Fixed(DOT_SIZE))
            .height(Length::Fixed(DOT_SIZE)),
    )
    .style(move |_theme: &iced::Theme| iced::widget::container::Style {
        background: Some(dot_color.into()),
        border: Border {
            radius: radius::PILL.into(),
            ..Default::default()
        },
        ..Default::default()
    });
    let conn_row = Row::new()
        .spacing(space::XS)
        .align_y(Alignment::Center)
        .push(dot)
        .push(Text::new(connection_text).size(text::MICRO).color(fg3));

    // ── Latency field ───────────────────────────────────────────────────────
    let latency_text = match cockpit.latency {
        Latency::Known { ms } => format!("{STATUS_BAR_LATENCY_LABEL} {ms} {STATUS_BAR_MS}"),
        Latency::Unknown => format!("{STATUS_BAR_LATENCY_LABEL} {STATUS_BAR_NO_LATENCY}"),
    };
    let latency_color = match cockpit.latency {
        Latency::Known { ms } => color_for_latency_ms(ms),
        Latency::Unknown => fg3,
    };
    let latency_label = Text::new(latency_text)
        .size(text::MICRO)
        .color(latency_color);

    // ── Account field ───────────────────────────────────────────────────────
    // `account_label` is set at boot from Config (live path) or the static
    // fixture string "Paper · Demo 3-symbol" (fixtures path). We render it
    // verbatim — the field is static for the session.
    let account_label = Text::new(cockpit.account_label.as_str())
        .size(text::MICRO)
        .color(fg3);

    // ── Server time field ───────────────────────────────────────────────────
    // Phase 1: read `server_time_now`; fall back to em-dash when not yet set.
    // The actual clock advance comes from the 1 Hz `ServerTimeTick` message
    // (driven by an iced `time::every` subscription in the binary).
    // The widget itself is pure — it never reads the system clock.
    let server_text = match cockpit.server_time_now {
        Some(ts) => {
            // Format to second precision: "HH:MM:SS".
            // Use `Timestamp::inner()` to access the `OffsetDateTime`, then
            // extract hour/minute/second components directly — no string
            // allocation detour through RFC-3339.
            let dt = ts.inner();
            format!(
                "{STATUS_BAR_SERVER_LABEL} {:02}:{:02}:{:02} {STATUS_BAR_UTC_SUFFIX}",
                dt.hour(),
                dt.minute(),
                dt.second(),
            )
        }
        None => {
            format!("{STATUS_BAR_SERVER_LABEL} {STATUS_BAR_NO_SERVER_TIME} {STATUS_BAR_UTC_SUFFIX}")
        }
    };
    let server_label = Text::new(server_text).size(text::MICRO).color(fg3);

    // ── CPU placeholder (R13.4 deferred) ───────────────────────────────────
    // TODO: replace with real CPU% once R13.4 lazy-metric infra lands.
    // Architect cited this as deferred in the Phase 1 brief (T1508).
    let cpu_label = Text::new(STATUS_BAR_CPU_PLACEHOLDER)
        .size(text::MICRO)
        .color(fg3);

    // ── Version ─────────────────────────────────────────────────────────────
    let version_label = Text::new(STATUS_BAR_VERSION).size(text::MICRO).color(fg3);

    // ── Assemble the bar ────────────────────────────────────────────────────
    // Items are separated by `space::L` (16 px) spacing; the version label
    // is right-aligned by pushing a fill spacer before it.
    let inner_row = Row::new()
        .spacing(space::L)
        .align_y(Alignment::Center)
        .push(conn_row)
        .push(latency_label)
        .push(account_label)
        .push(server_label)
        .push(cpu_label)
        .push(Space::new().width(Length::Fill))
        .push(version_label);

    // Outer container: PANEL background + 1 px BORDER_1 top hairline.
    // iced's Container API applies `border` on all four sides; we simulate
    // a top-only border by stacking a 1 px separator container above the
    // content row.
    let separator = Container::new(Space::new().width(Length::Fill).height(Length::Fixed(1.0)))
        .width(Length::Fill)
        .style(|_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(color::BORDER_1.current(MODE).into()),
            ..Default::default()
        });

    let content = Container::new(inner_row)
        .width(Length::Fill)
        .height(Length::Fixed(BAR_HEIGHT))
        .padding([0_u16, space::M as u16])
        .style(|_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(color::PANEL.current(MODE).into()),
            text_color: Some(color::FG_3.current(MODE)),
            ..Default::default()
        });

    iced::widget::Column::new()
        .push(separator)
        .push(content)
        .into()
}

/// Derive the dot colour and connection label from `cockpit.market_health`.
///
/// - Empty map → Disconnected (grey dot, "Disconnected").
/// - All venues `Fresh` → Connected (green dot, "Connected · {venues}").
/// - Any venue `Stale` → Reconnecting (amber dot, "Reconnecting · {venue}").
fn connection_state(cockpit: &Cockpit) -> (iced::Color, String) {
    let health = &cockpit.market_health;

    if health.is_empty() {
        return (
            color::FG_3.current(MODE),
            STATUS_BAR_DISCONNECTED.to_string(),
        );
    }

    // Collect stale venues.
    let mut stale_venues: Vec<String> = health
        .iter()
        .filter_map(|(venue, state)| {
            if *state == MarketHealthState::Stale {
                Some(venue.to_string())
            } else {
                None
            }
        })
        .collect();
    stale_venues.sort(); // deterministic order

    if !stale_venues.is_empty() {
        let label = format!("{STATUS_BAR_RECONNECTING} · {}", stale_venues.join(", "));
        return (color::WARN_500.current(MODE), label);
    }

    // All fresh — list all venues.
    let mut fresh_venues: Vec<String> = health.keys().map(ToString::to_string).collect();
    fresh_venues.sort();
    let label = format!("{STATUS_BAR_CONNECTED} · {}", fresh_venues.join(", "));
    (color::UP_500.current(MODE), label)
}
