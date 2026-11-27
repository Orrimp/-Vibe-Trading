//! T33 — Determinism test.
#![allow(clippy::unwrap_used)]
//!
//! Runs the `btc-2023-1m-sma-cross` scenario twice at seed `0xC0FFEE` and
//! asserts that:
//!   1. Both runs produce identical deterministic-content sha256 hashes of
//!      their report markdown.  The hash covers the report body only,
//!      **excluding** the YAML front-matter block (which contains the
//!      wall-clock `generated:` field).  See `backtest::report_body_hash` for
//!      the canonical convention.
//!   2. Both runs produce identical final equity and trade counts (ledger proxy).
//!
//! Why body-only hashing?  The `generated:` timestamp in the front matter is
//! intentionally kept for operator readability but is non-deterministic by
//! construction.  Everything else in the report — scenario parameters, equity,
//! trade counts, fees, Sharpe, drawdown — is purely a function of the seed and
//! the scenario definition, and must be byte-identical.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{
    Bar, Money, Order, OrderKind, Position, Price, Quantity, RiskLimits, Side, Symbol, TimeInForce,
    Timeframe, Timestamp, Usdt, Venue,
};

const SEED: u64 = 0xC0_FFEE;

// ── Inline mini-backtest (same logic as binary, extracted to a function) ──────

struct RunResult {
    trades: usize,
    final_equity: Decimal,
    signal_count: usize,
    equity_curve_len: usize,
}

fn synthetic_bars_det(count: usize) -> Vec<Bar> {
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    let mut rng = ChaCha20Rng::seed_from_u64(SEED);
    let mut bars = Vec::with_capacity(count);
    let mut close: f64 = 16_500.0;
    let epoch = time::OffsetDateTime::new_utc(
        time::Date::from_calendar_date(2023, time::Month::January, 1).unwrap(),
        time::Time::MIDNIGHT,
    );

    for i in 0..count {
        let u1: f64 = rng.random::<f64>().max(1e-10);
        let u2: f64 = rng.random::<f64>();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        let next = (close * (1.0 + 0.001_10 * z + 0.000_001_9)).clamp(1_000.0, 500_000.0);

        let open_ts = Timestamp::new(epoch + time::Duration::minutes(i as i64));
        let close_ts = Timestamp::new(
            epoch + time::Duration::minutes(i as i64 + 1) - time::Duration::seconds(1),
        );

        let to_dec = |v: f64| Decimal::try_from(v.max(0.01)).unwrap_or(dec!(0.01));
        let mk_price =
            |v: f64| Price::new(to_dec(v)).unwrap_or_else(|_| Price::new(dec!(1)).unwrap());

        bars.push(Bar {
            symbol: Symbol::new("BTCUSDT"),
            tf: Timeframe::OneMinute,
            open: mk_price(close),
            high: mk_price(next.max(close)),
            low: mk_price(next.min(close)),
            close: mk_price(next),
            volume: Quantity::new(to_dec(rng.random::<f64>() * 50.0 + 1.0)).unwrap(),
            trade_count: rng.random_range(10_u32..500_u32),
            local_recv_ts: close_ts,
            open_ts,
            close_ts,
            venue: Venue::Binance,
        });
        close = next;
    }
    bars
}

/// Run a mini-backtest with 1000 bars (fast proxy for the full scenario).
#[tokio::test]
async fn t33_determinism_mini_backtest() {
    let result1 = run_mini().await;
    let result2 = run_mini().await;

    assert_eq!(
        result1.trades, result2.trades,
        "trade count must be identical"
    );
    assert_eq!(
        result1.final_equity, result2.final_equity,
        "final equity must be identical"
    );
    assert_eq!(
        result1.signal_count, result2.signal_count,
        "signal count must be identical"
    );
    assert_eq!(
        result1.equity_curve_len, result2.equity_curve_len,
        "equity curve length must be identical"
    );
}

async fn run_mini() -> RunResult {
    use backtest::MatchingEngine;

    let bars = synthetic_bars_det(1000);
    let registry = strategy::StrategyRegistry::new();
    registry.register(Box::new(strategy::SmaCrossover::new(20, 50)));

    let risk_limits = RiskLimits {
        per_symbol_exposure_cap: dec!(0.40),
        price_sanity_band: dec!(0.20),
        portfolio_exposure_cap: None,
    };
    let sizer = risk::FixedFractionSizer::new(dec!(0.10));
    let config = backtest::paper::MatchConfig {
        slippage_bps: 2,
        taker_fee_bps: 4,
        maker_fee_bps: 2,
        fill_price_mode: backtest::paper::FillPriceMode::BarClose,
    };
    let mut engine = backtest::PaperEngine::new(config, SEED);

    let initial = dec!(100_000);
    let mut cash = initial;
    let mut position_qty = Decimal::ZERO;
    let mut position_cost = Decimal::ZERO;
    let mut trades = 0usize;
    let mut signal_count = 0usize;
    let mut equity_curve = vec![initial];
    let mut position = Position::empty(Symbol::new("BTCUSDT"));

    for bar in &bars {
        let mark = bar.close.get();
        position.last_mark = bar.close;
        let equity = cash + position_qty * mark;
        equity_curve.push(equity);

        let signals = registry.on_bar(bar);
        signal_count += signals.len();

        for sig in &signals {
            let side: Option<Side> = match sig.kind {
                trading_core::SignalKind::Buy if position_qty <= Decimal::ZERO => Some(Side::Buy),
                trading_core::SignalKind::Sell if position_qty > Decimal::ZERO => Some(Side::Sell),
                _ => None,
            };
            if let Some(s) = side {
                let ord = match s {
                    Side::Buy => {
                        let eq: Money<Usdt> = Money::from_decimal(equity);
                        risk::size_and_validate(
                            &sizer,
                            sig.strategy_id.clone(),
                            sig.symbol.clone(),
                            s,
                            eq,
                            bar.close,
                            &position,
                            &risk_limits,
                        )
                        .ok()
                    }
                    Side::Sell => Quantity::new(position_qty)
                        .ok()
                        .filter(|q| q.get() > Decimal::ZERO)
                        .and_then(|q| {
                            Order::new(
                                sig.strategy_id.clone(),
                                sig.symbol.clone(),
                                Side::Sell,
                                q,
                                OrderKind::Market,
                                TimeInForce::Ioc,
                                &position,
                                bar.close,
                                &risk_limits,
                                equity,
                            )
                            .ok()
                        }),
                };
                if let Some(order) = ord
                    && let Ok(fills) = engine.step(bar, vec![order]).await
                {
                    for fill in fills {
                        trades += 1;
                        match fill.side {
                            Side::Buy => {
                                let notional = fill.qty.get() * fill.price.get();
                                cash -= notional + fill.fee.amount();
                                position_qty += fill.qty.get();
                                position_cost += notional;
                                position.base_qty = position_qty;
                                position.cost_basis = Money::from_decimal(position_cost);
                            }
                            Side::Sell => {
                                let notional = fill.qty.get() * fill.price.get();
                                cash += notional - fill.fee.amount();
                                position_qty -= fill.qty.get();
                                if position_qty < Decimal::ZERO {
                                    position_qty = Decimal::ZERO;
                                    position_cost = Decimal::ZERO;
                                }
                                position.base_qty = position_qty;
                            }
                        }
                    }
                }
            }
        }
    }

    RunResult {
        trades,
        final_equity: cash + position_qty * position.last_mark.get(),
        signal_count,
        equity_curve_len: equity_curve.len(),
    }
}

// ── The backtest binary under test (bug-log #229) ─────────────────────────────

/// bug-log #229: the `backtest` binary cargo built for THIS test invocation, checked on
/// every call.
///
/// `CARGO_BIN_EXE_backtest` is the path cargo uplifts the bin to before it runs any
/// integration test of this crate (the bin has no `required-features`). It carries the
/// invocation's profile (`<target-dir>/debug/` under `cargo test`, `<target-dir>/release/`
/// under `cargo test --release`), its `--features`, its target dir and the platform's
/// `.exe`. Nothing to find and nothing to build. It replaces `<ws>/target/debug/backtest`
/// built only `if !exists()`: under `--release` (ADR-0045 § D7.2) that ran whatever debug
/// binary was on disk, and on Windows `exists()` never saw `backtest.exe`.
///
/// The check, made at every spawn so every pin makes it in either profile: the NAME of this
/// test binary's profile directory (`current_exe()`'s grandparent, read at run time from
/// `<target-dir>/<profile>/deps/<exe>`) equals the name of the binary's (`env!`, compile
/// time). Two derivations that share no code. It assumes cargo's `<profile>/deps/` layout
/// for test binaries (the cargo of the 1.94.1 pin; it holds under `--target`, `--profile`
/// and cargo-llvm-cov). If a toolchain bump or a `build.build-dir` layout moves test
/// binaries, every pin panics here: fix this check, not the pins.
///
/// It pins the PATH. Under a no-feature invocation nothing in this target rewrites that file
/// (R-REAL builds only under `--features`); another test binary of the same `cargo test`
/// run may re-link it before this one starts (ADR-0045 § D7.2a). Under `--features` the
/// featured builders below re-link it while these spawns run (bug-log #236, open).
///
/// # Panics
/// When the file is missing (a test binary run outside cargo, a removed `target/`) or the
/// two profile names differ. Neither path runs cargo or writes a file (bug-log #187).
fn backtest_bin() -> std::path::PathBuf {
    let bin = std::path::PathBuf::from(env!("CARGO_BIN_EXE_backtest"));
    assert!(
        bin.is_file(),
        "bug-log #229: CARGO_BIN_EXE_backtest names no file: {} (run this target through cargo)",
        bin.display()
    );
    let test_exe = std::env::current_exe().expect("current_exe");
    let ours = test_exe
        .parent()
        .and_then(|deps| deps.parent())
        .and_then(|profile| profile.file_name());
    let its = bin.parent().and_then(|profile| profile.file_name());
    assert!(
        ours.is_some() && ours == its,
        "bug-log #229: this test runs from profile dir {ours:?} ({}) but would spawn {} \
         (profile dir {its:?}): the pins would run another build's code",
        test_exe.display(),
        bin.display()
    );
    bin
}

/// bug-log #229: a `Command` for `backtest_bin()`. Every spawn in this file that T3
/// (`t229_every_spawn_takes_cargos_binary_or_a_builders`) sees goes through here, except a
/// builder's own and the two parameterised sites (`run_realdata_scenario_once` / `_try`,
/// which spawn the `bin` they are passed); `multi_pair_determinism.rs` spawns the same
/// `env!` inline. T3 sees only the spellings its doc lists.
fn backtest_cmd() -> std::process::Command {
    std::process::Command::new(backtest_bin())
}

// ── T33 report sha256 — real binary-level determinism ─────────────────────────

