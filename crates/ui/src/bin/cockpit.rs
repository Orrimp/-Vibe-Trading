//! Cockpit binary — live ops view.
//!
//! Wires the `ui` crate panels into an iced `Application` using the
//! functional builder API (`iced::application` / `iced::run`). The empty
//! `Subscription` in v0 is replaced by a real broadcast subscription in
//! T32 (Week 2).
//!
//! Feature flags:
//! - `fixtures` — boot against deterministic in-memory data from
//!   `ui::fixtures`; no `agent` process required. Best for layout smoke
//!   tests and demo runs.
//! - `live` — subscribe to a same-process `agent::EventBus` via
//!   [`ui::live::subscription`]. With no bus publishing, every panel
//!   stays in `Loading`; with a running agent publishing into the bus,
//!   fills / positions / P&L stream in via broadcast receivers.
//!
//! The `live` path uses a shared `Arc<EventBus>`. In a unified
//! agent+cockpit binary (future v0.5), the bus would be threaded in
//! here; for now the cockpit creates an empty bus at startup so it
//! boots cleanly in isolation — see
//! `spec/reports/dev-week2-broadcast-api-2026-04-18.md` § IPC model.

use iced::widget::{Column, Row};
use iced::{Element, Length};

use ui::state::{Cockpit, Message};
use ui::strings::APP_TITLE;
use ui::theme::{color, layout, space};
use ui::widgets::{kill, latency, pnl, positions, tape};

#[cfg(feature = "live")]
use std::sync::Arc;

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
    #[cfg(feature = "live")]
    bus: Option<Arc<agent::EventBus>>,
}

impl App {
    fn boot() -> (Self, iced::Task<Message>) {
        #[cfg(feature = "fixtures")]
        let cockpit = ui::fixtures::fake_cockpit_ready();
        #[cfg(not(feature = "fixtures"))]
        let cockpit = Cockpit::new();

        #[cfg(feature = "live")]
        let bus = Some(Arc::new(agent::EventBus::new(
            &agent::config::BusConfig::default(),
        )));

        (
            Self {
                cockpit,
                #[cfg(feature = "live")]
                bus,
            },
            iced::Task::none(),
        )
    }

    fn title(&self) -> String {
        APP_TITLE.to_string()
    }

    fn update(&mut self, msg: Message) {
        ui::state::update(&mut self.cockpit, msg);
    }

    /// Cockpit subscription — swapped by feature flag.
    ///
    /// - `live`  → real broadcast-bus stream (T32).
    /// - `fixtures` or default → empty subscription; the `fixtures` boot
    ///   already populates every panel, so no live stream is needed.
    #[allow(clippy::unused_self)]
    fn subscription(&self) -> iced::Subscription<Message> {
        #[cfg(feature = "live")]
        {
            if let Some(bus) = self.bus.as_ref() {
                return ui::live::subscription(Arc::clone(bus));
            }
        }
        iced::Subscription::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let left = Column::new()
            .spacing(layout::PANEL_OUTER_GAP)
            .push(pnl::view(&self.cockpit))
            .push(latency::view(&self.cockpit))
            .push(kill::view(&self.cockpit))
            .width(Length::FillPortion(1));

        let right = Column::new()
            .spacing(layout::PANEL_OUTER_GAP)
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
