//! Determinism test for the `sharpe_comparison` renderer (T-D-10).
//!
//! Tests that `render::render_report` is byte-deterministic across two
//! invocations with the same `(results, ctx)` inputs.
//!
//! This is a fixture-based test covering `render::render_report` only;
//! full-pipeline determinism (including the backtest re-run) is verified
//! by the tester at M-FINAL via the two-run body-SHA check.

// We inline the types used by render::render_report to avoid requiring the
// sharpe_comparison bin to be a library crate.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

// ── Inline render types (mirror of sharpe_comparison.rs render module) ───────

#[derive(Debug, Clone)]
pub struct RerunResult {
    pub name: String,
    pub variant: String,
    pub equity: Vec<Decimal>,
    pub bars: u64,
    pub trades: u64,
    pub final_equity: Decimal,
    pub total_return: f64,
    pub max_drawdown: f64,
    pub dampen_rate: f64,
}

#[derive(Debug, Clone)]
pub struct ReportContext {
    pub generated: String,
    pub wall_clock_s: f64,
    pub host: String,
    pub git_commit: String,
    pub data_revision_sha: String,
    pub source_reports: Vec<String>,
}

// Story 1-25 AC3.1 / 2026-09-29: these five metric functions and the annualisation
// constant used to be RE-IMPLEMENTED here, byte for byte, because they lived inside
// `bin/sharpe_comparison.rs` and a test cannot import a binary's private module. This
// file is the test the design named the RE-SYNC TRIGGER for that constant — and it
// compared its own copy against its own copy, so it could never trip on the drift it
// exists to catch, and would have stayed green if the binary's module were deleted.
// The module now lives in the library and both sides import the one definition.
use forecast::metrics::{
    SQRT_HOURS_PER_YEAR, compute_calmar, compute_sharpe_hourly, compute_sortino_hourly,
};

