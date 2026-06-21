//! Agent configuration (T12).
//!
//! `Config::load()` reads from a TOML file, applies defaults, and validates
//! every value.  `mode = "live"` is rejected — v0 supports `research` and
//! `paper` only.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use trading_core::ConfigError;

// ── F5 forward-paper-trade types ──────────────────────────────────────────────
//
// `ForwardRunConfig` carries the selection from the leaderboard → paper loop.
// It is built UI-side from `core` types only (`StrategyId`, `Symbol`,
// `Money<Usdt>`) so `ui` never gains a `strategy`/`exec`/`forecast`/`llm` dep.
// `agent` resolves the `StrategyId` → concrete strategy in
// `build_registry_for` (which already depends on `strategy`).
//
// ADR-0060 § D3.

/// F5 — the selection carried from the leaderboard into the forward paper run.
///
/// Constructed UI-side from the crowned/picked [`LeaderRow`] + the F3 budget +
/// the bake-off coin. Consumed by `build_registry_for` (strategy injection) and
/// `spawn_trading_loop` (budget capital + cap). `lookback` is `None` for the
/// real-time-only MVP; the v0.2 replay-preview will populate it (OQ-1).
///
/// **`core`-types-only invariant**: every field type must be from `trading_core`
/// (or `Decimal`) so the `ui` crate — which imports this via `agent` — never
/// gains a direct `strategy`/`exec`/`forecast`/`llm` dependency.
#[derive(Debug, Clone)]
pub struct ForwardRunConfig {
    /// The selected (or crowned) strategy id, e.g. `"v0.sma"`.
    pub strategy: trading_core::StrategyId,
    /// The bake-off coin, e.g. `"BTCUSDT"` — becomes the feed symbol.
    pub symbol: trading_core::Symbol,
    /// The user's budget — €200 ≈ 200 USDT (product § D4). Starting cash and
    /// per-trade notional cap (the `with_budget_cap` F4 modifier).
    pub budget: trading_core::Money<trading_core::Usdt>,
    /// v0.2 replay-preview window (OQ-1 deferred). `None` = real-time-only MVP.
    pub lookback: Option<backtest::engine::DateRange>,
}

// ── F6 forward-plan types (ADR-0062) ─────────────────────────────────────────
//
// `ForwardPlan` is the `core`-typed plan emitted by the supervisor after a
// `ForwardCommand::Launch` and returned to the `ui` via a second mpsc.
//
// **`core`-types-only invariant** (same as `ForwardRunConfig`): every field
// must be a `trading_core` type, a primitive, or an `agent`-owned closed enum.
// NO `strategy`/`exec`/`forecast`/`llm` type crosses this boundary — the `ui`
// imports `ForwardPlan` via `agent` and must not gain those edges.
//
// ADR-0062 § D4.

/// F6 — the engine's current holding stance (closed `agent`-owned enum).
///
/// Mirrors `strategy::PlanStance` but is redefined here so `ui` never gains
/// a direct `strategy` dependency (the ADR-0059 `LeaderRow`/`RobustnessLabel`
/// mirror discipline).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanStance {
    /// No position — the engine is waiting for an entry signal.
    Flat,
    /// Holding a long position — the engine entered and has not yet exited.
    Long,
}

/// F6 — the most recent signal kind from the engine (closed `agent`-owned enum).
///
/// `None` for buy-and-hold (no re-evaluation signal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanSignal {
    Buy,
    Sell,
    Hold,
}

/// F6 — the engine's rule family (closed `agent`-owned enum).
///
/// The `ui` exhaustively matches on this to generate IF/THEN copy.
/// NO engine string crosses the seam (ADR-0059 `Recommendation`-not-`String`
/// precedent) — the engine emits structured rule data; the `ui` owns the words.
///
/// Variants cover all F6 candidate engines (SMA, MACD, RSI, BBands, BuyAndHold).
/// The closed enum widens if new rule families are added without an F6 rework.
///
/// ## Field types
///
/// All integer window lengths use `u32` (not `usize`) and RSI thresholds use
/// `u32` (integer percent) to keep the enum `Copy + Eq` without any `Decimal`
/// or `f64` (consistent with the `ui`-side `PlanRuleView` mirror).
/// The Bollinger k is encoded as tenths: `k_tenths = 20` → 2.0σ (avoids a
/// `Decimal` field while staying `Copy + Eq`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanRuleKind {
    /// SMA crossover — buys when fast SMA > slow SMA, sells on reverse.
    SmaCross {
        /// Fast SMA window length (bars).
        fast_len: u32,
        /// Slow SMA window length (bars).
        slow_len: u32,
    },
    /// MACD crossover — buys when MACD > signal line, sells on reverse.
    MacdCross {
        /// Fast EMA length.
        fast: u32,
        /// Slow EMA length.
        slow: u32,
        /// Signal EMA length.
        signal: u32,
    },
    /// RSI mean-reversion — buys when RSI falls below `lower` (oversold) AND
    /// the close is above the recent support floor.  Exits when RSI climbs back
    /// above `lower` (the entry condition clears — flip-to-false exit; there is
    /// no separate upper/overbought threshold in this strategy).
    RsiReversion {
        /// RSI window length.
        len: u32,
        /// Oversold entry threshold; also the flip-to-false exit threshold
        /// (integer percent, e.g. 30).
        lower: u32,
    },
    /// Bollinger-band reversion — buys when price below the lower band.
    BollingerReversion {
        /// Band window length.
        len: u32,
        /// Band width in standard deviations ×10 (e.g. `k_tenths = 20` → 2.0σ).
        ///
        /// Encoded as tenths to keep the enum `Copy + Eq` without a `Decimal`
        /// field (consistent with the `ui`-side `PlanRuleView::BollingerReversion`
        /// mirror).
        k_tenths: u32,
    },
    /// Buy-and-hold — buy once, hold forever, no sell trigger.
    BuyAndHold,
}

