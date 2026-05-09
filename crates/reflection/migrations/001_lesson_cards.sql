-- T1805 — reflection-memory schema bootstrap.
--
-- One table holding all lesson cards.  TEXT amounts (Decimal-as-string)
-- match the audit DB convention.  Embedding stored as a packed TEXT
-- column (32 comma-separated `Decimal::to_string()` values) for byte-
-- comparable hex diffs across runs.

CREATE TABLE lesson_cards (
    card_id              TEXT PRIMARY KEY,
    closed_at            TEXT NOT NULL,
    symbol_or_pair       TEXT NOT NULL,
    strategy_id          TEXT NOT NULL,
    signed_pnl_usdt      TEXT NOT NULL,
    opening_capital_usdt TEXT NOT NULL,
    holding_period_bars  INTEGER NOT NULL,
    entry_regime         TEXT NOT NULL,
    exit_regime          TEXT NOT NULL,
    outcome_class        TEXT NOT NULL,
    embedding_blob       TEXT NOT NULL,
    note                 TEXT NULL
);

CREATE INDEX lesson_cards_strategy_idx  ON lesson_cards(strategy_id);
CREATE INDEX lesson_cards_closed_at_idx ON lesson_cards(closed_at);
