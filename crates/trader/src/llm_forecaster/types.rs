//! Core value types for the LLM forecaster.
//!
//! ## Type hierarchy
//!
//! ```text
//! ForecastContext     — input assembled by LlmForecasterStrategy::on_bar
//!   └── request_hash()  → [u8; 32] replay-cache key
//! LlmForecast         — output from LlmForecaster::forecast()
//!   └── Rating::to_signal_kind() → trading_core::SignalKind
//! LlmForecasterError  — error enum (thiserror)
//! ```
//!
//! ## Determinism
//!
//! `ForecastContext::request_hash()` produces a deterministic SHA-256
//! over a `CanonicalContext` struct serialised by `serde_json`. Field
//! declaration order in `CanonicalContext` is alphabetical; the serde
//! serialisation is therefore deterministic across serde-json versions.
//! See `decomp.md § T-AR-2` for the complete rationale.
//!
//! ## Money / decimal discipline
//!
//! `Confidence` is `rust_decimal::Decimal` per decomp.md determinism
//! guardrails (no `f64` in money/price/qty calculations). All cost
//! fields use `Decimal` and will be wrapped in `Money<Usdt>` at the
//! Wave E audit-emission step.
//!
//! ## Cross-references
//!
//! - `spec/v1/v3-llm-forecaster/decomp.md § T-AR-1` — signal-pipeline shape.
//! - `spec/v1/v3-llm-forecaster/decomp.md § T-AR-2` — prompt + cache key.
//! - `spec/architecture/adr/0039-llm-forecaster-verdict-criteria.md` — L0-L4.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use trading_core::{Bar, SignalKind, StrategyId, Symbol, Timestamp};

use reflection::{
    REPORT_TIME_TOP_K, ReflectionStore, SymbolOrPair, classify_regime, retrieve_top_k,
};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Cache schema version — bump when the `CanonicalContext` struct shape
/// changes to invalidate all existing `(request_hash, response)` rows.
/// v0.1.0 ships at `1`. Bumping requires a re-record run per T-AR-5 Layer 5.
pub const CACHE_SCHEMA_VERSION: u32 = 1;

/// Prompt template version — bump when the project/role/dynamic block
/// layout changes. v0.1.0 ships at `1`. Independent from `CACHE_SCHEMA_VERSION`.
pub const PROMPT_TEMPLATE_VERSION: u32 = 1;

/// Default model ID for v0.1.0 (Haiku; spike-confirmed cheapest viable tier).
/// Per decomp.md § T-AR-2 + spike delta-list D3 (corrected from claude-3-5-haiku).
pub const DEFAULT_MODEL_ID: &str = "claude-haiku-4-5-20251001";

/// How many lesson cards to retrieve (mirrors `reflection::REPORT_TIME_TOP_K`).
pub const TOP_K_LESSONS: usize = 5;

/// Default per-call wall-clock timeout in milliseconds (45s per decomp.md Q5b).
pub const DEFAULT_TIMEOUT_MS: u64 = 45_000;

/// Default fire-cadence: invoke the LLM once per N bars (24 = once/day on
/// hourly bars). Between fires the strategy carries forward the last forecast.
pub const DEFAULT_FIRE_EVERY_N_BARS: u32 = 24;

// ── Rating ─────────────────────────────────────────────────────────────────────

/// 5-tier directional rating emitted by the LLM via the `propose_forecast`
/// tool. Operator-locked at Q1=(a) (decomp.md T-OD1).
///
/// ## Signal mapping
///
/// The 5-tier rating collapses to 3 `SignalKind` variants (T-AR-1):
/// STRONG_BUY / BUY → `SignalKind::Buy`
/// HOLD → `SignalKind::Hold`
/// SELL / STRONG_SELL → `SignalKind::Sell`
///
/// STRONG vs regular is NOT distinguished by `quantity_scale` at v0.1.0;
/// it is preserved in `LlmForecast::reasoning_trace` + audit `JournalEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Rating {
    /// Very strong bullish directional view.
    StrongBuy,
    /// Bullish directional view.
    Buy,
    /// Neutral — no directional edge. Strategy emits `Hold` / no order.
    Hold,
    /// Bearish directional view.
    Sell,
    /// Very strong bearish directional view.
    StrongSell,
}

impl Rating {
    /// Map to the 3-variant `SignalKind` used by the `Signal` type.
    ///
    /// STRONG_BUY + BUY → `SignalKind::Buy`
    /// HOLD → `SignalKind::Hold`
    /// SELL + STRONG_SELL → `SignalKind::Sell`
    #[must_use]
    pub fn to_signal_kind(self) -> SignalKind {
        match self {
            Rating::StrongBuy | Rating::Buy => SignalKind::Buy,
            Rating::Hold => SignalKind::Hold,
            Rating::Sell | Rating::StrongSell => SignalKind::Sell,
        }
    }