/// F6 — the `core`-typed plan emitted by the agent supervisor after a
/// `ForwardCommand::Launch` (ADR-0062 § D4).
///
/// Constructed agent-side from the same `build_registry_for(Some(&cfg))`
/// registry the F5 hot-swap runs (consistency by construction, R7).
/// Crossed to `ui` as a `core`-typed struct over a second mpsc
/// (`RunHandles.forward_plan_rx`) symmetric with the `forward_rx` command
/// channel. The `ui` mirrors this into a `ForwardPlanView` for render.
///
/// ## `core`-types-only invariant
///
/// Every field is a `trading_core` type, a primitive, or an `agent`-owned
/// closed enum.  NO `strategy`/`exec`/`forecast`/`llm` type appears here.
/// Verify: `cargo tree -p ui` must not gain those deps after adding this.
#[derive(Debug, Clone)]
pub struct ForwardPlan {
    /// The resolved forward-run strategy id (what `build_registry_for` loaded).
    pub strategy: trading_core::StrategyId,
    /// The coin (bake-off symbol, e.g. `"BTCUSDT"`).
    pub symbol: trading_core::Symbol,
    /// Current holding stance as of the latest bar (non-mutating stance read).
    pub stance: PlanStance,
    /// The most recent signal from the engine on the latest bar.
    ///
    /// `None` for buy-and-hold (no re-evaluation signal; no sell trigger).
    pub latest_signal: Option<PlanSignal>,
    /// The engine's rule family — the `ui` maps this to IF/THEN copy.
    pub rule: PlanRuleKind,
    /// The latest bar's close price — the projection base for sizing.
    pub last_close: trading_core::Price,
    /// The latest bar's close timestamp — shown for honest-staleness labelling.
    pub last_bar_ts: trading_core::Timestamp,
    /// The user's budget (€200 ≈ 200 USDT, product § D4).
    pub budget: trading_core::Money<trading_core::Usdt>,
    /// Projected next-BUY units at `last_close`, bounded by the F4 `budget_cap`.
    ///
    /// `units ≈ budget / last_close`, capped.  Labelled "at the last close;
    /// the actual fill price will be the next bar's" in the UI copy (not a
    /// promised fill — labelled as a current-price estimate per R3).
    pub projected_units: trading_core::Quantity,
    /// `true` iff the F4 `budget_cap` constrained the projected units.
    pub sizing_capped: bool,
    /// Horizon framing in days (display-only — does NOT terminate the F5 run).
    ///
    /// ADR-0062 § D6 / OQ-C: the horizon is a UI label ("rules in force for
    /// the coming N days"), NOT a self-terminate condition.  The
    /// `paper_loop_supervisor` / `spawn_trading_loop` lifecycle is unchanged.
    pub horizon_days: u16,
}

// ── T1928 (pass 6) — `LlmConfig` re-exported from the llm crate ───────────────
//
// Per Design § "How it shows up in code" item 10, the canonical
// `LlmConfig` lives at `crates/agent/src/config.rs:300` as a new
// section. To avoid a circular dep (`agent → llm → cost`, never the
// inverse), the struct itself is defined in `crates/llm/src/config.rs`
// (its fields depend on `cost::ProviderKind`, `OverrideMap`, `ModelId`,
// all owned by the llm crate). The agent re-exports it here so the
// root `Config` struct can carry `pub llm: LlmConfig`.
pub use llm::config::{LlmConfig, ProviderConfig, TierConfig};

// ── Sub-config types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceSourceConfig {
    pub ws_url: String,
    pub rest_url: String,
}

impl Default for BinanceSourceConfig {
    fn default() -> Self {
        Self {
            ws_url: "wss://stream.binance.com:9443/ws".into(),
            rest_url: "https://api.binance.com".into(),
        }
    }
}

/// Coinbase Advanced Trade WS source (v1.5b T1408).
///
/// Default: `enabled = false` so the v1.5a backwards-compat default
/// (Binance only, USDT only) keeps working unchanged (R10.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinbaseSourceConfig {
    #[serde(default = "default_false")]
    pub enabled: bool,
    pub ws_url: String,
    pub rest_url: String,
}

impl Default for CoinbaseSourceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ws_url: "wss://advanced-trade-ws.coinbase.com".into(),
            rest_url: "https://api.coinbase.com".into(),
        }
    }
}

/// Kraken WS v2 source (v1.5b T1408).
///
/// Default: `enabled = false` so the v1.5a backwards-compat default
/// (Binance only, USDT only) keeps working unchanged (R10.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KrakenSourceConfig {
    #[serde(default = "default_false")]
    pub enabled: bool,
    pub ws_url: String,
    pub rest_url: String,
}

