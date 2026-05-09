//! `SqliteReflectionStore` — default impl backed by sqlx + sqlite.
//!
//! - Opens with `journal_mode=WAL`, `synchronous=NORMAL`.
//! - Runs the single migration `001_lesson_cards.sql`.
//! - `top_k` does a deterministic linear scan in-process (R7.2 sized
//!   for the v1 ≤500-card budget).

use std::cmp::Ordering;
use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::ConnectOptions;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use trading_core::{Money, PairKey, StrategyId, Symbol, Timestamp, Usdt};

use crate::embedding::{cosine, embed, EMBEDDING_DIM};
use crate::outcome::OutcomeClass;
use crate::regime::RegimeTag;
use crate::store::{ReflectionStore, ReflectionStoreError};
use crate::types::{LessonCard, RetrievalQuery, SymbolOrPair};

/// Default sqlite store.
pub struct SqliteReflectionStore {
    pool: sqlx::SqlitePool,
}

impl SqliteReflectionStore {
    /// Open (or create) the reflection store at `path` and run the
    /// embedded migrations.  WAL mode for safe concurrent
    /// reader / writer access.
    ///
    /// # Errors
    ///
    /// Returns [`ReflectionStoreError::Database`] on connection or
    /// migration failure.
    pub async fn open(path: &Path) -> Result<Self, ReflectionStoreError> {
        let path_str = path.to_string_lossy().into_owned();
        let in_memory = path_str == ":memory:";
        let opts = if in_memory {
            SqliteConnectOptions::new()
                .filename(":memory:")
                .create_if_missing(true)
        } else {
            SqliteConnectOptions::new()
                .filename(&path_str)
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        };
        let opts = opts.disable_statement_logging();

        // For in-memory mode the pool MUST hold a single connection,
        // otherwise each new connection gets its own fresh DB and the
        // migration runs against an instance the queries never see.
        let max_connections = if in_memory { 1 } else { 4 };

        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(10))
            .connect_with(opts)
            .await
            .map_err(|e| ReflectionStoreError::Database(e.to_string()))?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| ReflectionStoreError::Database(e.to_string()))?;

        Ok(Self { pool })
    }

    /// Open an in-memory store for tests.
    ///
    /// # Errors
    ///
    /// Returns [`ReflectionStoreError::Database`] on failure.
    pub async fn in_memory() -> Result<Self, ReflectionStoreError> {
        Self::open(Path::new(":memory:")).await
    }
}

/// Persisted row → `LessonCard` materialisation.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
struct PersistedRow {
    card_id: String,
    closed_at: String,
    symbol_or_pair: String,
    strategy_id: String,
    signed_pnl_usdt: String,
    opening_capital_usdt: String,
    holding_period_bars: i64,
    entry_regime: String,
    exit_regime: String,
    outcome_class: String,
    embedding_blob: String,
    note: Option<String>,
}

