//! Cockpit binary — live ops view.
//!
//! Wires the `ui` crate panels into an iced `Application` using the
//! functional builder API (`iced::application` / `iced::run`). The empty
//! `Subscription` in v0 is replaced by a real broadcast subscription in
//! T32 (Week 2).
//!
//! With `--features fixtures`, the cockpit boots against deterministic
//! in-memory data from `ui::fixtures` — no `agent` process required.

use iced::widget::{Column, Row};
use iced::{Element, Length};

use ui::state::{Cockpit, Message};
use ui::strings::APP_TITLE;
use ui::theme::{color, layout, space};
use ui::widgets::{kill, latency, pnl, positions, tape};

fn main() -> iced::Result {
    iced::application(App::boot, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .run()
}

#[derive(Default)]
struct App {
    cockpit: Cockpit,
}

impl App {
    fn boot() -> (Self, iced::Task<Message>) {
        #[cfg(feature = "fixtures")]
        let cockpit = ui::fixtures::fake_cockpit_ready();
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