impl Default for KrakenSourceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ws_url: "wss://ws.kraken.com/v2".into(),
            rest_url: "https://api.kraken.com".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DataSourcesConfig {
    #[serde(default)]
    pub binance: BinanceSourceConfig,
    /// Coinbase Advanced Trade WS source (v1.5b T1408).  Default off
    /// — operator opts in via `[data.sources.coinbase] enabled = true`.
    #[serde(default)]
    pub coinbase: CoinbaseSourceConfig,
    /// Kraken WS v2 source (v1.5b T1408).  Default off — operator
    /// opts in via `[data.sources.kraken] enabled = true`.
    #[serde(default)]
    pub kraken: KrakenSourceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataHistoricalConfig {
    /// Root of the Binance data tree.  The `ReplayFeed` resolves per-symbol
    /// subdirectories as `<parquet_root>/<symbol>/<year>/<month>.parquet`.
    /// Correct value: `"./data/binance"` (not `"./data/binance/BTCUSDT"`).
    pub parquet_root: String,
    /// When `true` (default) and `replay_pace_ms` is `None`, the `ReplayFeed`
    /// emits bars as fast as possible — suitable for headless backtests and
    /// the headless `trading` bin.  When `false` (and `replay_pace_ms` is
    /// `None`), bars are emitted at wallclock pace (1 real minute per 1m bar).
    ///
    /// Ignored when `replay_pace_ms` is `Some` — the pace takes precedence.
    #[serde(default = "default_true")]
    pub replay_fast: bool,
    /// Accelerated-but-streamed replay pace in milliseconds per bar.
    ///
    /// When `Some(n)`, the `ReplayFeed` sleeps `n` ms between emitting
    /// consecutive bars regardless of the bar interval and the `replay_fast`
    /// flag.  This is the correct mode for the cockpit live view: the SMA-50
    /// strategy warms up after ~50 bars (≈ 50 × n ms) and then emits fills +
    /// PnL updates over a watchable timeline, slow enough that the iced
    /// subscription layer (which subscribes after boot) catches every event.
    ///
    /// Recommended value for the cockpit: `30` (30 ms/bar → ~1.5 s warmup,
    /// ~9 min full replay of 17 520 bars).
    ///
    /// Default: `None` (falls back to `replay_fast`).  Leave `None` for
    /// headless backtests — the fast path is deterministic and anchor-stable.
    #[serde(default)]
    pub replay_pace_ms: Option<u64>,
}

impl Default for DataHistoricalConfig {
    fn default() -> Self {
        Self {
            parquet_root: "./data/binance".into(),
            replay_fast: true,
            replay_pace_ms: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConfig {
    #[serde(default)]
    pub sources: DataSourcesConfig,
    #[serde(default)]
    pub historical: DataHistoricalConfig,
    #[serde(default = "default_clock_skew_warn_ms")]
    pub clock_skew_warn_ms: i64,
    #[serde(default = "default_clock_skew_halt_ms")]
    pub clock_skew_halt_ms: i64,
}

fn default_clock_skew_warn_ms() -> i64 {
    2_000
}
fn default_clock_skew_halt_ms() -> i64 {
    10_000
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            sources: DataSourcesConfig::default(),
            historical: DataHistoricalConfig::default(),
            clock_skew_warn_ms: default_clock_skew_warn_ms(),
            clock_skew_halt_ms: default_clock_skew_halt_ms(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmaCrossoverConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_fast_len")]
    pub fast_len: usize,
    #[serde(default = "default_slow_len")]
    pub slow_len: usize,
}

fn default_true() -> bool {
    true
}
fn default_fast_len() -> usize {
    20
}
fn default_slow_len() -> usize {
    50
}

impl Default for SmaCrossoverConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            fast_len: 20,
            slow_len: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategiesConfig {
    #[serde(default)]
    pub sma_crossover: SmaCrossoverConfig,
    /// Phase D (ui-rethink-phase-d-trail T-D-N19) — TCN overlay audit gate.
    /// Opt-in via `[strategies.tcn_overlay_momentum] enabled = true` in
    /// `agent.toml`. Default `enabled = false` (conservative-off).
    #[serde(default)]
    pub tcn_overlay_momentum: TcnOverlayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizingConfig {
    pub fixed_fraction: f64,
}

impl Default for SizingConfig {
    fn default() -> Self {
        Self {
            fixed_fraction: 0.10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    pub per_symbol_exposure_cap: f64,
    pub daily_loss_stop_pct: f64,
    pub max_drawdown_stop_pct: f64,
    #[serde(default)]
    pub sizing: SizingConfig,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            per_symbol_exposure_cap: 0.40,
            daily_loss_stop_pct: -5.0,
            max_drawdown_stop_pct: -15.0,
            sizing: SizingConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestConfig {
    pub slippage_bps: u32,
    pub taker_fee_bps: u32,
    pub maker_fee_bps: u32,
    pub initial_capital_usdt: f64,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            slippage_bps: 2,
            taker_fee_bps: 4,
            maker_fee_bps: 2,
            initial_capital_usdt: 100_000.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub ledger_db_path: String,
    pub reconciliation_tolerance_usdt: f64,
    /// Capacity of the broadcast tick bus (R7.1 / Q1). `0` disables the tick
    /// bus entirely (uses `Ledger::open`); any positive value uses
    /// `Ledger::open_with_tick_bus(path, cap)`.
    /// Default: `1024` — matches `agent::bus::EventBus::fills_tx` capacity.
    #[serde(default = "default_tick_bus_capacity")]
    pub tick_bus_capacity: usize,
}

fn default_tick_bus_capacity() -> usize {
    1024
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            ledger_db_path: "./data/audit/ledger.db".into(),
            reconciliation_tolerance_usdt: 0.01,
            tick_bus_capacity: default_tick_bus_capacity(),
        }
    }
}

/// Phase D (ui-rethink-phase-d-trail T-D-N19) — TCN overlay audit gate.
///
/// Controls whether `TcnSyncForecaster` writes `forecast_events` rows
/// to the audit ledger. Default `enabled = false` (conservative-off;
/// operators opt in once they want the Trail view populated with
/// forecast-event rows). When `false` the `TcnSyncForecaster` wraps
/// forecasts in fire-and-forget mode with no ledger write.
///
/// Setting `[tcn_overlay] enabled = true` in `agent.toml` enables the
/// full Phase D trail correlation chain (Forecast → Signal → Fill).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TcnOverlayConfig {
    /// `false` (ship default) → no forecast_events rows written; Trail
    /// view's Forecast node renders empty-stage placeholder. `true` →
    /// ledger write is active and forecast rows land per infer/cache-hit.
    #[serde(default = "default_false")]
    pub enabled: bool,
}

/// chart-buy-sell-emphasis v1.9 (T2016) — strategy-signal-log writer
/// gate (Q1, R5.7, V12).
///
/// Defaults to `enabled = false`. Operators flip it to `true` in
/// `agent.toml` when they want the ghost-marker layer (R5) populated
/// in cockpit. With the gate off (the v1.9 default):
///
/// - the agent main loop NEVER calls
///   `audit::journal::post_strategy_signal`;
/// - the `strategy_signals` table (migration 009) stays empty;
/// - `audit::query::recent_signals` returns `Ok(vec![])` for every
///   `(venue, symbol, window)` tuple;
/// - the cockpit's ghost layer renders zero triangles.
///
/// **Different default than `ReflectionConfig::enable_writer = true`**
/// per architect Q1 — the audit-DB growth budget is real (≈ 8 MiB/month
/// at 4-strategy × 60-bar × 24-hour × 30-day volume), so the
/// conservative-off shipping default lets operators opt in once they
/// actually want the ghost-layer audit trail. Reflection-memory was
/// flipped to default-on on operator approval 2026-05-10 (presenter
/// deck `spec/reflection-memory/presentations/reflection-memory-
/// 2026-05-08.md`); the signal-log gate stays off in v1.9 and will be
/// re-evaluated for default-flip in a future brief once the live
/// agent-runtime tap point lands and operators are ready.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SignalLogConfig {
    /// `false` (the v1.9 ship default) → no rows written; ghost layer
    /// renders empty. `true` → live signal-emit tap is active and one
    /// row lands in `strategy_signals` per emitted `Signal`.
    // Architect Q1 — conservative-off default. Operator opts
    // in via `[signal_log] enabled = true` once they want the
    // ghost-layer audit trail.
    #[serde(default = "default_false")]
    pub enabled: bool,
}

/// Reflection-memory writer config (T1807 / Q3a / Q8).
///
/// `path` is the sibling sqlite file used by `SqliteReflectionStore`.
/// `channel_capacity` is the bounded mpsc capacity (Q8 default 1024).
/// `enable_writer = true` is the v1 default per operator approval
/// 2026-05-10 (presenter deck `spec/reflection-memory/presentations/reflection-memory-2026-05-08.md`,
/// "Approve with notes — flip enable_writer to true"). Tests that
/// need the writer off MUST set `enable_writer = false` explicitly
/// on their `ReflectionConfig`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionConfig {
    pub path: PathBuf,
    pub channel_capacity: usize,
    pub enable_writer: bool,
    /// v0.1.0 stub — observation-only broadcast consumer (R7.2 / R4.3).
    /// Default `false` → stub never runs; v2.x production write path
    /// (`ReflectionWriter` mpsc tap) is bit-identical. Flip to `true`
    /// to enable the `ReflectionAuditTickConsumer` stub.
    #[serde(default)]
    pub audit_tick_consumer_enabled: bool,
}

impl Default for ReflectionConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("./data/audit/reflection.db"),
            channel_capacity: 1024,
            // Operator-approved default 2026-05-10: cards land the
            // moment the agent restarts. Research / fixture profiles
            // that need the writer off override to `false` in their
            // local config.
            enable_writer: true,
            audit_tick_consumer_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillSwitchConfig {
    pub halt_file: String,
    pub heartbeat_timeout_ms: u64,
}

impl Default for KillSwitchConfig {
    fn default() -> Self {
        Self {
            halt_file: "./.halt".into(),
            heartbeat_timeout_ms: 5_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub prometheus_listen: String,
    /// When `false`, [`crate::observability::start_prometheus_exporter`]
    /// returns `Ok(())` without binding the listener and emits one
    /// `prometheus_listener_disabled` info line.
    ///
    /// Default: `true` (preserves pre-feature behavior). The unified
    /// `cockpit_live` binary may set this to `false` when running on a
    /// laptop where binding `:9100` is wrong (R7 / live-cockpit-unified Q4).
    #[serde(default = "default_true")]
    pub prometheus_enabled: bool,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            prometheus_listen: "0.0.0.0:9100".into(),
            prometheus_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    pub budget_usd_month: f64,
}

impl Default for CostConfig {
    fn default() -> Self {
        Self {
            budget_usd_month: 20.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusConfig {
    pub bars_capacity: usize,
    pub ticks_capacity: usize,
    pub fills_capacity: usize,
}

impl Default for BusConfig {
    fn default() -> Self {
        Self {
            bars_capacity: 1024,
            ticks_capacity: 8192,
            fills_capacity: 1024,
        }
    }
}

/// Funding-rate poller configuration (v1 T614).
///
/// Default: `enabled = false` so test infrastructure and research-mode backtests
/// never hit the Binance fapi endpoint unexpectedly.
///
/// Set `enabled = true` in `config/agent.toml` to activate the hourly poller
/// in paper mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingConfig {
    /// When `false` (default) the funding poller is not started.  Boot log emits
    /// `"funding_poller_disabled"`.  When `true`, the poller starts and emits
    /// `"funding_poller_started"` with the universe size.
    #[serde(default = "default_false")]
    pub enabled: bool,

    /// Poll interval in seconds (default: 3600 = 1 hour).
    #[serde(default = "default_funding_interval_secs")]
    pub interval_secs: u64,

    /// Universe symbols to poll.  Empty list means no symbols are polled even
    /// if `enabled = true`.
    #[serde(default)]
    pub universe: Vec<String>,
}

fn default_false() -> bool {
    false
}

fn default_funding_interval_secs() -> u64 {
    3600
}

impl Default for FundingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_secs: 3600,
            universe: Vec::new(),
        }
    }
}

/// Symbol-universe toggles (v1.5b T1410 / Q6).
///
/// Defaults preserve v1.5a behaviour: `usdt_enabled = true`,
/// `usdc_enabled = false` — operator opts in to the USDC mirror set.
///
/// The actual symbol lists are owned by [`trading_core::universe`]; this
/// struct only carries the operator-facing toggles. The loader in core
/// merges sets based on these flags:
///
/// - `usdt_enabled = true,  usdc_enabled = false` → 10 USDT symbols.
/// - `usdt_enabled = true,  usdc_enabled = true`  → 20 symbols.
/// - `usdt_enabled = false, usdc_enabled = true`  → 10 USDC symbols.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniverseConfig {
    #[serde(default = "default_true")]
    pub usdt_enabled: bool,
    #[serde(default)]
    pub usdc_enabled: bool,
}

impl Default for UniverseConfig {
    fn default() -> Self {
        Self {
            usdt_enabled: true,
            usdc_enabled: false,
        }
    }
}

// ── Root config ───────────────────────────────────────────────────────────────

/// Agent operating mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Research,
    Paper,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Research => write!(f, "research"),
            Mode::Paper => write!(f, "paper"),
        }
    }
}

/// Root configuration struct for the trading agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub mode: Mode,
    #[serde(default)]
    pub data: DataConfig,
    #[serde(default)]
    pub strategies: StrategiesConfig,
    #[serde(default)]
    pub risk: RiskConfig,
    #[serde(default)]
    pub backtest: BacktestConfig,
    #[serde(default)]
    pub audit: AuditConfig,
    #[serde(default)]
    pub kill_switch: KillSwitchConfig,
    #[serde(default)]
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub cost: CostConfig,
    #[serde(default)]
    pub bus: BusConfig,
    /// Funding-rate poller (v1 T614).  Default off — see [`FundingConfig`].
    #[serde(default)]
    pub funding: FundingConfig,
    /// Symbol-universe toggles (v1.5b T1410 / Q6).  Default: USDT only.
    #[serde(default)]
    pub universe: UniverseConfig,
    /// Reflection-memory writer config (T1807 / Q3a / Q8). Default
    /// `enable_writer = true` per operator approval 2026-05-10 — cards
    /// land on every closed trade by default. Research / fixture
    /// profiles that need the writer off override to `false`.
    #[serde(default)]
    pub reflection: ReflectionConfig,
    /// chart-buy-sell-emphasis v1.9 (T2016) — strategy-signal-log
    /// writer gate. Default `enabled = false` per architect Q1
    /// (conservative-off; operator opts in via `[signal_log]
    /// enabled = true` once they want the ghost-marker audit trail
    /// in cockpit). V12 hard-asserts the default.
    #[serde(default)]
    pub signal_log: SignalLogConfig,
    /// v2-llm-strategy v2.0.0 (T1928, pass 6) — LLM subsystem.
    /// Default `enabled = false` per architect Q1 = Option A
    /// (foundation-only, zero consumers at v2.0.0). When `enabled =
    /// true`, the agent's main wires `LlmProviderFactory::build(...)`
    /// at startup; when `false` (the default), no provider is
    /// constructed and no `.local` overlay is required.
    #[serde(default)]
    pub llm: LlmConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::Research,
            data: DataConfig::default(),
            strategies: StrategiesConfig::default(),
            risk: RiskConfig::default(),
            backtest: BacktestConfig::default(),
            audit: AuditConfig::default(),
            kill_switch: KillSwitchConfig::default(),
            observability: ObservabilityConfig::default(),
            cost: CostConfig::default(),
            bus: BusConfig::default(),
            funding: FundingConfig::default(),
            universe: UniverseConfig::default(),
            reflection: ReflectionConfig::default(),
            signal_log: SignalLogConfig::default(),
            llm: LlmConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from a TOML file and validate.
    ///
    /// **T1928 overlay (pass 6).** If a sibling `agent.toml.local`
    /// file lives next to `path`, its `[llm.providers.<name>] api_key
    /// = "..."` entries are merged into the parsed `Config.llm.providers`
    /// map. The merge touches the `[llm]` section only — other
    /// sections in the `.local` file are intentionally ignored (per
    /// Q3 = C convention: the `.local` file is LLM-keys-and-overrides
    /// exclusively). Missing `.local` under `cfg.llm.enabled = false`
    /// is a no-op; missing `.local` under `cfg.llm.enabled = true &&
    /// default_provider != "ollama"` falls through to `validate()`'s
    /// `LlmError::Auth` rejection.
    ///
    /// # Errors
    ///
    /// - [`ConfigError::Parse`] if the TOML cannot be read or deserialized.
    /// - [`ConfigError::UnsupportedMode`] if `mode = "live"`.
    /// - [`ConfigError::InvalidValue`] for non-positive caps, negative budgets.
    /// - [`ConfigError::SmaWindowOrder`] if `fast_len >= slow_len`.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path_ref = path.as_ref();
        let content =
            std::fs::read_to_string(path_ref).map_err(|e| ConfigError::Parse(e.to_string()))?;
        let mut cfg = Self::from_toml_str(&content)?;

        // T1928 overlay — merge `.local` LLM keys if the sibling file exists.
        let mut local_path = path_ref.as_os_str().to_owned();
        local_path.push(".local");
        let local_path = PathBuf::from(local_path);
        if local_path.exists()
            && let Ok(local_content) = std::fs::read_to_string(&local_path)
        {
            merge_llm_local_overlay(&mut cfg.llm, &local_content).map_err(|e| {
                ConfigError::Parse(format!("{} overlay: {}", local_path.display(), e))
            })?;
        }
        // Re-validate after the overlay merged keys.
        cfg.validate_llm_keys()?;
        Ok(cfg)
    }

    /// Parse from a TOML string and validate.
    ///
    /// # Errors
    ///
    /// Same as [`Config::load`].
    pub fn from_toml_str(toml: &str) -> Result<Self, ConfigError> {
        // We use a two-step parse to detect `mode = "live"` before the
        // Mode enum would reject it silently.
        let raw: toml::Value =
            toml::from_str(toml).map_err(|e| ConfigError::Parse(e.to_string()))?;

        if let Some(mode_str) = raw.get("mode").and_then(|v| v.as_str())
            && mode_str.eq_ignore_ascii_case("live")
        {
            return Err(ConfigError::UnsupportedMode(mode_str.to_string()));
        }

        let cfg: Config = toml::from_str(toml).map_err(|e| ConfigError::Parse(e.to_string()))?;

        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        // SMA window order
        let sma = &self.strategies.sma_crossover;
        if sma.fast_len >= sma.slow_len {
            return Err(ConfigError::SmaWindowOrder {
                fast: sma.fast_len,
                slow: sma.slow_len,
            });
        }

        // Exposure cap must be (0, 1]
        if self.risk.per_symbol_exposure_cap <= 0.0 || self.risk.per_symbol_exposure_cap > 1.0 {
            return Err(ConfigError::InvalidValue {
                field: "risk.per_symbol_exposure_cap".into(),
                reason: format!(
                    "must be in (0, 1], got {}",
                    self.risk.per_symbol_exposure_cap
                ),
            });
        }

        // Fixed fraction must be in (0, 1)
        if self.risk.sizing.fixed_fraction <= 0.0 || self.risk.sizing.fixed_fraction >= 1.0 {
            return Err(ConfigError::InvalidValue {
                field: "risk.sizing.fixed_fraction".into(),
                reason: format!("must be in (0, 1), got {}", self.risk.sizing.fixed_fraction),
            });
        }

        // Initial capital must be positive
        if self.backtest.initial_capital_usdt <= 0.0 {
            return Err(ConfigError::InvalidValue {
                field: "backtest.initial_capital_usdt".into(),
                reason: "must be positive".into(),
            });
        }

        // Cost budget must be non-negative
        if self.cost.budget_usd_month < 0.0 {
            return Err(ConfigError::InvalidValue {
                field: "cost.budget_usd_month".into(),
                reason: "must be non-negative".into(),
            });
        }

        // Reconciliation tolerance must be positive
        if self.audit.reconciliation_tolerance_usdt <= 0.0 {
            return Err(ConfigError::InvalidValue {
                field: "audit.reconciliation_tolerance_usdt".into(),
                reason: "must be positive".into(),
            });
        }

        // Clock skew: warn < halt
        if self.data.clock_skew_warn_ms >= self.data.clock_skew_halt_ms {
            return Err(ConfigError::InvalidValue {
                field: "data.clock_skew_warn_ms".into(),
                reason: format!(
                    "warn_ms ({}) must be less than halt_ms ({})",
                    self.data.clock_skew_warn_ms, self.data.clock_skew_halt_ms
                ),
            });
        }

        Ok(())
    }

    /// T1928 — re-run `LlmConfig::validate_keys` and map any
    /// `LlmError::Auth` into [`ConfigError::InvalidValue`] so the
    /// agent's startup loader uniformly rejects misconfiguration.
    /// Called from `load()` after the `.local` overlay merges keys.
    fn validate_llm_keys(&self) -> Result<(), ConfigError> {
        if let Err(e) = self.llm.validate_keys() {
            return Err(ConfigError::InvalidValue {
                field: "llm".into(),
                reason: e.to_string(),
            });
        }
        Ok(())
    }
}

