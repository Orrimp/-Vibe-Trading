//! Story 3-19 AC4 — the live EUR/USD rate, proven through the fake
//! `RateSource`. **No test here opens a socket** (ADR-0101 Consequences).
//!
//! ## The division of proof
//!
//! A pixel gate proves that something painted; it cannot read the words. So the
//! WORDS are asserted here, on the text the operator's surfaces compose — the
//! plan export, which runs the same `budget_line_text` the Forward-plan sizing
//! block mirrors — and the fact that the line PAINTS is asserted in
//! `advisor_live_fx_render.rs` (AD-10). Neither half is sufficient; claiming
//! either covers the other is the proxy this repo keeps getting burned by.
//!
//! What each test pins:
//!
//! - [`a_live_success_shows_live_provenance_in_the_label`] — AC4(a).
//! - [`a_forced_failure_falls_back_to_the_static_config_rate`] — AC4(b), via
//!   the fake's unreachable arm (the bridge, the resolution, and the rendered
//!   provenance, end to end).
//! - [`display_equals_engine_on_the_live_rate`] — AC4(c): ONE
//!   `BudgetConversion`, read by both.
//! - [`the_live_rate_is_fetched_only_when_asked`] — AC2: constructing the
//!   source fetches nothing; each explicit resolution is exactly one call.
//! - [`the_cockpit_resolves_live_then_static`] — the one place the choice is
//!   made, so three surfaces cannot show three provenances.
//! - [`the_label_never_claims_the_rate_is_current`] — ADR-0101 D4.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use data::fx_rate::FakeRateSource;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;
use trading_core::{BudgetConversion, FxNote, FxRate};
use ui::export::plan_export::serialize_plan_export;
use ui::leaderboard::NarrationState;
use ui::live_fx;
use ui::widgets::num::{fmt_fx_provenance, fmt_usdt_plain};

/// The ECB reference rate measured on 2026-11-08, with the publication date the
/// provider returned that day (a Friday's rate, asked for on a Sunday — D4).
const LIVE_RATE: Decimal = dec!(1.1165);
const LIVE_AS_OF: &str = "2026-11-06";

/// The configured static fallback and the operator's as-of label for it.
const STATIC_RATE: Decimal = dec!(1.08);
const STATIC_AS_OF: &str = "2026-06-22";

/// A multi-thread runtime, because [`live_fx::fetch_live_fx_blocking`] blocks
/// the calling thread while the fetch runs on the runtime's own workers —
/// exactly as it does on iced's thread in `cockpit_live`.
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a tokio runtime")
}

/// Resolve a rate the way the binary's plan seam does: one bounded fetch
/// through the source, then the ADR-0101 D5 fallback.
fn resolve_through(source: Arc<dyn data::fx_rate::RateSource>) -> FxRate {
    let rt = runtime();
    let live = live_fx::fetch_live_fx_blocking(&source, rt.handle(), live_fx::PLAN_FETCH_BUDGET);
    live_fx::resolve_fx_rate(live, STATIC_RATE, STATIC_AS_OF)
}

/// The operator-visible plan text for a €200 budget converted at `fx` — the
/// same composition the Forward-plan sizing block renders.
fn plan_text_for(fx: &FxRate) -> (String, BudgetConversion) {
    let conversion = BudgetConversion::new(dec!(200), fx.clone());
    let note: FxNote = conversion.fx_note();
    let text = serialize_plan_export(
        &ui::fixtures::fake_forward_plan(),
        &ui::fixtures::fake_bakeoff_report_mirror(),
        &NarrationState::NotRequested,
        Some(&note),
    );
    (text, conversion)
}

/// **AC4(a).** A live fetch puts the live rate AND its provenance into the
/// label the operator reads: the rate at the precision it was quoted at, the
/// amount that rate actually produces, and `live:ecb as of <publication date>`.
#[test]
fn a_live_success_shows_live_provenance_in_the_label() {
    let fx = resolve_through(Arc::new(FakeRateSource::live(LIVE_RATE, LIVE_AS_OF)));

    assert_eq!(fx.rate(), LIVE_RATE);
    assert_eq!(fx.source(), "live:ecb");
    assert_eq!(fx.as_of(), LIVE_AS_OF);

    let (text, conversion) = plan_text_for(&fx);
    assert!(
        text.contains("live:ecb as of 2026-11-06"),
        "the label must name the live source AND its as-of date:\n{text}"
    );
    assert!(
        text.contains("1.1165"),
        "the label must show the rate at the precision it was quoted at \
         (a 2-dp '1.12' would not reproduce the amount beside it):\n{text}"
    );
    // 200 × 1.1165 = 223.30 — the arithmetic the operator can redo from the line.
    assert_eq!(conversion.usdt().amount(), dec!(223.30));
    assert!(
        text.contains("223.30"),
        "the label must show the amount the LIVE rate produces, not the \
         static rate's 216.00:\n{text}"
    );
    assert!(
        !text.contains("216.00"),
        "the static rate's amount must not appear anywhere once a live rate \
         is in force:\n{text}"
    );
}

