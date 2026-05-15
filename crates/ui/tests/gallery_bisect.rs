//! Diagnostic: bisect which cell in `GALLERY_CELLS` triggers the
//! tiny-skia "Build quad rectangle" panic at render time.
//!
//! Renders `[0..n]` cells for each `n` and reports the first `n` that
//! panics. Run with:
//! ```bash
//! cargo test -p ui --features fixtures --test gallery_bisect -- --nocapture
//! ```

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::panic;
use std::time::Duration;

#[path = "fixtures/mod.rs"]
mod fixtures;

use ui::gallery::GALLERY_LOGICAL_HEIGHT;
use ui::state::{Cockpit, Message};

struct BisectApp {
    n: usize,
}

impl BisectApp {
    fn update(&mut self, _msg: Message) -> iced::Task<Message> {
        iced::Task::none()
    }
    fn view(&self) -> iced::Element<'_, Message> {
        ui::gallery::view_slice(0, self.n)
    }
    fn title(&self) -> String {
        format!("bisect-{}", self.n)
    }
    fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }
}

fn try_render(n: usize) -> Result<(), String> {
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let app = BisectApp { n };
        let cockpit = Cockpit::default();
        let _ = cockpit;
        let program = iced::application(
            move || (BisectApp { n }, iced::Task::none()),
            BisectApp::update,
            BisectApp::view,
        )
        .title(BisectApp::title)
        .theme(BisectApp::theme);
        let _ = app;
        let theme = iced::Theme::Dark;
        let _screenshot = iced_test::screenshot(
            &program,
            &theme,
            (1280, GALLERY_LOGICAL_HEIGHT),
            1.0,
            Duration::ZERO,
        );
    }));
    match result {
        Ok(()) => Ok(()),
        Err(_) => Err(format!("panic at n={n}")),
    }
}

/// Diagnostic — deliberately panics by design (it bisects until the
/// first failing render). Kept `#[ignore]` so `cargo test` stays
/// green; run manually:
/// ```bash
/// cargo test -p ui --features fixtures --test gallery_bisect -- --ignored --nocapture
/// ```
/// As of 2026-05-15, this reports `n=8` (first panicking cell is
/// `GALLERY_CELLS[7]` — `strategies :: ready_v1`). See
/// [`gallery_snapshots.rs`](gallery_snapshots.rs) module docs for the
/// follow-up plan.
#[test]
#[ignore = "diagnostic; run with --ignored --nocapture to bisect a render panic"]
fn bisect_first_panicking_cell() {
    // Try N=1..=36. Print the first N that fails.
    for n in 1..=36 {
        match try_render(n) {
            Ok(()) => eprintln!("n={n:2} OK"),
            Err(e) => {
                eprintln!("n={n:2} FAIL — {e}");
                eprintln!("first panicking cell is index {}", n - 1);
                let names: Vec<&str> = ui::gallery::GALLERY_CELLS[..n]
                    .iter()
                    .map(|c| c.widget)
                    .collect();
                eprintln!("cells included: {names:?}");
                panic!("bisect found panic at n={n}");
            }
        }
    }
}