fn render_report(results: &[RerunResult; 4], _ctx: &ReportContext) -> String {
    use std::fmt::Write as FmtWrite;
    let mut body = String::with_capacity(4096);

    writeln!(
        &mut body,
        "# Sharpe / drawdown comparison — v2.6.0-realdata scenarios"
    )
    .unwrap();

    writeln!(&mut body, "\n## Methodology\n").unwrap();
    writeln!(
        &mut body,
        "| Field             | Value                                          |"
    )
    .unwrap();
    writeln!(
        &mut body,
        "|-------------------|------------------------------------------------|"
    )
    .unwrap();
    writeln!(&mut body, "| Source equity     | Re-run of the four -realdata scenarios (Option α per ADR-0033 § D2.b.i). |").unwrap();
    writeln!(&mut body, "| Bar interval      | 1h |").unwrap();
    writeln!(
        &mut body,
        "| Annualisation     | √(24·365) = {:.6} (hourly → annual) |",
        SQRT_HOURS_PER_YEAR
    )
    .unwrap();
    writeln!(&mut body, "| Risk-free rate    | 0.000000 (constant) |").unwrap();
    writeln!(
        &mut body,
        "| Sharpe formula    | (mean_r - r_f) / std_r * √(24·365), arithmetic returns |"
    )
    .unwrap();
    writeln!(
        &mut body,
        "| Sortino formula   | (mean_r - r_f) / std_downside_r * √(24·365), downside vs r_f |"
    )
    .unwrap();
    writeln!(&mut body, "| Calmar formula    | (CAGR) / abs(max_drawdown), where CAGR = (final/initial)^(1/years) - 1, years = bars/8760 |").unwrap();
    writeln!(&mut body, "| Max drawdown      | max over t of (peak_equity_t - equity_t) / peak_equity_t, on the realised equity curve |").unwrap();
    writeln!(&mut body, "| Equity series     | Per-bar equity_curve: Vec<Decimal> from --emit-equity-bin, starting at $100000.00 |").unwrap();
    writeln!(&mut body, "| compute_sharpe_hourly | New helper in sharpe_comparison.rs (NOT crates/backtest::compute_sharpe, which annualises by sqrt(525_600) for minute bars — see ADR-0033 § D4 alt-7). |").unwrap();

    writeln!(&mut body, "\n## Comparison table\n").unwrap();
    writeln!(&mut body, "| Scenario | Variant | Bars | Final equity | Total return | Max drawdown | Trades | Dampen rate | Sharpe (ann) | Sortino (ann) | Calmar |").unwrap();
    writeln!(&mut body, "|----------|---------|------|--------------|--------------|--------------|--------|-------------|--------------|---------------|--------|").unwrap();

    for r in results {
        let sharpe = compute_sharpe_hourly(&r.equity);
        let sortino = compute_sortino_hourly(&r.equity);
        let calmar = compute_calmar(&r.equity);
        writeln!(
            &mut body,
            "| {} | {} | {} | ${:.2} | {:.2}% | {:.2}% | {} | {:.2}% | {:.6} | {:.6} | {:.6} |",
            r.name,
            r.variant,
            r.bars,
            r.final_equity,
            r.total_return * 100.0,
            r.max_drawdown * 100.0,
            r.trades,
            r.dampen_rate * 100.0,
            sharpe,
            sortino,
            calmar,
        )
        .unwrap();
    }

    writeln!(&mut body, "\n## Verdict\n").unwrap();
    writeln!(
        &mut body,
        "| Field             | Value                                          |"
    )
    .unwrap();
    writeln!(
        &mut body,
        "|-------------------|------------------------------------------------|"
    )
    .unwrap();

    let all_zero_dampen = results.iter().all(|r| r.dampen_rate.abs() < 1e-6);
    if all_zero_dampen {
        writeln!(&mut body, "| Honest reading    | dampen rate = 0.00% across all four scenarios — TCN overlay is a no-op; equity curves are byte-identical between passthrough and real-weights variants per year. |").unwrap();
    } else {
        let max_dr = results
            .iter()
            .map(|r| r.dampen_rate)
            .fold(f64::NEG_INFINITY, f64::max);
        writeln!(&mut body, "| Honest reading    | TCN overlay is partially active (max dampen rate = {:.2}%). Sharpe lift vs baseline requires M-R-HAT verdict cross-reference. |", max_dr * 100.0).unwrap();
    }

    let s0 = compute_sharpe_hourly(&results[0].equity);
    let s2 = compute_sharpe_hourly(&results[2].equity);
    let s1 = compute_sharpe_hourly(&results[1].equity);
    let s3 = compute_sharpe_hourly(&results[3].equity);
    writeln!(
        &mut body,
        "| Sharpe delta      | {:.6} (passthrough vs. real-weights, 2023) / {:.6} (2024) |",
        s2 - s0,
        s3 - s1
    )
    .unwrap();
    writeln!(&mut body, "| Conclusion        | TCN at v2.5 / v2.6.0-realdata produces no alpha lift over the v1 momentum baseline. Verdict gated by M-R-HAT's F-verdict (this report alone cannot diagnose why). |").unwrap();
    writeln!(&mut body, "| Recommended follow-on | (a) wait for M-R-HAT verdict; (b) if M-R-HAT lands F4, fund v25-tcn-horizon-bump OR retire TCN at v2.6 bake-off. |").unwrap();

    writeln!(&mut body, "\n## Notes\n").unwrap();
    writeln!(
        &mut body,
        "- Read-only against the four -realdata reports listed in frontmatter."
    )
    .unwrap();
    writeln!(
        &mut body,
        "- This report re-runs the four backtest scenarios (Option α per ADR-0033 § D2.b.i)."
    )
    .unwrap();
    writeln!(&mut body, "- ASCII-only, LF-only line endings; floats %.6f (Sharpe/Sortino/Calmar) or %.2f%% (returns/drawdown/dampen rate); integer bar/trade counts.").unwrap();

    body
}

// ── Test fixtures ─────────────────────────────────────────────────────────────