    /// String representation matching the `propose_forecast` tool-use schema.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Rating::StrongBuy => "STRONG_BUY",
            Rating::Buy => "BUY",
            Rating::Hold => "HOLD",
            Rating::Sell => "SELL",
            Rating::StrongSell => "STRONG_SELL",
        }
    }

    /// Try to parse from the `propose_forecast` JSON payload string value.
    ///
    /// Accepts both `SCREAMING_SNAKE_CASE` (canonical) and mixed-case
    /// (defensive for LLM drift per K4).
    ///
    /// Returns `None` if the string is not a recognised rating.
    #[must_use]
    pub fn try_parse(s: &str) -> Option<Self> {
        match s {
            "STRONG_BUY" | "strong_buy" => Some(Rating::StrongBuy),
            "BUY" | "buy" => Some(Rating::Buy),
            "HOLD" | "hold" => Some(Rating::Hold),
            "SELL" | "sell" => Some(Rating::Sell),
            "STRONG_SELL" | "strong_sell" => Some(Rating::StrongSell),
            _ => None,
        }
    }

    /// Index in the L1 bias-collapse histogram (ADR-0039 § D1.b).
    ///
    /// `hold_frac = rating_hist[2] / n_calls_total`
    #[must_use]
    pub fn histogram_index(self) -> usize {
        match self {
            Rating::StrongSell => 0,
            Rating::Sell => 1,
            Rating::Hold => 2,
            Rating::Buy => 3,
            Rating::StrongBuy => 4,
        }
    }
}

impl std::fmt::Display for Rating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Parse error for `Rating::from_str`.
#[derive(Debug, PartialEq, Eq)]
pub struct UnknownRating(pub String);

impl std::fmt::Display for UnknownRating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown rating: {:?}", self.0)
    }
}

impl std::error::Error for UnknownRating {}

impl std::str::FromStr for Rating {
    type Err = UnknownRating;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Rating::try_parse(s).ok_or_else(|| UnknownRating(s.to_string()))
    }
}

// ── Confidence ────────────────────────────────────────────────────────────────

/// Confidence score in `[0, 1]` emitted alongside the `Rating`.
///
/// Uses `rust_decimal::Decimal` per CLAUDE.md determinism guardrails
/// (no `f64` in money/price/qty calculations).
///
/// The strategy uses confidence for informational purposes only at v0.1.0
/// (displayed in the Phase F Assistant slot; logged in audit). Continuous
/// confidence-as-`quantity_scale` was explicitly rejected at T-AR-1 to
/// protect byte-identity under K4 Anthropic drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Confidence(Decimal);

impl Confidence {
    /// Construct a `Confidence` value, clamping to `[0, 1]`.
    #[must_use]
    pub fn new(v: Decimal) -> Self {
        Self(v.clamp(dec!(0), dec!(1)))
    }

    /// The underlying decimal in `[0, 1]`.
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0
    }
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.4}", self.0)
    }
}

// ── Horizon ───────────────────────────────────────────────────────────────────

/// Forecast horizon (currently fixed at 1-hour per R1.2 / decomp.md § T-AR-2).
///
/// v0.1.0 only supports `OneHour`. Future waves may add `FourHour` / `OneDay`
/// as operator-configure options; the enum is extensible at the JSON level
/// via `#[serde(rename_all = "snake_case")]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Horizon {
    /// Next 1-hour candle.
    #[default]
    OneHour,
}

// ── LessonCardRef ─────────────────────────────────────────────────────────────

/// Reference to a lesson card cited by the LLM in its reasoning trace.
///
/// The `card_id` is the content-hash key from `crates/reflection` (R2.3).
/// `cited_lesson_ids` is the JSON field in `propose_forecast` that the LLM
/// populates by citing cards from the retrieved top-K context block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LessonCardRef {
    /// Content-hash card ID from `crates/reflection::LessonCard::card_id`.
    pub card_id: String,
}

// ── CostEventRef ─────────────────────────────────────────────────────────────

/// Reference to the cost event emitted for this forecast call.
///
/// Populated at Wave E by the audit-emission step when
/// `BudgetedProvider` records the `CostEvent::Llm` row. At Wave A
/// this field is `None` in all forecast responses (audit wiring deferred).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostEventRef {
    /// Unique identifier for the `CostEvent::Llm` row.
    pub event_id: String,
    /// Actual cost in USD for this specific forecast call.
    pub usd_actual: Decimal,
}

// ── LlmForecast ──────────────────────────────────────────────────────────────

/// The full structured output from one `LlmForecaster::forecast()` call.
///
/// `LlmForecast` is the product of the `propose_forecast` tool-use response
/// decoded and validated by `LlmForecasterImpl`. It is:
/// - Cached in `LlmForecasterStrategy::last_forecast` for carry-forward
///   between fire ticks (R5.4).
/// - Serialised to the audit `JournalEntry { kind: "llm_forecast", payload }`
///   at Wave E (R7.1.2).
/// - Rendered in the Phase F Assistant slot body at Wave F (R9.2).
///
/// ## Serde
///
/// All fields implement `Serialize + Deserialize` so the replay-cache layer
/// can round-trip them through sqlite JSON without bespoke encoders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmForecast {
    /// The symbol this forecast is for (from `ForecastContext::symbol`).
    pub symbol: Symbol,
    /// UTC timestamp of the bar that triggered this forecast call.
    pub bar_ts: Timestamp,
    /// 5-tier directional rating.
    pub rating: Rating,
    /// Confidence in `[0, 1]`.
    pub confidence: Confidence,
    /// Forecast horizon (always `Horizon::OneHour` at v0.1.0).
    pub horizon: Horizon,
    /// The reasoning trace text (50-2000 chars per R1.2 / prompt schema).
    /// Rendered in the Phase F Assistant slot. SHA-256-hashed in the
    /// backtest report histogram (ADR-0039 § body shape).
    pub reasoning_trace: String,
    /// Lesson cards cited by the LLM (subset of the top-K retrieved context).
    /// May be empty if the LLM found no relevant prior trades.
    pub cited_lessons: Vec<LessonCardRef>,
    /// SHA-256 of `reasoning_trace` bytes (deterministic body anchor).
    /// Pre-computed on construction; avoids re-hashing at report time.
    pub reasoning_trace_sha256: [u8; 32],
    /// Reference to the cost event row (None at Wave A; populated at Wave E).
    pub cost_ref: Option<CostEventRef>,
    /// Name of the forecaster implementation that produced this forecast.
    pub forecaster_name: String,
    /// Correlation ID echoed from `ForecastContext::correlation_id`.
    pub correlation_id: uuid::Uuid,
}

