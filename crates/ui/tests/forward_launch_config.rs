//! Bug-log #213: which confidence check each forward launch carries.
//!
//! The live binary sends one of two `agent::ForwardRunConfig`s as
//! `ForwardCommand::Launch`. A bake-off that crowns a row launches its crowned
//! strategy, and "Use this config" on Tune launches the promoted tuned config.
//! `ForwardRunConfig::confidence` becomes `ForwardPlan.confidence`, which the
//! forward plan draws as "How much to trust this pick" and the plan export
//! writes as its TRUST section.
//!
//! Before #213 the binary built both configs inline. The promoted one copied
//! the scorecard of whatever bake-off was `Ready` on the leaderboard, whatever
//! its coin and crown, on the comment "same bake-off, same crown". That
//! scorecard's deflated Sharpe and trial count describe the bake-off's slate
//! and crown, not a tuned config, even on the same coin and family, and the
//! sweep cells a tuned config was picked from are not in the count. Both
//! configs now come from `ui::live` (`crowned_forward_run_config`,
//! `promoted_forward_run_config`), and a promotion carries `confidence: None`
//! (ADR-0070 § D11).
//!
//! ## What each test pins
//!
//! - **L1** `promoted_launch_carries_no_bakeoff_confidence`: (a) a BTCUSDT
//!   bake-off crowned on SMA, with a BTCUSDT SMA grid promoted. This is the
//!   case a "keep it when the coin and family match" rule would keep. (b) An
//!   ETHUSDT grid promoted under the BTCUSDT bake-off. (c) A BTCUSDT grid with
//!   no bake-off on the leaderboard. (d) A bake-off and a grid, with nothing
//!   promoted, gives no config. (e) An ETHUSDT MACD grid promoted with
//!   `Macd{8,21,5}` under the BTCUSDT bake-off, so the strategy and the
//!   override each take a second value. In (a)-(c) and (e) `confidence` is
//!   `None`, and the rest of the config is the promotion's: its strategy, its
//!   coin (ETHUSDT in (b) and (e), not the leaderboard's), its params as the
//!   override, no lookback, and the budget passed in.
//! - **L2** `crowned_launch_carries_its_own_bakeoffs_scorecard`: two bake-offs
//!   whose scorecards differ in every field, on different coins and crowns.
//!   Each config carries its own bake-off's four facts, its crowned strategy and
//!   coin, and no override. A bake-off with no scorecard gives `None`; one with
//!   no crown gives no config.
//! - **L3** `one_bakeoff_crowned_gets_its_scorecard_promoted_gets_none`: one
//!   cockpit, L1 (a)'s, from both sides. The crowned builder on the on-screen
//!   bake-off carries its scorecard; the promoted builder carries none. Same
//!   bake-off, different provenance, different outcome (the `#161` pair).
//! - **L4** `promote_params_map_one_for_one`: the four families, each with
//!   distinct numbers in every field, map to the matching
//!   `agent::ForwardParamOverride` field for field.
//! - **L5** `cockpit_live_builds_no_launch_config_of_its_own`: the binary's
//!   code, with comments and string and char literals blanked by a small lexer,
//!   then matched by substring line by line with whitespace removed, names no
//!   `ForwardRunConfig` (an alias needs the name), `ScorecardSummary`,
//!   `confidence:`, `.confidence`, `promote_params_to_override` or `r#` (a raw
//!   identifier), and names each `ui::live` builder exactly once (by its bare
//!   name, so a `use … as` import counts). The binary cannot be linked from a
//!   test; this guard is the only pin on its wiring.
//!
//! State is driven through `ui::state::update` only. Expected values come from
//! the inputs (the fixture mirrors, the grid's coin, the params; the strategy id
//! and the override are written out here per family), never from the cockpit.
//! Each test collects its failures and reports them once.
//!
//! The pixel proof is LR-213 in `forward_plan_confidence_render.rs` (macOS only):
//! the promoted chain renders no confidence block, against HEAD's rule rebuilt
//! as its negative control. Its assertion (4) is a view check, not a pixel one.
//!
//! ## T-T4 probes (bug-log #187-#191)
//!
//! One edit each, on the real file, located by symbol and quoted line (leading
//! spaces belong to OLD). Restore byte-identical (`cmp` against a copy) and see
//! `Compiling ui` in the next run's log. The command:
//!
//! `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.sdk cargo test -p ui --no-fail-fast --test forward_launch_config --test forward_plan_confidence_render`
//!
//! `include_str!` makes `src/bin/cockpit_live.rs` a build input of this target,
//! so a bin edit rebuilds it. Every test the command runs that a row's RED cell
//! does not name is expected GREEN. LR-213 runs on macOS only.
//!
//! | # | symbol | OLD → NEW | RED | GREEN |
//! |---|---|---|---|---|
//! | P213-1 | `ui::live::promoted_forward_run_config` | `        confidence: None,` → `        confidence: match &cockpit.leaderboard_screen_state.result { crate::state::PanelState::Ready(m) => bakeoff_confidence(m), _ => None },` (HEAD's rule, in the library) | L1 (a), (b), (e); L3; LR-213 (2), (4) | L1 (c), (d); L2; L4; L5; LR-213 (1), (3) |
//! | P213-2 | same | `        confidence: None,` → `        confidence: match &cockpit.leaderboard_screen_state.result { crate::state::PanelState::Ready(m) if m.coin.as_str() == promotion.coin.0.as_str() => bakeoff_confidence(m), _ => None },` ("keep it when the coins match") | L1 (a); L3; LR-213 (2), (4) | L1 (b)-(e); L2; L4; L5 |
//! | P213-3 | `ui::live::crowned_forward_run_config` | `        confidence: bakeoff_confidence(mirror),` → `        confidence: None,` | L2 (both bake-offs); L3; LR-213 (3) | L1; L2's no-scorecard and no-crown cases; L4; L5 |
//! | P213-4 | `ui::live::bakeoff_confidence` | `            deflated_sharpe: sc.deflated_sharpe,` → `            deflated_sharpe: sc.min_btl_years,` | L2; L3 | L1; L4; L5; LR-213 |
//! | P213-5 | `ui::live::promoted_forward_run_config` | `        symbol: promotion.coin.clone(),` → `        symbol: trading_core::Symbol::new("BTCUSDT"),` | L1 (b), (e) | L1 (a), (c), (d) (their grids are on BTCUSDT); L2-L5; LR-213 |
//! | P213-6 | `ui::live::promote_params_to_override` | `            agent::ForwardParamOverride::Sma { fast_len, slow_len }` → `            agent::ForwardParamOverride::Sma { fast_len: slow_len, slow_len: fast_len }` | L4 (sma); L1 (a), (b), (c) | L1 (d), (e); L2; L3; L5; LR-213 |
//! | P213-7 | `cockpit_live.rs`, the promote block | `ui::live::promoted_forward_run_config(&self.cockpit, budget)` → `None::<agent::ForwardRunConfig>` | L5 only | every other test |
//! | P213-8 | `cockpit_live.rs`, the crowned block | `ui::live::crowned_forward_run_config(mirror, budget)` → `ui::live::crowned_forward_run_config(mirror, budget).map(\|c\| agent::ForwardRunConfig { confidence: None, ..c })` | L5 only | every other test |
//! | P213-9 | `cockpit_live.rs`, the promote block (declared GREEN) | insert, above `            // Paint the Live P/L frame`, the line `            let _ = ("ForwardRunConfig { confidence: c.confidence } r#x", r#"fwd_cfg.confidence promoted_forward_run_config"#, '"'); // ScorecardSummary .confidence crowned_forward_run_config` and the line `            /* .confidence r#confidence promote_params_to_override */` | none | L5: literals and comments are blanked before matching |
//! | P213-10 | `cockpit_live.rs`, the promote block | `            if let Some(fwd_cfg) = ui::live::promoted_forward_run_config(&self.cockpit, budget) {` → the same with `mut fwd_cfg` and ` fwd_cfg.confidence = None;` after the brace (patching the builder's result) | L5 only (`.confidence`) | every other test |
//! | P213-11 | same line | the review's mutant A (`use agent::ForwardRunConfig as Cfg; use ui::live::crowned_forward_run_config as crowned;`, the crowned config's confidence moved onto the promoted one by destructuring, no `.confidence`) | L5 only (`ForwardRunConfig`; `crowned_forward_run_config` named 2 times) | every other test |
//! | P213-12 | same line | the review's mutant B (`fwd_cfg.r#confidence = ui::live::r#crowned_forward_run_config(m, budget).and_then(\|c\| c.r#confidence);`) | L5 only (`r#`; `crowned_forward_run_config` named 2 times) | every other test |
//! | P213-13 | same line | the review's control C, mutant B without the `r#` | L5 only (`.confidence`; named 2 times) | every other test |
//! | P213-14 | same line | the same with `mut fwd_cfg` and ` let _ = ('"', "//", r#"/*"#); fwd_cfg.confidence = None;` after the brace (a quote, a `//` and a `/*` inside literals before a real use) | L5 only (`.confidence`) | every other test |
//! | P213-15 | `ui::live::promoted_forward_run_config` | `        strategy: promotion.strategy_id.clone(),` → `        strategy: trading_core::StrategyId::new("v0.5.sma"),` | L1 (e) only | L1 (a)-(d); L2-L5; LR-213 |
//! | P213-16 | same | `        param_override: Some(promote_params_to_override(&promotion.params)),` → `        param_override: Some(agent::ForwardParamOverride::Sma { fast_len: 10, slow_len: 20 }),` | L1 (e) only | L1 (a)-(d); L2-L5; LR-213 |
//!
//! Step 0, red first: (a) L5 against the unmodified pre-fix binary, with the new
//! `ui::live` builders and this file present; (b) P213-1, HEAD's rule in the
//! library. The measured sets are recorded in bug-log `#213`.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rust_decimal_macros::dec;
use smol_str::SmolStr;
use trading_core::{Money, Symbol, Usdt};
use ui::leaderboard::{BakeoffReportMirror, LeaderboardLookback, ScorecardView};
use ui::live::{
    crowned_forward_run_config, promote_params_to_override, promoted_forward_run_config,
};
use ui::state::{Cockpit, Message, PanelState, update};
use ui::tune::{PromoteParams, SweepReportMirror, TuneFamily};

