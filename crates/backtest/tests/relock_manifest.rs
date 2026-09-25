//! Story 1-26 — derive the 34 regeneration invocations, and prove each one.
//!
//! ## Why this is a test and not a hand-written list
//!
//! The manifest `scripts/relock/run_surfaces.sh` consumes decides what 15-20 hours of
//! compute produces. A wrong row is discovered at hour nine, or — worse — not at all,
//! because a surface that runs happily under the wrong axis tuple emits a plausible
//! report under an anchored name. The binary's own forge tests exist because exactly that
//! is possible: `--grid tier1 --taker-fee-bps 20` emits anchor `#86`'s EXACT name while
//! filling at a fee it never ran at.
//!
//! So nothing here is written by hand. Every row is DERIVED from the binary's own tables
//! and then checked by the binary's own gatekeepers:
//!
//! - the axis values come from `GridKind::required_*` / `allowed_*`;
//! - `validate_grid_axis_pairing` must accept the tuple (it is what rejects a forged one);
//! - `build_scenario_name` must produce an anchored name from it — the same pure function
//!   the sweep itself uses to stamp the report.
//!
//! A row that survives all three cannot be a guess. And none of it runs a backtest, so
//! the manifest is proven before the compute window opens rather than during it.
//!
//! ## The 34 are read from `anchors.toml`, not copied
//!
//! Positions 86-119, per 1-26 AC2. That numbering is positional: of the 119 rows only 5
//! carry an `anchor #NN` comment, and all 5 match their index. Reading them live means a
//! change to the anchor corpus surfaces here instead of silently desynchronising a
//! committed list.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeMap;

use backtest::sweep_harness::{
    GridKind, SweepScoreSource, build_scenario_name, validate_grid_axis_pairing,
};

/// The generator segment every anchored surface carries.
const GENERATOR_TOKEN: &str = "block-bootstrap-real";

/// The two years the corpus was locked over.
const YEARS: [i32; 2] = [2023, 2024];

const ALL_GRIDS: [GridKind; 11] = [
    GridKind::Tier1,
    GridKind::MrTier1,
    GridKind::CarryTier1,
    GridKind::TsTier1,
    GridKind::TwoCell,
    GridKind::Ts4h,
    GridKind::TsDaily,
    GridKind::Carry4h,
    GridKind::CarryDaily,
    GridKind::BasisTier1,
    GridKind::MnTier1,
];

/// The anchored scenario names at positions 86-119 of `evidence/anchors.toml`.
fn anchored_surfaces() -> Vec<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../evidence/anchors.toml");
    let text = std::fs::read_to_string(&path).expect("anchors.toml must be readable");
    let lines: Vec<&str> = text.lines().collect();
    let mut scenarios = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if l.trim() == "[[anchors]]" {
            for probe in lines.iter().skip(i + 1).take(4) {
                if let Some(rest) = probe.strip_prefix("scenario") {
                    scenarios.push(rest.split('"').nth(1).expect("quoted scenario").to_owned());
                    break;
                }
            }
        }
    }
    assert_eq!(scenarios.len(), 119, "the corpus is 119 anchors");
    scenarios[85..119].to_vec()
}

/// One derived invocation.
struct Row {
    scenario: String,
    grid: GridKind,
    year: i32,
    score_source: SweepScoreSource,
    taker_fee_bps: u32,
}

/// Enumerate every axis tuple the binary would ACCEPT, and keep the ones that name an
/// anchored surface.
fn derive() -> BTreeMap<String, Row> {
    let mut out: BTreeMap<String, Row> = BTreeMap::new();
    let wanted: std::collections::BTreeSet<String> = anchored_surfaces().into_iter().collect();

    for grid in ALL_GRIDS {
        let direction = grid.required_direction();
        let mode = grid.required_selection_mode();
        let horizon = grid.required_horizon();
        let slippage = grid.required_slippage_bps();
        let paths = grid.required_paths().unwrap_or(200);
        let seed = grid.required_ensemble_seed().unwrap_or(0);

        for &score_source in grid.allowed_score_sources() {
            for &fee in grid.allowed_taker_fee_bps() {
                // The gatekeeper. A tuple it refuses is a tuple the sweep would refuse.
                if validate_grid_axis_pairing(
                    grid,
                    direction,
                    mode,
                    score_source,
                    horizon,
                    fee,
                    slippage,
                    paths,
                    seed,
                )
                .is_err()
                {
                    continue;
                }
                for year in YEARS {
                    let name = build_scenario_name(
                        grid,
                        direction,
                        score_source,
                        mode,
                        horizon,
                        year,
                        GENERATOR_TOKEN,
                        fee,
                    );
                    if !wanted.contains(&name) {
                        continue;
                    }
                    assert!(
                        out.insert(
                            name.clone(),
                            Row {
                                scenario: name.clone(),
                                grid,
                                year,
                                score_source,
                                taker_fee_bps: fee,
                            },
                        )
                        .is_none(),
                        "two different axis tuples produce the anchored name {name} — the \
                         manifest would be ambiguous and one of them would be a forgery"
                    );
                }
            }
        }
    }
    out
}