/// T1928 — merge the `[llm.providers.<name>] api_key = "..."` entries
/// from an `agent.toml.local` overlay into the parsed `LlmConfig`'s
/// providers map. Other sections in the overlay are ignored.
///
/// The overlay reuses the auth-crate's shape (see
/// `crates/llm/src/auth.rs`) — we redefine the minimal local-shape
/// parse here rather than depending on a public surface in `llm::auth`
/// so the agent's config loader stays self-contained.
fn merge_llm_local_overlay(llm: &mut LlmConfig, overlay_toml: &str) -> Result<(), toml::de::Error> {
    #[derive(Deserialize, Default)]
    struct LocalRoot {
        #[serde(default)]
        llm: Option<LocalLlmSection>,
    }
    #[derive(Deserialize, Default)]
    struct LocalLlmSection {
        #[serde(default)]
        providers: std::collections::HashMap<String, LocalProviderEntry>,
    }
    #[derive(Deserialize, Default)]
    struct LocalProviderEntry {
        #[serde(default)]
        api_key: Option<String>,
    }

    let parsed: LocalRoot = toml::from_str(overlay_toml)?;
    if let Some(section) = parsed.llm {
        for (name, entry) in section.providers {
            if let Some(api_key) = entry.api_key {
                let pc = llm.providers.entry(name).or_default();
                pc.api_key = Some(api_key);
            }
        }
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_TOML: &str = r#"
mode = "research"

[strategies.sma_crossover]
fast_len = 20
slow_len = 50
"#;

    #[test]
    fn t12_load_minimal_toml_applies_defaults() {
        let cfg = Config::from_toml_str(MINIMAL_TOML).expect("parse");
        assert_eq!(cfg.mode, Mode::Research);
        assert_eq!(cfg.data.clock_skew_warn_ms, 2_000);
        assert_eq!(cfg.data.clock_skew_halt_ms, 10_000);
        assert_eq!(cfg.risk.per_symbol_exposure_cap, 0.40);
        assert_eq!(cfg.backtest.initial_capital_usdt, 100_000.0);
        assert_eq!(cfg.audit.ledger_db_path, "./data/audit/ledger.db");
        assert_eq!(cfg.bus.bars_capacity, 1024);
    }

    /// Verify the `DataHistoricalConfig` default `parquet_root` is the
    /// *root* of the Binance data tree (`./data/binance`), NOT the
    /// symbol-level subdirectory.  `ReplayFeed` joins `<symbol>` onto
    /// this root, so `./data/binance/BTCUSDT/…` requires root =
    /// `./data/binance`.  Regression guard against re-introducing the
    /// `./data/binance/BTCUSDT` mis-default.
    #[test]
    fn historical_config_default_parquet_root_is_binance_root() {
        let cfg = Config::from_toml_str(MINIMAL_TOML).expect("parse");
        assert_eq!(
            cfg.data.historical.parquet_root, "./data/binance",
            "parquet_root default must be the Binance root (not a symbol subdirectory); \
             ReplayFeed appends the symbol name itself"
        );
    }

    /// Verify `replay_fast` defaults to `true` (fast replay for cockpit
    /// demo) and round-trips correctly for `false`.
    #[test]
    fn historical_config_replay_fast_defaults_true() {
        let cfg = Config::from_toml_str(MINIMAL_TOML).expect("parse");
        assert!(
            cfg.data.historical.replay_fast,
            "replay_fast must default to true so the cockpit demo reaches \
             sma_crossover warmup in seconds, not 50+ real minutes"
        );
    }

    #[test]
    fn historical_config_replay_fast_explicit_false_round_trips() {
        let toml = r#"
mode = "research"

[strategies.sma_crossover]
fast_len = 20
slow_len = 50

[data.historical]
parquet_root = "./data/binance"
replay_fast  = false
"#;
        let cfg = Config::from_toml_str(toml).expect("parse explicit replay_fast=false");
        assert!(
            !cfg.data.historical.replay_fast,
            "explicit replay_fast = false must round-trip through serde"
        );
    }

    /// Verify `replay_pace_ms` defaults to `None` (backwards compat — existing
    /// configs without the key must not change replay behaviour).
    #[test]
    fn historical_config_replay_pace_ms_defaults_none() {
        let cfg = Config::from_toml_str(MINIMAL_TOML).expect("parse");
        assert!(
            cfg.data.historical.replay_pace_ms.is_none(),
            "replay_pace_ms must default to None so existing configs that \
             don't set it are unaffected (fast-replay preserved for headless bins)"
        );
    }

    /// Verify `replay_pace_ms = 30` round-trips correctly — the cockpit
    /// sets this in config/agent.toml to get the paced UI feed.
    #[test]
    fn historical_config_replay_pace_ms_round_trips() {
        let toml = r#"
mode = "research"

[strategies.sma_crossover]
fast_len = 20
slow_len = 50

[data.historical]
parquet_root = "./data/binance"
replay_pace_ms = 30
"#;
        let cfg = Config::from_toml_str(toml).expect("parse");
        assert_eq!(
            cfg.data.historical.replay_pace_ms,
            Some(30),
            "replay_pace_ms = 30 must round-trip through serde"
        );
    }

    #[test]
    fn t12_mode_live_is_rejected() {
        let toml = r#"
mode = "live"
[strategies.sma_crossover]
fast_len = 20
slow_len = 50
"#;
        let err = Config::from_toml_str(toml).expect_err("should reject live");
        assert!(
            matches!(err, ConfigError::UnsupportedMode(ref m) if m == "live"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn t12_paper_mode_accepted() {
        let toml = r#"
mode = "paper"
[strategies.sma_crossover]
fast_len = 20
slow_len = 50
"#;
        let cfg = Config::from_toml_str(toml).expect("paper mode ok");
        assert_eq!(cfg.mode, Mode::Paper);
    }

    #[test]
    fn t12_sma_window_order_rejected() {
        let toml = r#"
mode = "research"
[strategies.sma_crossover]
fast_len = 50
slow_len = 20
"#;
        let err = Config::from_toml_str(toml).expect_err("bad sma");
        assert!(
            matches!(err, ConfigError::SmaWindowOrder { fast: 50, slow: 20 }),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn t12_equal_sma_windows_rejected() {
        let toml = r#"
mode = "research"
[strategies.sma_crossover]
fast_len = 20
slow_len = 20
"#;
        let err = Config::from_toml_str(toml).expect_err("equal windows bad");
        assert!(matches!(err, ConfigError::SmaWindowOrder { .. }), "{err}");
    }

    #[test]
    fn t12_invalid_exposure_cap_rejected() {
        let toml = r#"
mode = "research"
[strategies.sma_crossover]
fast_len = 20
slow_len = 50
[risk]
per_symbol_exposure_cap = 1.5
daily_loss_stop_pct     = -5.0
max_drawdown_stop_pct   = -15.0
"#;
        let err = Config::from_toml_str(toml).expect_err("bad cap");
        assert!(
            matches!(err, ConfigError::InvalidValue { ref field, .. } if field.contains("exposure_cap")),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn t901_prometheus_enabled_defaults_true_when_omitted() {
        // Pre-feature config files (no `prometheus_enabled` key in
        // `[observability]`) must load successfully with the field
        // defaulting to `true`.  V10 negative case.
        let toml = r#"
mode = "research"

[strategies.sma_crossover]
fast_len = 20
slow_len = 50

[observability]
prometheus_listen = "0.0.0.0:9100"
"#;
        let cfg = Config::from_toml_str(toml).expect("parse pre-feature config");
        assert!(
            cfg.observability.prometheus_enabled,
            "prometheus_enabled must default to true for backward compat"
        );
        assert_eq!(cfg.observability.prometheus_listen, "0.0.0.0:9100");
    }

    #[test]
    fn t901_prometheus_enabled_explicit_false_round_trips() {
        let toml = r#"
mode = "research"

[strategies.sma_crossover]
fast_len = 20
slow_len = 50

[observability]
prometheus_listen   = "0.0.0.0:9100"
prometheus_enabled  = false
"#;
        let cfg = Config::from_toml_str(toml).expect("parse explicit-false config");
        assert!(!cfg.observability.prometheus_enabled);
    }

    #[test]
    fn t1410_universe_defaults_when_section_omitted() {
        // No [universe] section → defaults: usdt on, usdc off.
        let cfg = Config::from_toml_str(MINIMAL_TOML).expect("parse minimal");
        assert!(
            cfg.universe.usdt_enabled,
            "usdt_enabled must default to true"
        );
        assert!(
            !cfg.universe.usdc_enabled,
            "usdc_enabled must default to false"
        );
    }

    #[test]
    fn t1410_universe_explicit_both_enabled_round_trips() {
        let toml = r#"
mode = "research"

[strategies.sma_crossover]
fast_len = 20
slow_len = 50

[universe]
usdt_enabled = true
usdc_enabled = true
"#;
        let cfg = Config::from_toml_str(toml).expect("parse explicit universe");
        assert!(cfg.universe.usdt_enabled);
        assert!(cfg.universe.usdc_enabled);
    }

    #[test]
    fn t1408_coinbase_kraken_default_disabled() {
        // v1.5a-style minimal config (no [data.sources.coinbase]
        // / [data.sources.kraken] sections) → both default to disabled
        // (R10 backwards compat).
        let cfg = Config::from_toml_str(MINIMAL_TOML).expect("parse minimal");
        assert!(
            !cfg.data.sources.coinbase.enabled,
            "coinbase must default to disabled (R10 backwards compat)"
        );
        assert!(
            !cfg.data.sources.kraken.enabled,
            "kraken must default to disabled (R10 backwards compat)"
        );
        // Default Binance config must continue to work unchanged.
        assert_eq!(
            cfg.data.sources.binance.ws_url,
            "wss://stream.binance.com:9443/ws"
        );
    }

    #[test]
    fn t1408_three_venues_explicit_enable_round_trips() {
        let toml = r#"
mode = "paper"

[strategies.sma_crossover]
fast_len = 20
slow_len = 50

[data.sources.coinbase]
enabled  = true
ws_url   = "wss://advanced-trade-ws.coinbase.com"
rest_url = "https://api.coinbase.com"

[data.sources.kraken]
enabled  = true
ws_url   = "wss://ws.kraken.com/v2"
rest_url = "https://api.kraken.com"
"#;
        let cfg = Config::from_toml_str(toml).expect("parse three-venue config");
        assert!(cfg.data.sources.coinbase.enabled);
        assert!(cfg.data.sources.kraken.enabled);
        assert_eq!(
            cfg.data.sources.coinbase.ws_url,
            "wss://advanced-trade-ws.coinbase.com"
        );
        assert_eq!(cfg.data.sources.kraken.ws_url, "wss://ws.kraken.com/v2");
    }

    #[test]
    fn t1410_universe_usdc_only_parses() {
        let toml = r#"
mode = "research"

[strategies.sma_crossover]
fast_len = 20
slow_len = 50

[universe]
usdt_enabled = false
usdc_enabled = true
"#;
        let cfg = Config::from_toml_str(toml).expect("parse usdc-only universe");
        assert!(!cfg.universe.usdt_enabled);
        assert!(cfg.universe.usdc_enabled);
    }

    /// T2016 / V12 — `[signal_log]` section omitted from `agent.toml`
    /// must default `enabled = false` (architect Q1 conservative-off
    /// resolution). Hard-asserts the v1.9 shipping default.
    #[test]
    fn config_signal_log_default_off() {
        let cfg = Config::from_toml_str(MINIMAL_TOML).expect("parse minimal");
        assert!(
            !cfg.signal_log.enabled,
            "signal_log.enabled must default to false (architect Q1 conservative-off); \
             flipping the default requires a follow-up brief + operator approval"
        );
    }

    /// T2016 / V12 — explicit `[signal_log] enabled = true` round-trips.
    /// Operator opt-in path; defends against the field being dropped
    /// by serde (a regression vector if the field were ever renamed).
    #[test]
    fn config_signal_log_explicit_enable_round_trips() {
        let toml = r#"
mode = "research"

[strategies.sma_crossover]
fast_len = 20
slow_len = 50

[signal_log]
enabled = true
"#;
        let cfg = Config::from_toml_str(toml).expect("parse signal-log opt-in");
        assert!(
            cfg.signal_log.enabled,
            "explicit `enabled = true` must round-trip through serde"
        );
    }

    #[test]
    fn t12_load_from_file() {
        // Load the canonical config/agent.toml from the workspace root.
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let path = std::path::PathBuf::from(&manifest)
            .join("..") // agent/
            .join("..") // crates/
            .join("..") // trading/
            .join("config")
            .join("agent.toml");
        if path.exists() {
            let cfg = Config::load(&path).expect("load config/agent.toml");
            assert_eq!(cfg.mode, Mode::Research);
        }
        // Skip silently if file not present (CI without workspace root)
    }

    // ── T1928 (pass 6) — LlmConfig wire-up tests ──────────────────────────────

    /// T1928 (a) — committed-shape `[llm]` block parses with defaults
    /// preserved.
    #[test]
    fn t1928_a_committed_agent_toml_with_llm_block_parses() {
        let toml = r#"
mode = "research"

[strategies.sma_crossover]
fast_len = 20
slow_len = 50

[llm]
enabled              = false
default_provider     = "anthropic"
budget_usd_month     = 200.0
replay_cache_path    = "./data/llm-replay.db"

[llm.deep_think]
provider = "anthropic"
model    = "claude-opus-4-7"

[llm.quick_think]
provider = "anthropic"
model    = "claude-haiku-4-5-20251001"

[llm.providers.anthropic]
base_url = "https://api.anthropic.com/v1"
"#;
        let cfg = Config::from_toml_str(toml).expect("parse committed shape");
        assert!(!cfg.llm.enabled, "default disabled");
        assert_eq!(cfg.llm.default_provider, "anthropic");
        assert_eq!(cfg.llm.deep_think.model.as_str(), "claude-opus-4-7");
    }

    /// T1928 (b) — overlay populates `api_key`. Drives the
    /// `Config::load` overlay-merge path with a tempdir-hosted
    /// `agent.toml` + `agent.toml.local`.
    #[test]
    fn t1928_b_overlay_populates_api_key() {
        let td = tempfile::tempdir().expect("tempdir");
        let agent_toml = td.path().join("agent.toml");
        let overlay = td.path().join("agent.toml.local");

        std::fs::write(
            &agent_toml,
            r#"
mode = "research"

[strategies.sma_crossover]
fast_len = 20
slow_len = 50

[llm]
enabled              = false
default_provider     = "anthropic"

[llm.deep_think]
provider = "anthropic"
model    = "claude-opus-4-7"

[llm.quick_think]
provider = "anthropic"
model    = "claude-haiku-4-5-20251001"

[llm.providers.anthropic]
base_url = "https://api.anthropic.com/v1"
"#,
        )
        .expect("write agent.toml");
        std::fs::write(
            &overlay,
            r#"
[llm.providers.anthropic]
api_key = "sk-ant-test-stub-12345"
"#,
        )
        .expect("write overlay");

        let cfg = Config::load(&agent_toml).expect("load overlay");
        assert_eq!(
            cfg.llm.providers["anthropic"].api_key.as_deref(),
            Some("sk-ant-test-stub-12345")
        );
    }

    /// T1928 (c) — `cfg.llm.enabled = true && mode = paper` with no
    /// `.local` overlay rejects at startup.
    #[test]
    fn t1928_c_enabled_without_local_overlay_rejects() {
        let td = tempfile::tempdir().expect("tempdir");
        let agent_toml = td.path().join("agent.toml");
        std::fs::write(
            &agent_toml,
            r#"
mode = "paper"

[strategies.sma_crossover]
fast_len = 20
slow_len = 50

[llm]
enabled              = true
default_provider     = "anthropic"

[llm.deep_think]
provider = "anthropic"
model    = "claude-opus-4-7"

[llm.quick_think]
provider = "anthropic"
model    = "claude-haiku-4-5-20251001"

[llm.providers.anthropic]
base_url = "https://api.anthropic.com/v1"
"#,
        )
        .expect("write agent.toml");
        // NO overlay written.
        let err = Config::load(&agent_toml).expect_err("must reject");
        let msg = err.to_string();
        assert!(
            msg.contains("anthropic"),
            "rejection must name the missing provider: {msg}"
        );
    }

    /// T1928 (d) — `cfg.llm.enabled = false` (default) boots without
    /// any `.local` requirement.
    #[test]
    fn t1928_d_default_disabled_boots_no_overlay() {
        let cfg = Config::from_toml_str(MINIMAL_TOML).expect("parse minimal");
        assert!(!cfg.llm.enabled, "default disabled");
        // validate_llm_keys would be called inside load(); replicate here.
        cfg.llm.validate_keys().expect("no-op when disabled");
    }

    // ── T-D-N19 — TcnOverlayConfig round-trip tests ───────────────────────────

    /// T-D-N19 (a) — `[strategies.tcn_overlay_momentum]` section omitted from
    /// `agent.toml` must default `enabled = false` (conservative-off per
    /// architect Q1). Hard-asserts the Phase D shipping default.
    #[test]
    fn config_tcn_overlay_default_off() {
        let cfg = Config::from_toml_str(MINIMAL_TOML).expect("parse minimal");
        assert!(
            !cfg.strategies.tcn_overlay_momentum.enabled,
            "tcn_overlay_momentum.enabled must default to false (conservative-off); \
             flipping the default requires a follow-up brief + operator approval"
        );
    }

    /// T-D-N19 (b) — explicit `[strategies.tcn_overlay_momentum] enabled = true`
    /// round-trips through serde. Defends against the field being dropped
    /// by serde (a regression vector if the field were ever renamed).
    #[test]
    fn config_tcn_overlay_explicit_enable_round_trips() {
        let toml = r#"
mode = "research"

[strategies.sma_crossover]
fast_len = 20
slow_len = 50

[strategies.tcn_overlay_momentum]
enabled = true
"#;
        let cfg = Config::from_toml_str(toml).expect("parse tcn-overlay opt-in");
        assert!(
            cfg.strategies.tcn_overlay_momentum.enabled,
            "explicit `enabled = true` must round-trip through serde"
        );
    }
}