/// **AC4(b).** A forced failure — the fake's unreachable arm, standing in for
/// offline / DNS / timeout / non-2xx / unparseable — falls back to the
/// configured static rate, and the `"config"` provenance is what the operator
/// sees. No panic, no empty label, no stale live claim.
#[test]
fn a_forced_failure_falls_back_to_the_static_config_rate() {
    let fx = resolve_through(Arc::new(FakeRateSource::unreachable()));

    assert_eq!(fx.rate(), STATIC_RATE);
    assert_eq!(fx.source(), "config");
    assert_eq!(fx.as_of(), STATIC_AS_OF);

    let (text, conversion) = plan_text_for(&fx);
    assert_eq!(conversion.usdt().amount(), dec!(216.00));
    assert!(
        text.contains("config as of 2026-06-22"),
        "a failed fetch must render the CONFIG provenance:\n{text}"
    );
    assert!(
        !text.contains("live:"),
        "a failed fetch must never leave a live claim in the label:\n{text}"
    );
    assert!(
        text.contains("216.00"),
        "the amount must come from the static rate:\n{text}"
    );
}

/// **AC4(c).** The engine's budget and the displayed amount are the SAME
/// number, because there is one [`BudgetConversion`] and both read it (the
/// ADR-0065 D2 anti-drift discipline, unchanged by the live path).
#[test]
fn display_equals_engine_on_the_live_rate() {
    let fx = resolve_through(Arc::new(FakeRateSource::live(LIVE_RATE, LIVE_AS_OF)));
    let (text, conversion) = plan_text_for(&fx);

    // The engine sizes against this.
    let engine_budget = conversion.usdt();
    // The display renders this — off the same object, by construction.
    let displayed = conversion.fx_note();

    assert_eq!(
        engine_budget.amount(),
        displayed.usdt,
        "the engine's budget and the display's amount must be the same number"
    );
    assert_eq!(displayed.rate, fx.rate(), "and the same rate");
    assert!(
        text.contains(&fmt_usdt_plain(engine_budget.amount())),
        "the rendered line must carry the engine's budget verbatim \
         ({}):\n{text}",
        fmt_usdt_plain(engine_budget.amount())
    );
    // A second conversion of the same inputs is the same value — i.e. the
    // single-multiply invariant is what makes this test pass, not luck.
    let again = BudgetConversion::new(dec!(200), fx.clone());
    assert_eq!(again.usdt().amount(), engine_budget.amount());
}

/// **AC2 — fetch timing.** Constructing a source fetches nothing (there is no
/// boot fetch), and each explicit resolution is exactly ONE call: no retry
/// (D5), no poll (D7). Rendering the label again does not fetch at all.
#[test]
fn the_live_rate_is_fetched_only_when_asked() {
    let fake = Arc::new(FakeRateSource::live(LIVE_RATE, LIVE_AS_OF));
    assert_eq!(fake.calls(), 0, "constructing the source must not fetch");

    let source: Arc<dyn data::fx_rate::RateSource> = fake.clone();
    let fx = resolve_through(source);
    assert_eq!(
        fake.calls(),
        1,
        "one resolution is exactly one attempt — no retry"
    );

    let _ = plan_text_for(&fx);
    let _ = plan_text_for(&fx);
    assert_eq!(
        fake.calls(),
        1,
        "rendering the label must not fetch; only a plan render or an \
         explicit refresh does"
    );
}

/// The resolution lives in ONE place, so the pre-launch budget hint, the
/// forward plan and the Live screen cannot disagree about the provenance.
#[test]
fn the_cockpit_resolves_live_then_static() {
    let mut cockpit = ui::state::Cockpit::new();
    cockpit.advisor_eur_usd_rate = STATIC_RATE;
    cockpit.advisor_eur_usd_as_of = STATIC_AS_OF.into();

    // Cold: nothing has been fetched (ADR-0101 D7), so the static path.
    assert!(cockpit.advisor_live_fx.is_none());
    let cold = cockpit.resolved_fx_rate();
    assert_eq!(cold.source(), "config");
    assert_eq!(cold.rate(), STATIC_RATE);

    // After a successful fetch: the live rate, everywhere.
    cockpit.advisor_live_fx = Some(resolve_through(Arc::new(FakeRateSource::live(
        LIVE_RATE, LIVE_AS_OF,
    ))));
    let warm = cockpit.resolved_fx_rate();
    assert_eq!(warm.source(), "live:ecb");
    assert_eq!(warm.rate(), LIVE_RATE);

    // After a failed refresh the binary CLEARS the cache, so the label stops
    // claiming a live rate rather than showing a stale one under a fresh press.
    cockpit.advisor_live_fx = None;
    assert_eq!(cockpit.resolved_fx_rate().source(), "config");
}

/// **ADR-0101 D4.** The ECB publishes once per working day, so this rate can
/// legitimately be one to three days old. No copy may present it as the rate
/// right now — which is why it ships WITH an as-of rather than as a bare
/// number, and why the word is "as of".
#[test]
fn the_label_never_claims_the_rate_is_current() {
    let composed = fmt_fx_provenance("live:ecb", LIVE_AS_OF);
    assert_eq!(composed, "live:ecb as of 2026-11-06");

    for forbidden in ["now", "current", "live rate", "real-time", "realtime"] {
        assert!(
            !ui::strings::FX_PROVENANCE_WITH_AS_OF_FMT
                .to_lowercase()
                .contains(forbidden),
            "the provenance format must not present an ECB working-day rate as \
             {forbidden:?} (ADR-0101 D4); it reads: {:?}",
            ui::strings::FX_PROVENANCE_WITH_AS_OF_FMT
        );
    }
    assert!(
        ui::strings::FX_PROVENANCE_WITH_AS_OF_FMT.contains("as of"),
        "the provenance format must date the rate it shows"
    );
}