/// The budget every config is built with: not a round number, so a builder that
/// substitutes a default fails.
fn budget() -> Money<Usdt> {
    Money::<Usdt>::from_decimal(dec!(187.42))
}

/// The four confidence facts of a launch config, with each `f64` by its bits,
/// so equal means equal.
type Facts = (usize, u64, bool, u64);

fn summary_facts(s: &backtest::bakeoff::ScorecardSummary) -> Facts {
    (
        s.n_candidates,
        s.deflated_sharpe.to_bits(),
        s.crown_clears_dsr,
        s.min_btl_years.to_bits(),
    )
}

fn view_facts(v: &ScorecardView) -> Facts {
    (
        v.n_candidates,
        v.deflated_sharpe.to_bits(),
        v.crown_clears_dsr,
        v.min_btl_years.to_bits(),
    )
}

/// `grid`, swept on `coin`.
fn swept_on(mut grid: SweepReportMirror, coin: &str) -> SweepReportMirror {
    grid.coin = SmolStr::new(coin);
    grid
}

/// The SMA grid fixture, swept on `coin`.
fn sma_grid(coin: &str) -> SweepReportMirror {
    swept_on(ui::fixtures::fake_sweep_report_mirror(), coin)
}

/// The MACD grid fixture, swept on `coin`.
fn macd_grid(coin: &str) -> SweepReportMirror {
    swept_on(ui::fixtures::fake_sweep_report_mirror_macd(), coin)
}