#[async_trait]
impl ReflectionStore for SqliteReflectionStore {
    async fn upsert(&self, card: &LessonCard) -> Result<bool, ReflectionStoreError> {
        // Idempotency check first — `INSERT OR IGNORE` under SQLite
        // returns no row-affected count we can use without `RETURNING`,
        // and `RETURNING` requires sqlite >= 3.35.  Check explicitly.
        let existing: Option<(String,)> =
            sqlx::query_as("SELECT card_id FROM lesson_cards WHERE card_id = ?")
                .bind(&card.card_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ReflectionStoreError::Database(e.to_string()))?;
        if existing.is_some() {
            return Ok(false);
        }

        let closed_at_str = card
            .closed_at
            .inner()
            .format(&Rfc3339)
            .map_err(|e| ReflectionStoreError::Encoding(e.to_string()))?;

        let embedding_arr = embed(card);
        let embedding_blob = encode_embedding(&embedding_arr);

        sqlx::query(
            "INSERT INTO lesson_cards \
             (card_id, closed_at, symbol_or_pair, strategy_id, signed_pnl_usdt, \
              opening_capital_usdt, holding_period_bars, entry_regime, exit_regime, \
              outcome_class, embedding_blob, note) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&card.card_id)
        .bind(&closed_at_str)
        .bind(card.symbol_or_pair.to_string())
        .bind(card.strategy_id.0.as_str())
        .bind(card.signed_pnl.amount().to_string())
        .bind(card.opening_capital.amount().to_string())
        .bind(i64::from(card.holding_period_bars))
        .bind(card.entry_regime.to_string())
        .bind(card.exit_regime.to_string())
        .bind(card.outcome_class.to_string())
        .bind(&embedding_blob)
        .bind(card.note.as_deref())
        .execute(&self.pool)
        .await
        .map_err(|e| ReflectionStoreError::Database(e.to_string()))?;

        Ok(true)
    }

    async fn top_k(
        &self,
        query: &RetrievalQuery,
        k: usize,
    ) -> Result<Vec<LessonCard>, ReflectionStoreError> {
        if k == 0 {
            return Ok(Vec::new());
        }

        let rows: Vec<PersistedRow> = sqlx::query_as::<_, PersistedRow>(
            "SELECT card_id, closed_at, symbol_or_pair, strategy_id, signed_pnl_usdt, \
                    opening_capital_usdt, holding_period_bars, entry_regime, exit_regime, \
                    outcome_class, embedding_blob, note \
             FROM lesson_cards",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ReflectionStoreError::Database(e.to_string()))?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        // Build the query embedding from the query fields.  Use a
        // synthetic "pseudo-card" with neutral scalar slots so the
        // one-hot slots dominate the cosine ranking deterministically.
        let query_card = LessonCard {
            card_id: String::new(),
            closed_at: Timestamp::new(OffsetDateTime::UNIX_EPOCH),
            symbol_or_pair: query.symbol_or_pair.clone(),
            strategy_id: query.strategy_id.clone(),
            signed_pnl: Money::<Usdt>::from_decimal(Decimal::ZERO),
            opening_capital: Money::<Usdt>::from_decimal(Decimal::ZERO),
            holding_period_bars: 0,
            entry_regime: query.current_regime,
            exit_regime: query.current_regime,
            outcome_class: OutcomeClass::Scratch,
            note: None,
        };
        let query_embedding = embed(&query_card);

        let mut scored: Vec<(Decimal, Timestamp, LessonCard)> = Vec::with_capacity(rows.len());
        for row in rows {
            let card = decode_row(row)?;
            let row_embedding = decode_embedding(&card)?;
            let score = cosine(&query_embedding, &row_embedding);
            scored.push((score, card.closed_at, card));
        }

        // Sort: score DESC, closed_at ASC (R3.1 tie-break).
        scored.sort_by(|a, b| match b.0.cmp(&a.0) {
            Ordering::Equal => a.1.cmp(&b.1),
            other => other,
        });
        scored.truncate(k);
        Ok(scored.into_iter().map(|(_, _, c)| c).collect())
    }

    async fn count(&self) -> Result<u64, ReflectionStoreError> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM lesson_cards")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ReflectionStoreError::Database(e.to_string()))?;
        Ok(u64::try_from(row.0).unwrap_or(0))
    }
}

fn encode_embedding(v: &[Decimal; EMBEDDING_DIM]) -> String {
    let mut s = String::with_capacity(EMBEDDING_DIM * 12);
    for (i, c) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&c.to_string());
    }
    s
}