impl LlmForecast {
    /// Construct a new `LlmForecast`, computing `reasoning_trace_sha256`.
    ///
    /// The 10-argument arity is intentional: `LlmForecast` is a rich value
    /// type populated from a single decode step in the forecaster impl; a
    /// builder pattern would add ceremony without value for a private inner
    /// constructor. The `#[allow]` suppresses the clippy lint.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        symbol: Symbol,
        bar_ts: Timestamp,
        rating: Rating,
        confidence: Confidence,
        horizon: Horizon,
        reasoning_trace: String,
        cited_lessons: Vec<LessonCardRef>,
        cost_ref: Option<CostEventRef>,
        forecaster_name: String,
        correlation_id: uuid::Uuid,
    ) -> Self {
        let reasoning_trace_sha256 = {
            let mut h = Sha256::new();
            h.update(reasoning_trace.as_bytes());
            h.finalize().into()
        };
        Self {
            symbol,
            bar_ts,
            rating,
            confidence,
            horizon,
            reasoning_trace,
            cited_lessons,
            reasoning_trace_sha256,
            cost_ref,
            forecaster_name,
            correlation_id,
        }
    }

    /// Human-readable hex of the `reasoning_trace_sha256` field.
    ///
    /// Used in the backtest report histogram per ADR-0039 § T-AR-6 body shape
    /// (`format!("{:x}", sha)` lowercase 64-hex).
    #[must_use]
    pub fn reasoning_trace_sha256_hex(&self) -> String {
        self.reasoning_trace_sha256
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

// ── Technical indicators ──────────────────────────────────────────────────────

/// Technical indicators passed to the LLM in the dynamic prompt block
/// (decomp.md § T-AR-2 architect-locked indicator set).
///
/// All values are `Decimal` per CLAUDE.md determinism guardrails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TechnicalIndicators {
    /// Relative Strength Index, 14-period.
    pub rsi_14: Decimal,
    /// MACD line (12,26,9).
    pub macd: Decimal,
    /// MACD signal line.
    pub macd_signal: Decimal,
    /// MACD histogram.
    pub macd_hist: Decimal,
    /// Bollinger Band upper (20,2).
    pub bb_upper: Decimal,
    /// Bollinger Band lower (20,2).
    pub bb_lower: Decimal,
    /// Average True Range, 14-period.
    pub atr_14: Decimal,
    /// Realized volatility over last 24 hours.
    pub realized_vol_24h: Decimal,
    /// Volatility-of-volatility over last 7 days.
    pub vol_of_vol_7d: Decimal,
}

// ── Recent decision snapshot ──────────────────────────────────────────────────

/// A past forecast decision included in the `ForecastContext` prompt block
/// (the "recent audit decisions" section, last N=10 per decomp.md § T-AR-2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentDecision {
    /// Audit journal entry ID (from `crates/audit`).
    pub audit_id: String,
    /// The rating emitted at that time.
    pub rating: Rating,
    /// The confidence at that time.
    pub confidence: Confidence,
    /// Outcome classification if the trade is closed (None if still open).
    pub outcome: Option<String>,
}

// ── ForecastContext ───────────────────────────────────────────────────────────

/// Input context assembled by `LlmForecasterStrategy::on_bar` before calling
/// `LlmForecaster::forecast()`.
///
/// The context carries ALL information that will be rendered in the prompt
/// dynamic block: OHLCV bars, technical indicators, top-K lesson cards,
/// and recent audit decisions. The `request_hash()` method produces the
/// canonical SHA-256 cache key over the deterministic subset of these fields.
///
/// ## Construction
///
/// - **In tests**: construct directly via struct literal or `ForecastContext::builder()`.
/// - **In `LlmForecasterStrategy::on_bar`**: call `ForecastContext::from_runtime()`
///   which pulls data from the per-symbol rolling window + indicator cache.
///
/// ## What is hashed vs not hashed
///
/// The `request_hash()` hashes over `CanonicalContext` (see below), which
/// includes: schema version, symbol, now (UTC ms), bars SHA, indicators SHA,
/// lesson-card IDs, decision IDs, model ID, and temperature. It does NOT
/// hash over the rendered markdown text — prompt format changes do not
/// invalidate the cache (only `PROMPT_TEMPLATE_VERSION` bumps do).
///
/// ## PartialEq note
///
/// `ForecastContext` does not derive `PartialEq`/`Eq` because `Bar` does not
/// implement those traits. Use `request_hash()` to compare contexts for
/// cache-key equality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastContext {
    /// Symbol being forecast.
    pub symbol: Symbol,
    /// UTC time of the triggering bar (bar open timestamp).
    pub now: Timestamp,
    /// Last 24 OHLCV bars (1h timeframe, N=24 default fire cadence).
    pub recent_bars: Vec<Bar>,
    /// Technical indicators computed over `recent_bars`.
    pub indicators: TechnicalIndicators,
    /// Top-K lesson cards retrieved from `crates/reflection` for this
    /// `(symbol, regime)`. Wiring lands at Wave C (`ForecastContext::from_runtime`);
    /// runtime retrieval uses `reflection::retrieve_top_k(query, REPORT_TIME_TOP_K)`.
    pub top_k_lessons: Vec<reflection::types::LessonCard>,
    /// Last N=10 past forecast decisions on this symbol (from audit ledger).
    pub recent_decisions: Vec<RecentDecision>,
    /// Model ID to use for this call (e.g. `claude-haiku-4-5-20251001`).
    pub model_id: String,
    /// Operator-provided correlation id echoed through to `LlmForecast`.
    pub correlation_id: uuid::Uuid,
}