/// The stock bake-off fixture: BTCUSDT, crowned `v0.sma`, scorecard 13 / 0.61 /
/// false / 6.4.
fn btc_bakeoff() -> BakeoffReportMirror {
    ui::fixtures::fake_bakeoff_report_mirror()
}

/// A second bake-off whose scorecard differs from the stock one in every field,
/// on another coin, with another crown (row 1, `v0.5.macd`).
fn eth_bakeoff() -> BakeoffReportMirror {
    let mut m = ui::fixtures::fake_bakeoff_report_mirror();
    m.coin = SmolStr::new("ETHUSDT");
    m.crowned = Some(1);
    m.scorecard = Some(ScorecardView {
        n_candidates: 7,
        n_eff: 3.3,
        deflated_sharpe: 0.97,
        min_btl_years: 2.1,
        crown_clears_dsr: true,
    });
    m
}

/// A cockpit driven through `update` only: the bake-off `bakeoff` lands on the
/// leaderboard (if any), Tune is opened on `family` and `grid`'s coin, `grid`
/// lands, and `promote` (if any) is pressed. Precondition failures go into
/// `fails`, tagged with `case`.
fn drive(
    case: &str,
    bakeoff: Option<&BakeoffReportMirror>,
    family: TuneFamily,
    grid: SweepReportMirror,
    promote: Option<&PromoteParams>,
    fails: &mut Vec<String>,
) -> Cockpit {
    let grid_coin = grid.coin.clone();
    let mut cockpit = Cockpit::new();
    if let Some(m) = bakeoff {
        update(&mut cockpit, Message::BakeoffRunCompleted(Ok(m.clone())));
        if !matches!(&cockpit.leaderboard_screen_state.result, PanelState::Ready(r) if r == m) {
            fails.push(format!(
                "[{case}] precondition: the bake-off is Ready on the leaderboard"
            ));
        }
    }
    update(
        &mut cockpit,
        Message::OpenTuneEditor {
            family,
            coin: Symbol::new(grid_coin.as_str()),
            lookback: LeaderboardLookback::H1_2024,
        },
    );
    update(&mut cockpit, Message::SweepRunRequested);
    update(&mut cockpit, Message::SweepRunCompleted(Ok(grid.clone())));
    if !matches!(&cockpit.tune_screen_state.result, PanelState::Ready(g) if *g == grid) {
        fails.push(format!(
            "[{case}] precondition: the {grid_coin} grid landed on Tune"
        ));
    }
    if let Some(params) = promote {
        update(&mut cockpit, Message::PromoteSweptConfig(params.clone()));
        match &cockpit.pending_forward_promotion {
            Some(p) if p.coin.0.as_str() == grid_coin.as_str() && p.params == *params => {}
            other => fails.push(format!(
                "[{case}] precondition: Use this config stored a promotion of the {grid_coin} \
                 grid's {params:?}, got {other:?}"
            )),
        }
    }
    cockpit
}

