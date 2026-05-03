-- T802 — Strategy attribution at the fill level (operator success reports R5.3 / Q2).
--
-- Adds a nullable `strategy_id` column on `journal_transactions` so the
-- audit ledger can attribute each fill to the strategy that emitted the
-- signal.  Pre-migration rows surface as NULL and bucket into the
-- synthetic `(unattributed)` strategy id at query time
-- (`audit::query::pnl_by_strategy`).
--
-- The column is storage-only — it does NOT surface in the rendered
-- backtest report body bytes (V6 anchor gate).  See:
-- `spec/features/operator-success-reports.md` Q2.

ALTER TABLE journal_transactions ADD COLUMN strategy_id TEXT;

CREATE INDEX IF NOT EXISTS journal_transactions_sid_idx
    ON journal_transactions(strategy_id, ts);