/// Error type for [`ForecastContext::from_runtime`].
#[derive(Debug, thiserror::Error)]
pub enum FromRuntimeError {
    /// Reflection-store retrieval failed.
    #[error("reflection store retrieval failed: {0}")]
    ReflectionStore(#[from] reflection::RetrievalError),
}

impl ForecastContext {
    /// Production constructor — assembles context from live runtime state.
    ///
    /// ## What this does (Wave C)
    ///
    /// 1. Builds a `RetrievalQuery` from `bar.symbol` + BTC regime tag.
    /// 2. Calls `reflection::retrieve_top_k(store, &query, REPORT_TIME_TOP_K)`
    ///    to fetch the top-K lesson cards.
    /// 3. Computes a zeroed `TechnicalIndicators` stub (real indicator
    ///    computation is deferred to Wave D / the indicator-cache wiring).
    /// 4. Assembles `ForecastContext` with the retrieved lessons + supplied
    ///    `recent_decisions`.
    ///
    /// ## Async
    ///
    /// This is `async` because `ReflectionStore::top_k` is async.  In the
    /// sync `Strategy::on_bar` call site use `pollster::block_on` (or the
    /// tokio `Handle::block_on`) to drive the future.  This mirrors the
    /// pattern used in `call_forecast`.
    ///
    /// ## Indicator stub (Wave C / analytical-only)
    ///
    /// Indicator computation requires a per-symbol indicator cache (Wave D).
    /// For Wave C, `TechnicalIndicators` is populated with placeholder values
    /// derived from the `recent_bars` (simple last-close RSI stub = `dec!(50)`
    /// and zeroes elsewhere).  The architect accepted this for the analytical
    /// correctness test — the noop-fix lesson is about `Signal.kind` mutation,
    /// not indicator precision.
    ///
    /// # Errors
    ///
    /// Returns [`FromRuntimeError::ReflectionStore`] if the store call fails.
    pub async fn from_runtime(
        bar: &Bar,
        reflection_store: &dyn ReflectionStore,
        btc_closes: &[(Timestamp, Decimal)],
        recent_decisions: Vec<RecentDecision>,
        model_id: &str,
        recent_bars: Vec<Bar>,
    ) -> Result<Self, FromRuntimeError> {
        // 1. Classify the current BTC regime (fallback to Chop on error).
        let regime =
            classify_regime(btc_closes, bar.open_ts).unwrap_or(reflection::RegimeTag::Chop);

        // 2. Build retrieval query.
        let query = reflection::RetrievalQuery {
            strategy_id: StrategyId::new("llm_forecaster_v3"),
            symbol_or_pair: SymbolOrPair::Single(bar.symbol.clone()),
            current_regime: regime,
        };

        // 3. Retrieve top-K lesson cards (K = REPORT_TIME_TOP_K = 5).
        let cards = retrieve_top_k(reflection_store, &query, REPORT_TIME_TOP_K).await?;
        tracing::debug!(
            target: "llm_forecaster::context",
            symbol = %bar.symbol,
            regime = %regime,
            n_cards = cards.len(),
            "retrieved top-K lesson cards for ForecastContext"
        );

        // 4. Stub indicators (Wave C / analytical-only; real compute in Wave D).
        let indicators = TechnicalIndicators {
            rsi_14: dec!(50),
            macd: dec!(0),
            macd_signal: dec!(0),
            macd_hist: dec!(0),
            bb_upper: dec!(1),
            bb_lower: dec!(0),
            atr_14: dec!(0),
            realized_vol_24h: dec!(0),
            vol_of_vol_7d: dec!(0),
        };

        Ok(Self {
            symbol: bar.symbol.clone(),
            now: bar.open_ts,
            recent_bars,
            indicators,
            top_k_lessons: cards,
            recent_decisions,
            model_id: model_id.to_string(),
            correlation_id: uuid::Uuid::new_v4(),
        })
    }

    /// Deterministic constructor for tests.
    ///
    /// Builds a minimal `ForecastContext` with empty lessons / decisions and
    /// zeroed indicators. Used by unit tests that want to exercise the hash
    /// canonicalisation without constructing live runtime state.
    #[must_use]
    pub fn test_fixture(symbol: Symbol, now: Timestamp, recent_bars: Vec<Bar>) -> Self {
        Self {
            symbol,
            now,
            recent_bars,
            indicators: TechnicalIndicators {
                rsi_14: dec!(50),
                macd: dec!(0),
                macd_signal: dec!(0),
                macd_hist: dec!(0),
                bb_upper: dec!(1),
                bb_lower: dec!(0),
                atr_14: dec!(0),
                realized_vol_24h: dec!(0),
                vol_of_vol_7d: dec!(0),
            },
            top_k_lessons: Vec::new(),
            recent_decisions: Vec::new(),
            model_id: DEFAULT_MODEL_ID.to_string(),
            correlation_id: uuid::Uuid::new_v4(),
        }
    }