/// The override a promotion of `params` must carry, written out here and not
/// through `promote_params_to_override` (the function under test), so a swapped
/// field there cannot also move the expectation (`#161`).
fn want_override(params: &PromoteParams) -> agent::ForwardParamOverride {
    use agent::ForwardParamOverride as O;
    match *params {
        PromoteParams::Sma { fast_len, slow_len } => O::Sma { fast_len, slow_len },
        PromoteParams::Macd { fast, slow, signal } => O::Macd { fast, slow, signal },
        PromoteParams::Rsi { period, oversold } => O::Rsi { period, oversold },
        PromoteParams::Bollinger { period, k_tenths } => O::Bollinger { period, k_tenths },
    }
}

/// The strategy id a promotion of `params` must launch, written out here and not
/// read from the promotion the reducer stored (`#161`).
fn want_strategy(params: &PromoteParams) -> &'static str {
    match params {
        PromoteParams::Sma { .. } => "v0.5.sma",
        PromoteParams::Macd { .. } => "v0.5.macd",
        PromoteParams::Rsi { .. } => "v0.5.rsi",
        PromoteParams::Bollinger { .. } => "v0.5.bbands",
    }
}

/// Check one promoted config against what its inputs say it must be.
fn check_promoted(
    case: &str,
    cockpit: &Cockpit,
    grid_coin: &str,
    params: &PromoteParams,
    fails: &mut Vec<String>,
) {
    let Some(cfg) = promoted_forward_run_config(cockpit, budget()) else {
        fails.push(format!(
            "[{case}] a promotion is pending, so there is a launch config"
        ));
        return;
    };
    if let Some(c) = &cfg.confidence {
        fails.push(format!(
            "[{case}] #213: a promoted launch carries no bake-off scorecard (ADR-0070 D11), \
             got {c:?}"
        ));
    }
    let want_id = want_strategy(params);
    if cfg.strategy.0.as_str() != want_id {
        fails.push(format!(
            "[{case}] strategy: want {want_id} for {params:?}, got {}",
            cfg.strategy.0
        ));
    }
    if cfg.symbol.0.as_str() != grid_coin {
        fails.push(format!(
            "[{case}] symbol: want the promoted grid's {grid_coin} (#200), got {}",
            cfg.symbol.0
        ));
    }
    let want = Some(want_override(params));
    if cfg.param_override != want {
        fails.push(format!(
            "[{case}] param_override: want {want:?}, got {:?}",
            cfg.param_override
        ));
    }
    if cfg.lookback.is_some() {
        fails.push(format!(
            "[{case}] lookback: want None (real-time only), got Some"
        ));
    }
    if cfg.budget != budget() {
        fails.push(format!(
            "[{case}] budget: want the {} USDT passed in, got {}",
            budget().amount(),
            cfg.budget.amount()
        ));
    }
}