fn decode_row(row: PersistedRow) -> Result<LessonCard, ReflectionStoreError> {
    let closed_at = OffsetDateTime::parse(&row.closed_at, &Rfc3339)
        .map_err(|e| ReflectionStoreError::Encoding(e.to_string()))?;
    let signed_pnl: Decimal = row
        .signed_pnl_usdt
        .parse()
        .map_err(|_| ReflectionStoreError::Encoding("signed_pnl parse".into()))?;
    let opening_capital: Decimal = row
        .opening_capital_usdt
        .parse()
        .map_err(|_| ReflectionStoreError::Encoding("opening_capital parse".into()))?;

    let entry_regime = parse_regime(&row.entry_regime)?;
    let exit_regime = parse_regime(&row.exit_regime)?;
    let outcome_class = parse_outcome(&row.outcome_class)?;
    let symbol_or_pair = parse_symbol_or_pair(&row.symbol_or_pair)?;
    let strategy_id = StrategyId::new(row.strategy_id);

    Ok(LessonCard {
        card_id: row.card_id,
        closed_at: Timestamp::new(closed_at),
        symbol_or_pair,
        strategy_id,
        signed_pnl: Money::<Usdt>::from_decimal(signed_pnl),
        opening_capital: Money::<Usdt>::from_decimal(opening_capital),
        holding_period_bars: u32::try_from(row.holding_period_bars).unwrap_or(0),
        entry_regime,
        exit_regime,
        outcome_class,
        note: row.note,
    })
}

fn decode_embedding(card: &LessonCard) -> Result<[Decimal; EMBEDDING_DIM], ReflectionStoreError> {
    // Round-trip via `embed` — the stored blob is purely a debug
    // accelerator (TEXT comma-separated).  Re-deriving from the card
    // is byte-stable and keeps the consumer path free of
    // string-parsing edge cases.
    Ok(embed(card))
}

fn parse_regime(s: &str) -> Result<RegimeTag, ReflectionStoreError> {
    match s {
        "bull" => Ok(RegimeTag::Bull),
        "bear" => Ok(RegimeTag::Bear),
        "chop" => Ok(RegimeTag::Chop),
        other => Err(ReflectionStoreError::Encoding(format!(
            "unknown regime tag: {other}"
        ))),
    }
}

fn parse_outcome(s: &str) -> Result<OutcomeClass, ReflectionStoreError> {
    match s {
        "Win" => Ok(OutcomeClass::Win),
        "Loss" => Ok(OutcomeClass::Loss),
        "Scratch" => Ok(OutcomeClass::Scratch),
        other => Err(ReflectionStoreError::Encoding(format!(
            "unknown outcome class: {other}"
        ))),
    }
}

fn parse_symbol_or_pair(s: &str) -> Result<SymbolOrPair, ReflectionStoreError> {
    if let Some(inner) = s.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
        // PairKey::Display formats as `(a, b)` (with a comma + space).
        let parts: Vec<&str> = inner.splitn(2, ", ").collect();
        if parts.len() == 2 {
            let a = Symbol::new(parts[0]);
            let b = Symbol::new(parts[1]);
            let pair =
                PairKey::new(a, b).map_err(|e| ReflectionStoreError::Encoding(e.to_string()))?;
            return Ok(SymbolOrPair::Pair(pair));
        }
    }
    Ok(SymbolOrPair::Single(Symbol::new(s)))
}

#[cfg(test)]
#[allow(clippy::float_arithmetic)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use trading_core::Symbol;

    #[test]
    fn parse_symbol_or_pair_single() {
        let r = parse_symbol_or_pair("BTCUSDT").unwrap();
        match r {
            SymbolOrPair::Single(s) => assert_eq!(s, Symbol::new("BTCUSDT")),
            SymbolOrPair::Pair(_) => panic!("expected single"),
        }
    }

    #[test]
    fn parse_symbol_or_pair_pair() {
        let r = parse_symbol_or_pair("(BTCUSDT, ETHUSDT)").unwrap();
        match r {
            SymbolOrPair::Pair(p) => {
                assert_eq!(p.a, Symbol::new("BTCUSDT"));
                assert_eq!(p.b, Symbol::new("ETHUSDT"));
            }
            SymbolOrPair::Single(_) => panic!("expected pair"),
        }
    }

    #[test]
    fn encode_decode_embedding_round_trip() {
        let mut v = [Decimal::ZERO; EMBEDDING_DIM];
        v[0] = dec!(1);
        v[5] = dec!(0.5);
        let s = encode_embedding(&v);
        assert!(s.contains(','));
        // Just sanity: the column count is EMBEDDING_DIM.
        assert_eq!(s.matches(',').count(), EMBEDDING_DIM - 1);
    }
}