    /// Compute the canonical SHA-256 cache key for this context.
    ///
    /// The key is deterministic: identical `ForecastContext` values (same
    /// symbol, same bars, same indicators, same lesson-card IDs, same model)
    /// always produce the same 32-byte key.
    ///
    /// ## Algorithm (T-AR-2 architect-locked)
    ///
    /// 1. Build `CanonicalContext` from the deterministic subset of `self`.
    /// 2. Serialise with `serde_json::to_vec` (field order = declaration order
    ///    in the struct, which is alphabetical per architect lock).
    /// 3. SHA-256 over the resulting bytes.
    ///
    /// ## Panics
    ///
    /// Never panics — `CanonicalContext` is always serialisable (all fields
    /// are primitive-or-string JSON types with no `f64`).
    #[must_use]
    pub fn request_hash(&self) -> [u8; 32] {
        let canonical = CanonicalContext {
            indicators_sha: Self::hash_indicators(&self.indicators),
            model_id: self.model_id.as_str(),
            now: (self.now.0.unix_timestamp_nanos() / 1_000_000) as i64, // UTC ms as i64
            prompt_template_version: PROMPT_TEMPLATE_VERSION,
            recent_bars_sha: Self::hash_bars(&self.recent_bars),
            recent_decision_ids: self
                .recent_decisions
                .iter()
                .map(|d| d.audit_id.as_str())
                .collect::<Vec<_>>(),
            schema_version: CACHE_SCHEMA_VERSION,
            symbol: self.symbol.0.as_str(),
            temperature: 0, // pinned per R3.4
            top_k_lesson_ids: self
                .top_k_lessons
                .iter()
                .map(|l| l.card_id.as_str())
                .collect::<Vec<_>>(),
        };

        let bytes =
            serde_json::to_vec(&canonical).expect("CanonicalContext is always-serialisable");
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        hasher.finalize().into()
    }

    /// SHA-256 of the deterministic OHLCV bar fields (excludes `local_recv_ts`
    /// and `venue` which may vary across data-sources for the same bar).
    fn hash_bars(bars: &[Bar]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        for bar in bars {
            // Deterministic field subset: symbol + open_ts + close + open + high + low.
            // open_ts is the bar identity; price fields are the data.
            // Use .get() since Price.0 is private.
            hasher.update(bar.symbol.0.as_bytes());
            hasher.update(bar.open_ts.0.unix_timestamp_nanos().to_le_bytes().as_ref());
            hasher.update(bar.close.get().serialize().as_ref());
            hasher.update(bar.open.get().serialize().as_ref());
            hasher.update(bar.high.get().serialize().as_ref());
            hasher.update(bar.low.get().serialize().as_ref());
        }
        hasher.finalize().into()
    }

    /// SHA-256 of the technical indicators as deterministic Decimal bytes.
    fn hash_indicators(ind: &TechnicalIndicators) -> [u8; 32] {
        let mut hasher = Sha256::new();
        // Alphabetical field order matches CanonicalContext declaration order.
        hasher.update(ind.atr_14.serialize().as_ref());
        hasher.update(ind.bb_lower.serialize().as_ref());
        hasher.update(ind.bb_upper.serialize().as_ref());
        hasher.update(ind.macd.serialize().as_ref());
        hasher.update(ind.macd_hist.serialize().as_ref());
        hasher.update(ind.macd_signal.serialize().as_ref());
        hasher.update(ind.realized_vol_24h.serialize().as_ref());
        hasher.update(ind.rsi_14.serialize().as_ref());
        hasher.update(ind.vol_of_vol_7d.serialize().as_ref());
        hasher.finalize().into()
    }
}

// ── CanonicalContext (internal; hash target) ──────────────────────────────────

/// Internal struct serialised to JSON for `request_hash()` computation.
///
/// ## Field ordering (CRITICAL for determinism)
///
/// Fields are declared in ALPHABETICAL order per decomp.md § T-AR-2 architect
/// lock. `serde_json::to_vec` respects struct-field-declaration order. Any
/// change to field order MUST be accompanied by a `CACHE_SCHEMA_VERSION` bump.
///
/// ## No `f64` anywhere
///
/// All numeric fields are `i64` or `u32` (integer) or `[u8; 32]` (hex bytes).
/// The `Decimal` values from indicators are pre-hashed into `indicators_sha`.
#[derive(Debug, Serialize)]
struct CanonicalContext<'a> {
    /// SHA-256 of the technical indicators (deterministic Decimal bytes).
    indicators_sha: [u8; 32],
    /// Model identifier string (e.g. `"claude-haiku-4-5-20251001"`).
    model_id: &'a str,
    /// UTC milliseconds since Unix epoch for `ForecastContext::now`.
    now: i64,
    /// Prompt template version (invalidates cache on layout changes).
    prompt_template_version: u32,
    /// SHA-256 of the OHLCV bar data.
    recent_bars_sha: [u8; 32],
    /// Ordered list of `RecentDecision::audit_id` values.
    recent_decision_ids: Vec<&'a str>,
    /// Cache schema version (invalidates all rows on struct changes).
    schema_version: u32,
    /// Symbol string (e.g. `"BTCUSDT"`).
    symbol: &'a str,
    /// Temperature always 0 (pinned per R3.4; embedded in hash for auditability).
    temperature: u8,
    /// Ordered list of `LessonCard::card_id` values (retrieval rank order).
    top_k_lesson_ids: Vec<&'a str>,
}

