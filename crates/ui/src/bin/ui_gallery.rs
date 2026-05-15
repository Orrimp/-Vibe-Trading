//! Widget gallery binary — ui-gallery-bin v0.1.
//!
//! Single scrollable window rendering every cockpit widget × every
//! fixture state on one page. Operator-facing artifact for visual review
//! and the presenter deck.
//!
//! ## Build + run
//!
//! ```bash
//! # V1 — compile check
//! cargo build -p ui --bin ui-gallery --features fixtures
//!
//! # V2 — smoke test (no window; exits in < 5 s)
//! cargo run -p ui --bin ui-gallery --features fixtures -- --smoke
//!
//! # Operator interactive session
//! cargo run -p ui --bin ui-gallery --features fixtures
//! ```
//!
//! ## `--smoke` semantics (Q-ARCH-1 / design.md)
//!
//! 1. Parse argv via `clap::Parser`.
//! 2. Build the default `GalleryApp` (seeded via `seed_for_all_cells`).
//! 3. Call `GalleryApp::view` once (construct + drop Element).
//! 4. Return `ExitCode::SUCCESS` without entering the iced event loop.
//!
//! This catches panicking fixture builders, missing `pub mod` entries,
//! and compile-time regressions without requiring a display server.
//!
//! ## Architecture
//!
//! - Snapshot test path: `GalleryApp::view` → `column!` (no scrollable).
//!   The test passes `(slot_w, GALLERY_LOGICAL_HEIGHT)` as viewport.
//! - Interactive bin path (this file): `GalleryApp::view_scrollable` wraps
//!   the column in `scrollable(...)` for operator window UX. The method-
//!   pointer form avoids HRTB issues with `iced::application`'s `ViewFn`.

use std::process::ExitCode;

use clap::Parser;

use ui::gallery::GalleryApp;

/// CLI arguments for the widget gallery binary.
#[derive(Parser)]
#[command(
    name = "ui-gallery",
    about = "Widget gallery — all cockpit widgets in one window"
)]
struct Args {
    /// Smoke-test mode: build fixtures + first-frame Element, then exit
    /// without opening a window. Sandbox-safe (no display server required).
    #[arg(long)]
    smoke: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();

    if args.smoke {
        // --smoke: construct the default GalleryApp and build the gallery
        // Element once. Any panicking fixture builder or missing view()
        // call surfaces here.
        let app = GalleryApp::default();
        let _element = app.view();
        eprintln!("ui-gallery --smoke OK");
        return ExitCode::SUCCESS;
    }

    // Interactive path — method pointers for HRTB compatibility with
    // iced::application's ViewFn. `view_scrollable` wraps the bare
    // gallery column in a scrollable for operator window UX.
    if let Err(e) = iced::application(
        || (GalleryApp::default(), iced::Task::none()),
        GalleryApp::update,
        GalleryApp::view_scrollable,
    )
    .title(GalleryApp::title)
    .theme(GalleryApp::theme)
    .run()
    {
        eprintln!("ui-gallery: iced error: {e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