fn report(test: &str, fails: &[String]) {
    assert!(
        fails.is_empty(),
        "{test}: {} failure(s):\n  {}",
        fails.len(),
        fails.join("\n  ")
    );
}

/// **L1.** A promoted launch carries no bake-off confidence, whatever bake-off
/// is on the leaderboard.
#[test]
fn promoted_launch_carries_no_bakeoff_confidence() {
    let mut fails = Vec::new();
    let sma = PromoteParams::Sma {
        fast_len: 10,
        slow_len: 20,
    };
    let btc = btc_bakeoff();

    // (a) Same coin, same family: the case "keep it when they match" would keep.
    let a = drive(
        "a",
        Some(&btc),
        TuneFamily::Sma,
        sma_grid("BTCUSDT"),
        Some(&sma),
        &mut fails,
    );
    let crowned_sma = btc
        .crowned_row()
        .is_some_and(|r| r.strategy.contains("sma"));
    let coins_match = a
        .pending_forward_promotion
        .as_ref()
        .is_some_and(|p| p.coin.0.as_str() == btc.coin.as_str());
    if !(btc.scorecard.is_some() && crowned_sma && coins_match) {
        fails.push(
            "[a] precondition: the leaderboard holds a scorecard, its crown is SMA, and the \
             promotion's coin is the bake-off's"
                .to_string(),
        );
    }
    check_promoted("a", &a, "BTCUSDT", &sma, &mut fails);

    // (b) Another coin under the BTC bake-off.
    let b = drive(
        "b",
        Some(&btc),
        TuneFamily::Sma,
        sma_grid("ETHUSDT"),
        Some(&sma),
        &mut fails,
    );
    check_promoted("b", &b, "ETHUSDT", &sma, &mut fails);

    // (c) No bake-off on the leaderboard (the grid on BTCUSDT, so P213-5 turns
    // only the two ETHUSDT cases, (b) and (e), RED).
    let c = drive(
        "c",
        None,
        TuneFamily::Sma,
        sma_grid("BTCUSDT"),
        Some(&sma),
        &mut fails,
    );
    if !matches!(c.leaderboard_screen_state.result, PanelState::Empty) {
        fails.push("[c] precondition: no bake-off result on the leaderboard".to_string());
    }
    check_promoted("c", &c, "BTCUSDT", &sma, &mut fails);

    // (d) Nothing promoted: no launch config.
    let d = drive(
        "d",
        Some(&btc),
        TuneFamily::Sma,
        sma_grid("BTCUSDT"),
        None,
        &mut fails,
    );
    if let Some(cfg) = promoted_forward_run_config(&d, budget()) {
        fails.push(format!(
            "[d] nothing is promoted, so there is no promoted launch config, got {cfg:?}"
        ));
    }

    // (e) Another family on another coin under the BTC bake-off, so the strategy
    // and the override each take a second value (a builder that hard-codes the
    // SMA id or the SMA override fails here).
    let macd = PromoteParams::Macd {
        fast: 8,
        slow: 21,
        signal: 5,
    };
    let e = drive(
        "e",
        Some(&btc),
        TuneFamily::Macd,
        macd_grid("ETHUSDT"),
        Some(&macd),
        &mut fails,
    );
    check_promoted("e", &e, "ETHUSDT", &macd, &mut fails);

    report("L1", &fails);
}

