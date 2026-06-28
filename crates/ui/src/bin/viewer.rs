//! Backtest report viewer — Phase 4.
//!
//! Read-only iced surface that renders a single committed
//! `spec/<feature>/reports/backtest-*.md` body alongside its KPI strip + equity
//! curve + drawdown band. CLI-arg-driven (`viewer <report-path>`);
//! offline / single-shot — no `Subscription`, no audit-bus channels,
//! no kill switch. Sibling of `cockpit` and `cockpit_live`.
//!
//! ## Master Constraints (operator-locked)
//!
//! 1. **No `"Lumen"` in the title bar.** Window title is
//!    `"Backtest report — {scenario}"`.
//! 2. **Zero-button surface.** No "Deploy live" CTA, no "Export"
//!    CTA, no file-picker UI (R14).
//! 3. **CLI-arg-only.** `clap`-parsed positional `<report-path>`.
//!    Missing arg → exit 2 (clap default); non-existent file → exit
//!    3 (custom early check before iced boots).
//!
//! ## Build-time R17.4 / V9 invariant
//!
//! The viewer is read-only on the spec tree. The integration test at
//! `crates/ui/tests/viewer_read_only.rs` greps this file for
//! `File::create` / `tokio::fs::write` against `spec/**` paths and
//! fails loudly if any surface.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use iced::widget::{Column, Container, container};
use iced::{Element, Length};
use ui::reports::body_render;
// cockpit-reports-viewer v0.1.0 (D2/AC5): the report-load parse now lives in
// the shared `ui::reports::loader` module (lifted out of this bin) so the
// viewer bin + the in-cockpit Reports screen call ONE implementation. The
// bin's `App::view` / `main` call sites are unchanged in behaviour.
use ui::reports::loader::load_report;
use ui::theme::{ThemeMode, color, layout, space};
use ui::viewer::{ReportLoadResult, ViewerMessage, ViewerModel};
use ui::widgets::{drawdown_band, equity_curve, kpi_strip};

/// CLI args.
#[derive(Parser)]
#[command(name = "viewer", about = "Backtest report viewer")]
struct Args {
    /// Path to a backtest report under `spec/<feature>/reports/backtest-*.md`.
    report_path: PathBuf,
}

fn main() -> ExitCode {
    // T1803 keeps tracing minimal — no global subscriber by default;
    // operators get plain stderr via `tracing_subscriber` below if
    // `RUST_LOG` is set.
    let args = Args::parse();

    if !args.report_path.exists() {
        eprintln!("viewer: report not found: {}", args.report_path.display());
        return ExitCode::from(3);
    }

    // Synchronous load — viewer is single-shot.
    let load = match load_report(&args.report_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("viewer: failed to load report: {e}");
            return ExitCode::from(3);
        }
    };

    let report_path = args.report_path.clone();
    let scenario = load.front_matter.scenario.clone();
    if let Err(e) = iced::application(
        move || boot(report_path.clone(), load.clone()),
        App::update,
        App::view,
    )
    .title(move |_app: &App| format!("Backtest report — {scenario}"))
    .theme(App::theme)
    // T2028 + T2029 — Layout-β min-size floor + Lumen brand icon.
    .window(ui::window_icon::standard_window_settings())
    .run()
    {
        eprintln!("viewer: iced crashed: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn boot(report_path: PathBuf, load: ReportLoadResult) -> (App, iced::Task<ViewerMessage>) {
    let model = ViewerModel::new(report_path, load);
    (App { model }, iced::Task::none())
}

#[derive(Clone)]
struct App {
    model: ViewerModel,
}

impl App {
    fn update(&mut self, msg: ViewerMessage) -> iced::Task<ViewerMessage> {
        ui::viewer::update(&mut self.model, msg);
        iced::Task::none()
    }

    fn view(&self) -> Element<'_, ViewerMessage> {
        let strip = kpi_strip::view(&self.model.metrics, self.model.mode);
        let curve = equity_curve::view(&self.model.equity, self.model.mode);
        let band = drawdown_band::view(&self.model.equity, self.model.mode);
        let body = body_render::view(&self.model.body_markdown, self.model.mode);
        let stack = Column::new()
            .spacing(space::M)
            .push(strip)
            .push(curve)
            .push(band)
            .push(body);

        let outer_padding = layout::PANEL_PADDING as u16;
        Container::new(stack)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(outer_padding)
            .style(move |_theme: &iced::Theme| container::Style {
                background: Some(color::CANVAS.current(self.model.mode).into()),
                text_color: Some(color::FG_1.current(self.model.mode)),
                ..Default::default()
            })
            .into()
    }

    fn theme(&self) -> iced::Theme {
        match self.model.mode {
            ThemeMode::Dark => iced::Theme::Dark,
            ThemeMode::Light => iced::Theme::Light,
        }
    }
}

// cockpit-reports-viewer v0.1.0 (D2/AC5): `load_report`,
// `load_equity_companion`, `parse_front_matter`, `strip_front_matter`, and
// `mod body_render` were lifted out of this bin into `ui::reports::loader` +
// `ui::reports::body_render` so the in-cockpit `Screen::Reports` and this bin
// share ONE parse implementation (no drift). The `parse_front_matter`
// scenario-extraction test moved with the fn into `reports/loader.rs`
// `#[cfg(test)]`; the CLI/exit-code tests below stay in the bin.

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_parser_accepts_report_path() {
        // Construct a parsed Args from a one-arg vector.
        let args = Args::try_parse_from([
            "viewer",
            "spec/v1/v05-composed-strategies/reports/backtest-20260420-152017-btc-2023-1m-rsi-reversion.md",
        ])
        .expect("parser must accept positional report path");
        assert_eq!(
            args.report_path,
            PathBuf::from(
                "spec/v1/v05-composed-strategies/reports/backtest-20260420-152017-btc-2023-1m-rsi-reversion.md"
            )
        );
    }

    #[test]
    fn cli_parser_rejects_no_args() {
        // Missing positional → clap emits an error.
        let res = Args::try_parse_from(["viewer"]);
        assert!(res.is_err(), "missing report path must error out");
    }

    #[test]
    fn cli_help_renders_without_lumen() {
        let mut cmd = Args::command();
        let mut out = Vec::<u8>::new();
        cmd.write_help(&mut out).expect("write help");
        let s = String::from_utf8_lossy(&out);
        assert!(
            !s.contains("Lumen"),
            "viewer help text must not mention Lumen — Constraint 1"
        );
    }

    // `parse_front_matter_extracts_scenario` moved to
    // `ui::reports::loader` `#[cfg(test)]` with the lifted fn (D2/AC5) — its
    // assertion survives there.
}