// ── LlmForecasterError ────────────────────────────────────────────────────────

/// Error type for `LlmForecaster::forecast()`.
///
/// ## Backtest-mode FATAL errors
///
/// `ReplayMiss` is FATAL in backtest / research mode (non-zero exit per
/// decomp.md § T-AR-5 Layer 2). The backtest binary checks for this error
/// variant and short-circuits with an explicit error log.
///
/// `BudgetExceeded` is also fatal in backtest mode (L3 verdict trigger
/// per ADR-0039 § D1.b).
#[derive(Debug, thiserror::Error)]
pub enum LlmForecasterError {
    /// Cache miss in `ReplayProvider` mode (backtest / research).
    ///
    /// Hex string is the `request_hash` that was not found.
    #[error("replay cache miss for hash {hash}; re-record this scenario to fix")]
    ReplayMiss {
        /// Hex-encoded `request_hash` ([u8; 32]).
        hash: String,
    },

    /// Per-call or per-backtest budget cap exceeded.
    ///
    /// `cap_usd` is the configured limit; `actual_usd` is the running total.
    #[error("LLM budget exceeded: actual ${actual_usd} > cap ${cap_usd}")]
    BudgetExceeded {
        /// Configured per-call or per-backtest cap in USD.
        cap_usd: Decimal,
        /// Actual running spend at the point of rejection.
        actual_usd: Decimal,
    },

    /// The `propose_forecast` tool-use response failed JSON-schema validation.
    #[error("invalid LLM response: {reason}")]
    InvalidResponse {
        /// Human-readable explanation of the validation failure.
        reason: String,
    },

    /// The per-call wall-clock timeout (`DEFAULT_TIMEOUT_MS`) was exceeded.
    #[error("LLM call timed out after {timeout_ms}ms")]
    Timeout {
        /// Configured timeout in milliseconds.
        timeout_ms: u64,
    },

    /// Upstream `llm::LlmProvider::complete` returned an error.
    #[error("LLM provider error: {0}")]
    Provider(#[from] llm::error::LlmError),

    /// Internal error (serialisation / deserialization).
    #[error("internal error: {0}")]
    Internal(String),
}

impl LlmForecasterError {
    /// True if this error is fatal in backtest mode (non-zero exit required).
    #[must_use]
    pub fn is_backtest_fatal(&self) -> bool {
        matches!(self, Self::ReplayMiss { .. } | Self::BudgetExceeded { .. })
    }
}

// ── LlmForecasterConfig ───────────────────────────────────────────────────────

/// Configuration for `LlmForecasterStrategy` and `LlmForecasterImpl`.
///
/// Deserialised from `config/agent.toml` under
/// `[[strategies]] kind = "llm_forecaster_v3"`. Sensible defaults via
/// `LlmForecasterConfig::default()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmForecasterConfig {
    /// Strategy enabled flag. When `false`, `on_bar` returns `Vec::new()`
    /// immediately (R9.3 default-disabled + Phase F byte-identity guard).
    pub enabled: bool,
    /// Model ID string (default: `DEFAULT_MODEL_ID`).
    pub model_id: String,
    /// Invoke the LLM once per N bars (default: `DEFAULT_FIRE_EVERY_N_BARS`).
    pub fire_every_n_bars: u32,
    /// Per-call wall-clock timeout in milliseconds (default: `DEFAULT_TIMEOUT_MS`).
    pub timeout_ms: u64,
    /// Per-backtest USD cost cap (default: $100 per Haiku scenario; T-AR-4).
    pub cost_cap_usd_per_backtest: Decimal,
    /// Per-call USD cost cap (default: $0.05 for Haiku; T-AR-4).
    pub cost_cap_usd_per_call: Decimal,
}

impl Default for LlmForecasterConfig {
    fn default() -> Self {
        Self {
            enabled: false, // opt-in per R9.3
            model_id: DEFAULT_MODEL_ID.to_string(),
            fire_every_n_bars: DEFAULT_FIRE_EVERY_N_BARS,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            cost_cap_usd_per_backtest: dec!(100.00),
            cost_cap_usd_per_call: dec!(0.05),
        }
    }
}

// ── Stub forecaster (Wave A testing) ─────────────────────────────────────────

/// A stub `LlmForecaster` implementation for Wave A tests.
///
/// Always returns `Rating::Hold` with confidence 0.5 and a fixed reasoning
/// trace. Does NOT call any LLM provider. Used in Wave A unit tests only.
///
/// Real impl: `LlmForecasterImpl` in `anthropic_impl.rs` (Wave B).
pub struct StubForecaster {
    /// Fixed rating to return (default `Rating::Hold`).
    pub fixed_rating: Rating,
    /// Fixed reasoning trace to return.
    pub fixed_trace: String,
}