/// Verify that the backtest binary produces byte-identical report bodies across
/// two runs at the same seed.
///
/// Strategy: spawn the `backtest` binary twice via `std::process::Command`,
/// capture the report file it writes, read the file content, compute the
/// deterministic-content hash (body only, excluding the `generated:` line in
/// the YAML front matter), and assert the two hashes are equal.
///
/// Spawns `backtest_cmd()` (bug-log #229): the binary cargo built for this invocation.
#[test]
fn t33_report_sha256_deterministic() {
    // Use a temp directory for report output so we don't pollute evidence/reports.
    let tmp = tempfile::tempdir().expect("create tempdir");
    let reports_dir = tmp.path().join("evidence/reports");
    std::fs::create_dir_all(&reports_dir).expect("create temp reports dir");

    let run_backtest = || -> String {
        let output = backtest_cmd()
            .args(["--scenario", "btc-2023-1m-sma-cross", "--seed", "0xC0FFEE"])
            .current_dir(tmp.path())
            .output()
            .expect("spawn backtest binary");
        assert!(
            output.status.success(),
            "backtest binary exited non-zero: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // The binary prints "Report written: backtest-runs/<feature>/reports/backtest-<stamp>-<scenario>.md"
        let stdout = String::from_utf8_lossy(&output.stdout);
        let report_rel = stdout
            .lines()
            .find(|l| l.starts_with("Report written: "))
            .map(|l| l.trim_start_matches("Report written: ").trim())
            .expect("could not find 'Report written:' line in binary output");
        // bug-log #210 (ADR-0104 D1): a default run writes under `backtest-runs/`, never
        // under `evidence/`. This pins the final SMA writer in `main.rs`.
        assert!(
            std::path::Path::new(report_rel).starts_with("backtest-runs"),
            "btc-2023-1m-sma-cross: default report {report_rel} is not under backtest-runs/ (bug-log #210)"
        );

        let report_path = tmp.path().join(report_rel);
        std::fs::read_to_string(&report_path)
            .unwrap_or_else(|e| panic!("could not read report {report_path:?}: {e}"))
    };

    let report1 = run_backtest();
    let report2 = run_backtest();

    // Hash only the report body (everything after the YAML front matter).
    // The front matter contains the `generated:` wall-clock timestamp which is
    // legitimately different between runs; everything else must be identical.
    let hash1 = backtest::report_body_hash(&report1);
    let hash2 = backtest::report_body_hash(&report2);

    let hex1 = hash1.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let hex2 = hash2.iter().map(|b| format!("{b:02x}")).collect::<String>();

    assert_eq!(
        hex1,
        hex2,
        "deterministic-content SHA-256 must be identical across two runs at the same seed.\n\
         Report body 1 (first 500 chars):\n{}\n\nReport body 2 (first 500 chars):\n{}",
        {
            let body = backtest::extract_report_body(&report1);
            &body[..500_usize.min(body.len())]
        },
        {
            let body = backtest::extract_report_body(&report2);
            &body[..500_usize.min(body.len())]
        },
    );
}

// ── T229 — the spawned binary is this invocation's own build (bug-log #229) ──

/// bug-log #229: the binary every non-builder spawn takes is this profile's own build.
///
/// `backtest_bin()` makes the check at every spawn; this test makes it once under a name the
/// D7.2 filter selects (`t229_`) and prints the path, so the success path is visible with
/// `--show-output`. Two-sided across the two invocations that run it (bug-log #161): CI's
/// `cargo test` (both legs) expects `…/debug/backtest`, the D7.2 command
/// `cargo test --release …` expects `…/release/backtest`. An accessor hard-coded to debug
/// (the pre-fix path) passes the first and fails the second (probe P2); one hard-coded to
/// release, the reverse (P2b).
#[test]
fn t229_spawned_backtest_is_this_profiles_build() {
    let bin = backtest_bin();
    eprintln!("t229: every non-builder spawn takes {}", bin.display());
}

/// bug-log #229: the only fns in `crates/backtest/tests/` allowed to name a fixed
/// `target/<profile>/backtest` path, each because it needs a binary whose declared condition
/// differs from the invocation's. T2 checks exactly this for each: a `Command::new("cargo")`
/// line; in the string literals from that line up to the fn's first fixed-path line, a
/// `"build"` and the pair `"--bin", "backtest"` in sequence (the build names the bin); that
/// first fixed-path line after the cargo line; and no file-existence probe (`.exists()`,
/// `.is_file()`, `try_exists(`, `metadata(`) before the cargo line. That the fn then spawns or
/// copies what it built is read, not checked. They read the DEFAULT target dir
/// (`<ws>/target`; bug-log #235). The other spawns T3 sees take `backtest_cmd()` (multi_pair:
/// its inline `env!`) or are the two parameterised sites in `BINARY_SPAWN_SITES`. Exact both
/// ways.
const BUILD_THEN_RUN_FNS: &[&str] = &[
    "run_scenario_once_candle",       // debug, --features candle (M3)
    "ensure_realdata_binary",         // debug, --features realdata (T-D-13/14)
    "ensure_realdata_release_binary", // release, --features realdata[,candle] (R-REPRO, #121)
    "ensure_realdata_candle_binary",  // debug, --features realdata,candle (T-D-15)
    "run_scenario_realdata_cwd",      // debug, no features, built only when featured (R-REAL)
];

/// bug-log #229: every fn of the two determinism targets with a `Command::new(` call on a
/// computed program path (not a string literal), as `"<file> <fn>"`. Exact both ways.
/// `backtest_cmd` and multi_pair's helper spawn cargo's own build, in exactly the accessor
/// form; `run_scenario_once_candle` and `run_scenario_realdata_cwd` spawn their own build;
/// `run_realdata_scenario_once` and `run_realdata_scenario_try` spawn the `bin` their caller
/// passes, at every call today an `ensure_realdata_*` result (read 2026-11-27, not pinned).
const BINARY_SPAWN_SITES: &[&str] = &[
    "determinism.rs backtest_cmd",
    "determinism.rs run_scenario_once_candle",
    "determinism.rs run_realdata_scenario_once",
    "determinism.rs run_realdata_scenario_try",
    "determinism.rs run_scenario_realdata_cwd",
    "multi_pair_determinism.rs run_pairs_scenario_once",
];

/// bug-log #229: the name a line declares as a `fn`, through any visibility (`pub`,
/// `pub(crate)`, `pub(super)`, `pub(in …)`) and any run of `const`, `async`, `unsafe` and
/// `extern "…"` in any order; `None` for every other line. T3 self-checks it.
fn fn_decl_name(line: &str) -> Option<String> {
    let mut t = line.trim_start();
    if let Some(rest) = t.strip_prefix("pub") {
        if let Some(inner) = rest.strip_prefix('(') {
            t = inner[inner.find(')')? + 1..].trim_start();
        } else if rest.starts_with(char::is_whitespace) {
            t = rest.trim_start();
        }
    }
    loop {
        let before = t;
        for qualifier in ["const ", "async ", "unsafe "] {
            if let Some(rest) = t.strip_prefix(qualifier) {
                t = rest.trim_start();
            }
        }
        if let Some(rest) = t.strip_prefix("extern ") {
            t = rest.trim_start();
            if let Some(abi) = t.strip_prefix('"') {
                t = abi[abi.find('"')? + 1..].trim_start();
            }
        }
        if t == before {
            break;
        }
    }
    let name: String = t
        .strip_prefix("fn ")?
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// bug-log #229: `(fn, 1-based line number, line)` for every line of a `tests/` source that
/// is not a `//` comment, each under the `fn` declared last above it ("(file scope)" before
/// the first; `fn_decl_name` reads the declaration). It does not parse nesting: a nested `fn`
/// takes over the lines after it. MEASURED 2026-11-25: both determinism targets declare every
/// fn at top level.
fn lines_by_fn(path: &std::path::Path) -> Vec<(String, usize, String)> {
    let src =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut current = String::from("(file scope)");
    let mut out = Vec::new();
    for (i, line) in src.lines().enumerate() {
        let t = line.trim_start();
        if t.starts_with("//") {
            continue;
        }
        if let Some(name) = fn_decl_name(t) {
            current = name;
        }
        out.push((current.clone(), i + 1, line.to_string()));
    }
    out
}

/// bug-log #229: every `*.rs` file under `dir`, walked recursively and sorted. A subdirectory
/// is walked, never refused (the `#227` design proposes a `tests/support/` helper); a
/// symlinked directory is not followed.
fn rs_files_under(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut entries: Vec<std::fs::DirEntry> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|e| e.expect("read a tests/ entry"))
        .collect();
    entries.sort_by_key(std::fs::DirEntry::path);
    let mut out = Vec::new();
    for entry in entries {
        let path = entry.path();
        if entry.file_type().expect("a tests/ entry's type").is_dir() {
            out.extend(rs_files_under(&path));
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push(path);
        }
    }
    out
}

/// bug-log #229, the fixed-path wiring (#190). The check in `backtest_bin()` sees a
/// regression only under `--release`; this sees a helper reverted to the old literal in CI's
/// debug run too, in any `*.rs` under `crates/backtest/tests/` (walked recursively), and holds
/// the builders to the order `BUILD_THEN_RUN_FNS` states. It catches the two literal spellings
/// in `needles`; a path built another way (a `join` chain) is T3's to catch, in the two
/// determinism targets.
#[test]
fn t229_only_build_then_run_helpers_name_a_fixed_profile_path() {
    let needles = [
        concat!("target/", "debug/backtest"),
        concat!("target/", "release/backtest"),
    ];
    let cargo = concat!("Command", "::new(\"cargo\")");
    let file_probes = [".exists()", ".is_file()", "try_exists(", "metadata("];
    let tests_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let files = rs_files_under(&tests_dir);
    let names: Vec<String> = files
        .iter()
        .map(|p| {
            p.strip_prefix(&tests_dir)
                .expect("a path under tests/")
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect();
    for subject in ["determinism.rs", "multi_pair_determinism.rs"] {
        assert!(
            names.iter().any(|n| n == subject),
            "t229 scan did not read {subject} under {} (it read {} files)",
            tests_dir.display(),
            names.len()
        );
    }
    let mut offenders: Vec<String> = Vec::new();
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (path, file) in files.iter().zip(&names) {
        for (f, n, line) in lines_by_fn(path) {
            if !needles.iter().any(|x| line.contains(x)) {
                continue;
            }
            if file == "determinism.rs" && BUILD_THEN_RUN_FNS.contains(&f.as_str()) {
                seen.insert(f);
            } else {
                offenders.push(format!("{file}:{n} fn {f}: {}", line.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "bug-log #229: a fixed target/<profile>/backtest path outside BUILD_THEN_RUN_FNS; spawn backtest_cmd():\n{}",
        offenders.join("\n")
    );
    let declared: std::collections::BTreeSet<String> = BUILD_THEN_RUN_FNS
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    assert_eq!(
        seen, declared,
        "bug-log #229: BUILD_THEN_RUN_FNS must equal the fns that name the path"
    );
    let own = lines_by_fn(&tests_dir.join("determinism.rs"));
    for f in BUILD_THEN_RUN_FNS {
        let body: Vec<&(String, usize, String)> = own.iter().filter(|(g, _, _)| g == f).collect();
        let build = body
            .iter()
            .position(|(_, _, l)| l.contains(cargo))
            .unwrap_or_else(|| {
                panic!("bug-log #229: {f} names a fixed profile path but runs no cargo build")
            });
        let path_at = body
            .iter()
            .position(|(_, _, l)| needles.iter().any(|x| l.contains(x)))
            .expect("an allowlisted fn names the path (seen == declared above)");
        assert!(
            path_at > build,
            "bug-log #229: determinism.rs:{} fn {f} names the fixed path before its cargo build at :{}",
            body[path_at].1,
            body[build].1
        );
        // The string literals from the cargo line up to the first fixed-path line, in order
        // (the odd pieces of a split on `"`; these lines hold no escaped quote).
        let literals: Vec<&str> = body[build..path_at]
            .iter()
            .flat_map(|(_, _, l)| l.split('"').skip(1).step_by(2))
            .collect();
        for (what, ok) in [
            ("`cargo build`", literals.contains(&"build")),
            (
                "`--bin backtest`",
                literals.windows(2).any(|w| w == ["--bin", "backtest"]),
            ),
        ] {
            assert!(
                ok,
                "bug-log #229: fn {f}'s cargo call at determinism.rs:{} never names {what} before its fixed path at :{}",
                body[build].1, body[path_at].1
            );
        }
        if let Some((_, n, l)) = body[..build]
            .iter()
            .find(|(_, _, l)| file_probes.iter().any(|p| l.contains(p)))
        {
            panic!(
                "bug-log #229: determinism.rs:{n} fn {f} probes the file before its cargo build: {}",
                l.trim()
            );
        }
    }
    let nested = names.iter().filter(|n| n.contains('/')).count();
    eprintln!(
        "t229 scan: {} files under {} ({nested} in subdirectories); fixed-path sites only in {:?}",
        names.len(),
        tests_dir.display(),
        BUILD_THEN_RUN_FNS
    );
}

/// bug-log #229, the spawn sites (#190, #191), in the two determinism targets only (#188).
/// The static scans catch these spellings and no others: a `Command::new(` call on a computed
/// program path in a fn outside `BINARY_SPAWN_SITES` (any spelling of the path, a `join`
/// chain included); an accessor site not in its exact accessor form; a `Command` alias
/// (`Command as`); a `Command::new` that is not called where it is named; and a `mod`
/// declaration or `#[path]` attribute that pulls another file into a target. They do not
/// follow a `type` alias, a `<Command>::new(` call, a macro, a path handed to a declared
/// site, a new computed-path spawn inside a declared builder or parameterised site, or a
/// spawn in a file they do not read. The dynamic guard is the per-spawn check in
/// `backtest_bin()` plus T1:
/// every spawn that goes through the accessor is checked, in either profile. This test also
/// self-checks `fn_decl_name`, which both scans use to attribute a line to its fn.
#[test]
fn t229_every_spawn_takes_cargos_binary_or_a_builders() {
    for (decl, want) in [
        ("fn a(", Some("a")),
        ("pub fn b(", Some("b")),
        ("pub(crate) fn c(", Some("c")),
        ("pub(super) async fn d(", Some("d")),
        ("const fn e(", Some("e")),
        ("unsafe fn f(", Some("f")),
        ("pub(crate) const unsafe fn g<T>(", Some("g")),
        ("pub async unsafe fn h(", Some("h")),
        ("extern \"C\" fn i(", Some("i")),
        ("const X: u8 = 0;", None),
        ("let fnord = 1;", None),
    ] {
        assert_eq!(
            fn_decl_name(decl).as_deref(),
            want,
            "bug-log #229: fn_decl_name misreads `{decl}`"
        );
    }
    let spawn = concat!("Command", "::new(");
    let named_new = concat!("Command", "::new");
    let alias = concat!("Command", " as ");
    let path_attr = concat!("#[", "path");
    let accessor = concat!("env!(\"CARGO_BIN_EXE_", "backtest\")");
    let forms = [
        (
            "determinism.rs backtest_cmd",
            concat!("Command", "::new(backtest_bin())"),
        ),
        (
            "multi_pair_determinism.rs run_pairs_scenario_once",
            concat!("Command", "::new(env!(\"CARGO_BIN_EXE_", "backtest\"))"),
        ),
    ];
    let tests_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut wrong: Vec<String> = Vec::new();
    let mut evasions: Vec<String> = Vec::new();
    for file in ["determinism.rs", "multi_pair_determinism.rs"] {
        for (f, n, line) in lines_by_fn(&tests_dir.join(file)) {
            let t = line.trim();
            let mut why: Vec<&str> = Vec::new();
            if t.contains(alias) {
                why.push("a Command alias");
            }
            if t.match_indices(named_new)
                .any(|(at, _)| !t[at + named_new.len()..].starts_with('('))
            {
                why.push("a Command constructor named but not called");
            }
            if t.contains(path_attr) {
                why.push("a `path` attribute");
            }
            let code = t.find("//").map_or(t, |at| t[..at].trim_end());
            if code.ends_with(';') && code.split_whitespace().rev().nth(1) == Some("mod") {
                why.push("a mod declaration that pulls in another file");
            }
            for reason in why {
                evasions.push(format!("{file}:{n} fn {f}: {reason}: {t}"));
            }
            let Some(at) = line.find(spawn) else { continue };
            if line[at + spawn.len()..].starts_with('"') {
                continue; // a program named by a literal, such as cargo: not a binary path
            }
            let site = format!("{file} {f}");
            if let Some((_, form)) = forms.iter().find(|(s, _)| *s == site)
                && !line.contains(form)
            {
                wrong.push(format!("{file}:{n} fn {f}: {t}"));
            }
            seen.insert(site);
        }
    }
    assert!(
        evasions.is_empty(),
        "bug-log #229: a spelling the spawn scan cannot follow; spawn through backtest_cmd():\n{}",
        evasions.join("\n")
    );
    assert!(
        wrong.is_empty(),
        "bug-log #229: these spawns must take cargo's own build in the accessor form:\n{}",
        wrong.join("\n")
    );
    let declared: std::collections::BTreeSet<String> = BINARY_SPAWN_SITES
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    assert_eq!(
        seen, declared,
        "bug-log #229: a binary spawn changed in the determinism targets: take backtest_cmd() (multi_pair: env! of CARGO_BIN_EXE_backtest), or declare a builder's here and in BUILD_THEN_RUN_FNS"
    );
    assert!(
        lines_by_fn(&tests_dir.join("determinism.rs"))
            .iter()
            .any(|(f, _, l)| f == "backtest_bin" && l.contains(accessor)),
        "bug-log #229: backtest_bin() no longer reads CARGO_BIN_EXE_backtest"
    );
    eprintln!("t229 spawns: {BINARY_SPAWN_SITES:?}");
}

/// bug-log #229: R-REAL's declared binary is the no-feature `backtest`. Under a no-feature
/// invocation that is cargo's own build, and `run_scenario_realdata_cwd` builds it itself
/// only when this compilation has a `backtest` feature. So its `cfg!(any(…))` must name
/// EVERY feature in `crates/backtest/Cargo.toml`: a missing one would hand R-REAL a featured
/// binary in silence. (A listed name that is not a feature fails `-D warnings` through
/// `unexpected_cfgs`, so only this direction needs a test.)
#[test]
fn t229_real_rows_build_under_every_backtest_feature() {
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = std::fs::read_to_string(crate_dir.join("Cargo.toml"))
        .expect("read crates/backtest/Cargo.toml");
    let mut in_features = false;
    let mut declared: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for line in manifest.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_features = t == "[features]";
        } else if in_features
            && !t.starts_with('#')
            && let Some((key, _)) = t.split_once('=')
        {
            declared.insert(key.trim().to_string());
        }
    }
    assert!(
        !declared.is_empty(),
        "read no [features] keys from crates/backtest/Cargo.toml"
    );
    let key = "feature = \"";
    let mut listed: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (f, _, line) in lines_by_fn(&crate_dir.join("tests/determinism.rs")) {
        if f != "run_scenario_realdata_cwd" {
            continue;
        }
        let mut rest = line.as_str();
        while let Some(at) = rest.find(key) {
            rest = &rest[at + key.len()..];
            if let Some(end) = rest.find('"') {
                listed.insert(rest[..end].to_string());
            }
        }
    }
    assert_eq!(
        listed, declared,
        "bug-log #229: run_scenario_realdata_cwd's cfg!(any(...)) must list every backtest feature"
    );
    eprintln!("t229: R-REAL builds its own binary under any of {listed:?}");
}

// ── T521 — extended determinism gate for v0.5 composed-strategy scenarios ────

/// bug-log #210 — the committed baselines' FILE NAMES, as empty stubs in a sandbox.
///
/// Since #210 the bin resolves a declared baseline (`cli_paths::BASELINE_DECLARATIONS`:
/// macd, rsi and bbands declare `btc-2023-1m-sma-baseline-refresh`;
/// `btc-2024-h1-sma-cross` declares `btc-2023-1m-sma-cross`) from `./evidence` and
/// refuses the run when it finds none. These sandboxes run from a tempdir with no
/// corpus, so they plant the two names the committed reports cite, at the committed
/// container depth (`evidence/v1/v0-paper-sma/reports/`). The resolver reads names
/// only, so the stubs are empty.
///
/// The stubs reach the `baseline_report:` FRONT-MATTER line only. Every body-SHA
/// constant in this file is unchanged by them, which is itself the check that only
/// front matter moved (`report_body_hash` strips the front matter).
fn seed_baseline_stubs(sandbox: &std::path::Path) {
    let reports = sandbox.join("evidence/v1/v0-paper-sma/reports");
    std::fs::create_dir_all(&reports).expect("create sandbox baseline dir");
    for name in [
        "backtest-20260420-202621-btc-2023-1m-sma-cross.md",
        "backtest-20260420-151944-btc-2023-1m-sma-baseline-refresh.md",
    ] {
        std::fs::write(reports.join(name), b"").expect("write baseline stub");
    }
}

/// Helper that runs a scenario twice and asserts byte-identical body hashes.
fn assert_scenario_deterministic(scenario: &str) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not locate workspace root");

    let tmp = tempfile::tempdir().expect("create tempdir");
    let reports_dir = tmp.path().join("evidence/reports");
    std::fs::create_dir_all(&reports_dir).expect("create temp reports dir");

    // Also need config/strategies/ accessible for composed scenarios.
    let config_dir = tmp.path().join("config/strategies");
    std::fs::create_dir_all(&config_dir).expect("create temp config/strategies");

    // Copy the canonical TOML recipes into the temp dir.
    let src_strategies = workspace_root.join("config/strategies");
    for entry in std::fs::read_dir(&src_strategies)
        .expect("read config/strategies")
        .flatten()
    {
        let dst = config_dir.join(entry.file_name());
        std::fs::copy(entry.path(), dst).expect("copy strategy TOML");
    }
    seed_baseline_stubs(tmp.path());

    let run = || -> String {
        let output = backtest_cmd()
            .args(["--scenario", scenario, "--seed", "0xC0FFEE"])
            .current_dir(tmp.path())
            .output()
            .expect("spawn backtest binary");
        assert!(
            output.status.success(),
            "backtest binary exited non-zero for scenario {scenario}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        let report_rel = stdout
            .lines()
            .find(|l| l.starts_with("Report written: "))
            .map(|l| l.trim_start_matches("Report written: ").trim())
            .expect("could not find 'Report written:' line");
        // bug-log #210 (ADR-0104 D1): pins the writer of every scenario this helper runs.
        assert!(
            std::path::Path::new(report_rel).starts_with("backtest-runs"),
            "{scenario}: default report {report_rel} is not under backtest-runs/ (bug-log #210)"
        );
        let report_path = tmp.path().join(report_rel);
        std::fs::read_to_string(&report_path)
            .unwrap_or_else(|e| panic!("could not read report {report_path:?}: {e}"))
    };

    let report1 = run();
    let report2 = run();

    let hash1 = backtest::report_body_hash(&report1);
    let hash2 = backtest::report_body_hash(&report2);

    let hex1 = hash1.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let hex2 = hash2.iter().map(|b| format!("{b:02x}")).collect::<String>();

    assert_eq!(
        hex1, hex2,
        "T521: scenario {scenario} body-SHA256 must be identical across two runs at seed 0xC0FFEE"
    );
}

/// T521 — sma baseline refresh is deterministic.
#[test]
fn t521_sma_baseline_refresh_deterministic() {
    assert_scenario_deterministic("btc-2023-1m-sma-baseline-refresh");
}

/// T521 — btc-2023-1m-macd-trend is deterministic.
#[test]
fn t521_macd_trend_deterministic() {
    assert_scenario_deterministic("btc-2023-1m-macd-trend");
}

/// T521 — btc-2023-1m-rsi-reversion is deterministic.
#[test]
fn t521_rsi_reversion_deterministic() {
    assert_scenario_deterministic("btc-2023-1m-rsi-reversion");
}

/// T521 — btc-2023-1m-bbands-mean-revert is deterministic.
#[test]
fn t521_bbands_mean_revert_deterministic() {
    assert_scenario_deterministic("btc-2023-1m-bbands-mean-revert");
}

// ── T622 — v0 + v0.5 anchor hash regression gate ──────────────────────────────
//
// These tests run each v0/v0.5 scenario once at seed 0xC0FFEE and compare the
// body-SHA256 against the locked anchor hashes.  If any anchor hash changes,
// the v1 changes have introduced a regression in the v0/v0.5 output.
//
// Anchor hashes re-locked 2026-05-30 to `v5-realdata-medium-2026-05` namespace
// per ADR-0045 § D6 (Decision 1). SMA rows use full-hash assert_eq.
// MACD/RSI/BBands rows (3-5) are PENDING orchestrator review — see block comment
// above the t717 section for details on the data-source mapping mismatch.
//   btc-2023-1m-sma-cross         d2fa7616c5ba763784f70eb6de5072866fe66f41bcb055f62f187e80703990e0
//   btc-2023-1m-sma-baseline-refresh  (same body as sma-cross)
//   btc-2023-1m-macd-trend        PENDING (tempdir synthetic SHA ≠ real-data anchor in anchors.toml)
//   btc-2023-1m-rsi-reversion     PENDING
//   btc-2023-1m-bbands-mean-revert PENDING

fn run_scenario_once(scenario: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not locate workspace root");

    let tmp = tempfile::tempdir().expect("create tempdir");
    let reports_dir = tmp.path().join("evidence/reports");
    std::fs::create_dir_all(&reports_dir).expect("create temp reports dir");

    let config_dir = tmp.path().join("config/strategies");
    std::fs::create_dir_all(&config_dir).expect("create temp config/strategies");
    let src_strategies = workspace_root.join("config/strategies");
    for entry in std::fs::read_dir(&src_strategies)
        .expect("read config/strategies")
        .flatten()
    {
        let dst = config_dir.join(entry.file_name());
        std::fs::copy(entry.path(), dst).expect("copy strategy TOML");
    }
    seed_baseline_stubs(tmp.path());

    let output = backtest_cmd()
        .args(["--scenario", scenario, "--seed", "0xC0FFEE"])
        .current_dir(tmp.path())
        .output()
        .expect("spawn backtest binary");

    assert!(
        output.status.success(),
        "backtest binary exited non-zero for scenario {scenario}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report_rel = stdout
        .lines()
        .find(|l| l.starts_with("Report written: "))
        .map(|l| l.trim_start_matches("Report written: ").trim())
        .expect("could not find 'Report written:' line");
    // bug-log #210 (ADR-0104 D1): a default run writes under `backtest-runs/`, never under
    // `evidence/`. This pins the writer of every scenario this helper runs: the final
    // SMA/composed writer (t622/t717 SMA, macd, rsi, bbands), Momentum (t717_top10_*) and
    // TcnOverlay (tt1_*).
    assert!(
        std::path::Path::new(report_rel).starts_with("backtest-runs"),
        "{scenario}: default report {report_rel} is not under backtest-runs/ (bug-log #210)"
    );

    let report_path = tmp.path().join(report_rel);
    std::fs::read_to_string(&report_path)
        .unwrap_or_else(|e| panic!("could not read report {report_path:?}: {e}"))
}

// NOTE (2026-08-22): a `binance_corpus_present()` skip-guard was added here and
// then REMOVED in the same session. It rested on a false premise — that CI lacks
// the pinned corpus. `data/binance` is **tracked**, so a runner HAS it; the
// gitignored (and therefore CI-absent) data dirs are `audit`, `audit.db`,
// `binance-dynamic` and `reflection`. The real reason these tests fail in CI is
// code-vs-evidence drift, handled by the known-red block below. Leaving a guard
// that documents a condition which never occurs would mislead the next reader.
fn scenario_body_hex(scenario: &str) -> String {
    let report = run_scenario_once(scenario);
    let hash = backtest::report_body_hash(&report);
    hash.iter().map(|b| format!("{b:02x}")).collect::<String>()
}

/// T622 — v0 SMA anchor hash unchanged after v1 changes.
///
/// Re-locked to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
/// Stale noop-baseline SHA `fc2e3b4a…` replaced with canonical 8-bps SHA.
#[test]
fn t622_sma_cross_anchor_hash_unchanged() {
    const ANCHOR: &str = "6fdfacc3fc4c895ebe8909475a17aa910c315c68d8767942fc072ccc11cde4c3";
    let hex = scenario_body_hex("btc-2023-1m-sma-cross");
    assert_eq!(
        hex, ANCHOR,
        "T622 REGRESSION: btc-2023-1m-sma-cross body-SHA256 changed.\n\
         Expected: {ANCHOR}\n\
         Got:      {hex}"
    );
}

/// T622 — v0.5 SMA baseline-refresh anchor (same body as sma-cross).
///
/// Re-locked to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
#[test]
fn t622_sma_baseline_refresh_anchor_hash_unchanged() {
    const ANCHOR: &str = "6fdfacc3fc4c895ebe8909475a17aa910c315c68d8767942fc072ccc11cde4c3";
    let hex = scenario_body_hex("btc-2023-1m-sma-baseline-refresh");
    assert_eq!(
        hex, ANCHOR,
        "T622 REGRESSION: btc-2023-1m-sma-baseline-refresh body-SHA256 changed.\n\
         Expected: {ANCHOR}\n\
         Got:      {hex}"
    );
}

/// T622 — v0.5 MACD trend anchor hash unchanged.
///
/// Re-locked to SYNTHETIC SHA (ADR-0045 § D6.3). The determinism test runs
/// the v0 synthetic-fallback path (CWD=tempdir → parquet lookup misses).
/// The v5-realdata-medium-2026-05 anchors.toml row for macd-trend was emitted
/// from the REAL-DATA path (17544-bar Binance bodies); the synthetic SHA is
/// held in D7.1's SYNTHETIC_DETERMINISM_SHAS dict (check_determinism_anchors.py).
#[test]
fn t622_macd_trend_anchor_hash_unchanged() {
    const ANCHOR: &str = "9c07d2c3fa3dbcb7645307790d03e710e3acc90e3435e7c6b43581492bb93077";
    let hex = scenario_body_hex("btc-2023-1m-macd-trend");
    assert_eq!(
        hex, ANCHOR,
        "T622 REGRESSION: btc-2023-1m-macd-trend body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T622 — v0.5 RSI reversion anchor hash unchanged.
///
/// Re-locked to SYNTHETIC SHA (ADR-0045 § D6.3). The determinism test runs
/// the v0 synthetic-fallback path (CWD=tempdir → parquet lookup misses).
/// The v5-realdata-medium-2026-05 anchors.toml row was emitted from the
/// REAL-DATA path (17544-bar Binance bodies). Synthetic SHA held in
/// SYNTHETIC_DETERMINISM_SHAS (check_determinism_anchors.py).
#[test]
fn t622_rsi_reversion_anchor_hash_unchanged() {
    const ANCHOR: &str = "14572b60986e9f94feb4ee5db59233d877655e75884ca9d15d3df876736a81d2";
    let hex = scenario_body_hex("btc-2023-1m-rsi-reversion");
    assert_eq!(
        hex, ANCHOR,
        "T622 REGRESSION: btc-2023-1m-rsi-reversion body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T622 — v0.5 BBands mean-revert anchor hash unchanged.
///
/// Re-locked to SYNTHETIC SHA (ADR-0045 § D6.3). The determinism test runs
/// the v0 synthetic-fallback path (CWD=tempdir → parquet lookup misses).
/// The v5-realdata-medium-2026-05 anchors.toml row was emitted from the
/// REAL-DATA path (17544-bar Binance bodies). Synthetic SHA held in
/// SYNTHETIC_DETERMINISM_SHAS (check_determinism_anchors.py).
#[test]
fn t622_bbands_mean_revert_anchor_hash_unchanged() {
    const ANCHOR: &str = "fa390116c31ac9c0c5d87b6b930688bf41c45cdb0ede55eb01aef7badcc1b97c";
    let hex = scenario_body_hex("btc-2023-1m-bbands-mean-revert");
    assert_eq!(
        hex, ANCHOR,
        "T622 REGRESSION: btc-2023-1m-bbands-mean-revert body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

// ── T717 — v0 + v0.5 + v1 full-hash regression gate ──────────────────────────
//
// These tests extend T622 with the complete 64-char anchor hashes for all 7
// v0/v0.5/v1 scenarios.  The v1.5a backend changes must not affect any of
// these anchors (architecture determinism contract R9.4).
//
// Anchor hashes re-locked 2026-05-30 to canonical namespace per ADR-0045 § D6.
// Original noop-baseline SHAs are preserved in evidence/anchors.toml.
// SMA/Momentum/tt1 re-locked to v5-realdata-medium-2026-05 (synthetic == real-data SHA).
// MACD/RSI/BBands re-locked to SYNTHETIC SHAs (ADR-0045 § D6.3): the v5-realdata-medium
// rows for these 3 were emitted from the REAL-DATA path (17544-bar Binance bodies);
// the determinism tests run the v0 synthetic fallback (CWD=tempdir, 525600 bars).
// Which SOURCE each of these pins resolves against — the VALUES deliberately live in one
// place each, not here (bug-log #181):
//
//   btc-2023-1m-sma-cross             anchors.toml  `v0 + v5-realdata-medium-2026-05`
//   btc-2023-1m-sma-baseline-refresh  anchors.toml  `v0 + v5-realdata-medium-2026-05`
//   btc-2023-1m-macd-trend            SYNTHETIC_DETERMINISM_SHAS  (synthetic path)
//   btc-2023-1m-rsi-reversion         SYNTHETIC_DETERMINISM_SHAS  (synthetic path)
//   btc-2023-1m-bbands-mean-revert    SYNTHETIC_DETERMINISM_SHAS  (synthetic path)
//   top10-2023-1h-momentum            anchors.toml  `v1 + v5-realdata-medium-2026-05`
//   top10-2024-h1-momentum            anchors.toml  `v1 + v5-realdata-medium-2026-05`
//
// This block listed the SHAs themselves until 2026-11-08. Three of the seven had rotted —
// `top10-{2023,2024}-*-momentum` carried values matching no anchors.toml row, current or
// superseded, and the two sma rows carried a pre-re-emission SHA. Nothing noticed, because a
// comment gates nothing: it is a copy of a value, and copies drift (bug-log #178, #180). The
// live values are in `anchors.toml` and in `check_determinism_anchors.py`, and
// `scripts/check_determinism_anchors.py` is what proves the consts match them.
//
// NOTE: T715 (pairs backtest) introduced a data_source regression where the
// momentum scenarios were emitting "synthetic (seeded RNG, v1.5a multi-symbol)"
// instead of the v1-locked "synthetic (seeded RNG, v1 multi-symbol)".  Fixed in
// the T717 hotfix: momentum data_source restored; pairs keep the v1.5a label.

/// T717 — SMA cross anchor unchanged after v1.5a backend changes.
///
/// Re-locked to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
/// Stale noop-baseline SHA `fc2e3b4a…` replaced with canonical 8-bps SHA.
#[test]
fn t717_sma_cross_anchor_hash_unchanged() {
    const ANCHOR: &str = "6fdfacc3fc4c895ebe8909475a17aa910c315c68d8767942fc072ccc11cde4c3";
    let hex = scenario_body_hex("btc-2023-1m-sma-cross");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: btc-2023-1m-sma-cross body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T717 — SMA baseline-refresh anchor unchanged after v1.5a backend changes.
///
/// Re-locked to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
#[test]
fn t717_sma_baseline_refresh_anchor_hash_unchanged() {
    const ANCHOR: &str = "6fdfacc3fc4c895ebe8909475a17aa910c315c68d8767942fc072ccc11cde4c3";
    let hex = scenario_body_hex("btc-2023-1m-sma-baseline-refresh");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: btc-2023-1m-sma-baseline-refresh body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T717 — MACD trend full anchor hash unchanged.
///
/// Re-locked to SYNTHETIC SHA (ADR-0045 § D6.3). The determinism test runs
/// the v0 synthetic-fallback path (CWD=tempdir → parquet lookup misses).
/// The v5-realdata-medium-2026-05 anchors.toml row was emitted from the
/// REAL-DATA path (17544-bar Binance bodies). Synthetic SHA held in
/// SYNTHETIC_DETERMINISM_SHAS (check_determinism_anchors.py, D7.1).
#[test]
fn t717_macd_trend_anchor_hash_unchanged() {
    const ANCHOR: &str = "9c07d2c3fa3dbcb7645307790d03e710e3acc90e3435e7c6b43581492bb93077";
    let hex = scenario_body_hex("btc-2023-1m-macd-trend");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: btc-2023-1m-macd-trend body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T717 — RSI reversion full anchor hash unchanged.
///
/// Re-locked to SYNTHETIC SHA (ADR-0045 § D6.3). The determinism test runs
/// the v0 synthetic-fallback path (CWD=tempdir → parquet lookup misses).
/// The v5-realdata-medium-2026-05 anchors.toml row was emitted from the
/// REAL-DATA path (17544-bar Binance bodies). Synthetic SHA held in
/// SYNTHETIC_DETERMINISM_SHAS (check_determinism_anchors.py, D7.1).
#[test]
fn t717_rsi_reversion_anchor_hash_unchanged() {
    const ANCHOR: &str = "14572b60986e9f94feb4ee5db59233d877655e75884ca9d15d3df876736a81d2";
    let hex = scenario_body_hex("btc-2023-1m-rsi-reversion");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: btc-2023-1m-rsi-reversion body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T717 — BBands mean-revert full anchor hash unchanged.
///
/// Re-locked to SYNTHETIC SHA (ADR-0045 § D6.3). The determinism test runs
/// the v0 synthetic-fallback path (CWD=tempdir → parquet lookup misses).
/// The v5-realdata-medium-2026-05 anchors.toml row was emitted from the
/// REAL-DATA path (17544-bar Binance bodies). Synthetic SHA held in
/// SYNTHETIC_DETERMINISM_SHAS (check_determinism_anchors.py, D7.1).
#[test]
fn t717_bbands_mean_revert_anchor_hash_unchanged() {
    const ANCHOR: &str = "fa390116c31ac9c0c5d87b6b930688bf41c45cdb0ede55eb01aef7badcc1b97c";
    let hex = scenario_body_hex("btc-2023-1m-bbands-mean-revert");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: btc-2023-1m-bbands-mean-revert body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// KNOWN-RED PENDING THE 1-26 RE-LOCK — these four are TELLING THE TRUTH
// ═══════════════════════════════════════════════════════════════════════════
//
// `t717_*` and `tt1_*` below RE-RUN a scenario from real data and compare the
// body-SHA against a pin. They are the ONLY gate in the repo that can observe
// CODE-vs-EVIDENCE drift: `scripts/verify_anchors.sh` hashes the COMMITTED report
// bodies and never re-runs, so it reports 119/119 green while the code that
// produced those bodies has moved underneath them.
//
// They are currently RED, and they are RIGHT to be. Measured 2026-08-22:
//   expected 0f6f6eb8…  (pinned, and present in evidence/anchors.toml)
//   got      b655e5e7…  (what the current code produces)
//
// ── CAUSE FOUND 2026-09-25 (story 1-27). The paragraph that used to stand here
// was WRONG, and it is kept below so the correction is legible next to it:
//
//     "Bisect against 83378c5 shows they failed BEFORE #67/#71/#75/#76 landed, so
//      the drift predates this session's harness fixes — those fixes moved the
//      numbers further, they did not cause the divergence."
//
// Measured, one commit either side, on the default invocation this test uses:
//   b3332d35 (2026-08-15)  ->  0f6f6eb8…   GOOD, reproduces the pin exactly
//   11acd126 (2026-08-16)  ->  b655e5e7…   BAD, and it is the hash we see today
//
// `83378c59` itself was measured GOOD, so the old claim is falsified at its own
// cited commit. `11acd126` is **the #67 fix** — "the harness was booking a ~1%
// gain for buying one symbol at another symbol's price — engine guard +
// per-symbol fill routing".
//
// The reasoning error worth keeping: the old note excluded #67 because these four
// scenarios do not go through `run_path`. True, and irrelevant — #67's fix landed
// in `engine.rs` (+20) and `paper.rs` (+98), the engine BELOW every lane. Anything
// that calls `engine.step` with a multi-symbol universe is in scope, `run_path` or
// not. Single-symbol scenarios are immune by the defect's own mechanism, which is
// why the `t622_*` gates above stayed green throughout.
//
// So these four are NOT a separate drift. The pinned bodies are #67-contaminated
// evidence, and these gates are red because the engine became CORRECT. Resolution
// is a D6.b re-lock (story 1-27), not a code fix.
//
// DO NOT re-baseline these pins to the current output. That converts a truthful
// regression gate into a rubber stamp — bug-log #77's exact failure — and it would
// silently bless whatever caused the drift. The pins are re-derived by story
// **1-26** (the re-lock), which regenerates the 34 affected surfaces under a new
// namespace and records per-scenario old-vs-new numbers in an errata.
//
// RE-MEASURED 2026-08-23, after the ADR-0089 D1 sizer wiring landed, and again
// 2026-09-25 after the 1-26 re-lock: all four produce the SAME hashes every time —
// `t717_top10_2023_1h_momentum` is still `b655e5e7…` exactly.
//
// That stability was originally read as "a separate, older drift". It is not. It
// means the move had ALREADY happened, at 11acd126 on 2026-08-16, and nothing
// since has touched it. D1 (2026-08-23) genuinely changed nothing here — that part
// of the old note holds — but the CAUSE is shared with the 34 inventory surfaces:
// both are #67.
//
// What 1-26 correctly kept separate: the 2026-09-25 regeneration RUN moved the 34
// and not these four. What it got wrong, on the strength of the note above: that
// the two therefore have different causes. Same cause, different dates.
//
// RESOLVED 2026-09-26 by the story 1-27 D6.b re-lock. The pins below are the bodies the
// corrected engine produces, re-emitted in place under their existing namespaces, and the
// `#[ignore]` attributes are GONE — these are live regression gates again. The block above
// is kept because the reasoning is the record: they were red because the engine became
// correct, and the fix was to re-price the evidence, never to re-pin the gate (#77).

/// T717 — top10-2023-1h-momentum anchor hash unchanged.
///
/// Re-locked to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
/// Stale noop-baseline SHA `3b60ef07…` replaced with canonical 8-bps SHA.
#[test]
fn t717_top10_2023_momentum_anchor_hash_unchanged() {
    // PROJECTION RE-LOCK 2026-10-29 (bug-log #143's lesson, applied in the same pass this
    // time). ADR-0045 D6.1 makes evidence/anchors.toml canonical and these in-test ANCHOR
    // constants a DERIVED PROJECTION. The #111/#95/#146 re-lock moved the canonical digest;
    // re-locking the corpus without re-locking this constant is exactly what left #143's two
    // gates red for 15 days. Superseded digest kept in the line below.
    // superseded: b655e5e7f3edf1cec7c3e3c019876372b0d2840ee59028264e8ad891cabada15
    const ANCHOR: &str = "470367c48d6bc85fa56f26441b8ecbf27b1d3296a377ce292bd52f25883cf22a";
    let hex = scenario_body_hex("top10-2023-1h-momentum");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: top10-2023-1h-momentum body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T717 — top10-2024-h1-momentum anchor hash unchanged.
///
/// Re-locked to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
/// Stale noop-baseline SHA `1f33534f…` replaced with canonical 8-bps SHA.
#[test]
fn t717_top10_2024_momentum_anchor_hash_unchanged() {
    // PROJECTION RE-LOCK 2026-10-29 (bug-log #143's lesson, applied in the same pass this
    // time). ADR-0045 D6.1 makes evidence/anchors.toml canonical and these in-test ANCHOR
    // constants a DERIVED PROJECTION. The #111/#95/#146 re-lock moved the canonical digest;
    // re-locking the corpus without re-locking this constant is exactly what left #143's two
    // gates red for 15 days. Superseded digest kept in the line below.
    // superseded: 37ce69e9f0da7bc2f322d5f3cba34f41cef5c6f8286e236d1229169d51b5e052
    const ANCHOR: &str = "60179c6285692a4aa196e5dd5771a2392d625fc4cd71fa1dcfb32d3a1f08ccaa";
    let hex = scenario_body_hex("top10-2024-h1-momentum");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: top10-2024-h1-momentum body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

// ── T-T-1 — v2.5 TCN overlay anchor regression gate ──────────────────────────
//
// Re-locked 2026-05-30 to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
// Original stale noop-baseline SHAs preserved in evidence/anchors.toml.
// These anchors capture the PassthroughForecaster path (candle feature
// absent in CI). The tt1_* scenarios are bare synthetic paths (no -realdata
// suffix) and take the Linear{bps:8} fallback in the default binary.
//
//   top10-2023-fy-tcn-overlay  64f51802e91ead6f7d555e91915b25582a2562814ab2e9a0346f58540dc1f9b1
//   top10-2024-fy-tcn-overlay  908b66e146c2c2f2ad5a6e0d22a2d62b22610e6e47b435e3e5b75371f143e63d

/// T-T-1 — top10-2023-fy-tcn-overlay (2023 full-year top-10, passthrough mode) anchor hash.
///
/// Re-locked to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
/// Stale noop-baseline SHA `01d02584…` replaced with canonical 8-bps SHA.
#[test]
fn tt1_top10_2023_fy_tcn_overlay_anchor_hash_unchanged() {
    // PROJECTION RE-LOCK 2026-10-29 (bug-log #143's lesson, applied in the same pass this
    // time). ADR-0045 D6.1 makes evidence/anchors.toml canonical and these in-test ANCHOR
    // constants a DERIVED PROJECTION. The #111/#95/#146 re-lock moved the canonical digest;
    // re-locking the corpus without re-locking this constant is exactly what left #143's two
    // gates red for 15 days. Superseded digest kept in the line below.
    // superseded: 64f51802e91ead6f7d555e91915b25582a2562814ab2e9a0346f58540dc1f9b1
    const ANCHOR: &str = "8da7ad5fdac42816ecc3d28bc72260f00ea8d8bbeba6e963476ae0f0c16a6f9f";
    let hex = scenario_body_hex("top10-2023-fy-tcn-overlay");
    assert_eq!(
        hex, ANCHOR,
        "T-T-1 REGRESSION: top10-2023-fy-tcn-overlay body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T-T-1 — top10-2024-fy-tcn-overlay (2024 full-year top-10, passthrough mode) anchor hash.
///
/// Re-locked to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
/// Stale noop-baseline SHA `e24c85ac…` replaced with canonical 8-bps SHA.
#[test]
fn tt1_top10_2024_fy_tcn_overlay_anchor_hash_unchanged() {
    // PROJECTION RE-LOCK 2026-10-29 (bug-log #143's lesson, applied in the same pass this
    // time). ADR-0045 D6.1 makes evidence/anchors.toml canonical and these in-test ANCHOR
    // constants a DERIVED PROJECTION. The #111/#95/#146 re-lock moved the canonical digest;
    // re-locking the corpus without re-locking this constant is exactly what left #143's two
    // gates red for 15 days. Superseded digest kept in the line below.
    // superseded: 908b66e146c2c2f2ad5a6e0d22a2d62b22610e6e47b435e3e5b75371f143e63d
    const ANCHOR: &str = "7c605e310c2ed61b7cdfed24198edad3912074657341615bfa9e86f2414e1b0b";
    let hex = scenario_body_hex("top10-2024-fy-tcn-overlay");
    assert_eq!(
        hex, ANCHOR,
        "T-T-1 REGRESSION: top10-2024-fy-tcn-overlay body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

// ── M3 real-weights anchor hash tests (candle feature required) ───────────────
//
// These tests require `--features candle` because the -weights scenarios call
// `TcnSyncForecaster::load_bs1()` / `load_bs2()` at runtime.  They also
// require the LFS checkpoints to be present on disk.
//
// The anchor hashes were locked by developer on 2026-05-18 after two
// deterministic runs:
//   top10-2023-fy-tcn-overlay-weights  175173b654aa59bde041f290cb4db5120df22104b1c6fd43d38e2803ca5e9462
//   top10-2024-fy-tcn-overlay-weights  3c1178fb3ee4c6727de17905b8fa5144a65a906738126eaa7f7bbeafa628acab
//
// NOTE: The existing 20 anchor tests (above) must remain --features candle
// independent — this `#[cfg(feature = "candle")]` block is additive only.

/// Build the backtest binary with --features candle and run `scenario` once.
///
/// Runs from the workspace root so that relative paths
/// (`crates/forecast/checkpoints/anchors/`, `config/strategies/`) resolve
/// correctly.  The report is written to a tempdir via `--reports-dir` (bug-log
/// #113), never into `evidence/`; the bin's own default is the git-ignored
/// `backtest-runs/<feature>/reports/` since bug-log #210.
///
/// Returns the full report text so the caller can body-hash it.
#[cfg(feature = "candle")]
fn run_scenario_once_candle(scenario: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not locate workspace root");

    // Always rebuild with candle feature so the binary has real-weights support.
    let status = std::process::Command::new("cargo")
        .args(["build", "--bin", "backtest", "--features", "candle"])
        .current_dir(workspace_root)
        .status()
        .expect("cargo build failed");
    assert!(
        status.success(),
        "cargo build --bin backtest --features candle failed"
    );

    let bin_path = workspace_root.join("target/debug/backtest");

    // Run from workspace root so checkpoint + config paths resolve correctly.
    //
    // bug-log #113 — the OUTPUT used to land in `evidence/<feature>/reports/`, inside
    // the anchored corpus, and `verify_anchors.sh` resolves each anchor to the NEWEST
    // matching report there. So running this test at a commit whose output differs
    // from the pin planted a drifted body in the corpus and flipped that gate to FAIL
    // as a side effect. `--reports-dir` sends it to a tempdir instead; only the output
    // location moves, the body is unchanged.
    let reports = tempfile::tempdir().expect("create reports tempdir");
    let output = std::process::Command::new(&bin_path)
        .args(["--scenario", scenario, "--seed", "0xC0FFEE"])
        .arg("--reports-dir")
        .arg(reports.path())
        .current_dir(workspace_root)
        .output()
        .expect("spawn backtest binary");

    assert!(
        output.status.success(),
        "backtest binary exited non-zero for scenario {scenario}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report_rel = stdout
        .lines()
        .find(|l| l.starts_with("Report written: "))
        .map(|l| l.trim_start_matches("Report written: ").trim())
        .expect("could not find 'Report written:' line");

    // Report path is relative to workspace root.
    let report_path = workspace_root.join(report_rel);
    std::fs::read_to_string(&report_path)
        .unwrap_or_else(|e| panic!("could not read report {report_path:?}: {e}"))
}

#[cfg(feature = "candle")]
fn scenario_body_hex_candle(scenario: &str) -> String {
    let report = run_scenario_once_candle(scenario);
    let hash = backtest::report_body_hash(&report);
    hash.iter().map(|b| format!("{b:02x}")).collect::<String>()
}

/// M3 — top10-2023-fy-tcn-overlay-weights real-weights anchor hash.
///
/// Requires `--features candle` + LFS checkpoints present on disk.
/// Locked anchor: `175173b654aa59bde041f290cb4db5120df22104b1c6fd43d38e2803ca5e9462`
#[cfg(feature = "candle")]
#[test]
fn m3_top10_2023_fy_tcn_overlay_weights_anchor_hash_unchanged() {
    // PROJECTION RE-LOCK 2026-10-29 — an #[ignore]d gate, re-locked anyway (bug-log #148).
    // This pin is candle+LFS-gated, so it would not have gone red in a routine run: it would
    // have fired the next time someone passed --ignored, reading as fresh drift of unknown
    // cause. That is #111's own history — four determinism gates sat red for weeks and were
    // believed to be an older, separate drift. A stale pin behind an #[ignore] is the same
    // trap with a longer fuse. Superseded digest below.
    // superseded: 175173b654aa59bde041f290cb4db5120df22104b1c6fd43d38e2803ca5e9462
    const ANCHOR: &str = "2a6dc6b3cd85349228a3f523a5ddb2e481e363f41e3ce2513965127cfba34b39";
    let hex = scenario_body_hex_candle("top10-2023-fy-tcn-overlay-weights");
    assert_eq!(
        hex, ANCHOR,
        "M3 REGRESSION: top10-2023-fy-tcn-overlay-weights body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}\n\
         This means the real-weights backtest output changed. Investigate \
         TcnSyncForecaster, TcnOverlayMomentumStrategy, or the report writer."
    );
}

/// M3 — top10-2024-fy-tcn-overlay-weights real-weights anchor hash.
///
/// Requires `--features candle` + LFS checkpoints present on disk.
/// Locked anchor: `3c1178fb3ee4c6727de17905b8fa5144a65a906738126eaa7f7bbeafa628acab`
#[cfg(feature = "candle")]
#[test]
fn m3_top10_2024_fy_tcn_overlay_weights_anchor_hash_unchanged() {
    // PROJECTION RE-LOCK 2026-10-29 — an #[ignore]d gate, re-locked anyway (bug-log #148).
    // This pin is candle+LFS-gated, so it would not have gone red in a routine run: it would
    // have fired the next time someone passed --ignored, reading as fresh drift of unknown
    // cause. That is #111's own history — four determinism gates sat red for weeks and were
    // believed to be an older, separate drift. A stale pin behind an #[ignore] is the same
    // trap with a longer fuse. Superseded digest below.
    // superseded: 3c1178fb3ee4c6727de17905b8fa5144a65a906738126eaa7f7bbeafa628acab
    const ANCHOR: &str = "798e04e9fbe6bc8b0e13e4c250da2ac46ba5bc8ebbce47f1c68ccd39cd4efc05";
    let hex = scenario_body_hex_candle("top10-2024-fy-tcn-overlay-weights");
    assert_eq!(
        hex, ANCHOR,
        "M3 REGRESSION: top10-2024-fy-tcn-overlay-weights body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}\n\
         This means the real-weights backtest output changed. Investigate \
         TcnSyncForecaster, TcnOverlayMomentumStrategy, or the report writer."
    );
}

// ── T-D-13 / T-D-14 — realdata determinism gate ───────────────────────────────
//
// Run each `-realdata` scenario twice with a synthetic 10-symbol parquet fixture
// (built under `<tmpdir>/data/binance/`) and assert byte-identical body hashes.
//
// Design: we place the fixture at `<tmpdir>/data/binance/` and run the binary
// from `<tmpdir>` so the hardcoded `data/binance` relative path resolves.
// Report output goes to `<tmpdir>/evidence/v1/backtest-real-binance-data/reports/`.
// `config/strategies/` is copied from the workspace so composed-strategy TOML
// lookup does not fail.
//
// These tests require `--features realdata` and are feature-gated accordingly.

/// Build the backtest binary with `--features realdata` and return the binary
/// path. Rebuilds if necessary but caches the binary between test runs.
#[cfg(feature = "realdata")]
/// Shared mutex over the `target/debug/backtest` filesystem path. Used by
/// `ensure_realdata_binary` AND `ensure_realdata_candle_binary` to serialise
/// the cargo-build sequence — without this, the two feature-variant builds
/// race for the same output path under parallel test execution and one
/// variant's binary leaks into the other variant's test invocation.
///
/// Each call additionally copies the freshly-built binary to a UNIQUE
/// per-call path under `target/debug/` so that two tests of the same variant
/// running in parallel cannot overwrite each other's binary mid-run.
static BACKTEST_BUILD_MU: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Monotonic counter so each `ensure_*_binary` call yields a unique
/// `target/debug/backtest-realdata-<n>` (or `-candle-<n>`) path even when
/// multiple tests of the same variant run in parallel.
#[cfg(feature = "realdata")]
static BACKTEST_COPY_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(feature = "realdata")]
fn copy_to_unique(
    src: &std::path::Path,
    target_dir: &std::path::Path,
    tag: &str,
) -> std::path::PathBuf {
    let n = BACKTEST_COPY_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dst = target_dir.join(format!("backtest-{tag}-{n}"));
    std::fs::copy(src, &dst)
        .unwrap_or_else(|e| panic!("copy backtest -> backtest-{tag}-{n} failed: {e}"));
    dst
}

#[cfg(feature = "realdata")]
fn ensure_realdata_binary() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not locate workspace root");

    let _guard = BACKTEST_BUILD_MU.lock().unwrap_or_else(|p| p.into_inner());

    // Always rebuild so we pick up any changes.
    let status = std::process::Command::new("cargo")
        .args(["build", "--bin", "backtest", "--features", "realdata"])
        .current_dir(workspace_root)
        .status()
        .expect("cargo build failed");
    assert!(
        status.success(),
        "cargo build --bin backtest --features realdata failed"
    );

    // Copy to a unique per-call path BEFORE releasing the mutex so a
    // concurrent rebuild (different feature set) cannot overwrite the
    // source between our build and our copy.
    copy_to_unique(
        &workspace_root.join("target/debug/backtest"),
        &workspace_root.join("target/debug"),
        "realdata",
    )
}

/// Run a realdata scenario once from the given `run_dir` working directory.
///
/// `run_dir` should be either the workspace root (when using real `data/binance/`)
/// or a synthetic tempdir (only valid when `expected_revision_sha` is `None`).
///
/// Returns the report body text.
#[cfg(feature = "realdata")]
fn run_realdata_scenario_once(
    bin: &std::path::Path,
    run_dir: &std::path::Path,
    scenario: &str,
) -> String {
    // bug-log #113 — this used to run with NO `--reports-dir`, so the report landed
    // in `evidence/<feature>/reports/` INSIDE the anchored corpus. `verify_anchors.sh`
    // resolves each anchor to the NEWEST matching report there, so running this test
    // at a commit whose output differs from the pin planted a drifted body in the
    // corpus and flipped that gate to FAIL as a side effect — a test able to break a
    // different gate by being run.
    //
    // CWD stays the workspace root: `data/binance/`, `config/strategies/` and the
    // forecast checkpoints are all resolved relative to it. Only the OUTPUT moves.
    let reports = tempfile::tempdir().expect("create reports tempdir");
    let output = std::process::Command::new(bin)
        .args(["--scenario", scenario, "--seed", "0xC0FFEE"])
        .arg("--reports-dir")
        .arg(reports.path())
        .current_dir(run_dir)
        .output()
        .expect("spawn backtest binary");

    assert!(
        output.status.success(),
        "backtest binary exited non-zero for scenario {scenario}:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report_rel = stdout
        .lines()
        .find(|l| l.starts_with("Report written: "))
        .map(|l| l.trim_start_matches("Report written: ").trim())
        .expect("could not find 'Report written:' line");

    // `--reports-dir` makes the printed path absolute; `join` on an absolute path
    // returns it unchanged, so this covers both shapes.
    let report_path = run_dir.join(report_rel);
    std::fs::read_to_string(&report_path)
        .unwrap_or_else(|e| panic!("could not read report {report_path:?}: {e}"))
}

/// Build the `backtest` binary in **release** and return its path — for the R-REPRO
/// gates only.
///
/// Why a separate builder (bug-log #121): the `*_determinism` tests above use the
/// debug binary, which is fine for them because they are cheap. The `-realdata`
/// reproduction gates are not: `top10-2023-fy-regime-dispatcher-realdata` needs
/// **270 s in release and over an hour in debug** — un-`#[ignore]`ing it on the debug
/// binary would make `cargo test -p backtest --features realdata` unusable.
///
/// The profile is safe to change here because it was **measured, not assumed**, not to
/// affect the hashed body: `top10-{2023,2024}-fy-tcn-overlay-realdata` and their
/// `-weights` siblings each produced byte-identical SHAs from a debug and a release
/// run on 2026-09-26 (4 scenarios, 8 runs). Independent corroboration: the anchored
/// report's own `wall_clock_s` is 3.2 s and the release run reports 3.2 s, while debug
/// takes ~19 s — so the anchor itself was locked in release.
///
/// Verified on 4 of 9 scenarios. If a future R-REPRO gate disagrees between profiles,
/// the profile IS part of its condition and belongs in its doc comment — that is
/// bug-log #112's dimension list growing again, not a reason to distrust these four.
#[cfg(feature = "realdata")]
fn ensure_realdata_release_binary(with_candle: bool) -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not locate workspace root");

    let _guard = BACKTEST_BUILD_MU.lock().unwrap_or_else(|p| p.into_inner());

    let features = if with_candle {
        "candle,realdata"
    } else {
        "realdata"
    };
    let status = std::process::Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            "backtest",
            "--bin",
            "backtest",
            "--features",
            features,
        ])
        .current_dir(workspace_root)
        .status()
        .expect("cargo build --release failed");
    assert!(
        status.success(),
        "cargo build --release --bin backtest --features {features} failed"
    );

    let bin = workspace_root.join("target/release/backtest");
    assert!(
        bin.is_file(),
        "release backtest binary missing at {}",
        bin.display()
    );
    bin
}

/// Fallible sibling of `run_realdata_scenario_once`: returns the report on success,
/// or the binary's own stderr on refusal.
///
/// bug-log #114's lesson, applied: do NOT hand-roll a precondition check when the
/// thing itself will tell you. That guard built a filename shape which had never
/// existed and so reported "absent" for four months on machines where the file was
/// present. The binary already refuses with a precise message naming the missing
/// feature or checkpoint — so ask it, and repeat what it says.
#[cfg(feature = "realdata")]
fn run_realdata_scenario_try(
    bin: &std::path::Path,
    run_dir: &std::path::Path,
    scenario: &str,
    declared_flags: &[&str],
) -> Result<String, String> {
    let reports = tempfile::tempdir().expect("create reports tempdir");
    let output = std::process::Command::new(bin)
        .args(["--scenario", scenario, "--seed", "0xC0FFEE"])
        .args(declared_flags)
        .arg("--reports-dir")
        .arg(reports.path())
        .current_dir(run_dir)
        .output()
        .expect("spawn backtest binary");

    if !output.status.success() {
        // Report the exit status explicitly. A process killed by a signal exits non-zero
        // with EMPTY stdout and stderr, and "exited non-zero: stdout: , stderr: " tells the
        // reader nothing — it reads like a refusal the binary never made. Naming the status
        // separates "the binary said no" from "something killed it".
        let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let diagnosis = if out.is_empty() && err.is_empty() {
            " — the process said NOTHING on either stream, which means it was almost \
             certainly killed (signal / OOM) rather than refusing the run. Re-run before \
             treating this as drift."
        } else {
            ""
        };
        return Err(format!(
            "exit status {:?}{diagnosis}\nstdout: {out}\nstderr: {err}",
            output.status
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report_rel = stdout
        .lines()
        .find(|l| l.starts_with("Report written: "))
        .map(|l| l.trim_start_matches("Report written: ").trim())
        .ok_or_else(|| format!("no 'Report written:' line for {scenario}"))?;

    let report_path = run_dir.join(report_rel);
    std::fs::read_to_string(&report_path).map_err(|e| format!("read {report_path:?}: {e}"))
}

/// Return the workspace root (two directories up from `CARGO_MANIFEST_DIR`).
#[cfg(feature = "realdata")]
fn workspace_root_path() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not locate workspace root")
        .to_path_buf()
}

/// Return `true` when the real `data/binance/REVISION.toml` is present in the
/// workspace root.  Used to gate the `-realdata` determinism tests: after
/// T-D-17 pins the real aggregate SHA into the four scenario arms, the binary
/// validates the revision at runtime.  A synthetic tempdir fixture has a
/// *different* SHA, so the binary exits non-zero if run from a tempdir.
/// Running from the workspace root uses the real data whose SHA matches the pin.
#[cfg(feature = "realdata")]
fn real_binance_data_available() -> bool {
    let revision_toml = workspace_root_path()
        .join("data")
        .join("binance")
        .join("REVISION.toml");
    revision_toml.exists()
}

/// T-D-13 — `top10-2023-fy-tcn-overlay-realdata` body-SHA256 is identical across
/// two runs against the real `data/binance/` directory.
///
/// Pre-condition: `data/binance/REVISION.toml` must exist in the workspace root
/// (populated by T-D-16 fetch).  If absent the test skips with a clear message
/// so CI (which does not carry the 240 parquets) remains green.
///
/// After T-D-17 pins the real aggregate SHA into the scenario arm, the binary
/// validates that the SHA of the on-disk manifest matches the pin.  The test
/// therefore MUST run against real data — a synthetic tempdir fixture would
/// produce a different SHA and cause the binary to exit non-zero.
#[cfg(feature = "realdata")]
#[test]
fn realdata_2023_fy_tcn_overlay_determinism() {
    if !real_binance_data_available() {
        eprintln!(
            "T-D-13: data/binance/REVISION.toml absent — skipping realdata determinism test \
             (run `cargo run -p data --bin fetch_binance_klines -- --emit-revision-manifest ...` \
             first, then re-run this test)"
        );
        return; // soft skip — does not count as failure
    }

    let bin = ensure_realdata_binary();
    let workspace = workspace_root_path();
    let scenario = "top10-2023-fy-tcn-overlay-realdata";

    let report1 = run_realdata_scenario_once(&bin, &workspace, scenario);
    let report2 = run_realdata_scenario_once(&bin, &workspace, scenario);

    let hash1 = backtest::report_body_hash(&report1);
    let hash2 = backtest::report_body_hash(&report2);

    let hex1: String = hash1.iter().map(|b| format!("{b:02x}")).collect();
    let hex2: String = hash2.iter().map(|b| format!("{b:02x}")).collect();

    assert_eq!(
        hex1, hex2,
        "T-D-13: {scenario} body-SHA256 must be identical across two runs at seed 0xC0FFEE.\n\
         Run1: {hex1}\nRun2: {hex2}"
    );
}

/// T-D-14 — `top10-2024-fy-tcn-overlay-realdata` body-SHA256 is identical across
/// two runs against the real `data/binance/` directory.
///
/// Pre-condition: same as T-D-13 (`data/binance/REVISION.toml` must exist).
/// Skips with a clear message when data is absent.
#[cfg(feature = "realdata")]
#[test]
fn realdata_2024_fy_tcn_overlay_determinism() {
    if !real_binance_data_available() {
        eprintln!("T-D-14: data/binance/REVISION.toml absent — skipping realdata determinism test");
        return; // soft skip
    }

    let bin = ensure_realdata_binary();
    let workspace = workspace_root_path();
    let scenario = "top10-2024-fy-tcn-overlay-realdata";

    let report1 = run_realdata_scenario_once(&bin, &workspace, scenario);
    let report2 = run_realdata_scenario_once(&bin, &workspace, scenario);

    let hash1 = backtest::report_body_hash(&report1);
    let hash2 = backtest::report_body_hash(&report2);

    let hex1: String = hash1.iter().map(|b| format!("{b:02x}")).collect();
    let hex2: String = hash2.iter().map(|b| format!("{b:02x}")).collect();

    assert_eq!(
        hex1, hex2,
        "T-D-14: {scenario} body-SHA256 must be identical across two runs at seed 0xC0FFEE.\n\
         Run1: {hex1}\nRun2: {hex2}"
    );
}

// ── T-D-15 — realdata + candle (weights) determinism gate ─────────────────────
//
// These tests require both `realdata` AND `candle` features because the
// `-weights-realdata` scenarios call `TcnSyncForecaster::load_bs1()` /
// `load_bs2()` at runtime.
//
// If the TCN checkpoint files are absent (LFS not resolved), the test emits a
// clear message and exits with PASS (skipped) — no panic.  This is consistent
// with the M3 tests above which gate on file presence.

/// Build the backtest binary with both `realdata` and `candle` features.
#[cfg(all(feature = "realdata", feature = "candle"))]
fn ensure_realdata_candle_binary() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");

    {
        let _guard = BACKTEST_BUILD_MU.lock().unwrap_or_else(|p| p.into_inner());

        let status = std::process::Command::new("cargo")
            .args([
                "build",
                "--bin",
                "backtest",
                "--features",
                "realdata,candle",
            ])
            .current_dir(workspace_root)
            .status()
            .expect("cargo build failed");
        assert!(
            status.success(),
            "cargo build --bin backtest --features realdata,candle failed"
        );

        copy_to_unique(
            &workspace_root.join("target/debug/backtest"),
            &workspace_root.join("target/debug"),
            "realdata-candle",
        )
    }
}

/// Check whether the TCN checkpoint files are present (LFS resolved).
///
/// Returns `None` if absent (test should skip), `Some(path)` if present.
#[cfg(all(feature = "realdata", feature = "candle"))]
fn tcn_checkpoint_present(checkpoint_name: &str) -> bool {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");

    // bug-log #114 — this used to build `<name>.safetensors` and test `.exists()`.
    // The real convention is `<name>-<content-hash>.safetensors`, e.g.
    // `tcn-bs1-d1c3696d…​.safetensors`, so the un-hashed path NEVER existed and this
    // returned false on every machine, resolved LFS or not. Both `_weights` tests
    // guarded by it therefore skipped silently — and reported green — from the day
    // they were written (`ce4ccbdd`, 2026-05-18) until 2026-09-26.
    //
    // Match the prefix instead, and require a real file rather than an unresolved
    // LFS pointer (a pointer is ~130 bytes; the smallest real checkpoint is ~1.6 MB).
    let dir = workspace_root
        .join("crates")
        .join("forecast")
        .join("checkpoints")
        .join("anchors");

    let prefix = format!("{checkpoint_name}-");
    let found = std::fs::read_dir(&dir).ok().and_then(|entries| {
        entries.flatten().find_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let is_match = name.starts_with(&prefix) && name.ends_with(".safetensors");
            let big_enough = e.metadata().map(|m| m.len() > 4096).unwrap_or(false);
            (is_match && big_enough).then(|| e.path())
        })
    });

    match found {
        Some(path) => {
            eprintln!("checkpoint {checkpoint_name} resolved to {path:?}");
            true
        }
        None => {
            eprintln!(
                "checkpoint {checkpoint_name}-*.safetensors absent or unresolved under {dir:?} \
                 — weights arms cannot run (bug-log #114)"
            );
            false
        }
    }
}

/// T-D-15 — `top10-2023-fy-tcn-overlay-weights-realdata` determinism.
///
/// Requires `--features realdata,candle` + LFS checkpoints AND real
/// `data/binance/` parquets.  Skips cleanly if either is absent.
#[cfg(all(feature = "realdata", feature = "candle"))]
#[test]
fn realdata_2023_fy_tcn_overlay_weights_determinism() {
    if !tcn_checkpoint_present("tcn-bs1") {
        return; // skip — LFS not resolved
    }
    if !real_binance_data_available() {
        eprintln!(
            "T-D-15: data/binance/REVISION.toml absent — skipping weights realdata determinism test"
        );
        return; // soft skip
    }

    let bin = ensure_realdata_candle_binary();
    let workspace = workspace_root_path();
    let scenario = "top10-2023-fy-tcn-overlay-weights-realdata";

    let report1 = run_realdata_scenario_once(&bin, &workspace, scenario);
    let report2 = run_realdata_scenario_once(&bin, &workspace, scenario);

    let hash1 = backtest::report_body_hash(&report1);
    let hash2 = backtest::report_body_hash(&report2);

    let hex1: String = hash1.iter().map(|b| format!("{b:02x}")).collect();
    let hex2: String = hash2.iter().map(|b| format!("{b:02x}")).collect();

    assert_eq!(
        hex1, hex2,
        "T-D-15: {scenario} body-SHA256 must be identical across two runs at seed 0xC0FFEE.\n\
         Run1: {hex1}\nRun2: {hex2}"
    );
}

/// T-D-15 — `top10-2024-fy-tcn-overlay-weights-realdata` determinism.
///
/// Requires `--features realdata,candle` + LFS checkpoints AND real
/// `data/binance/` parquets.  Skips cleanly if either is absent.
#[cfg(all(feature = "realdata", feature = "candle"))]
#[test]
fn realdata_2024_fy_tcn_overlay_weights_determinism() {
    if !tcn_checkpoint_present("tcn-bs2") {
        return; // skip — LFS not resolved
    }
    if !real_binance_data_available() {
        eprintln!(
            "T-D-15: data/binance/REVISION.toml absent — skipping weights realdata determinism test"
        );
        return; // soft skip
    }

    let bin = ensure_realdata_candle_binary();
    let workspace = workspace_root_path();
    let scenario = "top10-2024-fy-tcn-overlay-weights-realdata";

    let report1 = run_realdata_scenario_once(&bin, &workspace, scenario);
    let report2 = run_realdata_scenario_once(&bin, &workspace, scenario);

    let hash1 = backtest::report_body_hash(&report1);
    let hash2 = backtest::report_body_hash(&report2);

    let hex1: String = hash1.iter().map(|b| format!("{b:02x}")).collect();
    let hex2: String = hash2.iter().map(|b| format!("{b:02x}")).collect();

    assert_eq!(
        hex1, hex2,
        "T-D-15: {scenario} body-SHA256 must be identical across two runs at seed 0xC0FFEE.\n\
         Run1: {hex1}\nRun2: {hex2}"
    );
}

// ── R-REPRO — reproduction gates for the `-realdata` anchors (bug-log #111/#113) ──
//
// These are NOT the `*_determinism` tests above and must not be confused with them.
// Those run a scenario twice and compare it with ITSELF: they prove the engine is
// deterministic, which passes perfectly well while the code has stopped reproducing
// the anchored evidence. Both properties are worth having; only this one can see
// code-vs-evidence drift (bug-log #93).
//
// The comparison target is the CURRENT canonical namespace, `v5-sqrt-impact-2026-05`
// (ADR-0045 § D6 / v0.5.0). Each of these scenarios also carries two historical rows
// in evidence/anchors.toml — `v2.6.0-realdata + noop-baseline` (pre-friction oracle)
// and `v2.6.0-realdata + v5-realdata-medium-2026-05` (pre-sqrt-impact) — which are
// frozen history and are NOT what current code should produce.
//
// These began as a MEASUREMENT (the `-realdata` family had never had a reproduction check),
// were `#[ignore]`d while their state was unknown, and are now LIVE gates: the 2026-09-26
// D6.b re-lock re-priced their target rows, so each one asserts a body the current engine
// actually produces. Invoke the corpus-gated ones explicitly:
//
//     cargo test -p backtest --test determinism --features realdata,candle \
//         -- reproduces_anchor
//
// (Until 2026-11-27 this read `-- --ignored reproduces_anchor`. No test in this file is
// `#[ignore]`d, so `--ignored` selected none; bug-log #236.)
//
// Per bug-log #113 requirement 6, a missing precondition is UNMEASURED, not a pass:
// these panic with a loud message instead of returning green, because a test you
// invoked by name and that silently did nothing is worse than no test.
//
// The rule that got them here, for whoever adds the next one: while a gate's state is
// unknown it is `#[ignore]`d and its ignore-reason carries the measured delta; once the
// evidence is re-priced so the gate is GREEN, the attribute comes off and it becomes a
// live regression gate. A RED gate is never fixed by re-pinning it to current output —
// that is bug-log #77, and it converts a caught drift into a silent one.

/// R-REPRO-1 — `top10-2023-fy-tcn-overlay-realdata` reproduces its canonical anchor.
#[cfg(feature = "realdata")]
#[test]
fn realdata_2023_fy_tcn_overlay_reproduces_anchor() {
    // anchor-ns: noop-baseline
    const ANCHOR: &str = "d9db83385912f8f0ceac5432517133e5afd1dee04ba8c25b9d85521bf34d481a";
    assert_reproduces_with_flags("top10-2023-fy-tcn-overlay-realdata", ANCHOR, &[]);
}

/// R-REPRO-2 — `top10-2024-fy-tcn-overlay-realdata` reproduces its canonical anchor.
#[cfg(feature = "realdata")]
#[test]
fn realdata_2024_fy_tcn_overlay_reproduces_anchor() {
    // anchor-ns: noop-baseline
    const ANCHOR: &str = "49f346764b7df09f9fd40a65b3bc3740fe4eeac3480bfccda17a43822145a480";
    assert_reproduces_with_flags("top10-2024-fy-tcn-overlay-realdata", ANCHOR, &[]);
}

/// R-REPRO-3 — `top10-2023-fy-tcn-overlay-weights-realdata` reproduces its anchor.
#[cfg(all(feature = "realdata", feature = "candle"))]
#[test]
fn realdata_2023_fy_tcn_overlay_weights_reproduces_anchor() {
    // anchor-ns: noop-baseline
    const ANCHOR: &str = "d580ffdae9107188b4addd24c5bf1bc99e1410378d1641f581f0a21cac56dc4f";
    assert_reproduces_with_flags("top10-2023-fy-tcn-overlay-weights-realdata", ANCHOR, &[]);
}

/// R-REPRO-4 — `top10-2024-fy-tcn-overlay-weights-realdata` reproduces its anchor.
#[cfg(all(feature = "realdata", feature = "candle"))]
#[test]
fn realdata_2024_fy_tcn_overlay_weights_reproduces_anchor() {
    // anchor-ns: noop-baseline
    const ANCHOR: &str = "8eb080e7812d48291b970c8b4ab04107b250855af7d017fcb415193d263ae0ff";
    assert_reproduces_with_flags("top10-2024-fy-tcn-overlay-weights-realdata", ANCHOR, &[]);
}

// `assert_reproduces_canonical_anchor` lived here until 2026-09-26. It used the DEBUG
// binary and the PANICKING runner, while R-REPRO-5..9 used release and the fallible one —
// the same family running two unstated conditions, which is bug-log #112's shape in my own
// code. It was also how a silent process kill surfaced as `panicked … exited non-zero`
// with empty stdout AND stderr, telling the reader nothing. All nine gates now go through
// `assert_reproduces_with_flags`: one runner, one declared condition, one UNMEASURED path.

// ── R-REPRO-5..9 — the -realdata anchors that had no gate at all (bug-log #111) ──
//
// These five are anchored under `v5-sqrt-impact-2026-05` and, until now, nothing in
// the repo re-ran them: `verify_anchors.sh` hashes their committed bodies and the
// `*_determinism` tests never covered them. Their reproduction state was genuinely
// unknown, which is why bug-log #111 could only report a LOWER bound on the #67
// blast radius.
//
// They use `assert_reproduces_or_report_unmeasured`, which — per bug-log #114 —
// does NOT hand-roll a precondition check for the forecaster checkpoints each arm
// needs. The binary refuses with a precise message when a feature or checkpoint is
// missing; the helper repeats that message as UNMEASURED. A guard I write myself is
// one more thing that can be wrong in the direction of silence, and #114 is exactly
// that mistake costing four months.
//
//     cargo test -p backtest --test determinism --features realdata,candle \
//         -- reproduces_anchor
//
// (Until 2026-11-27 this read `-- --ignored reproduces_anchor`. No test in this file is
// `#[ignore]`d, so `--ignored` selected none; bug-log #236.)

/// R-REPRO-5 — `top10-2023-fy-momentum-realdata`.
#[cfg(feature = "realdata")]
#[test]
fn realdata_2023_fy_momentum_reproduces_anchor() {
    // bug-log #123, operator ruling (a) 2026-09-26. This scenario's ONLY anchor row is
    // `v5-sqrt-impact-2026-05`, so the gate must run the invocation that produces that
    // condition — the default one is zero-sim-slippage and would be compared against a
    // condition it never ran. The flags are declared here rather than assumed, which is
    // the whole point: the row names `SquareRoot { alpha=1.0, lookback=90 }`, so the
    // gate names it too.
    // anchor-ns: v5-sqrt-impact-2026-05
    const ANCHOR: &str = "c08e93655790c7126087513567e2171ef914192efa4f7f6c1ff0a0c0221c9c8d";
    assert_reproduces_with_flags(
        "top10-2023-fy-momentum-realdata",
        ANCHOR,
        &[
            "--sim-slippage-sqrt-alpha",
            "1.0",
            "--sim-slippage-sqrt-lookback-days",
            "90",
        ],
    );
}

/// R-REPRO-6 — `top10-2023-fy-patchtst-overlay-realdata`.
#[cfg(feature = "realdata")]
#[test]
fn realdata_2023_fy_patchtst_overlay_reproduces_anchor() {
    // anchor-ns: noop-baseline
    const ANCHOR: &str = "b228c0011be608336f7d53630f6ab83782f2f740d9e6af2635c19576911f312c";
    assert_reproduces_or_report_unmeasured("top10-2023-fy-patchtst-overlay-realdata", ANCHOR);
}

/// R-REPRO-7 — `top10-2023-fy-regime-dispatcher-realdata` reproduces its
/// **`v3.0.0-regime`** anchor. GREEN — measured 2026-09-26.
///
/// bug-log #120 — this gate was first written against the
/// `v5-sqrt-impact-2026-05` row (`857f9494…`) on the assumption that "the canonical
/// namespace" is one global choice. It is not: it is **per scenario**. The default
/// invocation reproduces the `v3.0.0-regime` row exactly, and the sqrt-impact row's
/// invocation is not established. Pinning the wrong row would have reported a
/// perfectly reproducing scenario as drifted.
///
/// Not `#[ignore]`d: this is a real regression gate now.
#[cfg(feature = "realdata")]
#[test]
fn realdata_2023_fy_regime_dispatcher_reproduces_anchor() {
    // anchor-ns: v3.0.0-regime
    const ANCHOR: &str = "321500b1e6ca39e910ce1dc894e0a5ec61b71dfeb0976fb40c71087d58031d8d";
    assert_reproduces_or_report_unmeasured("top10-2023-fy-regime-dispatcher-realdata", ANCHOR);
}

/// R-REPRO-8 — `top10-2024-fy-regime-dispatcher-realdata` reproduces its
/// **`v3.0.0-regime`** anchor. GREEN — measured 2026-09-26. See R-REPRO-7 for why
/// the namespace is not the sqrt-impact one (bug-log #120).
#[cfg(feature = "realdata")]
#[test]
fn realdata_2024_fy_regime_dispatcher_reproduces_anchor() {
    // anchor-ns: v3.0.0-regime
    const ANCHOR: &str = "0c85046f61393982cc6789e4b04bc914e0353aac934859a629eaf5068bb948cf";
    assert_reproduces_or_report_unmeasured("top10-2024-fy-regime-dispatcher-realdata", ANCHOR);
}

/// R-REPRO-9 — `top10-2023-fy-vol-target-overlay-realdata`.
#[cfg(feature = "realdata")]
#[test]
fn realdata_2023_fy_vol_target_overlay_reproduces_anchor() {
    // anchor-ns: noop-baseline
    const ANCHOR: &str = "97a11ad3d97400fdaad0fadbbfb0219660e97c6d3b0b7c3f55d6ccf8d87e8812";
    assert_reproduces_or_report_unmeasured("top10-2023-fy-vol-target-overlay-realdata", ANCHOR);
}

/// Compare a `-realdata` scenario against its canonical `v5-sqrt-impact-2026-05`
/// anchor, reporting a refused run as UNMEASURED **in the binary's own words**.
///
/// # Panics
///
/// Panics on drift (the point of the gate), and on an absent precondition — which is
/// reported as UNMEASURED rather than skipped, per bug-log #113 requirement 6: a test
/// invoked by name that silently does nothing is worse than no test.
#[cfg(feature = "realdata")]
fn assert_reproduces_or_report_unmeasured(scenario: &str, anchor: &str) {
    assert_reproduces_with_flags(scenario, anchor, &[]);
}

/// As `assert_reproduces_or_report_unmeasured`, but the gate **declares the CLI flags**
/// that produce the row it asserts.
///
/// bug-log #123 requirement: each gate names the invocation that produced its target
/// row. The default invocation is zero-sim-slippage, so it reproduces a
/// `+ noop-baseline` row; a scenario whose only row is `v5-sqrt-impact-2026-05` must be
/// run WITH the sqrt flags or it is being compared against a condition it never ran.
/// Passing `&[]` is itself a declaration — "this gate asserts the default condition".
///
/// # Panics
///
/// Panics on drift, and on an absent precondition (reported as UNMEASURED, per #113 req 6).
#[cfg(feature = "realdata")]
fn assert_reproduces_with_flags(scenario: &str, anchor: &str, declared_flags: &[&str]) {
    assert!(
        real_binance_data_available(),
        "UNMEASURED, not passed: {scenario} needs data/binance/REVISION.toml and it is absent."
    );

    // Release, deliberately — see `ensure_realdata_release_binary` (bug-log #121).
    let bin = ensure_realdata_release_binary(cfg!(feature = "candle"));

    let workspace = workspace_root_path();
    let report = match run_realdata_scenario_try(&bin, &workspace, scenario, declared_flags) {
        Ok(r) => r,
        Err(why) => panic!(
            "UNMEASURED, not passed: {scenario} — the binary refused this run. Its own words:\n\
             {why}\n\
             Reported as unmeasured rather than skipped (bug-log #113 req 6), and deliberately \
             NOT pre-guarded by a hand-rolled precondition check (bug-log #114)."
        ),
    };

    let hex: String = backtest::report_body_hash(&report)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    assert_eq!(
        hex, anchor,
        "R-REPRO: {scenario} no longer reproduces its canonical \
         `+ noop-baseline` anchor — the zero-sim-slippage row, which is what the DEFAULT\n\
         invocation produces (bug-log #123).\n\
         Expected: {anchor}\nGot:      {hex}\n\
         Code-vs-evidence drift — the thing verify_anchors.sh cannot see (bug-log #93). \
         Do NOT re-pin to the produced value (bug-log #77); the resolution is a D6.b \
         re-lock (story 1-27)."
    );
}

// ── R-REAL — the REAL-DATA rows of the btc/eth family (bug-log #125) ─────────
//
// The `t622_*` / `t717_*` gates above run from a **tempdir**, so the parquet lookup misses
// and they assert the SYNTHETIC body. Their pins are therefore not `anchors.toml` rows at
// all — they live in `check_determinism_anchors.py::SYNTHETIC_DETERMINISM_SHAS`, which is
// deliberate and documented (ADR-0045 § D6.3).
//
// The consequence nobody had acted on: for `btc-2023-1m-{macd-trend,rsi-reversion,
// bbands-mean-revert}` **both** `anchors.toml` rows were uncovered — the `noop-baseline`
// row (synthetic, pre-friction) and the `v5-realdata-medium-2026-05` row (real Binance
// Vision data) — while a green test named `*_anchor_hash_unchanged` sat beside each one.
// That is the anchor-gate coverage audit's "off-anchor pin" class.
//
// These four gates cover the real-data rows. Condition, declared: the plain `backtest`
// binary (**no features**; under a no-feature invocation that is cargo's own build
// (`backtest_cmd()`, debug under CI's `cargo test`; bug-log #229), and only a featured
// invocation builds it here), CWD = the **workspace root** so `data/binance/` resolves, and
// `--reports-dir` to a tempdir so a run can never touch the corpus (#113). The bodies say
// `Data source | real (Binance Vision)`, which is asserted before the SHA (#112 req 2) —
// a silent fall back to the synthetic path must read as a condition mismatch, not as drift.

/// Serialises the R-REAL runs and, under a featured invocation only (bug-log #229), their
/// plain-binary build. The `realdata`-gated `BACKTEST_BUILD_MU` above is not visible here
/// (different cfg), and two of these tests building `target/debug/backtest` concurrently is
/// the same output-path race its comment describes.
static REAL_ROW_BUILD_MU: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run `scenario` from the WORKSPACE ROOT with the plain binary, so real parquets resolve.
fn run_scenario_realdata_cwd(scenario: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let ws = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("locate workspace root");

    let _guard = REAL_ROW_BUILD_MU.lock().unwrap_or_else(|p| p.into_inner());
    // bug-log #229: under a no-feature invocation (CI, the D7.2 command) cargo's own build
    // IS the declared no-feature binary, so this target runs no nested build and nothing
    // re-links the file the other tests are spawning. `CARGO_BIN_EXE_backtest` carries any
    // `--features`, so only a featured invocation builds the plain binary here, as before
    // (`BUILD_THEN_RUN_FNS`; the list is pinned by `t229_real_rows_build_under_every_*`).
    let mut cmd = if cfg!(any(
        feature = "candle",
        feature = "realdata",
        feature = "yahoo"
    )) {
        let status = std::process::Command::new("cargo")
            .args(["build", "--bin", "backtest"])
            .current_dir(ws)
            .status()
            .expect("cargo build failed");
        assert!(status.success(), "cargo build --bin backtest failed");
        std::process::Command::new(ws.join("target/debug/backtest"))
    } else {
        backtest_cmd()
    };

    let reports = tempfile::tempdir().expect("create reports tempdir");
    let output = cmd
        .args(["--scenario", scenario, "--seed", "0xC0FFEE"])
        .arg("--reports-dir")
        .arg(reports.path())
        .current_dir(ws)
        .output()
        .expect("spawn backtest binary");

    let out = String::from_utf8_lossy(&output.stdout).to_string();
    let err = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(
        output.status.success(),
        "backtest exited {:?} for {scenario}{}\nstdout: {}\nstderr: {}",
        output.status,
        if out.trim().is_empty() && err.trim().is_empty() {
            " — nothing on either stream, so it was almost certainly killed rather than \
             refusing the run; re-run before treating this as drift (bug-log #124)"
        } else {
            ""
        },
        out.trim(),
        err.trim(),
    );

    let rel = out
        .lines()
        .find(|l| l.starts_with("Report written: "))
        .map(|l| l.trim_start_matches("Report written: ").trim())
        .expect("'Report written:' line");
    std::fs::read_to_string(ws.join(rel)).expect("read report")
}

/// Shared body: assert the real-data CONDITION, then the anchored SHA.
fn assert_realdata_row(scenario: &str, anchor: &str) {
    let report = run_scenario_realdata_cwd(scenario);
    assert!(
        report.contains("real (Binance Vision)"),
        "R-REAL condition mismatch for {scenario}: expected the real-data path and the body \
         does not say so. The anchored row is a real-data body; a run that silently fell back \
         to the synthetic path would differ for that reason, not because the engine drifted \
         (bug-log #112)."
    );
    let hex: String = backtest::report_body_hash(&report)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        hex, anchor,
        "R-REAL: {scenario} no longer reproduces its `v5-realdata-medium-2026-05` row.\n\
         Expected: {anchor}\nGot:      {hex}\n\
         Do NOT re-pin (bug-log #77); the resolution is an ADR-0038 § D6.b re-emission."
    );
}

/// R-REAL-1 — `btc-2023-1m-macd-trend` real-data row. GREEN, measured 2026-09-27.
#[test]
fn realdata_row_btc_2023_1m_macd_trend() {
    // anchor-ns: v5-realdata-medium-2026-05
    const ANCHOR: &str = "9bc8b6598ec85886ed37987c83838d6009d447f05404bec69573af016be6392f";
    assert_realdata_row("btc-2023-1m-macd-trend", ANCHOR);
}

/// R-REAL-2 — `btc-2023-1m-rsi-reversion` real-data row. GREEN, measured 2026-09-27.
#[test]
fn realdata_row_btc_2023_1m_rsi_reversion() {
    // anchor-ns: v5-realdata-medium-2026-05
    const ANCHOR: &str = "b2de9ef01828326a44afad23de450ee4fa67639df5446e942ecc4c04de8e8fcf";
    assert_realdata_row("btc-2023-1m-rsi-reversion", ANCHOR);
}

/// R-REAL-3 — `btc-2023-1m-bbands-mean-revert` real-data row. GREEN, measured 2026-09-27.
#[test]
fn realdata_row_btc_2023_1m_bbands_mean_revert() {
    // anchor-ns: v5-realdata-medium-2026-05
    const ANCHOR: &str = "d29a84f16498b8ac2d70b8f31ef002c47255976e2ad453dc934e355adc264580";
    assert_realdata_row("btc-2023-1m-bbands-mean-revert", ANCHOR);
}

/// R-REAL-4 — `eth-2024-h1-sma-cross`. GREEN since the 2026-09-27 re-lock.
///
/// This scenario had **no** re-run gate of any kind — its single `anchors.toml` row is a real
/// Binance Vision body and nothing re-ran it. Measured 2026-09-27: `405c2816…` against the
/// pinned `bd4001e4…`.
///
/// **`#67`'s mechanism does not explain this one, and that held up.** `#67` was "buying one
/// symbol at another symbol's price", which needs a multi-symbol universe — which is why the
/// single-symbol `t622_*` family stayed green through `11acd126`. This scenario is
/// single-symbol, so the prediction said it should be immune. It was: the cause is the v5
/// Q-D1=(a) decision (`7e8a7e03`, 2026-05-29) forcing `Linear{bps:8}` on every scenario not
/// listed in `REAL_DATA_SCENARIO_IDS`, against a body generated 2026-05-28 — one day earlier.
///
/// Quantified rather than inferred: the fees imply $4.24M of notional, 8 bps on that is
/// $3,394, and the measured equity delta was $3,475 — 2.3% apart, with trades/buys/sells
/// unchanged. Re-emitted under D6.b 2026-09-27 (bug-log `#125`).
#[test]
fn realdata_row_eth_2024_h1_sma_cross() {
    // anchor-ns: lab-yahoo-realdata-v0.1.3
    const ANCHOR: &str = "7ca89a5cc29684060c5f1fe9c3447e75911cde83f896ae60998ff94f75420853";
    assert_realdata_row("eth-2024-h1-sma-cross", ANCHOR);
}