/// **L2.** A crowned launch carries its own bake-off's scorecard, crown and coin.
#[test]
fn crowned_launch_carries_its_own_bakeoffs_scorecard() {
    let mut fails = Vec::new();
    let (btc, eth) = (btc_bakeoff(), eth_bakeoff());
    let (sb, se) = (btc.scorecard.unwrap(), eth.scorecard.unwrap());
    if sb.n_candidates == se.n_candidates
        || sb.deflated_sharpe.to_bits() == se.deflated_sharpe.to_bits()
        || sb.crown_clears_dsr == se.crown_clears_dsr
        || sb.min_btl_years.to_bits() == se.min_btl_years.to_bits()
        || btc.coin == eth.coin
        || btc.crowned_row().map(|r| &r.strategy) == eth.crowned_row().map(|r| &r.strategy)
    {
        fails.push(
            "fixture: the two bake-offs differ in every scorecard field, the coin and the crown"
                .to_string(),
        );
    }

    for (name, m) in [("btc", &btc), ("eth", &eth)] {
        let Some(cfg) = crowned_forward_run_config(m, budget()) else {
            fails.push(format!("[{name}] a crowned bake-off gives a launch config"));
            continue;
        };
        let want = m.scorecard.as_ref().map(view_facts);
        let got = cfg.confidence.as_ref().map(summary_facts);
        if got != want {
            fails.push(format!(
                "[{name}] confidence: want this bake-off's {:?}, got {:?}",
                m.scorecard, cfg.confidence
            ));
        }
        let crowned = m.crowned_row().map(|r| r.strategy.as_str());
        if Some(cfg.strategy.0.as_str()) != crowned {
            fails.push(format!(
                "[{name}] strategy: want the crowned {crowned:?}, got {:?}",
                cfg.strategy
            ));
        }
        if cfg.symbol.0.as_str() != m.coin.as_str() {
            fails.push(format!(
                "[{name}] symbol: want the bake-off's {}, got {}",
                m.coin, cfg.symbol.0
            ));
        }
        if cfg.param_override.is_some() || cfg.lookback.is_some() {
            fails.push(format!(
                "[{name}] a crowned launch has no override and no lookback, got {:?} / {}",
                cfg.param_override,
                cfg.lookback.is_some()
            ));
        }
        if cfg.budget != budget() {
            fails.push(format!("[{name}] budget: want the budget passed in"));
        }
    }

    // No scorecard: no confidence.
    let mut bare = btc_bakeoff();
    bare.scorecard = None;
    match crowned_forward_run_config(&bare, budget()) {
        Some(cfg) if cfg.confidence.is_none() => {}
        other => fails.push(format!(
            "[no scorecard] want a config with confidence None, got {other:?}"
        )),
    }

    // No crown: no config.
    let mut uncrowned = btc_bakeoff();
    uncrowned.crowned = None;
    if let Some(cfg) = crowned_forward_run_config(&uncrowned, budget()) {
        fails.push(format!("[no crown] want no config, got {cfg:?}"));
    }

    report("L2", &fails);
}

