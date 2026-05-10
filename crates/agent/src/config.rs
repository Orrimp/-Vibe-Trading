//! Agent configuration (T12).
//!
//! `Config::load()` reads from a TOML file, applies defaults, and validates
//! every value.  `mode = "live"` is rejected — v0 supports `research` and
//! `paper` only.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use trading_core::ConfigError;

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
    pub parquet_root: String,
}

impl Default for DataHistoricalConfig {
    fn default() -> Self {
        Self {
            parquet_root: "./data/binance/BTCUSDT".into(),
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
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            ledger_db_path: "./data/audit/ledger.db".into(),
            reconciliation_tolerance_usdt: 0.01,
        }
    }
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
        }
    }
}

impl Config {
    /// Load configuration from a TOML file and validate.
    ///
    /// # Errors
    ///
    /// - [`ConfigError::Parse`] if the TOML cannot be read or deserialized.
    /// - [`ConfigError::UnsupportedMode`] if `mode = "live"`.
    /// - [`ConfigError::InvalidValue`] for non-positive caps, negative budgets.
    /// - [`ConfigError::SmaWindowOrder`] if `fast_len >= slow_len`.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| ConfigError::Parse(e.to_string()))?;
        Self::from_toml_str(&content)
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

        if let Some(mode_str) = raw.get("mode").and_then(|v| v.as_str()) {
            if mode_str.eq_ignore_ascii_case("live") {
                return Err(ConfigError::UnsupportedMode(mode_str.to_string()));
            }
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
}
