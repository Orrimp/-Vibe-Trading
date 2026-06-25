//! leaderboard-timeframe-capital — Config-from-state wiring tests.
//!
//! Verifies that the two new leaderboard tuning knobs (`timeframe` +
//! `start_capital_input`) land correctly in the `BakeoffConfig` produced by
//! `bakeoff_config_from_state`. These are pure unit-level tests (no I/O) that
//! exercise the dispatch-boundary mapping without running a bake-off.
//!
//! ## Why here (not in leaderboard/runner.rs inline tests)?
//!
//! `bakeoff_config_from_state` pulls from `crate::leaderboard` state +
//! `backtest` types simultaneously. A top-level integration test in `tests/`
//! can import both crates cleanly without a circular dep risk.
//!
//! ## What is verified
//!
//! - Timeframe knob: `BakeoffTimeframe::FourHours` → `Horizon::FourHours` in
//!   the request (wiring, not a pass-through default).
//! - Capital knob: `start_capital_input = "50000"` → `initial_capital = 50_000`
//!   in the request (wiring, not the 100_000 legacy default).
//! - Default state → H1 identity + 100_000 capital (byte-identical legacy).

#[allow(clippy::unwrap_used, clippy::expect_used)]
mod bakeoff_config_state_wiring {
    use backtest::resample::Horizon;
    use rust_decimal_macros::dec;
    use ui::leaderboard::{BakeoffTimeframe, LeaderboardLookback, LeaderboardScreenState};

    /// A fixed `now_ms` for deterministic lookback → DateRange mapping (relative
    /// windows are computed against this value; the exact epoch does not matter).
    const FIXED_NOW_MS: i64 = 1_700_000_000_000;

    fn call_config(st: &LeaderboardScreenState) -> backtest::bakeoff::BakeoffConfig {
        ui::leaderboard::runner::bakeoff_config_from_state(st, FIXED_NOW_MS)
    }

    /// T_STATE_H1 — default state produces H1 horizon + 100_000 capital
    /// (byte-identical legacy behaviour).
    #[test]
    fn t_state_h1_default_is_identity_and_100k() {
        let st = LeaderboardScreenState::default();
        let cfg = call_config(&st);

        assert!(
            matches!(cfg.request.timeframe, Horizon::OneHour),
            "default timeframe must produce Horizon::OneHour (identity pass-through); \
             got {:?}",
            cfg.request.timeframe
        );
        assert_eq!(
            cfg.request.initial_capital,
            dec!(100_000),
            "default capital input must produce 100_000 USDT (legacy default)"
        );
    }

    /// T_STATE_H4 — `BakeoffTimeframe::FourHours` wires to `Horizon::FourHours`
    /// in the request.
    #[test]
    fn t_state_h4_timeframe_knob_wires_to_horizon() {
        let st = LeaderboardScreenState {
            timeframe: BakeoffTimeframe::FourHours,
            lookback: LeaderboardLookback::H1_2024,
            ..LeaderboardScreenState::default()
        };
        let cfg = call_config(&st);

        assert!(
            matches!(cfg.request.timeframe, Horizon::FourHours),
            "BakeoffTimeframe::FourHours must produce Horizon::FourHours; got {:?}",
            cfg.request.timeframe
        );
    }

    /// T_STATE_D1 — `BakeoffTimeframe::OneDay` wires to `Horizon::OneDay`.
    #[test]
    fn t_state_d1_timeframe_knob_wires_to_horizon() {
        let st = LeaderboardScreenState {
            timeframe: BakeoffTimeframe::OneDay,
            lookback: LeaderboardLookback::H1_2024,
            ..LeaderboardScreenState::default()
        };
        let cfg = call_config(&st);

        assert!(
            matches!(cfg.request.timeframe, Horizon::OneDay),
            "BakeoffTimeframe::OneDay must produce Horizon::OneDay; got {:?}",
            cfg.request.timeframe
        );
    }

    /// T_STATE_CAPITAL — `start_capital_input = "50000"` wires the parsed
    /// value to `initial_capital` in the request.
    #[test]
    fn t_state_capital_knob_wires_parsed_value() {
        let st = LeaderboardScreenState {
            start_capital_input: "50000".to_string(),
            lookback: LeaderboardLookback::H1_2024,
            ..LeaderboardScreenState::default()
        };
        let cfg = call_config(&st);

        assert_eq!(
            cfg.request.initial_capital,
            dec!(50_000),
            "start_capital_input=50000 must produce initial_capital=50_000 in the request"
        );
    }

    /// T_STATE_CAPITAL_FALLBACK — an invalid capital input falls back to the
    /// 100_000 USDT legacy default.
    #[test]
    fn t_state_capital_invalid_falls_back_to_100k() {
        let st = LeaderboardScreenState {
            start_capital_input: "not-a-number".to_string(),
            lookback: LeaderboardLookback::H1_2024,
            ..LeaderboardScreenState::default()
        };
        let cfg = call_config(&st);

        assert_eq!(
            cfg.request.initial_capital,
            dec!(100_000),
            "invalid capital input must fall back to 100_000 USDT legacy default"
        );
    }

    /// T_STATE_BOTH — timeframe + capital knobs are independently addressable:
    /// setting both produces both values in the request.
    #[test]
    fn t_state_both_knobs_independently_addressable() {
        let st = LeaderboardScreenState {
            timeframe: BakeoffTimeframe::FourHours,
            start_capital_input: "200000".to_string(),
            lookback: LeaderboardLookback::H1_2024,
            ..LeaderboardScreenState::default()
        };
        let cfg = call_config(&st);

        assert!(
            matches!(cfg.request.timeframe, Horizon::FourHours),
            "H4 timeframe must wire to Horizon::FourHours; got {:?}",
            cfg.request.timeframe
        );
        assert_eq!(
            cfg.request.initial_capital,
            dec!(200_000),
            "200000 capital must wire to 200_000 in the request"
        );
    }
}