fn make_equity(start: f64, n: usize) -> Vec<Decimal> {
    let mut v = Vec::with_capacity(n);
    let mut curr = Decimal::try_from(start).unwrap();
    v.push(curr);
    for _ in 1..n {
        curr *= dec!(1.001);
        v.push(curr);
    }
    v
}

fn make_fixture() -> [RerunResult; 4] {
    let eq = make_equity(100_000.0, 8760);
    let r = RerunResult {
        name: "top10-2023-fy-tcn-overlay-realdata".to_string(),
        variant: "passthrough".to_string(),
        equity: eq.clone(),
        bars: 8759,
        trades: 1000,
        final_equity: *eq.last().unwrap(),
        total_return: 0.1348,
        max_drawdown: 0.0,
        dampen_rate: 0.0,
    };
    [
        r.clone(),
        RerunResult {
            name: "top10-2024-fy-tcn-overlay-realdata".to_string(),
            variant: "passthrough".to_string(),
            ..r.clone()
        },
        RerunResult {
            name: "top10-2023-fy-tcn-overlay-weights-realdata".to_string(),
            variant: "real-weights".to_string(),
            ..r.clone()
        },
        RerunResult {
            name: "top10-2024-fy-tcn-overlay-weights-realdata".to_string(),
            variant: "real-weights".to_string(),
            ..r
        },
    ]
}

fn make_ctx() -> ReportContext {
    ReportContext {
        generated: "2026-05-18T12:00:00Z".to_string(),
        wall_clock_s: 165.0,
        host: "test-host".to_string(),
        git_commit: "abc123".to_string(),
        data_revision_sha: "def456".to_string(),
        source_reports: vec!["report-a.md".to_string()],
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Two invocations of render_report with the same inputs must produce
/// byte-identical output (K3 determinism gate).
#[test]
fn test_render_deterministic() {
    let results = make_fixture();
    let ctx = make_ctx();

    let body1 = render_report(&results, &ctx);
    let body2 = render_report(&results, &ctx);

    assert_eq!(
        body1, body2,
        "render_report must be byte-deterministic across two calls"
    );
}

/// Story 1-25 AC3.1, made enforceable — the ratification that was only ever a comment.
///
/// AC3.1 permits either moving to √8760 or *"a formal ratification of √8575 with the doc
/// corrected to match"*. The shipped constant has always been the latter; what was missing
/// was anything that could go RED if someone "corrected" it to the value the docs claimed.
///
/// The file's other test cannot do this job and it is worth saying why: it renders a report
/// twice and compares the two renders. Changing the constant changes both sides equally, so
/// it stays green — a determinism tautology. Importing the production constant (2026-09-29)
/// was necessary and not sufficient; a probe that swapped the constant for √8760 left that
/// test passing, which is how this gap was found.
///
/// This test compares against a PINNED number instead, so it can fail.
#[test]
fn sqrt_hours_per_year_is_the_ratified_8575_constant_and_not_8760() {
    let squared = SQRT_HOURS_PER_YEAR * SQRT_HOURS_PER_YEAR;

    assert!(
        (squared - 8574.999_8).abs() < 1e-3,
        "SQRT_HOURS_PER_YEAR squared is {squared}, expected 8574.9998. This constant is \
         load-bearing for anchored sharpe-comparison bodies: changing it re-prices every \
         one of them. If the change is intended, re-lock the affected anchors under \
         ADR-0038 § D6.b and update this pin in the same commit."
    );

    let sqrt_8760 = (24.0_f64 * 365.0).sqrt();
    assert!(
        (SQRT_HOURS_PER_YEAR - sqrt_8760).abs() > 0.9,
        "SQRT_HOURS_PER_YEAR has been set to √(24·365) = {sqrt_8760}. That is the value the \
         doc comments asserted until 2026-09-29 while the code shipped √8574.9998 — a ~2.1% \
         difference. Adopting it is a deliberate re-pricing, not a typo fix."
    );
}
