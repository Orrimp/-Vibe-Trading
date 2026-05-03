//! Cockpit binary — fixtures-only ops view.
//!
//! Wires the `ui` crate panels into an iced `Application` using the
//! functional builder API (`iced::application` / `iced::run`).
//!
//! ## Why `--features live` no longer applies here (T908)
//!
//! Pre-T908, this binary accepted a `--features live` build that
//! constructed an *empty* `Arc<EventBus>` — every panel stayed in
//! `Loading` forever because nothing was publishing. That dead-end was
//! the exact failure mode the unified [`cockpit_live`] binary exists
//! to delete. Per
//! [`spec/features/live-cockpit-unified.md` Q7](../../../../spec/features/live-cockpit-unified.md#q7--keep-two-binary-path-alive)
//! the standalone `cockpit` binary is now fixtures-only; an explicit
//! [`compile_error!`] below fires if anyone tries
//! `cargo run --bin cockpit --features live`, redirecting them to
//! `cargo run --bin cockpit_live --features live` (the unified binary
//! that actually wires the bus + kill switch + audit ledger end-to-end).
//!
//! Feature flag still supported here:
//! - `fixtures` — boot against deterministic in-memory data from
//!   `ui::fixtures`; no `agent` process required. Best for layout smoke
//!   tests and demo runs.
//!
//! [`cockpit_live`]: ../../bin/cockpit_live/index.html

// T908 — deprecation shim. The standalone `cockpit` bin no longer
// honors `--features live` as the live entry point; the new home for
// live wiring is the `cockpit_live` bin (see Cargo.toml
// `[[bin]] cockpit_live`, `required-features = ["live"]`).
//
// Two layers of gating defend against the dead empty-bus path:
//
// 1. **Cargo-level**: this bin declares `required-features =
//    ["fixtures"]` in `Cargo.toml`, so `cargo run --bin cockpit
//    --features live` fails at resolve time with "target requires the
//    features: fixtures" — pointing the operator at the right call.
// 2. **Source-level (this `compile_error!`)**: fires only when
//    `live` is requested *without* `fixtures`. That combination is
//    impossible to hit through the cargo gate above, but if a future
//    edit ever drops the `required-features` line, this shim still
//    routes the operator to `cockpit_live` with a clear message
//    instead of silently re-introducing the empty-bus dead end.
//
// Workspace-wide `cargo build --workspace --all-features` (which
// activates both `live` and `fixtures`) compiles cleanly because the
// `not(feature = "fixtures")` half of the gate is false.
#[cfg(all(feature = "live", not(feature = "fixtures")))]
compile_error!(
    "The `cargo run --bin cockpit --features live` path was retired in \
     live-cockpit-unified (T908). Use `cargo run --bin cockpit_live --features live` \
     for the unified agent+cockpit binary; the headless agent still runs via \
     `cargo run --bin trading`. The standalone `cockpit` bin is fixtures-only."
);

use iced::widget::{Column, Row};
use iced::{Element, Length};

use ui::state::{Cockpit, Message};
use ui::strings::APP_TITLE;
use ui::theme::{color, layout, space};
use ui::widgets::{kill, latency, pnl, positions, strategies, tape};

fn main() -> iced::Result {
    iced::application(App::boot, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        .run()
}

#[derive(Default)]
struct App {
    cockpit: Cockpit,
}

impl App {
    fn boot() -> (Self, iced::Task<Message>) {
        // Fixtures boot populates every panel so the layout smoke covers the
        // full column stack without a running agent. v1 (T623) extends this
        // to a top-3 momentum portfolio so the positions panel renders the
        // multi-row steady state for the V8 smoke (R11 negative confirmation
        // — same widget, more rows). v1.5a (T719) extends it again to the
        // mean-reversion-pairs steady state: 3 long-leg position rows +
        // `pairs_mr_h1` strategy row + a recent-events footer carrying both
        // new v1.5a kinds (`MeanReversionStop`, `PairShortObservation`).
        // Operators see the most recent feature set when they fixtures-boot
        // the cockpit — earlier presets stay available for snapshot tests.
        #[cfg(feature = "fixtures")]
        let cockpit = ui::fixtures::fake_cockpit_v15a_pairs_steady_state();
        #[cfg(not(feature = "fixtures"))]
        let cockpit = Cockpit::new();

        (Self { cockpit }, iced::Task::none())
    }

    fn title(&self) -> String {
        APP_TITLE.to_string()
    }

    fn update(&mut self, msg: Message) {
        ui::state::update(&mut self.cockpit, msg);
    }

    /// Cockpit subscription — fixtures path only.
    ///
    /// - `fixtures` or default → empty subscription; the `fixtures` boot
    ///   already populates every panel, so no live stream is needed.
    /// - The retired `live` arm now lives in `cockpit_live` (T908).
    #[allow(clippy::unused_self)]
    fn subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::none()
    }

    fn view(&self) -> Element<'_, Message> {
        // Left column — v0 layout unchanged (P&L, latency, kill).
        let left = Column::new()
            .spacing(layout::PANEL_OUTER_GAP)
            .push(pnl::view(&self.cockpit))
            .push(latency::view(&self.cockpit))
            .push(kill::view(&self.cockpit))
            .width(Length::FillPortion(1));

        // Right column — v0.5 Q4 resolution: strategies panel ABOVE Open
        // positions. Live tape stays at the bottom so the operator's eye
        // flows strategies → positions → ticker.
        let right = Column::new()
            .spacing(layout::PANEL_OUTER_GAP)
            .push(strategies::view(&self.cockpit))
            .push(positions::view(&self.cockpit))
            .push(tape::view(&self.cockpit))
            .width(Length::FillPortion(2));

        let body = Row::new()
            .spacing(layout::PANEL_OUTER_GAP)
            .push(left)
            .push(right);

        iced::widget::container(body)
            // iced 0.14 Padding accepts `u16`.
            .padding(space::L as u16)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme: &iced::Theme| iced::widget::container::Style {
                background: Some(color::BG.into()),
                text_color: Some(color::FG),
                ..Default::default()
            })
            .into()
    }

    fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }
}
