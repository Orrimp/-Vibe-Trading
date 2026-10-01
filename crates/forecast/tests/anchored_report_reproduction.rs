//! Reproduction gates for the anchored reports the **forecast binaries** produce
//! (story 1-29, bug-log `#128`).
//!
//! ## What this closes
//!
//! `scripts/verify_anchors.sh` hashes the COMMITTED bodies and never re-runs anything, so for
//! any row whose producer has no gate, `ANCHORS PASS (119 / 119)` means only *"the committed
//! bytes still hash to what we wrote down"* — never *"the code still produces them"*
//! (bug-log `#93`). `docs/dev-notes/anchor-gate-coverage-audit-2026-09-26.md` counted **15**
//! such rows in the forecast/report-binary family. This file gates **14** of them; the one
//! that remains declares, in code, why it cannot be — see `OMITTED`.
//!
//! The derivation of all 15, with every `file:line` behind the conditions below, is
//! `docs/dev-notes/forecast-bin-repro-recipes-2026-09-27.md`.
//!
//! ## The condition, declared (AC3)
//!
//! bug-log `#112`: a body-SHA comparison is meaningless without its condition. Here the
//! condition is **(binary × cargo features × build profile × CWD × invocation × corpus)**, and
//! every part of it is a field on `Gate` rather than a convention a reader has to reconstruct:
//!
//! - **profile** — `--release` for all fourteen. Not a preference: `determinism.rs:1091-1098`
//!   measured release-vs-debug to leave realdata bodies byte-identical, and the anchored
//!   front-matter `wall_clock_s` values are release numbers.
//! - **features** — `Gate::features`. `""` is a *declaration*, not an omission, and the two
//!   bins that need none say so in `features_means`. Cargo refuses `forecast_distribution`
//!   without `candle` (`crates/forecast/Cargo.toml:56`); `vol_verdict` does **not** refuse and
//!   silently takes a different code path, which is why `vol_verdict_is_candle_invariant`
//!   measures the claim the recipes doc could only infer.
//! - **CWD = the workspace root.** Every input in all six bins is bare-CWD-relative
//!   (recipes § 3); `cargo test` would otherwise run them from the package root.
//! - **`--out-dir` to a tempdir, always, and the runner owns the flag** (AC4). A gate cannot
//!   pass it, because `GATES` is asserted not to contain it. `verify_anchors.sh` resolves each
//!   anchor to the NEWEST match, so a red run that landed its body in `evidence/` would flip
//!   the corpus gate as a side effect of having been run (bug-log `#113`). Each gate also
//!   asserts `git status` over `evidence/` and the checkpoint dir is **unchanged across its own
//!   run** — AC4 as a measurement, not as a convention.
//! - **the anchor itself is read from `evidence/anchors.toml` at run time**, never copied into
//!   this file. There is no second place for it to drift from, and `anchored_sha` asserts the
//!   duplicate `(scenario, version)` rows agree rather than taking the first one it sees.
//!
//! ## Witness before SHA (AC3)
//!
//! Each gate names one or more human-legible lines from the report body and asserts them
//! **before** hashing, each with what it proves. `sigma_train` is the clearest case: it is the
//! single value `--metadata-path` moves (`10.954250` plain → `0.018016` recalibrated), so a
//! gate pointed at the wrong overlay fails as *"you ran the other variant"* instead of as an
//! opaque 64-hex mismatch. A changed condition must not be able to read as drift.
//!
//! ## Running these gates takes the corpus out of play while they run
//!
//! The AC4 check compares `git status` over `evidence/` and the checkpoint directory **across each
//! run**, so anything that changes that output mid-run makes the gate report UNMEASURED — including
//! a `git commit`, a landed re-emission, or an editor save. That is deliberate: from inside the
//! gate, "the binary wrote there" and "someone else wrote there" are indistinguishable, and
//! certifying the run anyway would be the whole point thrown away.
//!
//! Measured 2026-10-01: `threshold_sweep_bs1_reproduces_anchor` refused on `M evidence/anchors.toml`
//! during a § D6.b re-lock running in another pane. The refusal was right; its message was not —
//! it read as an accusation of the binary, and now names both causes.
//!
//! Practical order, then: settle the corpus, commit, **then** run the gates.
//!
//! ## Do NOT add this file to `check_determinism_anchors.py`'s `SCANNED_FILES`
//!
//! That script cross-checks hardcoded anchor-SHA literals against `anchors.toml`. This file has
//! none — it reads the anchor at run time, which is the stronger arrangement and leaves the script
//! nothing to do. Adding it would fail twice over: the script errors on a scanned file that yields
//! zero literals, and the 64-hex strings that ARE here are **witnesses** — `model_revision` and the
//! corpus `REVISION.toml` SHA — which are not anchors and will not resolve as any.
//!
//! ## What a RED means
//!
//! A measurement, not a bug in this file (story 1-29 AC6). Recipes § 4.4 *predicts* drift for
//! the three `sharpe-comparison-*` rows, because the backtest sub-scenarios they are derived
//! from were re-emitted on 2026-09-26 after those rows were locked. That prediction is dated
//! inference. A red is triaged to a cause at a `file:line` and resolved by an ADR-0038 § D6.b
//! re-emission — **never** by re-pinning the anchor to the produced value, which is bug-log
//! `#77` and converts a caught drift into a silent one.
//!
//! ## What this does NOT prove
//!
//! Reproducibility *on this machine's corpus*. The gates report UNMEASURED — never green —
//! when `data/binance/REVISION.toml` is absent, per bug-log `#113` requirement 6.

#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};

// ── The table ────────────────────────────────────────────────────────────────