impl Default for StubForecaster {
    fn default() -> Self {
        Self {
            fixed_rating: Rating::Hold,
            fixed_trace: "stub: no LLM call — Wave A type-only".to_string(),
        }
    }
}

impl StubForecaster {
    /// Create a stub that returns a specific rating.
    #[must_use]
    pub fn with_rating(rating: Rating) -> Self {
        Self {
            fixed_rating: rating,
            ..Default::default()
        }
    }
}

#[async_trait::async_trait]
impl super::trait_def::LlmForecaster for StubForecaster {
    fn name(&self) -> &str {
        "stub"
    }

    async fn forecast(&self, ctx: ForecastContext) -> Result<LlmForecast, LlmForecasterError> {
        let trace = self.fixed_trace.clone();
        Ok(LlmForecast::new(
            ctx.symbol.clone(),
            ctx.now,
            self.fixed_rating,
            Confidence::new(dec!(0.5)),
            Horizon::OneHour,
            trace,
            Vec::new(),
            None,
            self.name().to_string(),
            ctx.correlation_id,
        ))
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

    fn make_symbol(s: &str) -> Symbol {
        Symbol::new(s)
    }

    fn make_ts(epoch_s: i64) -> Timestamp {
        Timestamp::new(OffsetDateTime::from_unix_timestamp(epoch_s).expect("valid ts"))
    }

    fn make_bar(symbol: &str, open_ts_s: i64, close_price: Decimal) -> Bar {
        let sym = make_symbol(symbol);
        let ts = make_ts(open_ts_s);
        Bar {
            symbol: sym,
            tf: Timeframe::OneHour,
            open_ts: ts,
            close_ts: make_ts(open_ts_s + 3600),
            open: Price::new(close_price).expect("positive price"),
            high: Price::new(close_price).expect("positive price"),
            low: Price::new(close_price).expect("positive price"),
            close: Price::new(close_price).expect("positive price"),
            volume: Quantity::new(dec!(1000)).expect("positive qty"),
            trade_count: 100,
            local_recv_ts: ts,
            venue: Venue::Binance,
        }
    }

    // ── T-D-N(A5) rating round-trip tests ────────────────────────────────────

    /// Rating::to_signal_kind() maps each tier to the correct SignalKind.
    #[test]
    fn rating_to_signal_kind_covers_all_variants() {
        assert_eq!(Rating::StrongBuy.to_signal_kind(), SignalKind::Buy);
        assert_eq!(Rating::Buy.to_signal_kind(), SignalKind::Buy);
        assert_eq!(Rating::Hold.to_signal_kind(), SignalKind::Hold);
        assert_eq!(Rating::Sell.to_signal_kind(), SignalKind::Sell);
        assert_eq!(Rating::StrongSell.to_signal_kind(), SignalKind::Sell);
    }

    /// Rating serde round-trip preserves all variants.
    #[test]
    fn rating_serde_round_trip() {
        let ratings = [
            Rating::StrongBuy,
            Rating::Buy,
            Rating::Hold,
            Rating::Sell,
            Rating::StrongSell,
        ];
        for r in ratings {
            let json = serde_json::to_string(&r).expect("serialize");
            let back: Rating = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, r, "serde round-trip failed for {r:?}");
        }
    }

    /// Rating::try_parse parses SCREAMING_SNAKE_CASE values.
    #[test]
    fn rating_try_parse_parses_all_variants() {
        assert_eq!(Rating::try_parse("STRONG_BUY"), Some(Rating::StrongBuy));
        assert_eq!(Rating::try_parse("BUY"), Some(Rating::Buy));
        assert_eq!(Rating::try_parse("HOLD"), Some(Rating::Hold));
        assert_eq!(Rating::try_parse("SELL"), Some(Rating::Sell));
        assert_eq!(Rating::try_parse("STRONG_SELL"), Some(Rating::StrongSell));
        assert_eq!(Rating::try_parse("UNKNOWN"), None);
    }

    /// Rating::from_str (FromStr impl) returns Err on unknown values.
    #[test]
    fn rating_from_str_trait_returns_err_on_unknown() {
        use std::str::FromStr;
        assert_eq!(Rating::from_str("STRONG_BUY"), Ok(Rating::StrongBuy));
        assert!(Rating::from_str("UNKNOWN").is_err());
    }

    /// LlmForecast serde round-trip preserves all fields.
    #[test]
    fn llm_forecast_serde_round_trip() {
        let sym = make_symbol("BTCUSDT");
        let ts = make_ts(1_700_000_000);
        let forecast = LlmForecast::new(
            sym,
            ts,
            Rating::Buy,
            Confidence::new(dec!(0.75)),
            Horizon::OneHour,
            "Market shows bullish momentum with RSI above 60.".to_string(),
            vec![LessonCardRef {
                card_id: "card_abc123".to_string(),
            }],
            None,
            "stub".to_string(),
            uuid::Uuid::nil(),
        );

        let json = serde_json::to_string(&forecast).expect("serialize LlmForecast");
        let back: LlmForecast = serde_json::from_str(&json).expect("deserialize LlmForecast");
        assert_eq!(back.symbol, forecast.symbol);
        assert_eq!(back.rating, forecast.rating);
        assert_eq!(back.confidence, forecast.confidence);
        assert_eq!(back.reasoning_trace, forecast.reasoning_trace);
        assert_eq!(back.reasoning_trace_sha256, forecast.reasoning_trace_sha256);
    }

