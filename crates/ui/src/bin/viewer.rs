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

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use iced::widget::{Column, Container, container};
use iced::{Element, Length};
use trading_core::{BacktestMetrics, EquitySeries, Money, Timestamp, Usdt};
use ui::state::PanelState;
use ui::theme::{ThemeMode, color, layout, space};
use ui::viewer::{ReportFrontMatter, ReportLoadResult, ViewerMessage, ViewerModel};
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

/// Synchronous load. Reads the markdown body + parses the KPI table
/// + reads the companion equity CSV (when present).
fn load_report(path: &Path) -> Result<ReportLoadResult, std::io::Error> {
    let raw = std::fs::read_to_string(path)?;

    let front_matter = parse_front_matter(&raw);

    // KPI metrics — graceful fallback to `BacktestMetrics::all_absent`
    // when the parser hits a malformed body (R3.5 / Q3 graceful
    // fallback). Errors flip to `PanelState::Error(msg)` so the strip
    // renders the unavailable state with the muted body.
    let metrics: PanelState<BacktestMetrics> = match reports::parse::parse_from_report(path) {
        Ok(m) => PanelState::Ready(m),
        Err(e) => PanelState::Error(smol_str::SmolStr::new(e.to_string())),
    };

    // Equity CSV — companion file at `<dir>/artifacts/<run_id>/equity-*.csv`.
    // Phase 4 reads the existing committed companion via
    // `reports::csv_artifacts::read_equity_csv` (R11.2). When the
    // companion is missing or unreadable, the equity curve / drawdown
    // band render their empty state independently of the KPI strip
    // (R11.3).
    let equity = load_equity_companion(path)
        .unwrap_or_else(|e| PanelState::Error(smol_str::SmolStr::new(e.as_str())));

    let body_markdown = strip_front_matter(&raw).to_string();

    Ok(ReportLoadResult {
        front_matter,
        metrics,
        equity,
        body_markdown,
    })
}

/// Locate and read the companion equity CSV. Returns
/// `Ok(PanelState::Ready(series))` on success, `Ok(PanelState::Empty)`
/// when no companion exists, `Err(...)` on read / parse failure.
fn load_equity_companion(report_path: &Path) -> Result<PanelState<EquitySeries>, String> {
    let parent = report_path
        .parent()
        .ok_or_else(|| "report has no parent directory".to_string())?;
    // The reports binary writes the companion under
    // `<parent>/artifacts/<run_id>/equity-*.csv`. We don't have the
    // run_id here; scan for the first `equity-*.csv` under any
    // run-id folder. If none, return Empty.
    let artifacts_root = parent.join("artifacts");
    if !artifacts_root.exists() {
        return Ok(PanelState::Empty);
    }
    let mut candidate: Option<PathBuf> = None;
    let entries = std::fs::read_dir(&artifacts_root).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        if let Ok(inner) = std::fs::read_dir(&p) {
            for inner_entry in inner.flatten() {
                let ip = inner_entry.path();
                let name = ip.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with("equity-") && name.ends_with(".csv") {
                    candidate = Some(ip);
                    break;
                }
            }
        }
        if candidate.is_some() {
            break;
        }
    }
    let Some(csv_path) = candidate else {
        return Ok(PanelState::Empty);
    };
    let samples = reports::csv_artifacts::read_equity_csv(&csv_path).map_err(|e| e.to_string())?;
    if samples.is_empty() {
        return Ok(PanelState::Empty);
    }
    let points: Vec<(Timestamp, Money<Usdt>)> = samples
        .into_iter()
        .map(|s| (s.ts, Money::<Usdt>::from_decimal(s.equity_total)))
        .collect();
    let series = EquitySeries::from_points(points).map_err(|e| e.to_string())?;
    // Q5 — cap at 2000 points for paint budget.
    let series = series.downsample(2000);
    Ok(PanelState::Ready(series))
}

/// Parse the `scenario:` field out of the YAML front-matter.
fn parse_front_matter(raw: &str) -> ReportFrontMatter {
    let trimmed = raw.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---\n") && !trimmed.starts_with("---\r\n") {
        return ReportFrontMatter::default();
    }
    let after = match trimmed.find('\n') {
        Some(n) => &trimmed[n + 1..],
        None => return ReportFrontMatter::default(),
    };
    let end = after.find("\n---").unwrap_or(after.len());
    let yaml = &after[..end];
    let mut scenario = smol_str::SmolStr::default();
    for line in yaml.lines() {
        if let Some(rest) = line.strip_prefix("scenario:") {
            scenario = smol_str::SmolStr::new(rest.trim());
            break;
        }
    }
    ReportFrontMatter { scenario }
}

fn strip_front_matter(raw: &str) -> &str {
    let trimmed = raw.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---\n") && !trimmed.starts_with("---\r\n") {
        return trimmed;
    }
    let after_first = match trimmed.find('\n') {
        Some(n) => &trimmed[n + 1..],
        None => return trimmed,
    };
    if let Some(rel) = after_first.find("\n---") {
        let rest = &after_first[rel + 1..];
        if let Some(nl) = rest.find('\n') {
            return &rest[nl + 1..];
        }
    }
    trimmed
}

// ── body_render — minimal markdown pre-pass ──────────────────────────────────
mod body_render {
    use iced::Length;
    use iced::widget::{Column, Text, container, scrollable};

    use ui::theme::{ThemeMode, color, space, text};
    use ui::viewer::ViewerMessage;

    /// Render the report body verbatim with a tiny heading-level
    /// pre-pass: `# / ## / ###` lines map to `text::H2` / `text::H3`
    /// rows; everything else stays as monospaced `text::BODY`.
    pub fn view<'a>(markdown: &'a str, mode: ThemeMode) -> iced::Element<'a, ViewerMessage> {
        let mut col = Column::new().spacing(space::XS);
        for line in markdown.lines() {
            let stripped = line.trim_start();
            let element = if let Some(rest) = stripped.strip_prefix("### ") {
                Text::new(rest.to_string())
                    .size(text::H3)
                    .color(color::FG_1.current(mode))
            } else if let Some(rest) = stripped.strip_prefix("## ") {
                Text::new(rest.to_string())
                    .size(text::H2)
                    .color(color::FG_1.current(mode))
            } else if let Some(rest) = stripped.strip_prefix("# ") {
                Text::new(rest.to_string())
                    .size(text::H2)
                    .color(color::FG_1.current(mode))
            } else {
                Text::new(line.to_string())
                    .size(text::BODY)
                    .color(color::FG_2.current(mode))
            };
            col = col.push(element);
        }
        scrollable(container(col).padding(space::S as u16).width(Length::Fill))
            .height(Length::Fill)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_parser_accepts_report_path() {
        // Construct a parsed Args from a one-arg vector.
        let args = Args::try_parse_from([
            "viewer",
            "spec/v05-composed-strategies/reports/backtest-20260420-152017-btc-2023-1m-rsi-reversion.md",
        ])
        .expect("parser must accept positional report path");
        assert_eq!(
            args.report_path,
            PathBuf::from(
                "spec/v05-composed-strategies/reports/backtest-20260420-152017-btc-2023-1m-rsi-reversion.md"
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

    #[test]
    fn parse_front_matter_extracts_scenario() {
        let raw = "---\nscenario: btc-2023-1m-rsi-reversion\nseed: 0xC0FFEE\n---\n# Body\n";
        let fm = parse_front_matter(raw);
        assert_eq!(fm.scenario.as_str(), "btc-2023-1m-rsi-reversion");
    }
}