/// One reproduction gate: the full condition, plus the witnesses that prove it held.
struct Gate {
    /// `evidence/anchors.toml` `scenario = "…"` key. The anchor is READ, never copied here.
    scenario: &'static str,
    /// The `version = "…"` namespace this gate claims to assert. Checked to exist.
    anchor_ns: &'static str,
    /// Cargo package the bin lives in. Ten of the twelve are `forecast`; the two
    /// `threshold_sweep` rows are `backtest`, and they share this runner rather than growing a
    /// second one (bug-log `#112` requirement 0).
    package: &'static str,
    /// Cargo bin target in `-p {package}`.
    bin: &'static str,
    /// `--features` for the build. `""` means the flag is omitted — a declaration (AC3).
    features: &'static str,
    /// Why that feature set, in one line. Required: an unexplained condition is a guess.
    features_means: &'static str,
    /// argv after `--`, EXCLUDING `--out-dir`, which the runner owns (AC4).
    args: &'static [&'static str],
    /// Needs `target/release/backtest` built `--features candle,realdata` (the shell-out family).
    needs_backtest_bin: bool,
    /// Pass `--anchor-dir <tempdir>` as well. `recalibrate_sigma_train` has a SECOND write — the
    /// `.metadata.recalibrated.json` overlay — whose directory is that flag, defaulting to the
    /// committed checkpoint tree. The runner owns it for the same reason it owns `--out-dir`: a
    /// gate must not be able to write into the repo (AC4). Only possible since bug-log `#134`
    /// took the flag's value out of the hashed body.
    needs_tempdir_anchor_dir: bool,
    /// Body lines asserted BEFORE the SHA, each with what it proves (AC3).
    witnesses: &'static [(&'static str, &'static str)],
    /// The anchored report's own front-matter `wall_clock_s` — measured, not estimated.
    measured_s: u32,
}

const BS1_OVERLAY: &str = "crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json";
const BS2_OVERLAY: &str = "crates/forecast/checkpoints/anchors/tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.metadata.recalibrated.json";

/// The corpus revision pinned in `data/binance/REVISION.toml` and asserted by every realdata
/// arm at run time. It lands INSIDE the hashed body of `forecast_distribution`
/// (`forecast_distribution.rs:1137`), so it is a witness there and front-matter elsewhere.
const REVISION_SHA: &str = "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7";