    // ── T-D-N(A3) + T-D-N(A4): ForecastContext tests ─────────────────────────

    /// forecast_context_from_runtime: ForecastContext::test_fixture produces
    /// a deterministic context usable in unit tests.
    #[test]
    fn forecast_context_from_runtime() {
        let sym = make_symbol("BTCUSDT");
        let ts = make_ts(1_700_000_000);
        let bars = vec![make_bar("BTCUSDT", 1_700_000_000, dec!(45000))];
        let ctx = ForecastContext::test_fixture(sym.clone(), ts, bars.clone());

        assert_eq!(ctx.symbol, sym);
        assert_eq!(ctx.model_id, DEFAULT_MODEL_ID);
        assert!(ctx.top_k_lessons.is_empty());
        assert!(ctx.recent_decisions.is_empty());
        assert_eq!(ctx.indicators.rsi_14, dec!(50));
    }

    /// forecast_context_request_hash: identical contexts produce identical SHA.
    #[test]
    fn forecast_context_request_hash_identical_inputs_produce_identical_sha() {
        let sym = make_symbol("BTCUSDT");
        let ts = make_ts(1_700_000_000);
        let bars = vec![make_bar("BTCUSDT", 1_700_000_000, dec!(45000))];

        let ctx1 = ForecastContext {
            symbol: sym.clone(),
            now: ts,
            recent_bars: bars.clone(),
            indicators: TechnicalIndicators {
                rsi_14: dec!(65.23),
                macd: dec!(0.5),
                macd_signal: dec!(0.3),
                macd_hist: dec!(0.2),
                bb_upper: dec!(46000),
                bb_lower: dec!(44000),
                atr_14: dec!(500),
                realized_vol_24h: dec!(0.02),
                vol_of_vol_7d: dec!(0.005),
            },
            top_k_lessons: Vec::new(),
            recent_decisions: Vec::new(),
            model_id: DEFAULT_MODEL_ID.to_string(),
            correlation_id: uuid::Uuid::nil(),
        };

        // Second context with same values but fresh struct allocation.
        let ctx2 = ForecastContext {
            symbol: sym,
            now: ts,
            recent_bars: bars,
            indicators: TechnicalIndicators {
                rsi_14: dec!(65.23),
                macd: dec!(0.5),
                macd_signal: dec!(0.3),
                macd_hist: dec!(0.2),
                bb_upper: dec!(46000),
                bb_lower: dec!(44000),
                atr_14: dec!(500),
                realized_vol_24h: dec!(0.02),
                vol_of_vol_7d: dec!(0.005),
            },
            top_k_lessons: Vec::new(),
            recent_decisions: Vec::new(),
            model_id: DEFAULT_MODEL_ID.to_string(),
            correlation_id: uuid::Uuid::nil(),
        };

        assert_eq!(
            ctx1.request_hash(),
            ctx2.request_hash(),
            "identical contexts must produce identical SHA-256"
        );
    }

    /// Different symbols produce different hashes.
    #[test]
    fn forecast_context_request_hash_different_symbol_produces_different_sha() {
        let ts = make_ts(1_700_000_000);
        let ctx_btc = ForecastContext::test_fixture(
            make_symbol("BTCUSDT"),
            ts,
            vec![make_bar("BTCUSDT", 1_700_000_000, dec!(45000))],
        );
        let ctx_eth = ForecastContext::test_fixture(
            make_symbol("ETHUSDT"),
            ts,
            vec![make_bar("ETHUSDT", 1_700_000_000, dec!(2500))],
        );
        assert_ne!(
            ctx_btc.request_hash(),
            ctx_eth.request_hash(),
            "different symbols must produce different hashes"
        );
    }

    /// Different model_ids produce different hashes.
    #[test]
    fn forecast_context_request_hash_different_model_produces_different_sha() {
        let sym = make_symbol("BTCUSDT");
        let ts = make_ts(1_700_000_000);
        let bars = vec![make_bar("BTCUSDT", 1_700_000_000, dec!(45000))];

        let mut ctx1 = ForecastContext::test_fixture(sym.clone(), ts, bars.clone());
        ctx1.model_id = "claude-haiku-4-5-20251001".to_string();

        let mut ctx2 = ForecastContext::test_fixture(sym, ts, bars);
        ctx2.model_id = "claude-sonnet-4-6".to_string();

        assert_ne!(
            ctx1.request_hash(),
            ctx2.request_hash(),
            "different model_ids must produce different hashes"
        );
    }

    /// Confidence clamps to [0, 1].
    #[test]
    fn confidence_clamps_to_unit_interval() {
        assert_eq!(Confidence::new(dec!(-0.5)).value(), dec!(0));
        assert_eq!(Confidence::new(dec!(1.5)).value(), dec!(1));
        assert_eq!(Confidence::new(dec!(0.75)).value(), dec!(0.75));
    }

    /// LlmForecasterConfig defaults match the constants.
    #[test]
    fn llm_forecaster_config_defaults() {
        let cfg = LlmForecasterConfig::default();
        assert!(!cfg.enabled, "default must be disabled (R9.3)");
        assert_eq!(cfg.model_id, DEFAULT_MODEL_ID);
        assert_eq!(cfg.fire_every_n_bars, DEFAULT_FIRE_EVERY_N_BARS);
        assert_eq!(cfg.timeout_ms, DEFAULT_TIMEOUT_MS);
    }
}