/// **The gate.** Every one of the 34 anchored surfaces is reachable by exactly one
/// accepted axis tuple.
///
/// A surface that cannot be derived means the corpus contains a name this binary can no
/// longer produce — which is a finding about drift, not a reason to hand-write a row.
#[test]
fn every_anchored_surface_derives_from_exactly_one_accepted_tuple() {
    let wanted = anchored_surfaces();
    assert_eq!(
        wanted.len(),
        34,
        "1-26 AC2: positions 86-119 are 34 anchors"
    );

    let derived = derive();
    let missing: Vec<&String> = wanted
        .iter()
        .filter(|w| !derived.contains_key(*w))
        .collect();
    assert!(
        missing.is_empty(),
        "{} of 34 anchored surfaces are NOT reachable from any axis tuple this binary \
         accepts. Do NOT hand-write them into the manifest — a name the code can no \
         longer produce is drift, and 1-26 must say so rather than paper over it.\n\
         unreachable: {missing:#?}",
        missing.len()
    );
    assert_eq!(
        derived.len(),
        34,
        "derived more names than the corpus has — the wanted-set filter leaked"
    );
}

/// The forgery this guards against, asserted directly: the axis tuple that emits anchor
/// `#86`'s name at a fee it never ran is REFUSED. If this ever passes, the derivation
/// above is no longer filtering anything.
#[test]
fn the_known_forgery_is_still_refused() {
    let grid = GridKind::Tier1;
    let err = validate_grid_axis_pairing(
        grid,
        grid.required_direction(),
        grid.required_selection_mode(),
        SweepScoreSource::VolAdjustedReturn,
        grid.required_horizon(),
        20, // the forged fee
        grid.required_slippage_bps(),
        grid.required_paths().unwrap_or(200),
        grid.required_ensemble_seed().unwrap_or(0),
    );
    assert!(
        err.is_err(),
        "tier1 x 20 bps must be refused — it emits anchor #86's exact name at a fee that \
         anchor never ran at, and nothing in the body would show it"
    );
}

/// Write `scripts/relock/surfaces.tsv` from the derivation.
///
/// ```text
/// cargo test -p backtest --test relock_manifest write_manifest -- --ignored --nocapture
/// ```
///
/// Ignored because it writes into the repo. The gate above is what keeps the committed
/// file honest; this only regenerates it.
#[test]
#[ignore = "writes scripts/relock/surfaces.tsv"]
fn write_manifest() {
    let derived = derive();
    let mut out = String::from(
        "# Story 1-26 — the 34 regeneration invocations.\n\
         #\n\
         # DERIVED, NOT HAND-WRITTEN. Every row below was produced by\n\
         # `crates/backtest/tests/relock_manifest.rs` from the binary's own `required_*`\n\
         # tables, accepted by its own `validate_grid_axis_pairing`, and named by its own\n\
         # `build_scenario_name`. Re-generate rather than edit:\n\
         #\n\
         #   cargo test -p backtest --test relock_manifest write_manifest -- --ignored\n\
         #\n\
         # scenario\\tbinary\\targs\n",
    );
    for row in derived.values() {
        out.push_str(&format!(
            "{}\tparam_robustness_sweep\t--grid {} --year {} --score-source {} --taker-fee-bps {}\n",
            row.scenario,
            grid_flag(row.grid),
            row.year,
            score_flag(row.score_source),
            row.taker_fee_bps,
        ));
    }
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/relock/surfaces.tsv");
    std::fs::write(&path, out).expect("write the manifest");
    println!("wrote {} rows to {}", derived.len(), path.display());
}

/// clap's kebab-case rendering of a `GridKind`.
fn grid_flag(g: GridKind) -> &'static str {
    match g {
        GridKind::Tier1 => "tier1",
        GridKind::MrTier1 => "mr-tier1",
        GridKind::CarryTier1 => "carry-tier1",
        GridKind::TsTier1 => "ts-tier1",
        GridKind::TwoCell => "two-cell",
        GridKind::Ts4h => "ts4h",
        GridKind::TsDaily => "ts-daily",
        GridKind::Carry4h => "carry4h",
        GridKind::CarryDaily => "carry-daily",
        GridKind::BasisTier1 => "basis-tier1",
        GridKind::MnTier1 => "mn-tier1",
    }
}

fn score_flag(s: SweepScoreSource) -> &'static str {
    match s {
        SweepScoreSource::VolAdjustedReturn => "vol-adjusted-return",
        SweepScoreSource::Carry => "carry",
        SweepScoreSource::BasisReversal => "basis-reversal",
        SweepScoreSource::MnBasisSpread => "mn-basis-spread",
        SweepScoreSource::MnFundingSpread => "mn-funding-spread",
        SweepScoreSource::MnBasisFundingResidual => "mn-basis-funding-residual",
    }
}