const GATES: &[Gate] = &[
    // ── forecast_distribution ×5 — cargo REFUSES without `candle` ────────────
    Gate {
        scenario: "forecast-distribution-bs1-realdata",
        anchor_ns: "v2.6.0-alpha-investigation + noop-baseline",
        package: "forecast",
        bin: "forecast_distribution",
        features: "candle",
        features_means: "required-features at crates/forecast/Cargo.toml:56 — cargo refuses the \
                         target without it, so this condition cannot be got wrong silently",
        args: &["--scenario", "bs1"],
        needs_backtest_bin: false,
        needs_tempdir_anchor_dir: false,
        witnesses: &[
            (
                "| model_revision   | d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2 |",
                "the BS-1 checkpoint, not BS-2 or PatchTST",
            ),
            (
                "| sigma_train      | 10.954250 |",
                "the ORIGINAL metadata — the recalibrated overlay would read 0.018016 here, so \
                 this line is what separates this gate from its -recalibrated sibling",
            ),
            (
                REVISION_SHA,
                "the corpus revision the body itself records (forecast_distribution.rs:1137)",
            ),
        ],
        measured_s: 481,
    },
    Gate {
        scenario: "forecast-distribution-bs2-realdata",
        anchor_ns: "v2.6.0-alpha-investigation + noop-baseline",
        package: "forecast",
        bin: "forecast_distribution",
        features: "candle",
        features_means: "required-features — cargo refuses without it",
        args: &["--scenario", "bs2"],
        needs_backtest_bin: false,
        needs_tempdir_anchor_dir: false,
        witnesses: &[
            (
                "| model_revision   | 3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d |",
                "the BS-2 checkpoint",
            ),
            (REVISION_SHA, "the corpus revision, recorded in the body"),
        ],
        measured_s: 510,
    },
    Gate {
        scenario: "forecast-distribution-bs1-realdata-recalibrated",
        anchor_ns: "v2.6.1-alpha-investigation-recalibrated + noop-baseline",
        package: "forecast",
        bin: "forecast_distribution",
        features: "candle",
        features_means: "required-features — cargo refuses without it",
        args: &["--scenario", "bs1", "--metadata-path", BS1_OVERLAY],
        needs_backtest_bin: false,
        needs_tempdir_anchor_dir: false,
        witnesses: &[
            (
                "| sigma_train      | 0.018016 |",
                "the RECALIBRATED overlay was actually read. `--metadata-path` is the only \
                 switch that selects this variant (recipes § 5 R3); if it were ignored the run \
                 would silently be the plain BS-1 one and differ for that reason, not drift",
            ),
            (
                "## Recalibration delta",
                "the recal-delta section the overlay path unlocks (forecast_distribution.rs:987)",
            ),
            (REVISION_SHA, "the corpus revision, recorded in the body"),
        ],
        measured_s: 487,
    },
    Gate {
        scenario: "forecast-distribution-bs2-realdata-recalibrated",
        anchor_ns: "v2.6.1-alpha-investigation-recalibrated + noop-baseline",
        package: "forecast",
        bin: "forecast_distribution",
        features: "candle",
        features_means: "required-features — cargo refuses without it",
        args: &["--scenario", "bs2", "--metadata-path", BS2_OVERLAY],
        needs_backtest_bin: false,
        needs_tempdir_anchor_dir: false,
        witnesses: &[
            (
                "| model_revision   | 3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d |",
                "the BS-2 checkpoint",
            ),
            (
                "## Recalibration delta",
                "the overlay was read, not ignored",
            ),
            (REVISION_SHA, "the corpus revision, recorded in the body"),
        ],
        measured_s: 488,
    },
    Gate {
        scenario: "forecast-distribution-patchtst-bs1-realdata",
        anchor_ns: "v2.5a.0-patchtst + noop-baseline",
        package: "forecast",
        bin: "forecast_distribution",
        features: "candle",
        features_means: "required-features — cargo refuses without it",
        args: &["--scenario", "patchtst-bs1"],
        needs_backtest_bin: false,
        needs_tempdir_anchor_dir: false,
        witnesses: &[
            (
                "| model_revision   | 62520db92f68c1d323f0782bc367c742cf9439631106ddc0fd492188f6d1cd4d |",
                "the PatchTST checkpoint, loaded through a different arm and a different \
                 FeatureConfig (context_bars 336, horizon 24)",
            ),
            (
                "| Inferences       | 76800 |",
                "the PatchTST context window — the TCN arms report 77830 for the same span, so \
                 this number alone separates the two code paths",
            ),
        ],
        measured_s: 405,
    },
    // ── vol_verdict — no required-features, which is the hazard ──────────────
    Gate {
        scenario: "vol-verdict-bs1-realdata",
        anchor_ns: "v3.0.0-volatility + noop-baseline",
        package: "forecast",
        bin: "vol_verdict",
        features: "candle",
        features_means: "NOT enforced by cargo (crates/forecast/Cargo.toml:40-42 declares no \
                         required-features) — declared because the bin's own doc comment \
                         (vol_verdict.rs:12) prescribes it. Whether it is MATERIAL is measured \
                         by vol_verdict_is_candle_invariant, not assumed",
        args: &["--scenario", "bs1"],
        needs_backtest_bin: false,
        needs_tempdir_anchor_dir: false,
        witnesses: &[
            (
                "| checkpoint_revision | 991324772ba077355731c2f551e3412430070b76468f6044261161a9160c0c71 |",
                "the GARCH BS-1 checkpoint, found by prefix glob (vol_verdict.rs:82-105)",
            ),
            (
                "| n_predictions_total | 76800",
                "a real full-year × 10-symbol pass, not a truncated one",
            ),
        ],
        measured_s: 1,
    },
    // ── the shell-out family — the real condition is the backtest binary ─────
    Gate {
        scenario: "regime-verdict-bs1-realdata",
        anchor_ns: "v3.0.0-regime",
        package: "forecast",
        bin: "regime_verdict",
        features: "",
        features_means: "the bin itself needs none (Cargo.toml:44-46). The condition that \
                         matters is the SPAWNED binary: target/release/backtest built \
                         --features candle,realdata. Until bug-log #128c a refusing child was \
                         swallowed into an empty body and still produced a verdict",
        args: &[
            "--scenario",
            "bs1",
            "--backtest-bin",
            "target/release/backtest",
        ],
        needs_backtest_bin: true,
        needs_tempdir_anchor_dir: false,
        witnesses: &[
            (
                "| Scenario             | top10-2024-fy-regime-dispatcher-realdata |",
                "the sub-scenario the child actually ran",
            ),
            (
                "| Total bars           | 87840 |",
                "the child produced a REAL report. This is the #128c witness: a swallowed \
                 refusal zero-fills every statistic, so a non-zero bar count is what separates \
                 a measured run from a fabricated one",
            ),
        ],
        measured_s: 307,
    },
    Gate {
        scenario: "sharpe-comparison-vol-target-bs1-realdata",
        anchor_ns: "v3.0.0-volatility + noop-baseline",
        package: "forecast",
        bin: "sharpe_comparison",
        features: "",
        features_means: "the bin needs none and DOES bail on a refusing child \
                         (sharpe_comparison.rs:414-416) — the correct shape. The condition is \
                         the spawned target/release/backtest with candle,realdata",
        args: &[
            "--scenario",
            "vol-target-bs1",
            "--backtest-bin",
            "target/release/backtest",
        ],
        needs_backtest_bin: true,
        needs_tempdir_anchor_dir: false,
        witnesses: &[
            (
                "| Baseline scenario | top10-2023-1h-momentum",
                "the SYNTHETIC baseline — this family's distinguishing input, and the one \
                 recipes § 4.4 predicts has moved since the row was locked",
            ),
            (
                "| Overlay scenario  | top10-2023-fy-vol-target-overlay-realdata",
                "the vol-target overlay arm",
            ),
            (
                "**What this label does and does not say.**",
                "the `#137` qualifier is IN the body. The operator ruled that this row ships only                  with its label's limits stated inline, so a body carrying `T-VOL-ALPHA-UNLOCKED`                  without them must not pass this gate",
            ),
        ],
        measured_s: 11,
    },
    Gate {
        scenario: "sharpe-comparison-vol-target-bs1-realbaseline",
        anchor_ns: "v3.0.0-volatility-rebaseline + noop-baseline",
        package: "forecast",
        bin: "sharpe_comparison",
        features: "",
        features_means: "as above — the condition is the spawned backtest binary",
        args: &[
            "--scenario",
            "vol-target-bs1-rebaseline",
            "--backtest-bin",
            "target/release/backtest",
        ],
        needs_backtest_bin: true,
        needs_tempdir_anchor_dir: false,
        witnesses: &[
            (
                "| Baseline scenario | top10-2023-fy-momentum-realdata",
                "the REAL-data baseline — the single difference from the row above, and \
                 therefore the line that must not be allowed to read as drift",
            ),
            (
                "| Overlay scenario  | top10-2023-fy-vol-target-overlay-realdata",
                "the same overlay arm as the row above",
            ),
        ],
        measured_s: 11,
    },
    Gate {
        scenario: "sharpe-comparison-regime-dispatcher-bs1-realdata",
        anchor_ns: "v3.0.0-regime",
        package: "forecast",
        bin: "sharpe_comparison",
        features: "",
        features_means: "as above — the condition is the spawned backtest binary",
        args: &[
            "--scenario",
            "regime-dispatcher-bs1",
            "--backtest-bin",
            "target/release/backtest",
        ],
        needs_backtest_bin: true,
        needs_tempdir_anchor_dir: false,
        witnesses: &[
            (
                "| Dispatcher scenario | top10-2023-fy-regime-dispatcher-realdata",
                "the dispatcher arm, which determinism.rs:1603-1618 measured GREEN 2026-09-26",
            ),
            (
                "| Baseline scenario | top10-2023-fy-momentum-realdata",
                "the baseline arm, whose own reproduction state is NOT measured — which is why \
                 recipes § 4.4 calls this row uncertain rather than predicted",
            ),
        ],
        measured_s: 287,
    },
    // ── threshold_sweep ×2 — a `backtest` bin, through this same runner ──────
    //
    // Held while bug-log #129 was open (story 1-29 AC8): `run_cell` never received the routing
    // half of the #67 fix, so from 2026-08-16 its else-less `let Ok(fills) = engine.step(..)`
    // chain dropped cross-symbol orders IN SILENCE. Gating the rows then would have pinned a
    // body produced by that lane as correct-by-definition.
    //
    // AC8's condition is now discharged, in all three parts it named: the operator RULED
    // 2026-09-29 ("fix run_cell"), the lane was repaired, and the behaviour change is DISCLOSED
    // in anchors.toml beside both rows — the re-emission comment states in full which of the
    // three states is correct and why it is neither of the two previously pinned.
    //
    // AC8 assigned these two gates to the repair's story (1-30). That story re-emitted the
    // bodies and shipped WITHOUT building them, which left the rows gated nowhere while this
    // file's own omission note claimed they were gated elsewhere — a false provenance claim of
    // exactly the species filed as bug-log #134, in the file that files it. They are gated here.
    Gate {
        scenario: "threshold-sweep-bs1-realdata-recalibrated",
        anchor_ns: "v2.6.2-threshold-tuning + noop-baseline",
        package: "backtest",
        bin: "threshold_sweep",
        features: "candle,realdata",
        features_means: "required-features at crates/backtest/Cargo.toml:14 — cargo refuses the \
                         target without BOTH, so neither half can be got wrong silently. Note \
                         bug-log #128d: `candle` does not control whether candle is LINKED \
                         (backtest/Cargo.toml:63 enables strategy/forecast unconditionally); it \
                         flips #[cfg] blocks and the required-features line, nothing else",
        args: &["--scenario", "bs1", "--metadata-path", BS1_OVERLAY],
        needs_backtest_bin: false,
        needs_tempdir_anchor_dir: false,
        witnesses: &[
            (
                "| σ_train (recal)   | 0.018015675",
                "the RECALIBRATED overlay was read — the original is 10.954250, so this line is                  what separates the two possible inputs",
            ),
            (
                "| 0.900000    | 18087 |",
                "the LAST gate-survivor row parsed. This is bug-log #128b's witness: a partial                  parse of the predecessor report used to zero-fill the tail, leaving four                  measured values and five fabricated ones that are identical once rendered. A                  non-zero final row is what distinguishes a full parse from a truncated one",
            ),
        ],
        measured_s: 255,
    },
    Gate {
        scenario: "threshold-sweep-bs2-realdata-recalibrated",
        anchor_ns: "v2.6.2-threshold-tuning + noop-baseline",
        package: "backtest",
        bin: "threshold_sweep",
        features: "candle,realdata",
        features_means: "required-features — cargo refuses without both (see the bs1 row)",
        args: &["--scenario", "bs2", "--metadata-path", BS2_OVERLAY],
        needs_backtest_bin: false,
        needs_tempdir_anchor_dir: false,
        witnesses: &[
            (
                "| model_revision    | 3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d |",
                "the BS-2 checkpoint",
            ),
            (
                "| σ_train (recal)   | 0.011913909",
                "the BS-2 recalibrated overlay, not BS-1's 0.018015675",
            ),
        ],
        measured_s: 224,
    },
    // ── recalibrate_sigma_train ×2 — unblocked by bug-log #134 ───────────────
    //
    // #128e had these two down as the only genuinely ungateable pair in the family: the hashed
    // body carried the resolved `--anchor-dir` path, so the only value that reproduced the anchor
    // was the committed checkpoint directory — and a gate would have had to REWRITE the repo to
    // run. It carried it through a sentence that was also FALSE (it asserted read-only status for
    // the one file the run wrote). Correcting the sentence removed the dependency, so these are
    // gateable without any trade (#134, re-emitted 2026-10-01 under § D6.b).
    //
    // Both now take `--anchor-dir <tempdir>` from the runner, and
    // `recalibrate_sigma_train_readonly.rs::body_is_anchor_dir_invariant` is the measurement that
    // the body does not depend on it: two runs per scenario into two DIFFERENT anchor-dirs, equal
    // digests, and the committed checkpoints byte-identical afterwards.
    Gate {
        scenario: "recalibrate-sigma-train-bs1",
        anchor_ns: "v2.6.1-alpha-investigation-recalibrated + noop-baseline",
        package: "forecast",
        bin: "recalibrate_sigma_train",
        features: "candle",
        features_means: "required-features at crates/forecast/Cargo.toml:61 — cargo refuses the \
                         target without it",
        args: &["--scenario", "bs1"],
        needs_backtest_bin: false,
        needs_tempdir_anchor_dir: true,
        witnesses: &[
            (
                "| σ_train (original metadata) | 10.954250 |",
                "the ORIGINAL metadata was read — this bin DERIVES the recalibrated value, so                  seeing the original is what proves it started from the right place",
            ),
            (
                "| σ_train (recalibrated)      | 0.018015675 |",
                "the derived value, which is the whole output of this report",
            ),
            (
                ".safetensors` (anchored safetensors; never written)",
                "the #134 correction is IN the body. Before 2026-10-01 this line named the                  `.metadata.recalibrated.json` the run writes and called it safetensors; a gate                  that passed without this witness would be defending the false sentence",
            ),
        ],
        measured_s: 487,
    },
    Gate {
        scenario: "recalibrate-sigma-train-bs2",
        anchor_ns: "v2.6.1-alpha-investigation-recalibrated + noop-baseline",
        package: "forecast",
        bin: "recalibrate_sigma_train",
        features: "candle",
        features_means: "required-features — cargo refuses without it",
        args: &["--scenario", "bs2"],
        needs_backtest_bin: false,
        needs_tempdir_anchor_dir: true,
        witnesses: &[
            (
                "| model_revision    | 3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d",
                "the BS-2 checkpoint",
            ),
            (
                ".safetensors` (anchored safetensors; never written)",
                "the #134 correction is IN the body",
            ),
        ],
        measured_s: 620,
    },
];