/// **L3.** One bake-off on screen, two provenances: the crowned launch carries
/// its scorecard, the promoted one carries none.
#[test]
fn one_bakeoff_crowned_gets_its_scorecard_promoted_gets_none() {
    let mut fails = Vec::new();
    let btc = btc_bakeoff();
    let sma = PromoteParams::Sma {
        fast_len: 10,
        slow_len: 20,
    };
    let cockpit = drive(
        "L3",
        Some(&btc),
        TuneFamily::Sma,
        sma_grid("BTCUSDT"),
        Some(&sma),
        &mut fails,
    );

    let PanelState::Ready(on_screen) = &cockpit.leaderboard_screen_state.result else {
        fails.push("precondition: a bake-off is on the leaderboard".to_string());
        report("L3", &fails);
        return;
    };
    if btc.scorecard.is_none() {
        fails.push("fixture: the bake-off carries a scorecard".to_string());
    }
    let crowned = crowned_forward_run_config(on_screen, budget()).and_then(|c| c.confidence);
    if crowned.as_ref().map(summary_facts) != btc.scorecard.as_ref().map(view_facts) {
        fails.push(format!(
            "crowned side: want the on-screen bake-off's scorecard {:?}, got {crowned:?}",
            btc.scorecard
        ));
    }
    match promoted_forward_run_config(&cockpit, budget()) {
        Some(cfg) if cfg.confidence.is_none() => {}
        Some(cfg) => fails.push(format!(
            "promoted side #213: the same bake-off's scorecard must not ride on a promotion, \
             got {:?}",
            cfg.confidence
        )),
        None => fails.push("promoted side: a promotion is pending, so there is a config".into()),
    }

    report("L3", &fails);
}

/// **L4.** The params map is one for one: every family, every field.
#[test]
fn promote_params_map_one_for_one() {
    use agent::ForwardParamOverride as O;
    let mut fails = Vec::new();
    let cases = [
        (
            "sma",
            PromoteParams::Sma {
                fast_len: 10,
                slow_len: 20,
            },
            O::Sma {
                fast_len: 10,
                slow_len: 20,
            },
        ),
        (
            "macd",
            PromoteParams::Macd {
                fast: 8,
                slow: 21,
                signal: 5,
            },
            O::Macd {
                fast: 8,
                slow: 21,
                signal: 5,
            },
        ),
        (
            "rsi",
            PromoteParams::Rsi {
                period: 14,
                oversold: 30,
            },
            O::Rsi {
                period: 14,
                oversold: 30,
            },
        ),
        (
            "bollinger",
            PromoteParams::Bollinger {
                period: 20,
                k_tenths: 25,
            },
            O::Bollinger {
                period: 20,
                k_tenths: 25,
            },
        ),
    ];
    for (name, params, want) in cases {
        let got = promote_params_to_override(&params);
        if got != want {
            fails.push(format!("[{name}] {params:?}: want {want:?}, got {got:?}"));
        }
    }
    report("L4", &fails);
}

/// The binary's source. Reading it makes the file a build input of this target.
const COCKPIT_LIVE_SRC: &str = include_str!("../src/bin/cockpit_live.rs");

/// `src` with every comment and every string and char literal blanked to spaces.
/// Newlines are kept, so line numbers hold. A small Rust lexer, enough for this
/// guard: line comments, nested block comments, `"…"` and `b"…"` strings with
/// escapes, raw strings (`r"…"`, `r#"…"#`, `br#"…"#`) and char literals (`'x'`,
/// `'\n'`, `'"'`), told apart from lifetimes (`'a`, `'_`). A raw identifier
/// (`r#name`) is code and stays.
fn code_only(src: &str) -> String {
    fn blank(out: &mut String, ch: char) {
        out.push(if ch == '\n' { '\n' } else { ' ' });
    }
    let c: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        let next = c.get(i + 1).copied();
        // A line comment, to the end of the line.
        if ch == '/' && next == Some('/') {
            while i < c.len() && c[i] != '\n' {
                out.push(' ');
                i += 1;
            }
            continue;
        }
        // A block comment, nested.
        if ch == '/' && next == Some('*') {
            let mut depth = 0usize;
            while i < c.len() {
                if c[i] == '/' && c.get(i + 1) == Some(&'*') {
                    depth += 1;
                    out.push_str("  ");
                    i += 2;
                } else if c[i] == '*' && c.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    out.push_str("  ");
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    blank(&mut out, c[i]);
                    i += 1;
                }
            }
            continue;
        }
        // A raw string: `r`, or `br`, then hashes, then a quote.
        let after_ident = i > 0 && (c[i - 1].is_alphanumeric() || c[i - 1] == '_');
        if !after_ident && (ch == 'r' || (ch == 'b' && next == Some('r'))) {
            let mut j = if ch == 'b' { i + 2 } else { i + 1 };
            let mut hashes = 0;
            while c.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if c.get(j) == Some(&'"') {
                while i <= j {
                    blank(&mut out, c[i]);
                    i += 1;
                }
                while i < c.len() {
                    if c[i] == '"' && (1..=hashes).all(|h| c.get(i + h) == Some(&'#')) {
                        for _ in 0..=hashes {
                            out.push(' ');
                        }
                        i += 1 + hashes;
                        break;
                    }
                    blank(&mut out, c[i]);
                    i += 1;
                }
                continue;
            }
        }
        // A string, with escapes (a `b` prefix stays as code).
        if ch == '"' {
            out.push(' ');
            i += 1;
            while i < c.len() {
                if c[i] == '\\' {
                    blank(&mut out, c[i]);
                    if let Some(&e) = c.get(i + 1) {
                        blank(&mut out, e);
                    }
                    i += 2;
                } else if c[i] == '"' {
                    out.push(' ');
                    i += 1;
                    break;
                } else {
                    blank(&mut out, c[i]);
                    i += 1;
                }
            }
            continue;
        }
        // A char literal: `'\…'` or `'x'`. Anything else after a `'` is a
        // lifetime or a label, which is code.
        if ch == '\'' {
            let end = if next == Some('\\') {
                (i + 3..c.len()).find(|&k| c[k] == '\'')
            } else if c.get(i + 2) == Some(&'\'') {
                Some(i + 2)
            } else {
                None
            };
            if let Some(end) = end {
                while i <= end {
                    blank(&mut out, c[i]);
                    i += 1;
                }
                continue;
            }
        }
        out.push(ch);
        i += 1;
    }
    out
}