/// The row of the family that is deliberately NOT gated, with the reason.
///
/// This exists so the omissions are **visible**. A family that silently gates most of 15 and
/// reports success is the shape this whole programme was filed against: `coverage_is_complete`
/// below asserts `GATES ∪ OMITTED` is the whole family, so a row cannot fall out of coverage
/// without a test going red.
const OMITTED: &[(&str, &str)] = &[(
    "sharpe-comparison-realdata",
    "NO PRODUCER AT HEAD (bug-log #118, ruled 2026-09-27). Nothing emits this name; it is \
         not gateable, and the ruling is recorded in anchors.toml beside the row itself.",
)];

/// The audit's independent count of the family. If this stops matching `GATES ∪ OMITTED`,
/// either a row was added to the corpus or one quietly left this file.
const FAMILY_SIZE: usize = 15;

// ── Plumbing ─────────────────────────────────────────────────────────────────

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("locate workspace root")
        .to_path_buf()
}

/// The anchored SHA for `scenario`, read out of `evidence/anchors.toml` at run time.
///
/// Three properties this has and a copied constant does not: it cannot drift from the corpus
/// gate's own source of truth; it asserts the **duplicate rows agree** rather than taking the
/// first one (twelve of these scenarios carry two `(scenario, version)` rows); and it asserts
/// the gate's DECLARED namespace is one of the rows actually present — bug-log `#120`, where a
/// gate pinned to the wrong namespace reported a perfectly reproducing scenario as drifted.
///
/// # Panics
///
/// Panics when the scenario has no row, when its rows disagree, or when `declared_ns` is not
/// among them. All three are findings, never skips.
fn anchored_sha(scenario: &str, declared_ns: &str) -> String {
    let text = std::fs::read_to_string(workspace_root().join("evidence/anchors.toml"))
        .expect("read evidence/anchors.toml");

    let mut rows: Vec<(String, String)> = Vec::new(); // (version, sha)
    let (mut cur_scenario, mut cur_version) = (String::new(), String::new());
    for line in text.lines() {
        let t = line.trim();
        if t == "[[anchors]]" {
            cur_scenario.clear();
            cur_version.clear();
        } else if let Some(v) = field(t, "scenario") {
            cur_scenario = v;
        } else if let Some(v) = field(t, "version") {
            cur_version = v;
        } else if let Some(v) = field(t, "sha256")
            && cur_scenario == scenario
        {
            rows.push((cur_version.clone(), v));
        }
    }

    assert!(
        !rows.is_empty(),
        "no evidence/anchors.toml row for {scenario} — a gated scenario with no anchor is a \
         finding, not a skip"
    );
    let shas: std::collections::BTreeSet<&str> = rows.iter().map(|(_, s)| s.as_str()).collect();
    assert_eq!(
        shas.len(),
        1,
        "{scenario} has {} rows carrying DIFFERENT sha256 values: {rows:?}. This gate asserts \
         one body; which row it means is no longer determined.",
        rows.len()
    );
    assert!(
        rows.iter().any(|(v, _)| v == declared_ns),
        "{scenario}: this gate declares namespace {declared_ns:?}, which is not one of the \
         rows in anchors.toml: {:?}. bug-log #120 — the canonical namespace is PER SCENARIO, \
         and pinning the wrong one reports a reproducing scenario as drifted.",
        rows.iter().map(|(v, _)| v.as_str()).collect::<Vec<_>>()
    );
    rows[0].1.clone()
}

/// `key = "value"` → `Some(value)`, tolerating the column padding anchors.toml uses
/// (`version  = "…"`). A padding-blind regex was one of the two no-op probes of the 1-27 pass.
fn field(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?;
    let rest = rest.trim_start().strip_prefix('=')?.trim_start();
    let inner = rest.strip_prefix('"')?;
    let end = inner.find('"')?;
    Some(inner[..end].to_string())
}

fn real_corpus_available() -> bool {
    workspace_root()
        .join("data/binance/REVISION.toml")
        .is_file()
}

/// `git status --porcelain` over the two trees a gate must never modify (AC4).
fn protected_tree_status() -> String {
    let out = std::process::Command::new("git")
        .args([
            "status",
            "--porcelain",
            "--",
            "evidence/",
            "crates/forecast/checkpoints/anchors/",
        ])
        .current_dir(workspace_root())
        .output()
        .expect("spawn git status");
    // A failed `git status` returns EMPTY stdout, and an empty string compares equal to an empty
    // string — so without this the AC4 check would be vacuous exactly when it could not see.
    // That is the file's own subject (`#135` mechanism 3) turned on the file itself.
    assert!(
        out.status.success(),
        "`git status --porcelain` over the protected trees exited {:?}. AC4 cannot be CHECKED, so \
         it is UNMEASURED — not passed.\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr).trim()
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// Serialises `cargo build` across tests: two of these racing on the same output path is the
/// hazard `determinism.rs`'s `BACKTEST_BUILD_MU` exists for.
static BUILD_MU: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn cargo_build(package: &str, bin: &str, features: &str) -> PathBuf {
    let ws = workspace_root();
    let _guard = BUILD_MU.lock().unwrap_or_else(|p| p.into_inner());

    let mut args: Vec<&str> = vec!["build", "--release", "-p", package, "--bin", bin];
    if !features.is_empty() {
        args.push("--features");
        args.push(features);
    }
    let status = std::process::Command::new("cargo")
        .args(&args)
        .current_dir(&ws)
        .status()
        .expect("spawn cargo build");
    assert!(
        status.success(),
        "cargo build --release -p {package} --bin {bin} \
         {} failed — the gate is UNMEASURED, not passed",
        if features.is_empty() {
            "(no --features, declared)".to_string()
        } else {
            format!("--features {features}")
        }
    );
    let path = ws.join("target/release").join(bin);
    assert!(path.is_file(), "built but missing: {}", path.display());
    path
}

/// Run one gate's binary into a tempdir and return `(body, body_sha)`, or the binary's own
/// refusal. Never panics on a refusal — the caller decides how to report it (bug-log `#114`:
/// do not hand-roll a precondition check when the thing itself will tell you).
fn run_gate(gate: &Gate) -> Result<(String, String), String> {
    let ws = workspace_root();
    if gate.needs_backtest_bin {
        cargo_build("backtest", "backtest", "candle,realdata");
    }
    let bin = cargo_build(gate.package, gate.bin, gate.features);

    let out = tempfile::tempdir().expect("create out-dir tempdir");
    let anchor_dir = tempfile::tempdir().expect("create anchor-dir tempdir");
    let before = protected_tree_status();

    let mut cmd = std::process::Command::new(&bin);
    cmd.args(gate.args).arg("--out-dir").arg(out.path());
    if gate.needs_tempdir_anchor_dir {
        cmd.arg("--anchor-dir").arg(anchor_dir.path());
    }
    let output = cmd
        .current_dir(&ws)
        .output()
        .map_err(|e| format!("{}: spawn {}: {e}", gate.scenario, bin.display()))?;

    // AC4, measured rather than assumed: the run must not have touched either protected tree.
    let after = protected_tree_status();
    assert_eq!(
        before, after,
        "UNMEASURED, not failed: a protected tree CHANGED across {}'s run, so this gate cannot \
         certify that the run left the corpus alone.\n\
         before:\n{before}\nafter:\n{after}\n\
         Two causes, and the diff above distinguishes them — read it before blaming the binary:\n\
         (a) the run wrote there. That is the thing this check exists for: a gate that can write \
             into evidence/ flips verify_anchors.sh as a side effect of having been run, because \
             the corpus gate resolves each anchor to the NEWEST match (bug-log #113).\n\
         (b) SOMETHING ELSE touched evidence/ or the checkpoint dir while the gate was running — \
             a concurrent re-lock, a landed re-emission, an editor. The refusal is deliberately \
             conservative and fires either way, because from in here the two are \
             indistinguishable. Measured 2026-10-01: this fired on `M evidence/anchors.toml` \
             during a § D6.b re-lock happening in another pane. Settle the corpus, then re-run.",
        gate.scenario
    );

    if !output.status.success() {
        let so = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let se = String::from_utf8_lossy(&output.stderr).trim().to_string();
        // bug-log #124: a non-zero exit with NOTHING on either stream is a kill, not a refusal.
        let hint = if so.is_empty() && se.is_empty() {
            " — nothing on either stream, so it was almost certainly killed (signal / OOM) \
             rather than refusing the run; re-run before treating this as drift"
        } else {
            ""
        };
        return Err(format!(
            "{}: exit {:?}{hint}\nstdout: {so}\nstderr: {se}",
            gate.scenario, output.status
        ));
    }

    let mut written: Vec<PathBuf> = std::fs::read_dir(out.path())
        .map_err(|e| format!("{}: read out-dir: {e}", gate.scenario))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    written.sort();
    // Exactly one, not "the first one": recipes § 5.1 establishes that each of these
    // invocations emits exactly one report. A second file would mean the gate is hashing an
    // arbitrary member of a set it does not know the shape of.
    if written.len() != 1 {
        return Err(format!(
            "{}: expected exactly 1 .md in the out-dir, found {} ({written:?}). Recipes § 5.1 \
             says each invocation emits one report; this gate no longer knows which body it \
             is hashing.",
            gate.scenario,
            written.len()
        ));
    }

    let body = std::fs::read_to_string(&written[0])
        .map_err(|e| format!("{}: read {:?}: {e}", gate.scenario, written[0]))?;

    // The repo's OWN hasher, never a re-implementation — scripts/hash_report.py is what
    // verify_anchors.sh uses, so the comparison is apples-to-apples by construction.
    let hash = std::process::Command::new("python3")
        .arg(ws.join("scripts/hash_report.py"))
        .arg(&written[0])
        .output()
        .map_err(|e| format!("{}: spawn hash_report.py: {e}", gate.scenario))?;
    let text = String::from_utf8_lossy(&hash.stdout);
    let sha = text
        .split_whitespace()
        .next()
        .ok_or_else(|| {
            format!(
                "{}: hash_report.py printed nothing; stderr: {}",
                gate.scenario,
                String::from_utf8_lossy(&hash.stderr).trim()
            )
        })?
        .to_string();

    Ok((body, sha))
}

/// Save a body that is about to fail, before the tempdir takes it with it.
///
/// The out-dir is deliberately a **tempdir** and not a fixed path: a random directory per run means
/// a body that recorded its own out-dir could never reproduce, which is `#132`'s defect caught for
/// free. The cost is that a red gate deletes the one artefact you need to diff — so it is copied
/// out first and the panic message says where.
fn preserve(scenario: &str, body: &str) -> String {
    let dir = workspace_root().join("target/1-29-repro-failures");
    if std::fs::create_dir_all(&dir).is_err() {
        return "<could not create target/1-29-repro-failures>".to_string();
    }
    let path = dir.join(format!("{scenario}.md"));
    match std::fs::write(&path, body) {
        Ok(()) => path.display().to_string(),
        Err(e) => format!("<could not save the body: {e}>"),
    }
}

/// The one path all fourteen gates go through (AC2): condition → run → witness → SHA.
///
/// # Panics
///
/// On an absent corpus (UNMEASURED, per bug-log #113 req 6), on a refusal, on a witness
/// mismatch, and on drift.
fn assert_gate_reproduces(gate: &Gate) {
    assert!(
        real_corpus_available(),
        "UNMEASURED, not passed: {} needs data/binance/REVISION.toml and it is absent.",
        gate.scenario
    );
    assert!(
        !gate.witnesses.is_empty(),
        "{}: no witness declared. A gate that asserts only a SHA cannot tell a changed \
         condition from drift (AC3).",
        gate.scenario
    );

    let (body, got) = match run_gate(gate) {
        Ok(v) => v,
        Err(why) => panic!(
            "UNMEASURED, not passed: {} — the run did not complete. Its own words:\n{why}\n\
             Reported as unmeasured rather than skipped (bug-log #113 req 6).",
            gate.scenario
        ),
    };

    // Witness BEFORE SHA (AC3): a changed condition must read as a condition mismatch.
    for (needle, means) in gate.witnesses {
        if body.contains(needle) {
            continue;
        }
        let saved = preserve(gate.scenario, &body);
        assert!(
            body.contains(needle),
            "CONDITION MISMATCH for {}, not drift: the body does not contain\n  {needle}\n\
             which proves: {means}\n\
             Declared condition was: bin={} features={:?} ({}) args={:?} cwd=<workspace root> \
             profile=release.\n\
             Fix the condition before reading anything into the SHA (bug-log #112).\n\
             The emitted body was saved to: {saved}",
            gate.scenario,
            gate.bin,
            gate.features,
            gate.features_means,
            gate.args,
        );
    }

    let want = anchored_sha(gate.scenario, gate.anchor_ns);
    let saved = if got == want {
        String::new()
    } else {
        preserve(gate.scenario, &body)
    };
    assert_eq!(
        got, want,
        "DRIFT: {} no longer reproduces its anchored body.\n\
         namespace: {}\n expected: {want}\n got:      {got}\n\
         Every witness PASSED, so the condition held and this is real code-vs-evidence drift — \
         the thing verify_anchors.sh cannot see (bug-log #93).\n\
         Do NOT re-pin the anchor to the produced value (bug-log #77): that converts a caught \
         drift into a silent one. The resolution is an ADR-0038 § D6.b re-emission, worked in \
         docs/dev-notes/1-2{{6,7,8}}-d6b-re-emission-*.md.\n\
         The produced body was saved for diffing: {saved}",
        gate.scenario, gate.anchor_ns,
    );
}

fn gate(scenario: &str) -> &'static Gate {
    GATES
        .iter()
        .find(|g| g.scenario == scenario)
        .unwrap_or_else(|| panic!("no gate named {scenario}"))
}

// ── The gates ────────────────────────────────────────────────────────────────
//
// `#[ignore]`d because the fourteen together are ~76 min of measured compute (the sum of the
// anchored bodies' own wall_clock_s). They are invoked explicitly:
//
//     cargo test -p forecast --test anchored_report_reproduction -- --ignored
//
// The always-running `coverage_is_complete` below is what makes that `#[ignore]` honest: it
// fails when the table goes stale, when an anchor vanishes, when a declared namespace is not
// real, or when a row leaves the family without a written reason. A skip you cannot see is
// indistinguishable from a pass.

/// R-FD-1 — `forecast-distribution-bs1-realdata`. ~481 s.
#[test]
#[ignore = "corpus-gated + ~481 s (anchored wall_clock_s): run with --ignored"]
fn fd_bs1_reproduces_anchor() {
    assert_gate_reproduces(gate("forecast-distribution-bs1-realdata"));
}

/// R-FD-2 — `forecast-distribution-bs2-realdata`. ~510 s.
#[test]
#[ignore = "corpus-gated + ~510 s (anchored wall_clock_s): run with --ignored"]
fn fd_bs2_reproduces_anchor() {
    assert_gate_reproduces(gate("forecast-distribution-bs2-realdata"));
}

/// R-FD-3 — `forecast-distribution-bs1-realdata-recalibrated`. ~487 s.
#[test]
#[ignore = "corpus-gated + ~487 s (anchored wall_clock_s): run with --ignored"]
fn fd_bs1_recalibrated_reproduces_anchor() {
    assert_gate_reproduces(gate("forecast-distribution-bs1-realdata-recalibrated"));
}

/// R-FD-4 — `forecast-distribution-bs2-realdata-recalibrated`. ~488 s.
#[test]
#[ignore = "corpus-gated + ~488 s (anchored wall_clock_s): run with --ignored"]
fn fd_bs2_recalibrated_reproduces_anchor() {
    assert_gate_reproduces(gate("forecast-distribution-bs2-realdata-recalibrated"));
}

/// R-FD-5 — `forecast-distribution-patchtst-bs1-realdata`. ~405 s.
#[test]
#[ignore = "corpus-gated + ~405 s (anchored wall_clock_s): run with --ignored"]
fn fd_patchtst_bs1_reproduces_anchor() {
    assert_gate_reproduces(gate("forecast-distribution-patchtst-bs1-realdata"));
}

/// R-VV-1 — `vol-verdict-bs1-realdata`. ~1 s of run, but a release build first.
#[test]
#[ignore = "corpus-gated (run is ~0.7 s; the cost is the release build): run with --ignored"]
fn vol_verdict_bs1_reproduces_anchor() {
    assert_gate_reproduces(gate("vol-verdict-bs1-realdata"));
}

/// R-RV-1 — `regime-verdict-bs1-realdata`. ~307 s. The most likely of the four shell-out rows
/// to reproduce: its only sub-scenario was measured GREEN at its anchor on 2026-09-26
/// (`determinism.rs:1621-1629`).
#[test]
#[ignore = "corpus-gated + ~307 s (anchored wall_clock_s): run with --ignored"]
fn regime_verdict_bs1_reproduces_anchor() {
    assert_gate_reproduces(gate("regime-verdict-bs1-realdata"));
}

/// R-SC-1 — `sharpe-comparison-vol-target-bs1-realdata`. ~11 s after the backtest build.
///
/// Defends the body re-emitted 2026-10-01 under § D6.b after an **operator ruling** (bug-log
/// `#137`). This row's verdict moved `T-VOL-NO-ALPHA` → `T-VOL-ALPHA-UNLOCKED` when its inherited
/// sub-scenario re-emissions landed, so it was escalated before landing (AD-19) and held while its
/// two siblings went in. The ruling was not simply "re-emit": **re-emit and make the label's limits
/// unmissable inside the body.**
///
/// The body therefore now carries a computed qualifier under the T-classifier row saying that both
/// arms lose money, that the baseline is synthetic, and that the real-baseline comparison of the
/// same overlay is still NO-ALPHA. One of this gate's witnesses asserts that qualifier is present,
/// so the gate cannot go green on a body that prints the label without its limits.
#[test]
#[ignore = "corpus-gated + ~11 s run (the cost is the backtest release build): run with --ignored"]
fn sharpe_comparison_vol_target_reproduces_anchor() {
    assert_gate_reproduces(gate("sharpe-comparison-vol-target-bs1-realdata"));
}

/// R-SC-2 — `sharpe-comparison-vol-target-bs1-realbaseline`. ~11 s. Also predicted to drift.
#[test]
#[ignore = "corpus-gated + ~11 s run (the cost is the backtest release build): run with --ignored"]
fn sharpe_comparison_realbaseline_reproduces_anchor() {
    assert_gate_reproduces(gate("sharpe-comparison-vol-target-bs1-realbaseline"));
}

/// R-SC-3 — `sharpe-comparison-regime-dispatcher-bs1-realdata`. ~287 s. Reproduction
/// **uncertain**: one sub-scenario is measured GREEN, the other is not measured at all.
#[test]
#[ignore = "corpus-gated + ~287 s (anchored wall_clock_s): run with --ignored"]
fn sharpe_comparison_regime_dispatcher_reproduces_anchor() {
    assert_gate_reproduces(gate("sharpe-comparison-regime-dispatcher-bs1-realdata"));
}

/// R-RS-1 — `recalibrate-sigma-train-bs1`. ~487 s. Unblocked by `#134`; defends the body
/// re-emitted 2026-10-01 under § D6.b, whose one read-only claim finally names the file it read.
#[test]
#[ignore = "corpus-gated + ~487 s (anchored wall_clock_s): run with --ignored"]
fn recalibrate_sigma_train_bs1_reproduces_anchor() {
    assert_gate_reproduces(gate("recalibrate-sigma-train-bs1"));
}

/// R-RS-2 — `recalibrate-sigma-train-bs2`. ~620 s, the most expensive row in the family.
#[test]
#[ignore = "corpus-gated + ~620 s (anchored wall_clock_s): run with --ignored"]
fn recalibrate_sigma_train_bs2_reproduces_anchor() {
    assert_gate_reproduces(gate("recalibrate-sigma-train-bs2"));
}

/// R-TS-1 — `threshold-sweep-bs1-realdata-recalibrated`. ~255 s.
///
/// Defends the body re-emitted 2026-09-29 under ADR-0038 § D6.b, which is the FIRST state of
/// this lane that is correct: the anchor it replaced priced cross-symbol fills wrongly, and the
/// state between them dropped those orders silently (bug-log `#129`). The gate exists so the
/// lane cannot walk away from the repaired state the way it walked away from the first one —
/// undisclosed, for six weeks, on an anchored row.
#[test]
#[ignore = "corpus-gated + ~255 s (re-emitted body's own wall_clock_s): run with --ignored"]
fn threshold_sweep_bs1_reproduces_anchor() {
    assert_gate_reproduces(gate("threshold-sweep-bs1-realdata-recalibrated"));
}

/// R-TS-2 — `threshold-sweep-bs2-realdata-recalibrated`. ~224 s. Same lane, same repair.
#[test]
#[ignore = "corpus-gated + ~224 s (re-emitted body's own wall_clock_s): run with --ignored"]
fn threshold_sweep_bs2_reproduces_anchor() {
    assert_gate_reproduces(gate("threshold-sweep-bs2-realdata-recalibrated"));
}

/// The one unresolved caveat story 1-29 refused to close by reading: `vol_verdict` compiles
/// both with and without `candle` and takes a different code path either way
/// (`vol_verdict.rs:811-831`). A code-reading argument says only the CONTAINER differs
/// (`Tensor` vs `Vec<f32>`, `features.rs:811-824`) and the numbers are identical.
///
/// That is a reading, not a measurement. This runs it both ways and compares the bodies.
/// Cheap — 0.7 s per run — so there was never a cost reason to keep inferring it.
#[test]
#[ignore = "corpus-gated; two release builds of vol_verdict: run with --ignored"]
fn vol_verdict_is_candle_invariant() {
    assert!(
        real_corpus_available(),
        "UNMEASURED, not passed: needs data/binance/REVISION.toml and it is absent."
    );
    let with = gate("vol-verdict-bs1-realdata");
    let without = Gate {
        features: "",
        features_means: "the OTHER half of the measurement — deliberately the undeclared \
                         condition, to find out whether it is material",
        ..*with
    };
    let (_, sha_with) = run_gate(with).expect("vol_verdict --features candle");
    let (_, sha_without) = run_gate(&without).expect("vol_verdict with no --features");
    assert_eq!(
        sha_with, sha_without,
        "vol_verdict is NOT candle-invariant: --features candle produces {sha_with} and a \
         featureless build produces {sha_without}.\n\
         The recipes doc (§ 2.2) recorded the invariance as an INFERENCE from \
         features.rs:811-824 and said it needed asserting. It is now measured, and it is \
         false: `candle` is part of this row's condition and belongs in its declaration, not \
         in a comment saying it probably does not matter."
    );
}

// ── The companion that always runs ───────────────────────────────────────────

/// Cheap, corpus-independent, and the reason the gates above may be `#[ignore]`d.
///
/// Modelled on `theta_surface_reproduction.rs::corpus_gated_theta_tests_are_declared`: it does
/// not fail when the corpus is absent — it fails when the skips, or the omissions, become
/// **invisible**.
#[test]
fn coverage_is_complete() {
    assert_eq!(
        GATES.len() + OMITTED.len(),
        FAMILY_SIZE,
        "the forecast-binary family is {FAMILY_SIZE} anchored rows \
         (anchor-gate-coverage-audit-2026-09-26.md § 2). This file accounts for {} gated + {} \
         omitted. A row that is neither is a row nobody is watching.",
        GATES.len(),
        OMITTED.len()
    );

    // No scenario may appear twice, or in both lists.
    let mut seen = std::collections::BTreeSet::new();
    for s in GATES
        .iter()
        .map(|g| g.scenario)
        .chain(OMITTED.iter().map(|(s, _)| *s))
    {
        assert!(seen.insert(s), "{s} is listed twice");
    }

    // Every gated row resolves to a real, single-valued anchor under its DECLARED namespace.
    for g in GATES {
        let sha = anchored_sha(g.scenario, g.anchor_ns);
        assert_eq!(sha.len(), 64, "{}: malformed anchor sha", g.scenario);
        assert!(
            !g.witnesses.is_empty(),
            "{}: no witness — AC3 requires a declared condition asserted before the SHA",
            g.scenario
        );
        assert!(
            !g.features_means.is_empty(),
            "{}: features {:?} declared with no reason. `\"\"` is a declaration only when it \
             says why.",
            g.scenario,
            g.features
        );
        // AC4, structurally: the runner owns --out-dir, and no gate may name evidence/.
        for flag in ["--out-dir", "--anchor-dir"] {
            assert!(
                !g.args.contains(&flag),
                "{}: passes {flag} itself. The runner owns every write-destination flag so that \
                 no gate can write into the repo (AC4). For --anchor-dir, declare \
                 `needs_tempdir_anchor_dir: true` instead.",
                g.scenario
            );
        }
        assert!(
            !g.args.iter().any(|a| a.contains("evidence/")),
            "{}: argv names evidence/ — a gate must not read a path out of the corpus it is \
             testing, and must never write into it (AC4).",
            g.scenario
        );
    }

    // Every omitted row carries a reason long enough to be one.
    for (s, why) in OMITTED {
        assert!(
            why.len() > 60,
            "{s} is omitted with a {}-character reason. An omission without a cause is a \
             silent gap (story 1-29 AC7).",
            why.len()
        );
        // An omitted row must still BE in the corpus; otherwise the reason is stale.
        let text = std::fs::read_to_string(workspace_root().join("evidence/anchors.toml"))
            .expect("read anchors.toml");
        assert!(
            text.contains(&format!("scenario = \"{s}\"")),
            "{s} is listed as omitted but has no anchors.toml row — the omission is stale"
        );
    }

    let total: u32 = GATES.iter().map(|g| g.measured_s).sum();
    if real_corpus_available() {
        eprintln!(
            "[1-29] the corpus IS on this machine, so these {} gates SHOULD be run before any \
             verdict about the forecast-binary family:\n\
             \x20   cargo test -p forecast --test anchored_report_reproduction -- --ignored\n\
             \x20   ≈ {} min of measured compute, plus release builds.",
            GATES.len(),
            total / 60
        );
    } else {
        eprintln!(
            "[skip] {} forecast-binary reproduction gates UNMEASURED on this machine: \
             data/binance/REVISION.toml is absent. This is not a pass.",
            GATES.len()
        );
    }
}