/// **L5.** The binary builds no launch config of its own: it calls the two
/// `ui::live` builders, once each, and names none of the pieces a config is
/// made of.
///
/// Comments and string and char literals are blanked first ([`code_only`]), so
/// neither prose nor a literal can trip the guard, and a `//` or a quote inside a
/// literal cannot hide the code after it. Each remaining line is then matched by
/// substring with its whitespace removed (so `x . confidence` is `x.confidence`).
/// Banned: the config type's name at all (an alias needs it), `ScorecardSummary`,
/// the field as `confidence:` or `.confidence`, the params map's name, and `r#`
/// (a raw identifier would spell any of them past a substring match). Each
/// builder is counted by its bare name, so a `use … as` import is a second hit.
#[test]
fn cockpit_live_builds_no_launch_config_of_its_own() {
    let code = code_only(COCKPIT_LIVE_SRC);
    let lines: Vec<(usize, String)> = code
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.split_whitespace().collect::<String>()))
        .collect();
    let mut fails = Vec::new();
    for banned in [
        "ForwardRunConfig",
        "ScorecardSummary",
        "confidence:",
        ".confidence",
        "promote_params_to_override",
        "r#",
    ] {
        let hits: Vec<usize> = lines
            .iter()
            .filter(|(_, l)| l.contains(banned))
            .map(|(n, _)| *n)
            .collect();
        if !hits.is_empty() {
            fails.push(format!(
                "`{banned}` at cockpit_live.rs line(s) {hits:?}; want none"
            ));
        }
    }
    for builder in ["crowned_forward_run_config", "promoted_forward_run_config"] {
        let hits: Vec<usize> = lines
            .iter()
            .flat_map(|(n, l)| std::iter::repeat_n(*n, l.matches(builder).count()))
            .collect();
        if hits.len() != 1 {
            fails.push(format!(
                "`{builder}` named {} time(s), at line(s) {hits:?}; want exactly 1 (the call)",
                hits.len()
            ));
        }
    }
    assert!(
        fails.is_empty(),
        "L5 (bug-log #213): cockpit_live.rs must launch the configs `ui::live` builds, not \
         build or patch one itself. The promoted launch once copied the leaderboard's \
         scorecard in the binary, where no test could see it. Launch configs, their \
         confidence and the params map belong in `ui::live` \
         (`crowned_forward_run_config`, `promoted_forward_run_config`, \
         `promote_params_to_override`), pinned by this file's L1-L4. {} finding(s):\n  {}",
        fails.len(),
        fails.join("\n  ")
    );
}
